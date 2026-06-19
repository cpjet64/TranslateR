# Translating Repository Markdown

TranslateR welcomes translated repository READMEs and quick start guides. These
files help GitLab and GitHub visitors find project information in their own
language.

This guide is for translating the repository documentation. TranslateR's app
interface translation files live in `i18n/` and use gettext `.po` catalogs.

## Naming

Translated READMEs and quick starts live at the repository root and use these
naming schemes:

```text
README.<lang>.md
QUICKSTART.<lang>.md
```

Use BCP 47 style language tags:

- `README.es.md`
- `README.fr.md`
- `README.de.md`
- `README.zh-Hans.md`
- `README.pt-BR.md`
- `QUICKSTART.es.md`
- `QUICKSTART.zh-Hans.md`

Use lowercase base languages, title-case script names, and uppercase regions.

## Contribution Process

1. Open an issue before starting.
   - Use the issue to claim the language.
   - Check whether someone is already translating the same language.
   - Mention the target file name, such as `README.es.md` or
     `QUICKSTART.es.md`.
2. Create a branch for the translation.
   - Use a branch name like `docs/readme-es`.
3. Copy the English Markdown file to the translated file name.
   - Example: copy `README.md` to `README.es.md`.
   - Example: copy `QUICKSTART.md` to `QUICKSTART.es.md`.
4. Translate the prose.
   - Keep commands, file names, URLs, code blocks, package names, license names,
     and extension names unchanged unless there is a clear reason.
   - Keep Markdown headings and section order aligned with `README.md` where
     practical.
   - Keep links relative when they point to files in this repository.
5. Add the translated Markdown link to the matching `## Translations` section.
   - Example: `- Espanol: [README.es.md](README.es.md)`
   - Example: `- Espanol: [QUICKSTART.es.md](QUICKSTART.es.md)`
6. Open a pull request.
   - Link the issue you opened.
   - Mention whether the translation is complete or still needs review.

## Review Notes

Maintainers should check that:

- The translated file follows the `README.<lang>.md` or
  `QUICKSTART.<lang>.md` naming scheme.
- The matching top-level `## Translations` section links to the new file.
- Commands, package names, file extensions, and license references still match
  the English README.
- The translation does not remove release, license, or security-relevant notes.

It is fine for translated READMEs to lag slightly behind `README.md`, but large
structure changes should be synced when practical.
