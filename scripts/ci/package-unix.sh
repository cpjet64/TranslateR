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

cargo build --release

rm -rf "$STAGE_DIR"
mkdir -p "$STAGE_DIR/LICENSES" "$ARCHIVE_DIR"

cp "target/release/$BIN_NAME" "$STAGE_DIR/"
cp README.md LICENSE NOTICE.md "$STAGE_DIR/"
cp LICENSES/* "$STAGE_DIR/LICENSES/"
if [ -f release-notes.md ]; then
  cp release-notes.md "$STAGE_DIR/CHANGELOG.md"
else
  cp CHANGELOG.md "$STAGE_DIR/"
fi

tar -C "target/package" -czf "$ARCHIVE_DIR/$ARTIFACT_NAME.tar.gz" "$ARTIFACT_NAME"
echo "$ARCHIVE_DIR/$ARTIFACT_NAME.tar.gz"
