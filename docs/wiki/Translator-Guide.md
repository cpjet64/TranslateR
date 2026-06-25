# Translator Guide

Use Translator Mode when a maintainer gives you TranslateR and one `.trpack`
file. Translator Mode is intentionally narrow: it lets you edit translations,
save unfinished `.trdraft` work, and export a `.tpatch` file, but it does not
save the maintainer's final `.po`.

## What You Need

- A TranslateR portable package for your operating system.
- One `.trpack` file from the project maintainer.
- Optional: your own `.trdraft` file if you are continuing unfinished work.
- A place to save the exported `.tpatch` file.

You do not need Git, GitHub, GitLab, or a developer account.

## Basic Workflow

1. Start TranslateR.
2. Choose Translator Mode.
3. Select the `.trpack` file you received, or reopen your `.trdraft`.
4. Work through the translation entries.
5. Review warnings in the validation area.
6. Click Save TRDraft if you need to stop before finishing.
7. Export a `.tpatch`.
8. Send the `.tpatch` file back to the maintainer.

## What You Can Edit

Translator Mode focuses on the translated text:

- `msgstr` for singular entries.
- `msgstr[n]` fields for plural entries.
- Translator comments for notes that should be preserved in the `.po` file.
- The `fuzzy` flag when a completed translation no longer needs review.

The editor also shows the header `Language` field with an Edit button, and a
Question for maintainer box on each source and translation form (see below).

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

Use Translator comments for short notes that should travel with the `.po` entry.
Use Question for maintainer when you need an answer before translating; questions
are stored in `.trdraft` and `.tpatch` metadata instead of changing the `.po`
entry.

## Asking Questions

Translator Mode shows a Question for maintainer box beside the source text and
each translation form. Use it when you need more context, a screenshot, tone
guidance, or clarification about where the text appears.

Questions are saved in `.trdraft` files and exported inside `.tpatch` metadata.
They do not modify the `.po` file.

## Saving a Draft

Use Save TRDraft when you need to pause before the translation is ready to send
back. A `.trdraft` stores the package version you started from and your current
edited PO text, including translator questions. Reopen the `.trdraft` later in
Translator Mode to continue.

Do not send `.trdraft` files as the normal maintainer handoff. Send `.tpatch`
files when the translation is ready for review.

## Exporting Your Work

When your translation is ready:

1. Click Export TPatch.
2. Choose a file name ending in `.tpatch`.
3. Send that `.tpatch` to the maintainer.

The `.tpatch` contains only TranslateR patch data plus the package id, package
version, and base hash. It is not a generic Git patch, and it should be applied
only in TranslateR Maintainer Mode.

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
