#!/usr/bin/env bash
# Packs a universal (arm64 + x86_64) release build into Culm-<version>-macos.dmg holding Culm.app (F8.5).
#
#   packaging/macos.sh <arm64 culm> <x86_64 culm> <version> <output dir>
#
# Runs on macOS. Needs: lipo, iconutil, codesign, hdiutil (Xcode command line tools) and rsvg-convert (librsvg).
# The app is signed ad hoc (`codesign -s -`, which Apple Silicon needs to run it at all), not with a Developer ID:
# people allow it once in System Settings → Privacy & Security → "Open Anyway".
set -euo pipefail

arm=$1 intel=$2 version=$3 out=$4
here=$(cd "$(dirname "$0")" && pwd)
# The bundle id is the app id (crates/app/src/brand.rs).
id=$(sed -n 's/^pub const APP_ID: &str = "\(.*\)";/\1/p' "$here/../crates/app/src/brand.rs")
[[ -n $id ]] || { echo "no APP_ID in brand.rs" >&2; exit 1; }
name="Culm-$version-macos.dmg"

mkdir -p "$out"
out=$(cd "$out" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
app="$work/dmg/Culm.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"

lipo -create -output "$app/Contents/MacOS/culm" "$arm" "$intel"
lipo -info "$app/Contents/MacOS/culm"

iconset="$work/culm.iconset"
mkdir "$iconset"
for size in 16 32 128 256 512; do
    rsvg-convert -w "$size" -h "$size" "$here/culm.svg" -o "$iconset/icon_${size}x${size}.png"
    rsvg-convert -w $((size * 2)) -h $((size * 2)) "$here/culm.svg" -o "$iconset/icon_${size}x${size}@2x.png"
done
iconutil -c icns -o "$app/Contents/Resources/culm.icns" "$iconset"

# The oldest macOS the binaries were built for (set by the release workflow).
min_macos=${MACOSX_DEPLOYMENT_TARGET:?set MACOSX_DEPLOYMENT_TARGET to the one the binaries were built with}
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>Culm</string>
    <key>CFBundleDisplayName</key><string>Culm</string>
    <key>CFBundleIdentifier</key><string>$id</string>
    <key>CFBundleExecutable</key><string>culm</string>
    <key>CFBundleIconFile</key><string>culm</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>$version</string>
    <key>CFBundleVersion</key><string>$version</string>
    <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
    <key>LSApplicationCategoryType</key><string>public.app-category.utilities</string>
    <key>LSMinimumSystemVersion</key><string>$min_macos</string>
    <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST
plutil -lint "$app/Contents/Info.plist"

codesign --force --sign - "$app"
codesign --verify --strict --verbose=2 "$app"

ln -s /Applications "$work/dmg/Applications"
# hdiutil now and then fails with "Resource busy" on CI runners: try again.
for try in 1 2 3; do
    hdiutil create -volname "Culm $version" -srcfolder "$work/dmg" -fs HFS+ -format UDZO -ov "$out/$name" && break
    [[ $try == 3 ]] && exit 1
    sleep 5
done
echo "$out/$name"
