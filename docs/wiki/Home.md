# TranslateR Wiki

TranslateR is a portable desktop editor for GNU gettext `.po` translation files.
It is built for a handoff workflow where translators return TranslateR-specific
`.tpatch` files and maintainers review, merge, and save the final `.po` file.

The most important rule is that the `.po` file remains the source of truth.
TranslateR is designed to preserve comments, order, flags, contexts, plural
forms, multiline strings, obsolete entries, and unrelated layout whenever
possible.

## Start Here

- [Translator Guide](Translator-Guide): receive one `.po` file, translate it,
  and export a `.tpatch`.
- [Maintainer Guide](Maintainer-Guide): open one base `.po`, review one or more
  `.tpatch` files, merge matching changes, and save the updated `.po`.
- [Downloads and Installation](Downloads-and-Installation): choose the portable
  package for Windows, Ubuntu, Debian, or macOS.
- [Validation and Status Legend](Validation-and-Status-Legend): understand
  untranslated, fuzzy, plural, context, formatting, and warning markers.
- [Troubleshooting](Troubleshooting): fix common open, font, patch, and release
  download problems.

## Reference

- [TPatch Format](TPatch-Format): what `.tpatch` files are and why TranslateR
  does not accept generic Git patches.
- [PO Preservation](PO-Preservation): the lossless editing rules and regression
  expectations.
- [History and Restore](History-and-Restore): how local version snapshots work.
- [Release Process](Release-Process): automatic CI, packages, and mirrored
  releases.
- [Development](Development): build, test, and contribution workflow.
- [License and Fonts](License-and-Fonts): MIT app license and bundled font
  licensing.

## Workflow Summary

Translator workflow:

1. Start TranslateR.
2. Choose Translator Mode.
3. Open the `.po` file from the maintainer.
4. Translate entries and fix warnings where appropriate.
5. Export a `.tpatch`.
6. Send the `.tpatch` back to the maintainer.

Maintainer workflow:

1. Start TranslateR.
2. Choose Maintainer Mode.
3. Open the base `.po` file.
4. Open a folder containing any number of `.tpatch` files.
5. Review diffs and apply matching patches.
6. Save the merged `.po` file.

## Project Links

- Primary GitLab repository: `git@gitlab.curtpme.com:cpjet64/TranslateR.git`
- GitHub mirror: `git@github.com:cpjet64/TranslateR.git`
- GitLab wiki: `git@gitlab.curtpme.com:cpjet64/TranslateR.wiki.git`
- GitHub wiki: `git@github.com:cpjet64/TranslateR.wiki.git`
