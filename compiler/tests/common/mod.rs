//! Shared integration-test harness: hermetic temp dirs, golden sidecars and
//! process execution with a timeout.
//!
//! Used by `golden.rs` and `differential.rs`. Separate stdout/stderr
//! collection always drains both pipes on reader threads while the parent
//! polls `try_wait`, so a program writing more than a pipe buffer cannot
//! deadlock. Merged runs redirect both streams to one file opened once and
//! cloned for the two `Stdio` handles, which makes stdout/stderr interleaving
//! observable exactly as a shell's `>FILE 2>&1` would.

#![allow(dead_code)]

use std::ffi::OsStr;
use std::fs::{self, File};
use std::io;
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

/// A spawned process that runs longer than this is killed.
pub const PROCESS_TIMEOUT: Duration = Duration::from_secs(30);

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("compiler/ has a parent")
        .to_path_buf()
}

pub fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

pub fn show_bytes(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// The result of a process run with separate stdout/stderr capture.
pub struct Output {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// The result of a process run with stdout and stderr sent to one file.
pub struct MergedOutput {
    pub status: ExitStatus,
    pub bytes: Vec<u8>,
}

/// Read a golden sidecar such as `.args`, one entry per line. Missing is empty.
pub fn read_lines(path: &Path) -> Result<Vec<String>, String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text.lines().map(str::to_string).collect()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(format!("cannot read {}: {error}", path.display())),
    }
}

/// Read an expected byte sidecar; missing is empty.
pub fn read_expected(path: &Path) -> Result<Vec<u8>, String> {
    match fs::read(path) {
        Ok(bytes) => Ok(bytes),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(format!("cannot read {}: {error}", path.display())),
    }
}

/// Validate a `.files` entry: only plain relative components are allowed.
pub fn checked_relative(file: &str) -> Result<PathBuf, String> {
    if file.is_empty() {
        return Err(".files entry is empty".to_string());
    }
    let path = Path::new(file);
    if path.is_absolute() {
        return Err(format!(".files entry `{file}` is absolute"));
    }
    if !path
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            ".files entry `{file}` must be a plain relative path without `.`/`..`, root or prefix components"
        ));
    }
    Ok(path.to_path_buf())
}

/// A temporary directory removed by [`Drop`], including on test failure.
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Create a unique directory with `fs::create_dir`. An existing name
    /// (EEXIST) simply advances the counter.
    pub fn new() -> io::Result<Self> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        loop {
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!("aleph-test-{}-{id}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

pub fn copy_dir(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create copied directory");
    for entry in fs::read_dir(source).expect("read source directory") {
        let entry = entry.expect("readable directory entry");
        let target = destination.join(entry.file_name());
        let file_type = entry.file_type().expect("read file type");
        if file_type.is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("copy file");
        }
    }
}

/// Prepare a hermetic run directory for `aph`: copy the source and
/// `tests/aph/data/`, and create the parent directories of every output file
/// the case is expected to write.
pub fn prepare_case(temp: &TempDir, aph: &Path, expected_files: &[PathBuf]) -> Result<(), String> {
    let name = file_name(aph);
    fs::copy(aph, temp.path().join(&name))
        .map_err(|error| format!("cannot copy {name}: {error}"))?;
    let data = repo_root().join("tests/aph/data");
    if data.is_dir() {
        copy_dir(&data, &temp.path().join("data"));
    }
    for file in expected_files {
        if let Some(parent) = file.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(temp.path().join(parent))
                    .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
            }
        }
    }
    Ok(())
}

/// Poll `child` until it exits or [`PROCESS_TIMEOUT`] elapses.
fn wait_for(child: &mut Child) -> Result<ExitStatus, String> {
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {
                if start.elapsed() >= PROCESS_TIMEOUT {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "program did not finish within {} s",
                        PROCESS_TIMEOUT.as_secs()
                    ));
                }
                thread::sleep(Duration::from_millis(20));
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("failed to wait for program: {error}"));
            }
        }
    }
}

fn read_thread(
    stream: &'static str,
    handle: thread::JoinHandle<io::Result<Vec<u8>>>,
) -> Result<Vec<u8>, String> {
    handle
        .join()
        .map_err(|_| format!("{stream} reader thread panicked"))?
        .map_err(|error| format!("cannot read child {stream}: {error}"))
}

/// Run `program` with separate stdout/stderr pipes.
///
/// Both pipes are drained on reader threads while the child runs, so output
/// larger than the pipe buffer cannot deadlock the parent.
pub fn run_separate<S: AsRef<OsStr>>(
    directory: &Path,
    program: impl AsRef<OsStr>,
    args: &[S],
) -> Result<Output, String> {
    let mut child = Command::new(program)
        .current_dir(directory)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("failed to spawn program: {error}"))?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| "child stdout was not piped".to_string())?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| "child stderr was not piped".to_string())?;
    let stdout_thread = thread::spawn(move || {
        let mut buffer = Vec::new();
        io::copy(&mut stdout, &mut buffer)?;
        Ok(buffer)
    });
    let stderr_thread = thread::spawn(move || {
        let mut buffer = Vec::new();
        io::copy(&mut stderr, &mut buffer)?;
        Ok(buffer)
    });

    let status = wait_for(&mut child)?;
    let stdout = read_thread("stdout", stdout_thread)?;
    let stderr = read_thread("stderr", stderr_thread)?;
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

/// Run `program` with stdout and stderr redirected to the same file.
///
/// `merged` is truncated first. Both `Stdio` handles are `dup`s of one open
/// file description, so the two streams share the file offset and interleave
/// exactly like `>merged 2>&1`.
pub fn run_merged<S: AsRef<OsStr>>(
    directory: &Path,
    program: impl AsRef<OsStr>,
    args: &[S],
    merged: &Path,
) -> Result<MergedOutput, String> {
    let file = File::create(merged)
        .map_err(|error| format!("cannot create {}: {error}", merged.display()))?;
    let stdout = file
        .try_clone()
        .map_err(|error| format!("cannot clone {}: {error}", merged.display()))?;
    let stderr = file
        .try_clone()
        .map_err(|error| format!("cannot clone {}: {error}", merged.display()))?;

    let mut child = Command::new(program)
        .current_dir(directory)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .map_err(|error| format!("failed to spawn program: {error}"))?;
    drop(file);

    let status = wait_for(&mut child)?;
    let bytes =
        fs::read(merged).map_err(|error| format!("cannot read {}: {error}", merged.display()))?;
    Ok(MergedOutput { status, bytes })
}
