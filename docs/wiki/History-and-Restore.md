# Package Version History

TranslateR stores durable version history inside `.trpack` files. There is no
app-local SQLite history for package truth.

## What Is Recorded

Each `.trpack` version records:

- Package version.
- Creation time.
- Author name from app configuration.
- Reason, such as `Initial TRPack export`, `Save PO`, or `Apply TPatch`.
- Base and content hashes.
- Line additions and deletions.
- A PO-aware summary of changed translation fields.

## Where History Fits

The `.trpack` is the maintainer-owned handoff package. Maintainers distribute it
to translators, receive `.tpatch` files back, merge those patches, and save a new
`.trpack` version.

Translator Mode can save unfinished work as `.trdraft`, and the draft preserves
the package history from the `.trpack` it came from. Exported `.tpatch` files
still target the package version and base hash that the translator received.

## Viewing History

Use the History button in Maintainer Mode to open the Version History window.
The left side lists package versions. The right side shows the saved time,
author, reason, hashes, and change summary for the selected version.

## Restore Policy

TranslateR no longer keeps hidden local restore snapshots. If a merge goes wrong,
reopen the previous `.trpack` version from your project files or source control,
then reapply the needed `.tpatch` files.
