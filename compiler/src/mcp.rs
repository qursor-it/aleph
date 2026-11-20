//! MCP server for `aleph mcp` (ADR-0030, design doc §5.5).
//!
//! JSON-RPC 2.0 over stdio, one JSON message per line, written by hand with
//! `serde_json`. Tools take the program as inline `source`; each call writes
//! it into a fresh temporary directory and runs this same `aleph` binary
//! there with an empty environment, a timeout and capped output.

use std::fs;
use std::io::{self, BufRead, Read, Write};
use std::os::unix::fs::DirBuilderExt;
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
use std::os::unix::process::CommandExt;
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::{llms, parser};

/// The only protocol revision this server speaks; answered whatever the
/// client asks for, so the client can decide whether to continue.
const PROTOCOL_VERSION: &str = "2025-06-18";
/// Per-stream output cap for tool subprocesses.
const OUTPUT_CAP: usize = 1 << 20;
const DEFAULT_TIMEOUT_MS: u64 = 5_000;
const MAX_TIMEOUT_MS: u64 = 60_000;
const DEFAULT_FILE: &str = "main.aph";

type RpcError = (i64, String);

/// Serve requests from `input` until end of input.
pub fn serve(mut input: impl BufRead, mut output: impl Write) -> io::Result<()> {
    let mut raw = Vec::new();
    loop {
        raw.clear();
        if input.read_until(b'\n', &mut raw)? == 0 {
            return Ok(());
        }
        // A line that is not UTF-8 is a parse error, not the end of the session.
        let response = match std::str::from_utf8(&raw) {
            Ok(line) if line.trim().is_empty() => continue,
            Ok(line) => handle_line(line),
            Err(_) => Some(error_response(Value::Null, -32700, "parse error")),
        };
        if let Some(response) = response {
            writeln!(output, "{response}")?;
            output.flush()?;
        }
    }
}

/// The response to one input line, or `None` for notifications.
pub fn handle_line(line: &str) -> Option<Value> {
    let Ok(message) = serde_json::from_str::<Value>(line) else {
        return Some(error_response(Value::Null, -32700, "parse error"));
    };
    let Some(object) = message.as_object() else {
        return Some(error_response(Value::Null, -32600, "invalid request"));
    };
    let id = object.get("id").cloned();
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0")
        || id
            .as_ref()
            .is_some_and(|id| !(id.is_string() || id.is_number()))
    {
        // MCP ids are strings or numbers, never null.
        let id = id.filter(|id| id.is_string() || id.is_number());
        return Some(error_response(
            id.unwrap_or(Value::Null),
            -32600,
            "invalid request",
        ));
    }
    let method = object.get("method").and_then(Value::as_str);
    let (Some(id), Some(method)) = (id.clone(), method) else {
        // Notifications (no id) and responses (`result`/`error`) get no
        // answer; any other id without a method is an invalid request.
        let response = object.contains_key("result") || object.contains_key("error");
        return match (id, method) {
            (Some(id), None) if !response => Some(error_response(id, -32600, "invalid request")),
            _ => None,
        };
    };
    let params = object.get("params").cloned().unwrap_or_else(|| json!({}));
    Some(match dispatch(method, &params) {
        Ok(result) => json!({"jsonrpc": "2.0", "id": id, "result": result}),
        Err((code, message)) => error_response(id, code, &message),
    })
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn dispatch(method: &str, params: &Value) -> Result<Value, RpcError> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {"tools": {}, "resources": {}},
            "serverInfo": {"name": "aleph", "version": env!("CARGO_PKG_VERSION")},
        })),
        "ping" | "shutdown" => Ok(json!({})),
        "tools/list" => Ok(json!({ "tools": tools() })),
        "tools/call" => call_tool(params),
        "resources/list" => Ok(json!({ "resources": resources() })),
        "resources/read" => read_resource(params),
        _ => Err((-32601, format!("method not found: {method}"))),
    }
}

fn source_schema(extra: Value) -> Value {
    let mut properties = json!({
        "source": {"type": "string", "description": "Aleph program text"},
        "file": {"type": "string", "description": "file name used in diagnostics (default main.aph)"},
        "timeout_ms": {"type": "integer", "description": "default 5000, max 60000"},
    });
    if let (Some(map), Some(more)) = (properties.as_object_mut(), extra.as_object()) {
        map.extend(more.clone());
    }
    json!({"type": "object", "properties": properties, "required": ["source"]})
}

fn tools() -> Vec<Value> {
    let tool = |name: &str, description: &str, schema: Value| json!({"name": name, "description": description, "inputSchema": schema});
    vec![
        tool(
            "spec",
            "Aleph language spec: compact (llms.txt) or full (SPEC.md)",
            json!({"type": "object", "properties": {
                "detail": {"type": "string", "enum": ["compact", "full"]}
            }}),
        ),
        tool(
            "check",
            "aleph check: parse, canonical form (F001 warning; strict: error), types, capabilities, examples; JSON diagnostics",
            source_schema(json!({"strict": {"type": "boolean", "description": "F001 is an error and stops checking (default false)"}})),
        ),
        tool(
            "test",
            "aleph test: run the inline examples",
            source_schema(json!({})),
        ),
        tool("fmt", "aleph fmt: canonical text", source_schema(json!({}))),
        tool(
            "ast",
            "aleph ast: AST as JSON with node ids",
            source_schema(json!({})),
        ),
        tool(
            "explain",
            "aleph explain: capabilities, types, signatures, docs, examples, call graph",
            source_schema(json!({"json": {"type": "boolean"}})),
        ),
        tool(
            "edit",
            "aleph edit: node-level edits (ADR-0029); returns the edited canonical text",
            source_schema(json!({"edits": {
                "description": "array of {op,node,text?,position?} (or its JSON text)"
            }})),
        ),
        tool(
            "tokens",
            "aleph tokens: token counts (chars4, bytes, chars, words, aleph lexer)",
            source_schema(
                json!({"tokenizer": {"type": "string", "description": "name or all (default all)"}}),
            ),
        ),
        tool(
            "run",
            "aleph run in a sandbox: empty temp dir as cwd, empty environment, timeout, \
             output capped at 1 MiB; absolute capability prefixes are rejected",
            source_schema(json!({"args": {"type": "array", "items": {"type": "string"}}})),
        ),
    ]
}

fn invalid_params(message: impl Into<String>) -> RpcError {
    (-32602, message.into())
}

/// A tool result: one text item, flagged as an error when the tool failed.
fn tool_result(text: String, is_error: bool) -> Value {
    json!({"content": [{"type": "text", "text": text}], "isError": is_error})
}

fn call_tool(params: &Value) -> Result<Value, RpcError> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_params("missing tool name"))?;
    let empty = json!({});
    let args = params.get("arguments").unwrap_or(&empty);

    if name == "spec" {
        return match args.get("detail").and_then(Value::as_str) {
            None | Some("compact") => Ok(tool_result(llms::render(), false)),
            Some("full") => Ok(tool_result(llms::SPEC.to_string(), false)),
            Some(other) => Err(invalid_params(format!("unknown detail: {other}"))),
        };
    }

    let extra: Vec<String> = match name {
        "check" => match args.get("strict") {
            None | Some(Value::Bool(false)) => Vec::new(),
            Some(Value::Bool(true)) => vec!["--strict".to_string()],
            Some(_) => return Err(invalid_params("strict must be a boolean")),
        },
        "test" | "fmt" | "ast" => Vec::new(),
        "explain" => match args.get("json").and_then(Value::as_bool) {
            Some(true) => vec!["--json".to_string()],
            _ => Vec::new(),
        },
        "edit" => {
            let edits = match args.get("edits") {
                Some(Value::String(text)) => text.clone(),
                Some(value @ (Value::Array(_) | Value::Object(_))) => value.to_string(),
                _ => return Err(invalid_params("edit needs edits")),
            };
            vec!["--json".to_string(), edits]
        }
        "tokens" => {
            let tokenizer = args
                .get("tokenizer")
                .and_then(Value::as_str)
                .unwrap_or("all");
            ["--json", "--tokenizer", tokenizer]
                .map(str::to_string)
                .to_vec()
        }
        "run" => string_list(args.get("args"))?,
        other => return Err(invalid_params(format!("unknown tool: {other}"))),
    };
    let source = args
        .get("source")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_params("missing source"))?;
    let file = file_label(args.get("file"))?;
    let timeout = match args.get("timeout_ms") {
        None => DEFAULT_TIMEOUT_MS,
        Some(value) => value
            .as_u64()
            .ok_or_else(|| invalid_params("timeout_ms must be a non-negative integer"))?,
    }
    .clamp(1, MAX_TIMEOUT_MS);

    if name == "run" && has_absolute_prefix(source) {
        return Err(invalid_params(
            "run rejects absolute capability prefixes in the sandbox",
        ));
    }

    if extra.iter().map(String::len).sum::<usize>() > MAX_ARG_BYTES {
        return Err(invalid_params(format!(
            "arguments longer than {MAX_ARG_BYTES} bytes"
        )));
    }
    if extra.iter().any(|arg| arg.contains('\0')) {
        return Err(invalid_params("argument contains a NUL byte"));
    }
    let outcome = run_sandboxed(name, &file, source, &extra, timeout)
        .map_err(|err| (-32603, format!("internal error: {err}")))?;
    let failed = outcome.timed_out || outcome.exit_code != Some(0);
    let text = json!({
        "exit_code": outcome.exit_code,
        "signal": outcome.signal,
        "timed_out": outcome.timed_out,
        "truncated": outcome.truncated,
        "stdout": outcome.stdout,
        "stderr": outcome.stderr,
    });
    Ok(tool_result(text.to_string(), failed))
}

fn string_list(value: Option<&Value>) -> Result<Vec<String>, RpcError> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    value
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .map(|item| item.as_str().map(str::to_string))
                .collect::<Option<Vec<_>>>()
        })
        .ok_or_else(|| invalid_params("args must be an array of strings"))
}

/// A plain `.aph` file name: it becomes a path inside the temp directory.
fn file_label(value: Option<&Value>) -> Result<String, RpcError> {
    let Some(value) = value else {
        return Ok(DEFAULT_FILE.to_string());
    };
    let label = value
        .as_str()
        .ok_or_else(|| invalid_params("file must be a string"))?;
    let valid = label.len() <= 64
        && label.ends_with(".aph")
        && !label.starts_with('.')
        && !label.starts_with('-')
        && label
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.');
    if valid {
        Ok(label.to_string())
    } else {
        Err(invalid_params("file must be a plain NAME.aph"))
    }
}

/// Whether a parsable `source` declares an absolute filesystem prefix.
fn has_absolute_prefix(source: &str) -> bool {
    parser::parse(DEFAULT_FILE, source)
        .file
        .is_some_and(|file| {
            file.uses.iter().any(|item| {
                item.path
                    .as_deref()
                    .is_some_and(|path| path.starts_with('/'))
            })
        })
}

fn resources() -> Vec<Value> {
    RESOURCES
        .iter()
        .map(|(uri, mime)| json!({"uri": uri, "name": uri.trim_start_matches("aleph://"), "mimeType": mime}))
        .collect()
}

const RESOURCES: &[(&str, &str)] = &[
    ("aleph://llms.txt", "text/plain"),
    ("aleph://spec/SPEC.md", "text/markdown"),
    ("aleph://spec/grammar.ebnf", "text/plain"),
    ("aleph://spec/diagnostics.md", "text/markdown"),
];

fn read_resource(params: &Value) -> Result<Value, RpcError> {
    let uri = params
        .get("uri")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_params("missing uri"))?;
    let text = match uri {
        "aleph://llms.txt" => llms::render(),
        "aleph://spec/SPEC.md" => llms::SPEC.to_string(),
        "aleph://spec/grammar.ebnf" => llms::GRAMMAR.to_string(),
        "aleph://spec/diagnostics.md" => llms::DIAGNOSTICS.to_string(),
        _ => return Err((-32002, format!("resource not found: {uri}"))),
    };
    let mime = RESOURCES
        .iter()
        .find(|(known, _)| *known == uri)
        .map_or("text/plain", |(_, mime)| mime);
    Ok(json!({"contents": [{"uri": uri, "mimeType": mime, "text": text}]}))
}

struct Outcome {
    exit_code: Option<i32>,
    /// The signal that killed the child (e.g. SIGXFSZ past the file limit).
    signal: Option<i32>,
    timed_out: bool,
    truncated: bool,
    stdout: String,
    stderr: String,
}

/// A temporary directory removed on drop.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> io::Result<Self> {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        loop {
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("aleph-mcp-{}-{n}", std::process::id()));
            // Private to this user: other local users must not read sources.
            match fs::DirBuilder::new().mode(0o700).create(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(err) => return Err(err),
            }
        }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Address-space limit for tool subprocesses (the interpreter reserves a
/// 256 MiB stack).
const MEMORY_LIMIT: u64 = 1 << 30;
/// Largest file a tool subprocess may write (`fs.write` in its temp dir).
const FILE_LIMIT: u64 = 64 << 20;
/// Cap on all tool arguments together, below Linux's 128 KiB per-argument
/// limit and far below the total argument limit.
const MAX_ARG_BYTES: usize = 100 * 1024;

/// Run `aleph COMMAND FILE EXTRA..` on `source` in a fresh temp directory.
// shortcut: no disk quota for fs.write inside the temp dir, and the temp dir
// is left behind if the server is killed mid-call; add both if the server is
// ever exposed to untrusted clients.
fn run_sandboxed(
    tool: &str,
    file: &str,
    source: &str,
    extra: &[String],
    timeout_ms: u64,
) -> io::Result<Outcome> {
    let dir = TempDir::new()?;
    fs::write(dir.0.join(file), source)?;
    // `/proc/self/exe` is this very binary even after cargo replaced the file
    // on disk; `current_exe()` would then name a deleted path.
    #[cfg(target_os = "linux")]
    let exe = PathBuf::from("/proc/self/exe");
    #[cfg(not(target_os = "linux"))]
    let exe = std::env::current_exe()?;
    let mut command = Command::new(exe);
    command
        .arg(tool)
        .arg(file)
        .args(extra)
        .current_dir(&dir.0)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // SAFETY: the closure runs in the forked child before exec and only makes
    // two async-signal-safe system calls.
    #[cfg(all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    ))]
    unsafe {
        command.pre_exec(limit_child);
    }
    let mut child = command.spawn()?;

    let stdout = child
        .stdout
        .take()
        .map(|pipe| thread::spawn(|| read_capped(pipe)));
    let stderr = child
        .stderr
        .take()
        .map(|pipe| thread::spawn(|| read_capped(pipe)));

    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    let mut timed_out = false;
    let status = loop {
        let polled = child.try_wait();
        if polled.is_err() {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(status) = polled? {
            break Some(status);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            timed_out = true;
            break None;
        }
        thread::sleep(Duration::from_millis(5));
    };

    let join = |handle: Option<thread::JoinHandle<(Vec<u8>, bool)>>| {
        handle
            .and_then(|handle| handle.join().ok())
            .unwrap_or_default()
    };
    let (out, out_cut) = join(stdout);
    let (err, err_cut) = join(stderr);
    Ok(Outcome {
        exit_code: status.and_then(|status| status.code()),
        signal: status.and_then(|status| status.signal()),
        timed_out,
        truncated: out_cut || err_cut,
        stdout: String::from_utf8_lossy(&out).into_owned(),
        stderr: String::from_utf8_lossy(&err).into_owned(),
    })
}

#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
#[repr(C)]
struct RLimit {
    current: u64,
    max: u64,
}

#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
extern "C" {
    fn prctl(option: i32, ...) -> i32;
    fn setrlimit(resource: i32, limit: *const RLimit) -> i32;
}

/// In the child: die with the server (no orphan outlives a killed server),
/// cap memory and file size so one call cannot exhaust the machine. The
/// constants and `RLimit` layout hold for 64-bit Linux on x86_64/aarch64.
// shortcut: a server killed between fork and prctl leaves an orphan without
// timeout; check getppid() after prctl if that window ever matters.
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
fn limit_child() -> io::Result<()> {
    const PR_SET_PDEATHSIG: i32 = 1;
    const SIGKILL: u64 = 9;
    const RLIMIT_FSIZE: i32 = 1;
    const RLIMIT_AS: i32 = 9;
    let limit = |bytes| RLimit {
        current: bytes,
        max: bytes,
    };
    // SAFETY: plain system calls with valid arguments.
    let failed = unsafe {
        prctl(PR_SET_PDEATHSIG, SIGKILL) != 0
            || setrlimit(RLIMIT_AS, &limit(MEMORY_LIMIT)) != 0
            || setrlimit(RLIMIT_FSIZE, &limit(FILE_LIMIT)) != 0
    };
    if failed {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Read a stream to its end, keeping at most [`OUTPUT_CAP`] bytes.
fn read_capped(mut stream: impl Read) -> (Vec<u8>, bool) {
    let mut kept = Vec::new();
    let mut truncated = false;
    let mut chunk = [0u8; 8192];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                let room = OUTPUT_CAP - kept.len();
                if n > room {
                    truncated = true;
                }
                kept.extend_from_slice(&chunk[..n.min(room)]);
            }
        }
    }
    (kept, truncated)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(request: Value) -> Value {
        handle_line(&request.to_string()).expect("a response")
    }

    #[test]
    fn initialize_ping_and_notifications() {
        let response = call(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}));
        assert_eq!(response["id"], 1);
        assert_eq!(response["result"]["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(response["result"]["serverInfo"]["name"], "aleph");
        let response = call(json!({"jsonrpc":"2.0","id":"a","method":"ping"}));
        assert_eq!(response["result"], json!({}));
        assert!(handle_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).is_none());
    }

    #[test]
    fn invalid_utf8_lines_do_not_end_the_session() {
        let input: &[u8] = b"\xff\xfe\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n";
        let mut output = Vec::new();
        serve(input, &mut output).unwrap();
        let replies: Vec<Value> = String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(replies.len(), 2);
        assert_eq!(replies[0]["error"]["code"], -32700);
        assert_eq!(replies[1]["result"], json!({}));
    }

    #[test]
    fn protocol_errors() {
        assert_eq!(handle_line("{").unwrap()["error"]["code"], -32700);
        assert_eq!(handle_line("[1]").unwrap()["error"]["code"], -32600);
        assert_eq!(
            handle_line(r#"{"jsonrpc":"2.0","id":3}"#).unwrap()["error"]["code"],
            -32600
        );
        // Missing jsonrpc, null id and object id are invalid requests.
        for bad in [
            r#"{"id":3,"method":"ping"}"#,
            r#"{"jsonrpc":"2.0","id":null,"method":"ping"}"#,
            r#"{"jsonrpc":"2.0","id":{"a":1},"method":"ping"}"#,
        ] {
            let response = handle_line(bad).unwrap();
            assert_eq!(response["error"]["code"], -32600, "{bad}");
        }
        // The server answers only the revision it speaks.
        let response = call(json!({"jsonrpc":"2.0","id":1,"method":"initialize",
            "params":{"protocolVersion":"bogus"}}));
        assert_eq!(response["result"]["protocolVersion"], PROTOCOL_VERSION);
        let response = call(json!({"jsonrpc":"2.0","id":2,"method":"tools/call",
            "params":{"name":"edit","arguments":{"source":"","edits":"x".repeat(MAX_ARG_BYTES + 1)}}}));
        assert_eq!(response["error"]["code"], -32602);
        let response = call(json!({"jsonrpc":"2.0","id":2,"method":"tools/call",
            "params":{"name":"run","arguments":{"source":"","args":["a\u{0}b"]}}}));
        assert_eq!(response["error"]["code"], -32602);
        let many: Vec<String> = (0..40).map(|_| "x".repeat(10_000)).collect();
        let response = call(json!({"jsonrpc":"2.0","id":2,"method":"tools/call",
            "params":{"name":"run","arguments":{"source":"","args":many}}}));
        assert_eq!(response["error"]["code"], -32602);
        let response = call(json!({"jsonrpc":"2.0","id":2,"method":"tools/call",
            "params":{"name":"check","arguments":{"source":"","timeout_ms":"10"}}}));
        assert_eq!(response["error"]["code"], -32602);
        // A response from the client is not answered.
        assert!(handle_line(r#"{"jsonrpc":"2.0","id":6,"result":{}}"#).is_none());
        let response = call(json!({"jsonrpc":"2.0","id":2,"method":"nope"}));
        assert_eq!(response["error"]["code"], -32601);
        let response =
            call(json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"x"}}));
        assert_eq!(response["error"]["code"], -32602);
        let response = call(json!({"jsonrpc":"2.0","id":2,"method":"tools/call",
            "params":{"name":"check","arguments":{}}}));
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn lists_tools_and_reads_resources() {
        let response = call(json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}));
        let names: Vec<&str> = response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            ["spec", "check", "test", "fmt", "ast", "explain", "edit", "tokens", "run"]
        );

        let response = call(json!({"jsonrpc":"2.0","id":1,"method":"resources/read",
            "params":{"uri":"aleph://llms.txt"}}));
        assert_eq!(response["result"]["contents"][0]["text"], llms::render());
        let response = call(json!({"jsonrpc":"2.0","id":1,"method":"resources/read",
            "params":{"uri":"aleph://nope"}}));
        assert_eq!(response["error"]["code"], -32002);
    }

    #[test]
    fn spec_tool_returns_compact_or_full_text() {
        let response = call(json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"spec","arguments":{"detail":"full"}}}));
        assert_eq!(response["result"]["content"][0]["text"], llms::SPEC);
        assert_eq!(response["result"]["isError"], false);
    }

    #[test]
    fn file_labels_are_plain_names() {
        assert_eq!(file_label(None).unwrap(), "main.aph");
        assert_eq!(file_label(Some(&json!("a-b_1.aph"))).unwrap(), "a-b_1.aph");
        for bad in [
            "../x.aph", "/x.aph", ".x.aph", "x.txt", "a/b.aph", "", "-w.aph",
        ] {
            assert!(file_label(Some(&json!(bad))).is_err(), "{bad}");
        }
    }

    #[test]
    fn absolute_prefixes_are_detected() {
        assert!(has_absolute_prefix(
            "use fs.read \"//etc\"\nfn main -> unit\n nil\n"
        ));
        assert!(has_absolute_prefix(
            "use fs.read \"/etc\"\nfn main -> unit\n nil\n"
        ));
        assert!(!has_absolute_prefix(
            "use fs.read \"data\"\nfn main -> unit\n nil\n"
        ));
    }

    #[test]
    fn read_capped_truncates() {
        let data = vec![b'x'; OUTPUT_CAP + 10];
        let (kept, truncated) = read_capped(&data[..]);
        assert_eq!(kept.len(), OUTPUT_CAP);
        assert!(truncated);
    }
}
