#!/bin/bash
# scripts/create-macos-pkg.sh
# Usage: ./create-macos-pkg.sh <version> <arch>
# Example: ./create-macos-pkg.sh 0.1.2 aarch64

set -euo pipefail

VERSION=${1:-}
ARCH=${2:-}

if [ -z "$VERSION" ] || [ -z "$ARCH" ]; then
    echo "Usage: $0 <version> <arch>"
    echo "  arch: aarch64 or x64"
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
APP_NAME="Pisum Langue"
PKG_PREFIX="Pisum.Langue"
BUNDLE_ID="com.pisumlangue.app"

# Determine source .app location based on architecture
if [ "$ARCH" == "aarch64" ]; then
    APP_PATH="$PROJECT_ROOT/src-tauri/target/aarch64-apple-darwin/release/bundle/macos/${APP_NAME}.app"
else
    APP_PATH="$PROJECT_ROOT/src-tauri/target/x86_64-apple-darwin/release/bundle/macos/${APP_NAME}.app"
fi

if [ ! -d "$APP_PATH" ]; then
    echo "Error: App bundle not found at $APP_PATH"
    exit 1
fi

# Create temporary directory for package contents
PKG_ROOT=$(mktemp -d)
OUTPUT_DIR="$PROJECT_ROOT/dist"
mkdir -p "$OUTPUT_DIR"

# Copy .app to package root
mkdir -p "$PKG_ROOT/Applications"
cp -R "$APP_PATH" "$PKG_ROOT/Applications/"

# Build the component package
COMPONENT_PKG="$OUTPUT_DIR/${PKG_PREFIX}_${VERSION}_${ARCH}_component.pkg"
pkgbuild \
    --root "$PKG_ROOT" \
    --identifier "$BUNDLE_ID" \
    --version "$VERSION" \
    --install-location "/" \
    --scripts "$SCRIPT_DIR" \
    "$COMPONENT_PKG"

# Build the final distribution package (product archive)
FINAL_PKG="$OUTPUT_DIR/${PKG_PREFIX}_${VERSION}_${ARCH}.pkg"
productbuild \
    --package "$COMPONENT_PKG" \
    --identifier "${BUNDLE_ID}.installer" \
    --version "$VERSION" \
    "$FINAL_PKG"

# Clean up
rm -rf "$PKG_ROOT"
rm -f "$COMPONENT_PKG"

echo "Created: $FINAL_PKG"
echo "  Size: $(du -h "$FINAL_PKG" | cut -f1)"
