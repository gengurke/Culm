#!/bin/sh
# Dumps the element tree (`--tree`: keys, rects, fills, text colours, text) of every headless scene
# into <out-dir>, so a refactor can prove it left every screen alone: run it on master, then on the
# branch with the first folder as <before-dir>, and it prints the differences.
# Usage: scripts/tree-diff.sh <out-dir> [<before-dir>]
set -eu
out=${1:?usage: scripts/tree-diff.sh <out-dir> [<before-dir>]}
cargo build --quiet
mkdir -p "$out"
root=$(mktemp -d)
trap 'rm -rf "$root"' EXIT
target/debug/culm --list-scenes | OUT=$out ROOT=$root xargs -P 8 -I {} sh -c \
  'RUNE_DIRS_ROOT="$ROOT/$1" target/debug/culm --tree "$OUT/$1.json" --scene "$1" >/dev/null 2>&1 ||
   { echo "FAIL $1" >&2; exit 1; }' _ {}
[ $# -lt 2 ] || diff -r "$2" "$out"
