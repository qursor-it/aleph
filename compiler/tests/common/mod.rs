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

/// Default deadline for a native build (`aleph build`, and the `cc` it runs).
pub const BUILD_TIMEOUT: Duration = Duration::from_secs(300);

/// Deadline for a native build, from `ALEPH_TEST_BUILD_TIMEOUT` seconds when
/// set to a positive integer, otherwise [`BUILD_TIMEOUT`].
pub fn build_timeout() -> Duration {
    std::env::var("ALEPH_TEST_BUILD_TIMEOUT")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|seconds| *seconds > 0)
        .map(Duration::from_secs)
        .unwrap_or(BUILD_TIMEOUT)
}

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

/// Put the child in its own process group (Unix) so a timeout can kill it and
/// everything it spawned, such as the `cc1` run by `cc`.
fn isolate_process_group(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
}

/// Kill the child and, on Unix, its process group. A plain `Child::kill`
/// leaves grandchildren such as `cc1` running.
fn kill_process_group(child: &mut Child) {
    #[cfg(unix)]
    {
        let _ = Command::new("kill")
            .args(["-s", "KILL", "--", &format!("-{}", child.id())])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

/// Poll `child` until it exits or `timeout` elapses.
fn wait_for(child: &mut Child, timeout: Duration, label: &str) -> Result<ExitStatus, String> {
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {
                let elapsed = start.elapsed();
                if elapsed >= timeout {
                    kill_process_group(child);
                    return Err(format!(
                        "{label} did not finish within {} s (killed after {:.1} s)",
                        timeout.as_secs_f64(),
                        elapsed.as_secs_f64()
                    ));
                }
                thread::sleep(Duration::from_millis(20));
            }
            Err(error) => {
                kill_process_group(child);
                return Err(format!("{label}: failed to wait: {error}"));
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

/// Run `program` with separate stdout/stderr pipes and [`PROCESS_TIMEOUT`].
pub fn run_separate<S: AsRef<OsStr>>(
    directory: &Path,
    program: impl AsRef<OsStr>,
    args: &[S],
) -> Result<Output, String> {
    run_separate_with_timeout(directory, program, args, PROCESS_TIMEOUT, "program")
}

/// Run `program` with separate stdout/stderr pipes and a deadline.
///
/// Both pipes are drained on reader threads while the child runs, so output
/// larger than the pipe buffer cannot deadlock the parent. On timeout the
/// child's process group is killed and the label plus elapsed time is reported.
pub fn run_separate_with_timeout<S: AsRef<OsStr>>(
    directory: &Path,
    program: impl AsRef<OsStr>,
    args: &[S],
    timeout: Duration,
    label: &str,
) -> Result<Output, String> {
    let mut command = Command::new(program);
    command
        .current_dir(directory)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    isolate_process_group(&mut command);

    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to spawn {label}: {error}"))?;

    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| format!("{label}: child stdout was not piped"))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| format!("{label}: child stderr was not piped"))?;
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

    let status = wait_for(&mut child, timeout, label);
    let stdout = read_thread("stdout", stdout_thread)?;
    let stderr = read_thread("stderr", stderr_thread)?;
    Ok(Output {
        status: status?,
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

    let mut command = Command::new(program);
    command
        .current_dir(directory)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    isolate_process_group(&mut command);

    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to spawn program: {error}"))?;
    drop(file);

    let status = wait_for(&mut child, PROCESS_TIMEOUT, "program")?;
    let bytes =
        fs::read(merged).map_err(|error| format!("cannot read {}: {error}", merged.display()))?;
    Ok(MergedOutput { status, bytes })
}
