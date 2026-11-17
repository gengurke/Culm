#!/usr/bin/env bash
# Packs a release build into Culm-<version>-x86_64.AppImage plus its .zsync (F8.3).
#
#   packaging/appimage.sh <culm binary> <version> <output dir>
#
# Needs: rsvg-convert (librsvg), zsyncmake (zsync), curl. Downloads appimagetool into the output dir.
set -euo pipefail

bin=$1 version=$2 out=$3
here=$(cd "$(dirname "$0")" && pwd)
arch=x86_64
name="Culm-$version-$arch.AppImage"
# Read by AppImageUpdate, Gear Lever and AppImageLauncher: the newest non-prerelease GitHub release's matching .zsync.
update_info="gh-releases-zsync|gengurke|Culm|latest|Culm-*-$arch.AppImage.zsync"

mkdir -p "$out"
out=$(cd "$out" && pwd)
appdir="$out/Culm.AppDir"
rm -rf "$appdir" "$out/$name" "$out/$name.zsync"

install -Dm755 "$bin" "$appdir/usr/bin/culm"
install -Dm644 "$here/dev.culm.app.desktop" "$appdir/usr/share/applications/dev.culm.app.desktop"
install -Dm644 "$here/dev.culm.app.metainfo.xml" "$appdir/usr/share/metainfo/dev.culm.app.metainfo.xml"
install -Dm644 "$here/culm.svg" "$appdir/usr/share/icons/hicolor/scalable/apps/culm.svg"
for size in 32 48 64 128 256 512; do
    dir="$appdir/usr/share/icons/hicolor/${size}x${size}/apps"
    mkdir -p "$dir"
    rsvg-convert -w "$size" -h "$size" "$here/culm.svg" -o "$dir/culm.png"
done
ln -s usr/bin/culm "$appdir/AppRun"
ln -s usr/share/applications/dev.culm.app.desktop "$appdir/dev.culm.app.desktop"
ln -s usr/share/icons/hicolor/256x256/apps/culm.png "$appdir/culm.png"
ln -s culm.png "$appdir/.DirIcon"

# shortcut: appimagetool only publishes a rolling "continuous" build; pin a copy if it ever breaks a release.
tool="$out/appimagetool-$arch.AppImage"
if [[ ! -x $tool ]]; then
    curl -fsSL -o "$tool" "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-$arch.AppImage"
    chmod +x "$tool"
fi

# Extract-and-run: CI runners and containers have no FUSE. appimagetool writes the .zsync itself when given -u
# and zsyncmake is on PATH.
(cd "$out" && APPIMAGE_EXTRACT_AND_RUN=1 ARCH=$arch "$tool" --no-appstream -u "$update_info" "$appdir" "$name")
[[ -f $out/$name.zsync ]] || zsyncmake -u "$name" -o "$out/$name.zsync" "$out/$name"
rm -rf "$appdir"
echo "$out/$name"
