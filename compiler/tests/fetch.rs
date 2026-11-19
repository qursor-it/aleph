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
use std::thread;

use common::TempDir;

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

fn serve_connection(mut stream: TcpStream) {
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
    let (status, body): (&str, &[u8]) = match path {
        "/ok" => ("200 OK", b"hello"),
        "/bin" => ("200 OK", b"\xff\xfe\x80"),
        _ => ("404 Not Found", b"missing"),
    };
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
}

fn start_server() -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind loopback server");
    let port = listener.local_addr().expect("server address").port();
    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => serve_connection(stream),
                Err(_) => break,
            }
        }
    });
    port
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
    let port = start_server();
    let directory = TempDir::new().expect("temporary directory");
    write_program(directory.path(), port, "127.0.0.1");
    let native = build_native(directory.path());

    let cases = [
        ("/ok", "hello\n", "", 0, "ok body"),
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
