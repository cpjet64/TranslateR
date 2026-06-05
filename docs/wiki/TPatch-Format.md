# TPatch Format

`.tpatch` is TranslateR's own patch format. It is designed for the TranslateR
translator-to-maintainer workflow and is not a generic Git patch format.

## Purpose

TPatches let translators send translation changes without sending a rewritten
`.po` file. Maintainers can then review diffs, apply matching patches, and save
the final merged `.po`.

## Format

TranslateR v1 patch files begin with:

```text
# TranslateR TPatch v1
--- original-name
+++ changed-name
```

The body uses diff-style context lines:

- Lines beginning with a space are context.
- Lines beginning with `-` are removed from the base `.po`.
- Lines beginning with `+` are inserted into the merged `.po`.

## Context Matching

When a maintainer applies a `.tpatch`, TranslateR checks that the patch context
matches the active `.po` file. If the context does not match, the patch is
rejected instead of being applied in the wrong place.

This protects maintainers from silently merging translator work into the wrong
base file.

## What TPatch Is Not

TPatch is not:

- A Git patch.
- A gettext standard.
- A standalone translation memory format.
- A format intended for manual editing.

Maintainers should only import `.tpatch` files created by TranslateR.

## Common Failure

If applying a patch fails with:

```text
patch context did not match active PO file
```

the `.tpatch` probably does not match the active `.po`. Open the correct base
file or ask the translator to export a new `.tpatch` from the current `.po`.
