#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

export ALEPH_REQUIRE_CC=1

cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all
