# TranslateR Interface Translation Files

This directory contains gettext catalogs for translating TranslateR itself.

- `translater.pot` is the template generated from the Rust source.
- `en.po` is the English source catalog and fallback catalog.

To add a new interface language, copy `translater.pot` to `<language>.po`,
translate the `msgstr` values, and keep the file beside the TranslateR binary
in an `i18n` directory. Packaged releases already include this directory.

The CI pipeline runs:

```text
python3 scripts/i18n/generate-translater-po.py --check
```

That check fails when source strings change without refreshing these catalogs.
