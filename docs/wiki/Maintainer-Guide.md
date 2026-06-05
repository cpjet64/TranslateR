# Maintainer Guide

Use Maintainer Mode when you have a base `.po` file and one or more translator
`.tpatch` files.

1. Start TranslateR.
2. Choose Maintainer Mode.
3. Open the base `.po` file.
4. Open the folder containing translator `.tpatch` files.
5. Review each diff.
6. Apply selected matching patches or apply all matching patches.
7. Save the merged `.po` file.

TranslateR rejects a `.tpatch` when its expected context does not match the
active `.po` file. This prevents silently applying a translator patch to the
wrong source text.

The `.po` file remains the source of truth. TranslateR preserves comments,
ordering, flags, contexts, plural entries, multiline strings, and unrelated
layout as much as possible.

