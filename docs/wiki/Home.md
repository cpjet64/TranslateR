# TranslateR Wiki

TranslateR is a portable desktop editor for GNU gettext `.po` translation files.
It is built for a handoff workflow where maintainers distribute versioned
`.trpack` files, translators return TranslateR-specific `.tpatch` files, and
maintainers review, merge, and save the final `.po` file.

The most important rule is that the `.po` file remains the source of truth.
TranslateR is designed to preserve comments, order, flags, contexts, plural
forms, multiline strings, obsolete entries, and unrelated layout whenever
possible.

## Start Here

- [Translator Guide](Translator-Guide): receive one `.trpack`, translate it,
  save `.trdraft` files when unfinished, and export a `.tpatch`.
- [Maintainer Guide](Maintainer-Guide): export `.trpack` files, open one base
  `.po`, review one or more `.tpatch` files, merge matching changes, and save
  the updated `.po`.
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
- [Package Version History](History-and-Restore): how `.trpack` version history
  works.
- [Release Process](Release-Process): CI, user-controlled releases, packages,
  and mirroring.
- [Development](Development): build, test, and contribution workflow.
- [Multiagent Workflow](Multiagent-Workflow): commit batches, optional isolated
  worktrees, checklist tracking, and local CI expectations.
- [License and Fonts](License-and-Fonts): MIT app license and bundled font
  licensing.

## Workflow Summary

Translator workflow:

1. Start TranslateR.
2. Choose Translator Mode.
3. Open the `.trpack` file from the maintainer, or reopen your `.trdraft`.
4. Translate entries and fix warnings where appropriate.
5. Save a `.trdraft` if you are not finished.
6. Export a `.tpatch`.
7. Send the `.tpatch` back to the maintainer.

Maintainer workflow:

1. Start TranslateR.
2. Choose Maintainer Mode.
3. Open the base `.po` file.
4. Open a folder containing any number of `.tpatch` files.
5. Export a `.trpack` for translators when starting a new round.
6. Review diffs and apply matching patches.
7. Save the merged `.po` file.

## Project Links

- Primary GitLab repository: `git@gitlab.curtpme.com:cpjet64/TranslateR.git`
- GitHub mirror: `git@github.com:cpjet64/TranslateR.git`
- GitLab wiki: `git@gitlab.curtpme.com:cpjet64/TranslateR.wiki.git`
- GitHub wiki: `git@github.com:cpjet64/TranslateR.wiki.git`
