#!/usr/bin/env bash
set -euo pipefail

APP_ID="tech.soventa.strata"
BIN_PATH="${1:-dist/strata-linux}"
OUT_BUNDLE="${2:-dist/strata-linux-x86_64.flatpak}"
ARCH="${3:-x86_64}"

if [ ! -f "$BIN_PATH" ]; then
    echo "Error: Linux binary not found at $BIN_PATH" >&2
    exit 1
fi

WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

BUILD_DIR="$WORK_DIR/build"
REPO_DIR="$WORK_DIR/repo"

mkdir -p "$BUILD_DIR/files/bin"
mkdir -p "$BUILD_DIR/files/share/applications"
mkdir -p "$BUILD_DIR/files/share/metainfo"
mkdir -p "$BUILD_DIR/files/share/icons/hicolor/scalable/apps"
mkdir -p "$BUILD_DIR/export/share/applications"
mkdir -p "$BUILD_DIR/export/share/icons/hicolor/scalable/apps"

cat > "$BUILD_DIR/metadata" <<EOF
[Application]
name=$APP_ID
runtime=org.freedesktop.Platform/$ARCH/24.08
sdk=org.freedesktop.Sdk/$ARCH/24.08
command=strata

[Context]
shared=network;ipc;
sockets=x11;wayland;fallback-x11;
devices=dri;
filesystems=host;

[Session Bus Policy]
org.freedesktop.Flatpak=talk
org.freedesktop.portal.Desktop=talk
EOF

install -Dm755 "$BIN_PATH" "$BUILD_DIR/files/bin/strata"
install -Dm644 packaging/flatpak/tech.soventa.strata.desktop "$BUILD_DIR/files/share/applications/$APP_ID.desktop"
install -Dm644 packaging/flatpak/tech.soventa.strata.desktop "$BUILD_DIR/export/share/applications/$APP_ID.desktop"
install -Dm644 packaging/flatpak/tech.soventa.strata.metainfo.xml "$BUILD_DIR/files/share/metainfo/$APP_ID.metainfo.xml"
install -Dm644 assets/icon.svg "$BUILD_DIR/files/share/icons/hicolor/scalable/apps/$APP_ID.svg"
install -Dm644 assets/icon.svg "$BUILD_DIR/export/share/icons/hicolor/scalable/apps/$APP_ID.svg"

for sz in 16 24 32 48 64 128 256 512; do
    if [ -f "assets/icon-${sz}.png" ]; then
        install -Dm644 "assets/icon-${sz}.png" "$BUILD_DIR/files/share/icons/hicolor/${sz}x${sz}/apps/$APP_ID.png"
        install -Dm644 "assets/icon-${sz}.png" "$BUILD_DIR/export/share/icons/hicolor/${sz}x${sz}/apps/$APP_ID.png"
    fi
done

mkdir -p "$(dirname "$OUT_BUNDLE")"

if command -v flatpak >/dev/null 2>&1 && command -v ostree >/dev/null 2>&1; then
    ostree init --mode=archive-z2 --repo="$REPO_DIR"
    flatpak build-export --arch="$ARCH" "$REPO_DIR" "$BUILD_DIR" stable
    flatpak build-bundle --arch="$ARCH" \
        --repo-url="https://git.soventa.tech/moha/strata/releases" \
        "$REPO_DIR" "$OUT_BUNDLE" "$APP_ID" stable
    echo "Built Flatpak bundle: $OUT_BUNDLE"
else
    echo "flatpak/ostree not installed; creating portable flatpak archive bundle at $OUT_BUNDLE" >&2
    tar -C "$BUILD_DIR" -czf "$OUT_BUNDLE" metadata files export
fi
