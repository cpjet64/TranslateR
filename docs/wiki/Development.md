# Development

TranslateR is a Rust desktop app using `eframe` and `egui`.

## Build

Agent-driven or multi-worktree development must follow `AGENTS.md` in the
repository root. Use `.worktrees/<branch-or-task-name>` only for isolated task
worktrees when they are helpful, and use `CHECKLIST.md` to track commit batches
and completed checklist annotations. The full workflow is documented in
`Multiagent-Workflow.md`. Each agent must read the repo instructions,
README/spec docs, execution plan, and checklist before editing.
If no checklist item or execution plan exists for a change, create or update
the repo-local checklist before editing.

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

## Commit-Only Multiagent Workflow

Keep simultaneous work isolated and history reviewable:

- Work on the active branch or a clearly named task branch.
- Use one worktree per isolated task only when a worktree is useful.
- Keep worktrees under `<repo>/.worktrees/<branch-or-task-name>`.
- Do not create sibling worktrees outside the repository.
- Use subagents when tasks can be split without shared mutable state.
- Keep each branch scoped to one change when branches are used.
- Do not bundle unrelated fixes in the same commit.
- Sync with `main` regularly inside a task branch.
- Record completed checklist items with the commit hash, commit range, branch,
  pushed remote reference, or remote commit link that landed the work and the
  validation evidence.

Work in batches of up to 10 atomic commits. Keep `CHECKLIST.md` current, run
required local checks after each completed checklist cluster or 10 commits,
push the batch, annotate the checklist with landing evidence, and then start
the next 10-commit batch.

PR/MR review is not required by this workflow unless repository protection or a
separate user instruction requires it. Remote pipelines may run after push when
configured. GitLab remains the canonical remote CI/CD surface; do not add
GitHub-hosted CI/CD unless project policy explicitly requires it.

The user decides when to cut releases. Do not create releases, tags, package
publishes, or deployments unless the user explicitly instructs it.

This is a physical developer machine. Prefer repo-local scripts, temporary
process environment changes, reversible setup, and documented rollback. Do not
modify global/user `PATH`, `PATHEXT`, `PATHEX`, registry, shell profiles,
credentials, services, package-manager globals, or machine-wide toolchain state
unless explicitly approved. Keep host safety, ease of use, and predictable
rollback as first-class requirements.

## Tests

Run the full test suite:

```powershell
cargo test
```

Run formatting before committing and before every push batch:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci/prepush.ps1
```

On Linux or macOS, run the shell pre-push gate instead:

```sh
sh scripts/ci/prepush.sh
```

Install the repo-local hooks before commit-batch work:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/install-git-hooks.ps1
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
