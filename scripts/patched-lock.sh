#!/usr/bin/env bash
# Regenerates third_party/rust/Cargo.patched.lock: Cargo.lock with the patched crates applied, for
# the Nix `wayland-chrome` build. Run after changing dependencies or the vendored crates.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
saved=$(mktemp)
cp Cargo.lock "$saved"
trap 'cp "$saved" Cargo.lock; rm -f "$saved"' EXIT

cargo --config third_party/rust/patch.toml metadata --format-version 1 >/dev/null
cp Cargo.lock third_party/rust/Cargo.patched.lock
echo "third_party/rust/Cargo.patched.lock updated" >&2
