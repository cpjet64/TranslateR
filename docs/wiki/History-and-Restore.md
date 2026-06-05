# History and Restore

TranslateR uses local SQLite-backed history for version snapshots. This is
separate from Git and is intended as a translator-safe restore mechanism.

## What Is Recorded

When a version is recorded, TranslateR stores:

- File path identity.
- Version number.
- Creation time.
- Translator name from app configuration.
- Content hash.
- The `.po` file bytes.
- A small validation summary.
- An optional note.

Versions increment locally for the active `.po` file.

## Where History Fits

History is not a replacement for the project maintainer's source control. It is
a local safety feature for the person using TranslateR.

Translator Mode uses `.tpatch` export for handoff. Maintainer Mode saves the
merged `.po` and records local versions as part of the local workflow.

## Restore Behavior

Restore uses the latest saved version for the active file. Use restore when a
local edit or merge went wrong and you need to return to the latest TranslateR
snapshot.

Project maintainers should still commit merged `.po` files to their normal
source control after review.
