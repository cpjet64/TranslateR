# TranslateR

TranslateR is a small desktop editor for GNU gettext `.po` translation files.
It is designed for a simple maintainer-to-translator workflow:

1. A maintainer gives a translator a copy of TranslateR and one `.po` file.
2. The translator edits translations and exports a `.tpatch` file.
3. The maintainer opens the base `.po` file, reviews one or more `.tpatch` files,
   applies matching changes, and saves the merged `.po`.

TranslateR keeps the `.po` file as the source of truth. It preserves comments,
ordering, flags, contexts, plural entries, multiline strings, and untouched file
layout as much as possible.

## Features

- Cross-platform Rust desktop app using `eframe`/`egui`.
- Translator Mode:
  - Opens one `.po` file.
  - Edits `msgstr` and plural `msgstr[n]` values.
  - Exports TranslateR-specific `.tpatch` files.
  - Does not write merged `.po` files.
- Maintainer Mode:
  - Opens one base `.po` file.
  - Loads any number of `.tpatch` files from a folder.
  - Shows a diff for each `.tpatch`.
  - Applies selected patches or all matching patches.
  - Saves the merged `.po` file and records local versions.
- SQLite-backed local history for versioned PO snapshots.
- Atomic file saves.
- Validation for common translation issues:
  - Empty translations.
  - Fuzzy entries.
  - Missing plural forms.
  - `c-format` placeholder mismatches.
  - Trailing newline mismatches.
- Bundled Noto fallback fonts for broad script coverage.
- Regression tests against `gettext-po-samples`.

## TPatch Files

`.tpatch` is TranslateR's own patch format. It is not intended to be a generic
Git patch format.

Maintainers should only import `.tpatch` files created by TranslateR. TPatches
include context lines, and Apply will reject a TPatch when the expected context
does not match the active `.po` file.

## Building

Install Rust, then run:

```powershell
cargo build
```

Run tests:

```powershell
cargo test
```

Run the app:

```powershell
cargo run
```

On Windows, the debug executable is:

```text
target\debug\translater.exe
```

## CI and Portable Packages

GitLab CI runs on the self-hosted runner matrix:

- Windows 11
- Ubuntu 24
- Debian 12
- macOS Sequoia Intel

The pipeline validates formatting, runs the Rust test suite on each OS, and
builds portable release artifacts:

- `translater-windows-x86_64.zip`
- `translater-ubuntu-x86_64.tar.gz`
- `translater-debian-x86_64.tar.gz`
- `translater-macos-x86_64.tar.gz`

Each package contains the TranslateR binary, README, MIT license, notice file,
and third-party font license files.

Linux and macOS packages are exposed as GitLab job artifacts. The Windows
package is uploaded to the GitLab Generic Package Registry because the
self-hosted Windows shell runner does not reliably collect artifact paths.

The GitLab pipeline can mirror `main` to GitHub when these protected CI
variables are configured:

- `GITHUB_MIRROR_URL`: SSH URL of the GitHub repository.
- `GITHUB_MIRROR_SSH_KEY`: private deploy key with write access to that GitHub
  repository.

## Test Corpus

The repository includes a pinned fixture copy of:

```text
https://github.com/ergenius/gettext-po-samples
```

Fixture metadata is recorded in:

```text
tests/fixtures/gettext-po-samples.METADATA.md
```

Important test gates:

- Every fixture `.po` file parses.
- No-edit parse/write round-trips fixture files byte-for-byte.
- Translation edits preserve unrelated content.
- TPatches can be exported and applied.
- Bundled fonts cover representative script samples.

## Fonts

TranslateR bundles Noto fonts for broad language/script coverage. TranslateR
itself is MIT licensed. Bundled Noto fonts are licensed separately under the
SIL Open Font License 1.1.

See:

```text
LICENSE
LICENSES/OFL-1.1-Noto.txt
LICENSES/README.md
```

## License

TranslateR is licensed under the MIT License. See `LICENSE`.
