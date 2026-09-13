#!/usr/bin/env bash
# Regenerates patches/*.patch by diffing each vendored crate against its pristine crates.io release.
#
#   third_party/rust/regen-patches.sh
#
# Pristine sources come from the .crate files in the cargo registry cache; `cargo fetch` in
# chrome-probe/ downloads them if missing.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
crates=("winit 0.30.13" "iced_core 0.14.0" "iced_winit 0.14.1")
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

(cd "$here/chrome-probe" && cargo fetch --quiet)
mkdir -p "$here/patches"
for entry in "${crates[@]}"; do
  read -r name version <<< "$entry"
  archive=$(ls ~/.cargo/registry/cache/*/"$name-$version.crate" | head -1)
  tar -xzf "$archive" -C "$tmp"
  # diff exits 1 when the trees differ.
  (cd "$tmp" && diff -ruN --exclude target --exclude .cargo_vcs_info.json --exclude .cargo-ok \
    "$name-$version" "$here/$name" | sed "s|$here/$name|$name|; s|^--- $name-$version|--- a/$name|; s|^+++ $name|+++ b/$name|") \
    > "$here/patches/$name.patch" || [ $? -eq 1 ]
  echo "patches/$name.patch: $(grep -c '^+++ ' "$here/patches/$name.patch") files"
done
