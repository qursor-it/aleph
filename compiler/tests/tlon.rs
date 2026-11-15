//! Tlön runtime integration test (T-017).
//!
//! Compiles `tlon/tlon.c` with the C test driver `tlon/test_tlon.c`, runs the
//! driver's builtin/trap checks, compares the C `show` of ~2000 f64 values
//! with Rust's `{:?}`, and compares `split` with Rust's `str::split`. If `cc`
//! is missing the test prints a notice and passes, unless ALEPH_REQUIRE_CC=1.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("compiler/ has a parent")
        .to_path_buf()
}

fn temp_dir(name: &str) -> TempDir {
    let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("aleph-tlon-{}-{name}-{id}", std::process::id()));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir(&path).expect("create temporary directory");
    TempDir(path)
}

/// The compiler to probe: `$CC` when set, otherwise `cc`.
///
/// `CC` must be a bare executable path, not a command line with arguments.
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

/// Return true when the caller should skip because `cc` is absent.
///
/// ALEPH_REQUIRE_CC=1 turns the skip into a failure so CI cannot silently lose
/// the C runtime tests.
fn skip_without_cc(label: &str) -> bool {
    if !cc_is_missing() {
        return false;
    }
    if std::env::var("ALEPH_REQUIRE_CC").ok().as_deref() == Some("1") {
        panic!("cc is required by ALEPH_REQUIRE_CC=1 ({label})");
    }
    println!("tlon: cc is not available; skipping {label}");
    true
}

fn build_driver(directory: &Path) -> PathBuf {
    let repo = repo_root();
    let runtime = repo.join("tlon/tlon.c");
    let driver_source = repo.join("tlon/test_tlon.c");
    let driver = directory.join("test_tlon");
    let output = Command::new("nice")
        .args(["-n", "15"])
        .arg(cc_program())
        .args(["-std=c99", "-O1", "-Wall", "-Wextra", "-Werror", "-pthread"])
        .arg(&runtime)
        .arg(&driver_source)
        .arg("-o")
        .arg(&driver)
        .output()
        .expect("failed to run cc");
    assert!(
        output.status.success(),
        "cc failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    driver
}

#[test]
fn tlon_runtime_self_tests_pass() {
    if skip_without_cc("runtime self-tests") {
        return;
    }
    let directory = temp_dir("selftest");
    let driver = build_driver(&directory.0);
    let output = Command::new(&driver)
        .output()
        .expect("failed to run the tlon test driver");
    assert!(
        output.status.success(),
        "tlon test driver failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Edge-case doubles plus deterministic LCG bit patterns.
#[allow(clippy::excessive_precision)]
fn f64_cases() -> Vec<u64> {
    let mut values = vec![
        0.0f64.to_bits(),
        (-0.0f64).to_bits(),
        1.0f64.to_bits(),
        (-1.0f64).to_bits(),
        0.1f64.to_bits(),
        0.5f64.to_bits(),
        1.5f64.to_bits(),
        1e-5f64.to_bits(),
        1e-4f64.to_bits(),
        1e15f64.to_bits(),
        1e16f64.to_bits(),
        1e20f64.to_bits(),
        1e23f64.to_bits(),
        1e100f64.to_bits(),
        5e-324f64.to_bits(),
        f64::MIN_POSITIVE.to_bits(),
        f64::MAX.to_bits(),
        f64::MIN.to_bits(),
        f64::INFINITY.to_bits(),
        f64::NEG_INFINITY.to_bits(),
        f64::NAN.to_bits(),
        (-f64::NAN).to_bits(),
        9.999e-5f64.to_bits(),
        1.234567890123456e15f64.to_bits(),
        123000.0f64.to_bits(),
        137839762462415.625f64.to_bits(),
        (-137839762462415.625f64).to_bits(),
    ];
    let mut state: u64 = 0x1234_5678_9abc_def0;
    while values.len() < 2000 {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        values.push(state);
    }
    values
}

#[test]
fn tlon_f64_show_matches_rust_debug() {
    if skip_without_cc("f64 formatting test") {
        return;
    }
    let directory = temp_dir("f64");
    let driver = build_driver(&directory.0);
    let values = f64_cases();

    let mut child = Command::new(&driver)
        .arg("--format")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn the tlon formatter");
    {
        let stdin = child.stdin.as_mut().expect("piped stdin");
        for bits in &values {
            writeln!(stdin, "{bits:016x}").expect("write f64 bits");
        }
    }
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("wait for the formatter");
    assert!(
        output.status.success(),
        "formatter failed:\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("formatter stdout is UTF-8");
    let actual: Vec<&str> = stdout.lines().collect();
    assert_eq!(actual.len(), values.len(), "one output line per f64 value");

    let mut mismatches = Vec::new();
    for (bits, got) in values.iter().zip(actual.iter()) {
        let expected = format!("{:?}", f64::from_bits(*bits));
        if *got != expected {
            mismatches.push(format!("bits {bits:016x}: rust {expected:?}, tlon {got:?}"));
        }
    }
    assert!(
        mismatches.is_empty(),
        "{} f64 mismatch(es):\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}

fn hex_encode(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return "-".to_string();
    }
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

fn hex_decode(text: &str) -> Vec<u8> {
    if text == "-" {
        return Vec::new();
    }
    let mut bytes = Vec::with_capacity(text.len() / 2);
    let raw = text.as_bytes();
    let mut index = 0;
    while index + 1 < raw.len() {
        let high = (raw[index] as char).to_digit(16).expect("hex high nibble") as u8;
        let low = (raw[index + 1] as char)
            .to_digit(16)
            .expect("hex low nibble") as u8;
        bytes.push((high << 4) | low);
        index += 2;
    }
    bytes
}

const SPLIT_CASES: &[(&str, &str)] = &[
    ("abc", "xy"),
    ("abcab", "abc"),
    ("a--b--", "--"),
    ("", ""),
    ("", ","),
    ("", "xy"),
    ("abc", ""),
    ("", "abc"),
    ("a,b", ","),
    (",a,,b,", ","),
    ("aaa", "aa"),
    ("aaaa", "aa"),
    ("abc", "abc"),
    ("abcabc", "abc"),
    ("h\u{e9}llo", ""),
    ("h\u{e9}llo", "\u{e9}"),
    ("\u{3b1}\u{3b2}\u{3b3}", "\u{3b2}"),
    ("a\tb\nc", " "),
    ("no-sep", "zzz"),
];

#[test]
fn tlon_split_matches_rust_str_split() {
    if skip_without_cc("split test") {
        return;
    }
    let directory = temp_dir("split");
    let driver = build_driver(&directory.0);
    let mut child = Command::new(&driver)
        .arg("--split")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn the tlon splitter");
    {
        let stdin = child.stdin.as_mut().expect("piped stdin");
        for (source, separator) in SPLIT_CASES {
            writeln!(
                stdin,
                "{} {}",
                hex_encode(source.as_bytes()),
                hex_encode(separator.as_bytes())
            )
            .expect("write split case");
        }
    }
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("wait for the splitter");
    assert!(
        output.status.success(),
        "splitter failed:\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("splitter stdout is UTF-8");
    let mut lines = stdout.lines();
    let mut mismatches = Vec::new();
    for (source, separator) in SPLIT_CASES {
        let count_line = lines
            .next()
            .unwrap_or_else(|| panic!("missing part count for {source:?} / {separator:?}"));
        let count: usize = count_line.parse().expect("part count");
        let mut actual = Vec::with_capacity(count);
        for _ in 0..count {
            let line = lines
                .next()
                .unwrap_or_else(|| panic!("missing part for {source:?} / {separator:?}"));
            actual.push(hex_decode(line));
        }
        let expected: Vec<Vec<u8>> = if separator.is_empty() {
            source
                .chars()
                .map(|character| character.to_string().into_bytes())
                .collect()
        } else {
            source
                .split(separator)
                .map(|part| part.as_bytes().to_vec())
                .collect()
        };
        if actual != expected {
            mismatches.push(format!(
                "split {source:?} on {separator:?}: rust {expected:?}, tlon {actual:?}"
            ));
        }
    }
    assert!(lines.next().is_none(), "unexpected extra split output");
    assert!(
        mismatches.is_empty(),
        "{} split mismatch(es):\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
}
