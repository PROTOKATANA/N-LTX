#!/bin/sh
# The checks of .github/workflows/ci.yml, run on this machine. The git hooks in .githooks/ call it:
# `rust` before each commit, `wasm` and `python` before each push.
#
#   scripts/check.sh            every check
#   scripts/check.sh rust       fmt, clippy and the Rust tests
#   scripts/check.sh wasm       the wasm tests, the site and the npm package as users get it
#   scripts/check.sh python     the Python module with pytest
#
# Needs: rustup with the wasm32-unknown-unknown target, wasm-bindgen-cli at the version in Cargo.lock,
# Node 22 and Python 3.12.
set -e
cd "$(dirname "$0")/.."

name=voltik

check_rust() {
  cargo fmt --all --check
  cargo clippy --workspace --exclude "$name-python" --all-targets -- -D warnings
  cargo test --workspace --exclude "$name-python"
}

check_wasm() {
  CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner \
    cargo test -p "$name-wasm" --target wasm32-unknown-unknown
  site/build.sh
  npm/build.sh && npm/test/run.sh
}

check_python() {
  (
    cd crates/python
    [ -d .venv ] || python3 -m venv .venv
    . .venv/bin/activate
    pip install --quiet maturin pytest
    maturin develop --quiet
    python -m pytest --quiet tests
  )
}

[ $# -eq 0 ] && set -- rust wasm python
for check in "$@"; do
  case "$check" in
    rust | wasm | python) echo "== $check"; "check_$check" ;;
    *) echo "unknown check: $check (rust, wasm or python)" >&2; exit 2 ;;
  esac
done
