# Translator Guide

Use Translator Mode when you receive a `.po` file from a maintainer.

1. Start TranslateR.
2. Choose Translator Mode.
3. Open the `.po` file you were given.
4. Fill in untranslated or incomplete translation entries.
5. Use validation warnings to check placeholders, plural forms, fuzzy entries,
   and trailing newlines.
6. Export a `.tpatch` file.
7. Send the `.tpatch` file back to the maintainer.

Translator Mode exports TranslateR-specific `.tpatch` files. It does not write
merged `.po` files for maintainers.

