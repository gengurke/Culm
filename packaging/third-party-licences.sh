#!/usr/bin/env bash
# Writes THIRD-PARTY-LICENSES.md: every Rust crate's licence (cargo-about, about.toml) plus the bundled fonts' OFL
# texts. CI fails when the committed file is out of date: run this after changing dependencies or bumping Rune.
set -euo pipefail
cd "$(dirname "$0")/.."
out=THIRD-PARTY-LICENSES.md
# --frozen: licence texts come only from the crates themselves (no web lookups), the same on every machine.
cargo fetch --locked
cargo about generate --frozen --fail packaging/about.hbs -o "$out"
printf '## Fonts\n\n' >> "$out"
for f in crates/app/assets/fonts/*/OFL.txt; do
    printf '### %s\n\n~~~~text\n' "$(basename "$(dirname "$f")")" >> "$out"
    cat "$f" >> "$out"
    printf '~~~~\n\n' >> "$out"
done
