#!/usr/bin/env bash
# Builds a Linux flake check from any host and copies its output to out/<check>.
#
#   scripts/linux.sh <check> [system]      checks: clock-wayland, chrome-probe
#
# On Linux with Nix it runs `nix build` directly; elsewhere it runs Nix inside Docker (nixos/nix)
# with a persistent /nix volume (NOCTALIA_ICED_NIX_VOLUME, default noctalia-iced-nix).
set -euo pipefail

name=${1:?usage: scripts/linux.sh <check> [system]}
system=${2:-aarch64-linux}
root=$(cd "$(dirname "$0")/.." && pwd)
out="$root/out/$name"

if [ "$(uname -s)" = Linux ] && command -v nix >/dev/null 2>&1; then
  nix build "$root#checks.$system.$name" -L --out-link "$root/out/.result-$name"
  rm -rf "$out"
  mkdir -p "$out"
  cp -rL "$root/out/.result-$name/." "$out/"
  chmod -R u+w "$out"
else
  docker run --rm \
    -v "$root":/src \
    -v "${NOCTALIA_ICED_NIX_VOLUME:-noctalia-iced-nix}":/nix \
    -w /src \
    -e CHECK="$name" \
    -e SYSTEM="$system" \
    nixos/nix:latest \
    sh -euc '
      git config --global --add safe.directory "*"
      mkdir -p /etc/nix
      # One derivation at a time: parallel Rust release builds run the Docker VM out of memory.
      printf "experimental-features = nix-command flakes\nsandbox = false\nfilter-syscalls = false\nmax-jobs = 1\ncores = 0\n" \
        > /etc/nix/nix.conf
      nix build "git+file:///src#checks.$SYSTEM.$CHECK" -L --out-link /tmp/result
      out=/src/out/$CHECK
      rm -rf "$out"
      mkdir -p "$out"
      # Docker Desktop bind mounts reject chmod from the container: recreate directories and stream files.
      cd /tmp/result
      find -L . -type d -exec mkdir -p "$out/{}" \;
      find -L . -type f -exec sh -c '"'"'cat "$1" > "$2/$1"'"'"' _ {} "$out" \;
    '
fi

echo "linux.sh: $name -> out/$name" >&2
