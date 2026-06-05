#!/bin/sh
set -eu

if [ "$#" -ne 1 ]; then
  echo "usage: scripts/ci/package-unix.sh <artifact-name>" >&2
  exit 2
fi

ARTIFACT_NAME="$1"
BIN_NAME="translater"
STAGE_DIR="target/package/$ARTIFACT_NAME"
ARCHIVE_DIR="target/artifacts"
APP_NAME="TranslateR.app"

cargo build --release

rm -rf "$STAGE_DIR"
mkdir -p "$STAGE_DIR/LICENSES" "$ARCHIVE_DIR"

if [ "$ARTIFACT_NAME" = "translater-macos-x86_64" ]; then
  APP_DIR="$STAGE_DIR/$APP_NAME"
  mkdir -p "$APP_DIR/Contents/MacOS" "$APP_DIR/Contents/Resources"
  cp "target/release/$BIN_NAME" "$APP_DIR/Contents/MacOS/$BIN_NAME"
  chmod 755 "$APP_DIR/Contents/MacOS/$BIN_NAME"
  cat > "$APP_DIR/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleExecutable</key>
  <string>translater</string>
  <key>CFBundleIdentifier</key>
  <string>com.curtpme.translater</string>
  <key>CFBundleName</key>
  <string>TranslateR</string>
  <key>CFBundleDisplayName</key>
  <string>TranslateR</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>0.1.0</string>
  <key>CFBundleVersion</key>
  <string>0.1.0</string>
  <key>LSMinimumSystemVersion</key>
  <string>11.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
PLIST
else
  cp "target/release/$BIN_NAME" "$STAGE_DIR/"
fi
cp README.md LICENSE NOTICE.md "$STAGE_DIR/"
cp LICENSES/* "$STAGE_DIR/LICENSES/"
if [ -f release-notes.md ]; then
  cp release-notes.md "$STAGE_DIR/CHANGELOG.md"
else
  cp CHANGELOG.md "$STAGE_DIR/"
fi

tar -C "target/package" -czf "$ARCHIVE_DIR/$ARTIFACT_NAME.tar.gz" "$ARTIFACT_NAME"
echo "$ARCHIVE_DIR/$ARTIFACT_NAME.tar.gz"
