# Maintainer Guide

Use Maintainer Mode when you have one base `.po` file and one or more
translator `.tpatch` files. Maintainer Mode is where patches are reviewed,
merged, saved, and versioned.

## Maintainer Workflow

1. Start TranslateR.
2. Choose Maintainer Mode.
3. Open the base `.po` file.
4. Open the folder containing `.tpatch` files.
5. Review the diff for each patch.
6. Apply selected matching patches, or apply all matching patches.
7. Save the merged `.po` file.
8. Commit or distribute the merged `.po` through your normal project process.

Only one `.po` file should be active at a time. Any number of `.tpatch` files can
be loaded for that active `.po`.

## Diff Review

Each `.tpatch` is shown as a diff against the active `.po` file. Review the
source context and translation changes before applying it.

Use the diff view to check:

- The patch belongs to the currently open `.po` file.
- The changed entries are expected.
- Placeholders and trailing newlines still match the source.
- Plural entries include all required forms.
- The translator did not unintentionally remove useful existing translations.

## Applying Patches

TranslateR applies a `.tpatch` only when its context matches the active `.po`.
If the context does not match, TranslateR rejects the patch with an error such
as:

```text
patch context did not match active PO file
```

This usually means one of these is true:

- The `.tpatch` was made from a different `.po` file.
- The base `.po` changed after the translator exported the `.tpatch`.
- Another patch already changed the same area.
- The patch file was edited outside TranslateR.

When that happens, compare the translator's source `.po` and the maintainer's
current base `.po`, then decide whether to ask for a new `.tpatch` or manually
recreate the intended translation.

## Saving the Merged PO

Maintainer Mode can save the merged `.po` file. TranslateR writes atomically
where practical and records local version snapshots in its SQLite history.

The saved `.po` remains a normal gettext `.po` file. TranslateR does not require
downstream projects to understand `.tpatch`.

## Recommended Handoff Pattern

For a clean translation round:

1. Start from the current project `.po`.
2. Give each translator that same base `.po`.
3. Ask translators to return only `.tpatch` files.
4. Load all returned `.tpatch` files in Maintainer Mode.
5. Apply non-conflicting patches first.
6. Resolve rejected or conflicting patches one at a time.
7. Save the merged `.po`.
8. Run the project's normal gettext checks.

See also:

- [TPatch Format](TPatch-Format)
- [PO Preservation](PO-Preservation)
- [History and Restore](History-and-Restore)
- [Validation and Status Legend](Validation-and-Status-Legend)
