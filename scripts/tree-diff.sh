#!/bin/sh
# Dumps the element tree (`--tree`: keys, rects, fills, text colours, text) of every headless scene
# into <out-dir>, so a refactor can prove it left every screen alone: run it on master, then on the
# branch with the first folder as <before-dir>, and it prints the differences.
# Usage: scripts/tree-diff.sh <out-dir> [<before-dir>]
set -eu
abs() { case $1 in /*) echo "$1" ;; *) echo "$PWD/$1" ;; esac; }
out=$(abs "${1:?usage: scripts/tree-diff.sh <out-dir> [<before-dir>]}")
before=${2:+$(abs "$2")}
cd "$(dirname "$0")/.."
cargo build --quiet
mkdir -p "$out"
root=$(mktemp -d)
trap 'rm -rf "$root"' EXIT
target/debug/culm --list-scenes >"$root/scenes"
[ -s "$root/scenes" ] || { echo "no scenes listed" >&2; exit 1; }
OUT=$out ROOT=$root xargs -P 8 -I {} sh -c \
  'RUNE_DIRS_ROOT="$ROOT/$1" target/debug/culm --tree "$OUT/$1.json" --scene "$1" >/dev/null 2>&1 ||
   { echo "FAIL $1" >&2; exit 1; }' _ {} <"$root/scenes"
[ -z "$before" ] || diff -r "$before" "$out"
