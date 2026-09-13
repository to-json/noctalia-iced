#!/usr/bin/env bash
# Runs cargo with the patched windowing crates (third_party/rust/patch.toml) and puts Cargo.lock
# back afterwards, so the committed lockfile stays the crates.io one.
#
#   scripts/with-patches.sh run -p noctalia-clock-iced --features wayland-chrome
#   scripts/with-patches.sh test --workspace --features noctalia-iced/wayland-chrome
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"
saved=$(mktemp)
cp Cargo.lock "$saved"
trap 'cp "$saved" Cargo.lock; rm -f "$saved"' EXIT

cargo --config third_party/rust/patch.toml "$@"
