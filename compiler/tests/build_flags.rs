//! `aleph build` must not depend on the C compiler's default hardening:
//! at `-O0` without `_FORTIFY_SOURCE`, Tlön once called an undeclared
//! `realpath` (pointer truncated to int) and fs programs crashed.

use std::path::PathBuf;
use std::process::Command;

#[test]
fn fs_program_works_at_o0_without_fortify() {
    let cc = std::env::var_os("CC")
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "cc".into());
    if Command::new(&cc).arg("--version").output().is_err() {
        assert_ne!(
            std::env::var("ALEPH_REQUIRE_CC").ok().as_deref(),
            Some("1"),
            "cc is required by ALEPH_REQUIRE_CC=1"
        );
        return;
    }
    let aph = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/aph");
    let out = std::env::temp_dir().join(format!("aleph-o0-{}", std::process::id()));
    let built = Command::new(env!("CARGO_BIN_EXE_aleph"))
        .current_dir(&aph)
        .env(
            "ALEPH_CFLAGS",
            "-O0 -U_FORTIFY_SOURCE -Wall -Wextra -Werror",
        )
        .args(["build", "bench_linecount.aph", "-o"])
        .arg(&out)
        .output()
        .expect("run aleph build");
    assert!(built.status.success(), "{built:?}");
    let ran = Command::new(&out)
        .current_dir(&aph)
        .output()
        .expect("run program");
    let _ = std::fs::remove_file(&out);
    let expected = std::fs::read(aph.join("bench_linecount.out")).expect("read .out");
    assert_eq!(ran.status.code(), Some(0), "{ran:?}");
    assert_eq!(ran.stdout, expected);
}
