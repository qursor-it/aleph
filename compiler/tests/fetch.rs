//! `fetch` integration tests (ADR-0038).
//!
//! A loopback HTTP/1.1 server exercises the interpreter and the C backend with
//! the same URL. `fetch` needs `curl` on the machine, so the test skips (with a
//! note) when it is absent. The native half skips without `cc` unless
//! `ALEPH_REQUIRE_CC=1`.

mod common;

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use std::thread;

use common::TempDir;

const STREAM_BYTES: usize = 17 * 1024 * 1024;
const BIG_BYTES: usize = 17_000_000;

fn curl_is_missing() -> bool {
    match Command::new("curl").arg("--version").output() {
        Ok(_) => false,
        Err(error) => error.kind() == std::io::ErrorKind::NotFound,
    }
}

fn cc_program() -> std::ffi::OsString {
    std::env::var_os("CC")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "cc".into())
}

fn cc_is_missing() -> bool {
    match Command::new(cc_program()).arg("--version").output() {
        Ok(_) => false,
        Err(error) => error.kind() == std::io::ErrorKind::NotFound,
    }
}

fn respond(stream: &mut TcpStream, status: &str, body: &[u8]) {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
}

fn write_chunks(stream: &mut TcpStream, total: usize, byte: u8) {
    let chunk = vec![byte; 64 * 1024];
    let mut sent = 0;
    while sent < total {
        let count = (total - sent).min(chunk.len());
        if stream.write_all(&chunk[..count]).is_err() {
            return;
        }
        sent += count;
    }
}

fn serve_connection(mut stream: TcpStream, paths: Arc<Mutex<Vec<String>>>) {
    let mut request = Vec::new();
    let mut buffer = [0u8; 1024];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => return,
            Ok(count) => {
                request.extend_from_slice(&buffer[..count]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            Err(_) => return,
        }
    }
    let text = String::from_utf8_lossy(&request);
    let path = text
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .unwrap_or("/");
    paths.lock().unwrap().push(path.to_string());
    match path {
        "/ok" => respond(&mut stream, "200 OK", b"hello"),
        "/bin" => respond(&mut stream, "200 OK", b"\xff\xfe\x80"),
        "/redirect" => {
            let head =
                "HTTP/1.1 302 Found\r\nLocation: /ok\r\nContent-Length: 5\r\nConnection: close\r\n\r\n";
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(b"moved");
        }
        "/stream" => {
            let head = "HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n";
            let _ = stream.write_all(head.as_bytes());
            write_chunks(&mut stream, STREAM_BYTES, b'x');
        }
        "/big" => {
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {BIG_BYTES}\r\nConnection: close\r\n\r\n"
            );
            let _ = stream.write_all(head.as_bytes());
            write_chunks(&mut stream, BIG_BYTES, b'y');
        }
        _ => respond(&mut stream, "404 Not Found", b"missing"),
    }
}

fn start_server() -> (u16, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind loopback server");
    let port = listener.local_addr().expect("server address").port();
    let paths = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&paths);
    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => serve_connection(stream, Arc::clone(&recorded)),
                Err(_) => break,
            }
        }
    });
    (port, paths)
}

fn aleph(directory: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aleph"))
        .current_dir(directory)
        .args(args)
        .output()
        .expect("run aleph")
}

fn build_native(directory: &Path) -> Option<PathBuf> {
    if cc_is_missing() {
        if std::env::var("ALEPH_REQUIRE_CC").ok().as_deref() == Some("1") {
            panic!("cc is required by ALEPH_REQUIRE_CC=1 (fetch)");
        }
        println!("fetch: cc is not available; skipping the native half");
        return None;
    }
    let output = aleph(directory, &["build", "prog.aph", "-o", "prog"]);
    assert!(
        output.status.success(),
        "aleph build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Some(directory.join("prog"))
}

fn check(label: &str, output: &Output, stdout: &str, stderr: &str, code: i32) {
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        stdout,
        "{label}: stdout\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        stderr,
        "{label}: stderr"
    );
    assert_eq!(output.status.code(), Some(code), "{label}: exit code");
}

fn write_program(directory: &Path, port: u16, host: &str) {
    let source = format!(
        "use net.host \"127.0.0.1\"\nfn main -> res unit str\n let path else at args 0 \"/ok\"\n let url cat \"http://{host}:{port}\" path\n let body try fetch url\n do print body\n ok nil\n"
    );
    fs::write(directory.join("prog.aph"), source).expect("write program");
}

#[test]
fn fetch_runs_identically_in_both_engines() {
    if curl_is_missing() {
        println!("fetch: curl is not available; skipping the fetch integration test");
        return;
    }
    let (port, _paths) = start_server();
    let directory = TempDir::new().expect("temporary directory");
    write_program(directory.path(), port, "127.0.0.1");
    let native = build_native(directory.path());

    let cases = [
        ("/ok", "hello\n", "", 0, "ok body"),
        ("/redirect", "moved\n", "", 0, "3xx body"),
        (
            "/missing",
            "",
            "fetch: The requested URL returned error: 404\n",
            1,
            "HTTP error",
        ),
        (
            "/bin",
            "",
            "fetch: body is not valid UTF-8\n",
            1,
            "invalid UTF-8",
        ),
    ];

    for (path, stdout, stderr, code, label) in cases {
        let output = aleph(directory.path(), &["run", "prog.aph", path]);
        check(label, &output, stdout, stderr, code);
        if let Some(binary) = &native {
            let output = Command::new(binary)
                .current_dir(directory.path())
                .arg(path)
                .output()
                .expect("run native program");
            check(label, &output, stdout, stderr, code);
        }
    }

    // A host that is reachable but not declared must trap before any request.
    write_program(directory.path(), port, "localhost");
    let native_invalid = build_native(directory.path());
    let output = aleph(directory.path(), &["run", "prog.aph", "/ok"]);
    assert_eq!(output.status.code(), Some(70), "R003 exit code");
    assert_eq!(output.stdout, b"");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("\"code\":\"R003\""),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    if let Some(binary) = &native_invalid {
        let output = Command::new(binary)
            .current_dir(directory.path())
            .arg("/ok")
            .output()
            .expect("run native program");
        assert_eq!(output.status.code(), Some(70), "native R003 exit code");
        assert_eq!(output.stdout, b"");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("\"code\":\"R003\""),
            "native stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn fetch_caps_large_bodies_and_does_not_glob_in_both_engines() {
    if curl_is_missing() {
        println!("fetch: curl is not available; skipping the large-body test");
        return;
    }
    let (port, paths) = start_server();
    let directory = TempDir::new().expect("temporary directory");
    write_program(directory.path(), port, "127.0.0.1");
    let native = build_native(directory.path());

    for (path, label) in [("/stream", "streamed body"), ("/big", "declared body")] {
        let output = aleph(directory.path(), &["run", "prog.aph", path]);
        check(label, &output, "", "fetch: response too large\n", 1);
        if let Some(binary) = &native {
            let output = Command::new(binary)
                .current_dir(directory.path())
                .arg(path)
                .output()
                .expect("run native program");
            check(label, &output, "", "fetch: response too large\n", 1);
        }
    }

    // `-g` disables curl globbing: `[1-3]` is requested literally, once.
    let glob = "/[1-3]";
    paths.lock().unwrap().clear();
    let output = aleph(directory.path(), &["run", "prog.aph", glob]);
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "fetch: The requested URL returned error: 404\n",
        "glob: stderr"
    );
    assert_eq!(
        paths.lock().unwrap().clone(),
        vec![glob.to_string()],
        "glob: interpreter requests"
    );
    if let Some(binary) = &native {
        paths.lock().unwrap().clear();
        let output = Command::new(binary)
            .current_dir(directory.path())
            .arg(glob)
            .output()
            .expect("run native program");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "fetch: The requested URL returned error: 404\n",
            "glob native: stderr"
        );
        assert_eq!(
            paths.lock().unwrap().clone(),
            vec![glob.to_string()],
            "glob native: requests"
        );
    }
}

#[test]
fn fetch_reports_missing_curl_in_both_engines() {
    let (port, _paths) = start_server();
    let directory = TempDir::new().expect("temporary directory");
    write_program(directory.path(), port, "127.0.0.1");
    let native = build_native(directory.path());
    let empty = TempDir::new().expect("empty PATH directory");

    let output = Command::new(env!("CARGO_BIN_EXE_aleph"))
        .current_dir(directory.path())
        .env("PATH", empty.path())
        .args(["run", "prog.aph", "/ok"])
        .output()
        .expect("run aleph");
    check(
        "missing curl (interpreter)",
        &output,
        "",
        "fetch: curl not found\n",
        1,
    );
    if let Some(binary) = &native {
        let output = Command::new(binary)
            .current_dir(directory.path())
            .env("PATH", empty.path())
            .arg("/ok")
            .output()
            .expect("run native program");
        check(
            "missing curl (native)",
            &output,
            "",
            "fetch: curl not found\n",
            1,
        );
    }
}
