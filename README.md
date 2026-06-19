# TranslateR

TranslateR is a small desktop editor for GNU gettext `.po` translation files.
It is designed for a simple maintainer-to-translator workflow:

1. A maintainer exports a versioned `.trpack` from the current `.po` file.
2. The translator opens the `.trpack`, edits translations, and can save a
   `.trdraft` if the work is unfinished.
3. The translator exports a `.tpatch` file and sends that file back.
4. The maintainer opens the base `.trpack` or `.po` file, reviews one or more
   `.tpatch` files, applies matching changes, and saves a new `.trpack`
   version.

TranslateR keeps the `.po` file as the source of truth. It preserves comments,
ordering, flags, contexts, plural entries, multiline strings, and untouched file
layout as much as possible.

## Features

- Cross-platform Rust desktop app using `eframe`/`egui`.
- Translator Mode:
  - Opens one `.trpack`, `.trdraft`, or direct `.po` file.
  - Edits `msgstr` and plural `msgstr[n]` values.
  - Saves unfinished work as TranslateR-specific `.trdraft` files.
  - Exports TranslateR-specific `.tpatch` files.
  - Does not write merged `.po` files.
- Maintainer Mode:
  - Opens one base `.trpack` or `.po` file.
  - Exports versioned `.trpack` files for translators.
  - Loads any number of `.tpatch` files from a folder.
  - Shows a diff for each `.tpatch`.
  - Applies selected patches or all matching patches.
  - Saves merged package versions into `.trpack` history.
- Portable `.trpack` version history with change summaries.
- Atomic file saves.
- Validation for common translation issues:
  - Empty translations.
  - Fuzzy entries.
  - Missing plural forms.
  - `c-format` placeholder mismatches.
  - Trailing newline mismatches.
- Bundled Noto fallback fonts for broad script coverage.
- Regression tests against `gettext-po-samples`.

## TranslateR Workflow Files

`.trpack` is the maintainer-to-translator package. It stores preserved PO text
plus TranslateR metadata such as project id, package version, language, and base
hash. It also carries the portable package history log, so the version history
travels with the handoff file instead of living in an app-local database.

`.trdraft` is a translator's unfinished local draft. It stores both the original
package PO text and the current edited PO text, so exported patches still use
the correct package version as their base.

`.tpatch` is TranslateR's own patch format. It is not intended to be a generic
Git patch format. TPatches exported from a `.trpack` or `.trdraft` include the
package id, package version, and base hash.

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

## CI, Releases, and Portable Packages

GitLab CI runs on the self-hosted runner matrix:

- Windows 11
- Ubuntu 24
- Debian 12
- macOS Sequoia Intel

The pipeline validates formatting, runs the Rust test suite on each OS, and
builds portable packages:

- `translater-windows-x86_64.zip`
- `translater-ubuntu-x86_64.tar.gz`
- `translater-debian-x86_64.tar.gz`
- `translater-macos-x86_64.tar.gz`

Each package contains the TranslateR binary, `README.md`, `LICENSE`,
`NOTICE.md`, and `LICENSES/`. Runtime fallback fonts are embedded into the
binary; the package includes the third-party font license files.

The Windows archive contains an Authenticode-signed `translater.exe` when built
by the protected GitLab release pipeline. Signing uses the CurtPME signing
service configured through protected CI variables.

### macOS Gatekeeper

The macOS package is currently an unsigned, non-notarized portable binary.
macOS Gatekeeper may show:

```text
"translater" not opened. Apple could not verify "translater" is free of malware
that may harm your Mac or compromise your privacy.
```

That warning is expected for a downloaded binary that is not signed with an
Apple Developer ID and notarized by Apple. A personal CA certificate does not
satisfy Gatekeeper for public macOS downloads.

For a trusted internal copy, macOS users can approve the app from System
Settings after the first failed open attempt, or remove the download quarantine
attribute after verifying the archive came from the expected release:

```sh
xattr -dr com.apple.quarantine TranslateR.app
open TranslateR.app
```

Public macOS releases that open without this warning require Apple Developer ID
signing and Apple notarization.

Pushes to `main` automatically cut the next patch release after CI passes. The
release job:

1. Finds the latest `vX.Y.Z` tag.
2. Computes the next patch tag.
3. Generates release notes from commit subjects since the previous tag.
4. Uploads all packages to the GitLab Generic Package Registry.
5. Creates or updates the GitLab release.
6. Creates or updates the matching GitHub release and uploads the same assets.

The generated changelog text is attached to the GitLab and GitHub releases.
Release tags matching `v*` are protected in GitLab and created by CI.

Normal `main` package artifacts are retained temporarily for CI inspection.
Release downloads should come from the GitLab or GitHub release pages.

The GitLab pipeline can mirror `main` to GitHub when these protected CI
variables are configured:

- `GITHUB_MIRROR_URL`: SSH URL of the GitHub repository.
- `GITHUB_MIRROR_SSH_KEY`: private deploy key with write access to that GitHub
  repository.
- `GITHUB_RELEASE_TOKEN`: GitHub token with permission to create releases and
  upload release assets.
- `GITLAB_RELEASE_TOKEN`: optional fallback GitLab token with permission to
  create protected `v*` tags and GitLab releases if the built-in CI job token is
  unavailable.
- `CURTPME_SIGNER_URL`: CurtPME signing service URL for Windows Authenticode
  signing.
- `CURTPME_SIGNER_TOKEN`: CurtPME signing service bearer token.

## Wikis

Project wiki pages are maintained in the GitLab and GitHub wiki repositories:

- GitLab: `git@gitlab.curtpme.com:cpjet64/TranslateR.wiki.git`
- GitHub: `git@github.com:cpjet64/TranslateR.wiki.git`

The wikis contain user-facing workflow notes for translators, maintainers, and
release handling.

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
