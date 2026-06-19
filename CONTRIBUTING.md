# Contributing to TranslateR

Thanks for helping improve TranslateR.

## Project Priorities

TranslateR is built around translator-safe `.po` handling. When making changes,
keep these priorities in order:

1. Preserve `.po` files losslessly when no edits are made.
2. Keep translator and maintainer workflows simple and explicit.
3. Make validation helpful without blocking legitimate translation work.
4. Keep `.tpatch` behavior scoped to TranslateR's own format.

## Development Setup

Install Rust, then run:

```powershell
cargo test
```

Enable the local pre-push coverage gate:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/install-git-hooks.ps1
```

On Linux or macOS:

```sh
sh scripts/install-git-hooks.sh
```

Build the app:

```powershell
cargo build
```

Run the app:

```powershell
cargo run
```

## Required Tests

Before submitting changes, run:

```powershell
cargo fmt
cargo test
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci/coverage.ps1
```

The most important tests are:

- `tests/po_corpus.rs`
- `tests/po_edit_validate.rs`
- `tests/workflow_files.rs`
- `tests/font_coverage.rs`

Any change to PO parsing or writing must preserve the no-edit round-trip tests.
The pre-push hook runs the coverage gate automatically once installed. For an
emergency push only, set `TRANSLATER_SKIP_COVERAGE_HOOK=1`.

## PO Handling Rules

Do not normalize or rewrite unrelated PO content.

Preserve:

- Comments.
- Source references.
- Flags.
- Entry order.
- Contexts.
- Plural entries.
- Obsolete entries.
- Multiline strings.
- Existing line endings where practical.

Translator Mode should only produce `.tpatch` files. Maintainer Mode is where
merged `.po` files are saved.

## Font Changes

Bundled fonts must be open licensed and their license text must be included in
`LICENSES/`.

When adding or removing bundled fonts, update:

- `src/ui/fonts.rs`
- `tests/font_coverage.rs`
- `LICENSES/README.md` if needed

## Commit Messages

Use clear, concise commit messages. Do not add AI attribution trailers or
`Co-authored-by` trailers for AI tools.
