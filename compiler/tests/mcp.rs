//! T-025: `aleph mcp` speaks newline-delimited JSON-RPC 2.0 over stdio.

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::{json, Value};

/// Send `requests` (one per line) to a fresh server and parse its replies.
fn session(requests: &[Value]) -> Vec<Value> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_aleph"))
        .arg("mcp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start aleph mcp");
    {
        let mut stdin = child.stdin.take().expect("stdin");
        for request in requests {
            writeln!(stdin, "{request}").expect("write request");
        }
    }
    let output = child.wait_with_output().expect("aleph mcp exits");
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    String::from_utf8(output.stdout)
        .expect("utf-8")
        .lines()
        .map(|line| serde_json::from_str(line).expect("reply is JSON"))
        .collect()
}

fn tool_call(id: u64, name: &str, arguments: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":arguments}})
}

/// The JSON object a sandboxed tool returns as its text content.
fn outcome(reply: &Value) -> Value {
    let text = reply["result"]["content"][0]["text"]
        .as_str()
        .expect("text content");
    serde_json::from_str(text).expect("outcome JSON")
}

const CLEAN: &str = "fn inc x:i64 -> i64\n ex inc 1 = 2\n + x 1\n";

#[test]
fn handshake_and_notifications() {
    let replies = session(&[
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        json!({"jsonrpc":"2.0","id":3,"method":"shutdown"}),
    ]);
    // The notification gets no reply.
    assert_eq!(replies.len(), 3);
    assert_eq!(replies[0]["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(replies[1]["id"], 2);
    assert!(replies[1]["result"]["tools"].as_array().unwrap().len() >= 8);
    assert_eq!(replies[2]["result"], json!({}));
}

#[test]
fn check_fmt_edit_and_explain_tools() {
    let replies = session(&[
        tool_call(1, "check", json!({"source": CLEAN})),
        tool_call(
            2,
            "check",
            json!({"source": "fn inc x:i64 -> i64\n ex inc 1 = 3\n + x 1\n", "file": "inc.aph"}),
        ),
        tool_call(3, "fmt", json!({"source": CLEAN})),
        tool_call(
            4,
            "edit",
            json!({"source": CLEAN, "edits": [{"op":"replace","node":"n9","text":"+ x 2"}]}),
        ),
        tool_call(5, "explain", json!({"source": CLEAN, "json": true})),
        tool_call(6, "tokens", json!({"source": CLEAN})),
    ]);

    assert_eq!(replies[0]["result"]["isError"], false);
    assert_eq!(outcome(&replies[0])["exit_code"], 0);

    assert_eq!(replies[1]["result"]["isError"], true);
    let failed = outcome(&replies[1]);
    let diagnostic: Value =
        serde_json::from_str(failed["stdout"].as_str().unwrap().lines().next().unwrap()).unwrap();
    assert_eq!(diagnostic["code"], "E002");
    assert_eq!(diagnostic["file"], "inc.aph");

    assert_eq!(outcome(&replies[2])["stdout"], CLEAN);
    assert_eq!(
        outcome(&replies[3])["stdout"],
        "fn inc x:i64 -> i64\n ex inc 1 = 2\n + x 2\n"
    );
    let explained: Value =
        serde_json::from_str(outcome(&replies[4])["stdout"].as_str().unwrap()).unwrap();
    assert!(explained.is_object());
    let counted: Value =
        serde_json::from_str(outcome(&replies[5])["stdout"].as_str().unwrap()).unwrap();
    assert_eq!(counted["tokens"].as_array().unwrap().len(), 5);
}

#[test]
fn run_is_sandboxed() {
    let hello = "fn main -> unit\n print \"hi\"\n";
    let reads_etc = "use fs.read \"/etc\"\nfn main -> res unit str\n let t try read \"/etc/hostname\"\n ok nil\n";
    // A file next to the server is not visible: cwd is an empty temp dir.
    let local = "use fs.read \"Cargo.toml\"\nfn main -> res unit str\n let t try read \"Cargo.toml\"\n ok nil\n";
    let slow = "fn fib n:i64 -> i64\n ex fib 1 = 1\n if < n 2 n + fib - n 1 fib - n 2\nfn main -> unit\n print show fib 40\n";
    let replies = session(&[
        tool_call(1, "run", json!({"source": hello})),
        tool_call(2, "run", json!({"source": reads_etc})),
        tool_call(3, "run", json!({"source": local})),
        tool_call(4, "run", json!({"source": slow, "timeout_ms": 200})),
    ]);

    assert_eq!(outcome(&replies[0])["stdout"], "hi\n");
    assert_eq!(replies[1]["result"]["isError"], true);
    assert!(replies[1]["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("absolute"));
    let local_reply = outcome(&replies[2]);
    assert_eq!(local_reply["exit_code"], 1, "{local_reply}");
    let timed = outcome(&replies[3]);
    assert_eq!(timed["timed_out"], true);
    assert_eq!(replies[3]["result"]["isError"], true);
}

// The memory limit exists only on 64-bit Linux (ADR-0030).
#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
#[test]
fn run_output_is_capped_and_memory_is_limited() {
    // 2^21 bytes of output, twice the 1 MiB cap.
    let loud = "fn grow s:str n:i64 -> str
 if == n 0 s grow cat s s - n 1
fn main -> unit
 print grow \"ab\" 20
";
    // A 2 GiB string: beyond the 1 GiB address-space limit.
    let greedy = "fn grow s:str n:i64 -> str
 if == n 0 s grow cat s s - n 1
fn main -> unit
 print show count grow \"ab\" 30
";
    let replies = session(&[
        tool_call(1, "run", json!({"source": loud, "timeout_ms": 30000})),
        tool_call(2, "run", json!({"source": greedy, "timeout_ms": 30000})),
    ]);
    let capped = outcome(&replies[0]);
    assert_eq!(capped["truncated"], true);
    assert_eq!(capped["stdout"].as_str().unwrap().len(), 1 << 20);
    let limited = outcome(&replies[1]);
    assert_eq!(limited["timed_out"], false);
    assert_ne!(limited["exit_code"], 0, "{limited}");
    assert_eq!(replies[1]["result"]["isError"], true);
}
