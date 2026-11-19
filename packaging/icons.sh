#!/usr/bin/env bash
# Renders packaging/culm.svg into the committed app icons: crates/app/assets/icon/culm.ico (Windows exe and installer)
# and culm.icns (macOS bundle). Run it after changing the SVG. Needs: inkscape, icotool (icoutils), python3.
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)
dest="$here/../crates/app/assets/icon"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

for size in 16 24 32 48 64 128 256 512 1024; do
    inkscape "$here/culm.svg" -w "$size" -h "$size" -o "$work/$size.png" 2>/dev/null
done

# Small sizes as bitmaps, 128 and 256 as PNG (what Windows expects for the large ones).
icotool -c -o "$dest/culm.ico" "$work"/{16,24,32,48,64}.png -r "$work/128.png" -r "$work/256.png"

# An .icns is a list of (type, length, PNG) entries; iconutil only exists on macOS.
python3 -I - "$work" "$dest/culm.icns" <<'PY'
import struct, sys
work, out = sys.argv[1], sys.argv[2]
types = [("icp4", 16), ("icp5", 32), ("ic11", 32), ("ic12", 64), ("ic07", 128),
         ("ic13", 256), ("ic08", 256), ("ic14", 512), ("ic09", 512), ("ic10", 1024)]
body = b""
for kind, size in types:
    png = open(f"{work}/{size}.png", "rb").read()
    body += kind.encode() + struct.pack(">I", len(png) + 8) + png
open(out, "wb").write(b"icns" + struct.pack(">I", len(body) + 8) + body)
PY
