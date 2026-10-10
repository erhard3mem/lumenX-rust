#!/usr/bin/env bash
# Builds LumenX-<arch>.AppImage from this Cargo project.
#
#   ./build-appimage.sh
#
# Note: the AppImage runs on distros with the same or newer glibc as the
# build machine. Build on an older distro (see README, Docker) for portability.
set -euo pipefail
cd "$(dirname "$0")"

ARCH="$(uname -m)"                      # x86_64 or aarch64
APPDIR="build/LumenX.AppDir"
TOOL="build/appimagetool-${ARCH}.AppImage"

# 1) Compile the release binary
cargo build --release
BIN="target/release/lumenx"

# 2) Assemble the AppDir
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" \
         "$APPDIR/usr/share/icons/hicolor/scalable/apps"

install -m755 "$BIN"                  "$APPDIR/usr/bin/lumenx"
install -m755 packaging/AppRun        "$APPDIR/AppRun"
install -m644 packaging/lumenx.desktop "$APPDIR/lumenx.desktop"
install -m644 packaging/lumenx.desktop "$APPDIR/usr/share/applications/lumenx.desktop"
install -m644 packaging/lumenx.svg    "$APPDIR/lumenx.svg"
install -m644 packaging/lumenx.svg    "$APPDIR/usr/share/icons/hicolor/scalable/apps/lumenx.svg"
ln -sf lumenx.svg "$APPDIR/.DirIcon"

# 3) Fetch appimagetool once
if [[ ! -x "$TOOL" ]]; then
    mkdir -p build
    curl -fL -o "$TOOL" \
      "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-${ARCH}.AppImage"
    chmod +x "$TOOL"
fi

# 4) Pack. APPIMAGE_EXTRACT_AND_RUN avoids needing FUSE (e.g. in Docker/CI).
APPIMAGE_EXTRACT_AND_RUN=1 ARCH="$ARCH" "$TOOL" "$APPDIR" "LumenX-${ARCH}.AppImage"

echo
echo "Done: $(pwd)/LumenX-${ARCH}.AppImage"
