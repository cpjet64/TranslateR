# Translator Guide

Use Translator Mode when a maintainer gives you TranslateR and one `.po` file.
Translator Mode is intentionally narrow: it lets you edit translations and
export a `.tpatch` file, but it does not save the maintainer's final `.po`.

## What You Need

- A TranslateR portable package for your operating system.
- One `.po` file from the project maintainer.
- A place to save the exported `.tpatch` file.

You do not need Git, GitHub, GitLab, or a developer account.

## Basic Workflow

1. Start TranslateR.
2. Choose Translator Mode.
3. Select the `.po` file you received.
4. Work through the translation entries.
5. Review warnings in the validation area.
6. Export a `.tpatch`.
7. Send the `.tpatch` file back to the maintainer.

## What You Can Edit

Translator Mode focuses on the translated text:

- `msgstr` for singular entries.
- `msgstr[n]` fields for plural entries.
- Translator-facing comments when the app exposes them.
- Fuzzy status when the maintainer wants translators to clear or mark it.

Do not edit source text, source references, contexts, extracted comments, or
message identifiers. Those come from the application source and are owned by the
maintainer.

## Entry List

The entry list shows every translatable entry in the active `.po` file. Use the
search box and filters to focus the list.

Common filters:

- All: every visible entry.
- Untranslated: entries with empty translations.
- Fuzzy: entries marked as needing review.
- Warnings: entries with validation warnings.
- Plural: entries with plural forms.
- Context: entries that have `msgctxt`.

The selected entry opens in the editor area.

## Translation Editor

For a singular entry, fill in the Translation box.

For a plural entry, fill in each form shown by TranslateR. Some languages have
two plural forms, while others have more. Arabic, for example, can have six
plural forms. Use the scrollbars when the form list is taller than the window.

If the source ends with a visible `\n`, the translation usually needs to end
with `\n` too. If the source contains placeholders such as `%s` or `%d`, keep
the matching placeholders in the translation.

## Exporting Your Work

When your translation is ready:

1. Click Export TPatch.
2. Choose a file name ending in `.tpatch`.
3. Send that `.tpatch` to the maintainer.

The `.tpatch` contains only TranslateR patch data. It is not a generic Git
patch, and it should be applied only in TranslateR Maintainer Mode.

## Good Translator Habits

- Keep placeholders such as `%s`, `%d`, and `%1$s` intact.
- Preserve required trailing newlines.
- Translate all plural forms shown by the app.
- Leave source text and context unchanged.
- Use validation warnings as a checklist before exporting.

See also:

- [Validation and Status Legend](Validation-and-Status-Legend)
- [TPatch Format](TPatch-Format)
- [Troubleshooting](Troubleshooting)
