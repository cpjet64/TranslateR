# Development

TranslateR is a Rust desktop app using `eframe` and `egui`.

## Build

Install Rust, then run:

```powershell
cargo build
```

Run the app:

```powershell
cargo run
```

On Windows, the debug executable is:

```text
target\debug\translater.exe
```

## Tests

Run the full test suite:

```powershell
cargo test
```

Run formatting before committing:

```powershell
cargo fmt
```

Important tests:

- `tests/po_corpus.rs`
- `tests/po_edit_validate.rs`
- `tests/workflow_files.rs`
- `tests/font_coverage.rs`
- `tests/i18n_catalog.rs`

Any change to PO parsing or writing must preserve no-edit round-trip behavior.

## Architecture

Important modules:

- `src/po/`: parser, writer, escaping, header parsing, validation, and stats.
- `src/ui/`: egui panels, display helpers, and font loading.
- `src/vcs/`: TPatch diff generation and application.
- `src/workflow.rs`: `.trpack`, `.trdraft`, TPatch metadata, and portable
  package history.
- `src/project/`: file scanning, document store, and config.
- `src/util/`: atomic saving, hashing, and path helpers.
- `src/i18n.rs`: runtime interface translation catalogs.
- `src/update.rs`: GitHub release update checker and package downloader.

## Contribution Priorities

Keep changes aligned with these priorities:

1. Preserve `.po` files losslessly when no edits are made.
2. Keep translator and maintainer workflows simple.
3. Make validation useful without blocking legitimate translation work.
4. Keep `.trpack`, `.trdraft`, and `.tpatch` scoped to TranslateR's own format.
5. Keep release packages portable and self-contained.

See `CONTRIBUTING.md` in the repository for project contribution rules.
