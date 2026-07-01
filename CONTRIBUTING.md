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

Agent-driven or multi-worktree work must follow `AGENTS.md`. Tracked batches
should use `CHECKLIST.md` for commit evidence and completion notes. The
expanded batch workflow is documented in `docs/wiki/Multiagent-Workflow.md`.
If no checklist item or execution plan exists for a change, create or update
the repo-local checklist before editing.

Install Rust, then run:

```powershell
cargo test
```

Enable the local pre-commit formatting hook and pre-push coverage gate:

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

## Commit-Only Multiagent Workflow

Use disciplined development with atomic commits, local validation, predictable
pushes, and optional isolated worktrees. Each agent must read `AGENTS.md`,
`README.md`, `CONTRIBUTING.md`, `docs/wiki/Development.md`, `CHECKLIST.md`,
and any task-specific spec or checklist item before editing.

Work on the active branch or a clearly named task branch. If isolated worktrees
are useful, keep them inside the repository:

```text
<repo>/.worktrees/<branch-or-task-name>
```

Do not create sibling worktrees outside the repository.

Work in batches of up to 10 atomic commits:

- Keep `CHECKLIST.md` current while working.
- Make each commit one coherent, reviewable unit.
- Do not bundle unrelated fixes.
- Run required local checks after each completed checklist cluster or 10
  commits.
- Push after every 10 commits at minimum.
- Push sooner after meaningful milestones, completed checklist clusters, risky
  refactors, or before handoff.
- Annotate completed checklist items with the commit hash, commit range,
  branch, pushed remote reference, or remote commit link that landed the work.
- Start the next 10-commit batch after the current batch is pushed and
  annotated.

PR/MR review is not required by this workflow unless repository protection or a
separate user instruction requires it. Remote pipelines may run after push when
configured, and GitLab remains the canonical remote CI/CD surface. Do not add
GitHub-hosted CI/CD unless project policy explicitly requires it.

The user decides when to cut releases. Do not create releases, tags, package
publishes, or deployments unless the user explicitly instructs it.

This is a physical developer machine. Prefer repo-local scripts, temporary
process environment changes, reversible setup, and documented rollback. Do not
modify global/user `PATH`, `PATHEXT`, `PATHEX`, registry, shell profiles,
credentials, services, package-manager globals, or machine-wide toolchain state
unless explicitly approved. Avoid shared mutable state across agents.

## Required Tests

Before every push batch, run:

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

The most important tests are:

- `tests/po_corpus.rs`
- `tests/po_edit_validate.rs`
- `tests/workflow_files.rs`
- `tests/font_coverage.rs`

Any change to PO parsing or writing must preserve the no-edit round-trip tests.
The pre-commit hook runs formatting checks automatically once installed. The
pre-push hook runs the coverage gate automatically once installed. For an
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
