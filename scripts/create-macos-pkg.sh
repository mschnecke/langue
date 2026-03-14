#!/bin/bash
set -euo pipefail

APP_NAME="PisumLangue"
APP_PATH="src-tauri/target/aarch64-apple-darwin/release/bundle/macos/${APP_NAME}.app"
PKG_OUTPUT="src-tauri/target/aarch64-apple-darwin/release/bundle/macos/${APP_NAME}.pkg"
IDENTIFIER="com.pisumlangue.app"

if [ ! -d "$APP_PATH" ]; then
  echo "Error: $APP_PATH not found"
  exit 1
fi

pkgbuild \
  --component "$APP_PATH" \
  --install-location "/Applications" \
  --identifier "$IDENTIFIER" \
  "$PKG_OUTPUT"

echo "Created: $PKG_OUTPUT"
