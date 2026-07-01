# Commit-Only Multiagent Workflow

Use disciplined, high-throughput development with atomic commits, local
validation, predictable pushes, and optional isolated worktrees. This
`AGENTS.md` file is the authoritative instruction source for agents working in
this repository. The expanded workflow is also documented in
`docs/wiki/Multiagent-Workflow.md`.

These instructions apply to the whole repository unless a deeper `AGENTS.md`
adds more specific rules for a subdirectory.

## Required Reading

Before editing, each agent must read:

- `AGENTS.md`
- `README.md`
- `CONTRIBUTING.md`
- `docs/wiki/Development.md`
- `CHECKLIST.md`
- Any task-specific spec, execution plan, or checklist item

If no checklist item or execution plan exists for the work, create or update
the repo-local checklist before editing.

## Core Workflow

- Work with atomic commits on the active branch or a clearly named task branch.
- Each commit must represent one coherent, reviewable unit of work.
- Do not bundle unrelated fixes.
- Push after every 10 commits at minimum.
- Push sooner after meaningful milestones, completed checklist clusters, risky
  refactors, before handoff, or whenever remote backup is useful.
- Do not require a PR/MR unless repository protection or a separate user
  instruction explicitly requires it.
- Do not open PRs/MRs, merge PRs/MRs, create tags, cut releases, publish
  packages, or deploy unless the user separately instructs it.
- The user decides when to cut releases.

## Commit Batch Loop

- Work in batches of up to 10 atomic commits.
- Keep the repo-local master checklist in `CHECKLIST.md` current while working.
- After each completed checklist cluster or 10 commits, run required local
  validation.
- Push the commits.
- Annotate completed checklist items with the commit hash, commit range,
  branch, pushed remote reference, or remote commit link that landed the work.
- Start the next 10-commit batch.
- Continue until the checklist is complete or the user changes direction.

## Optional Worktrees and Subagents

- Use subagents when work can be split safely without shared mutable state.
- Coordinate subagents through the active checklist.
- Avoid shared mutable state across agents.
- If multiple local worktrees are useful, keep them inside this repository
  under `<repo>/.worktrees/<branch-or-task-name>`.
- Do not create sibling worktrees outside the repository.
- Keep branches and worktrees scoped to one task when they are used.
- Sync with `main` regularly when working on a task branch.

## Checklist Tracking

- Maintain the repo-local master checklist in `CHECKLIST.md`.
- Do not mark completed checklist items without landing evidence.
- Landing evidence must be a commit hash, commit range, branch, pushed remote
  reference, or remote commit link.
- If work in another repo must be started or unblocked, record:
  - the target repo,
  - the reason for spin-up,
  - the commit or branch reference,
  - the checklist item that triggered it.

## Local Validation and Hooks

- Install and use repo-local pre-commit and pre-push hooks when available:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/install-git-hooks.ps1
```

On Linux or macOS:

```sh
sh scripts/install-git-hooks.sh
```

- Run local validation before every push batch.
- The pre-commit hook runs formatting checks. The pre-push hook runs the
  repository pre-push CI gate.
- For this Rust repo, run at minimum:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/ci/prepush.ps1
```

On Linux or macOS, use the shell pre-push gate instead:

```sh
sh scripts/ci/prepush.sh
```

- Add any extra checks documented by the task or touched subsystem.
- Remote pipelines may run after push when configured, but this workflow does
  not require PR/MR review unless a separate repo policy or user instruction
  requires it.
- Do not add GitHub-hosted CI/CD unless explicitly required by project policy.
- Do not weaken existing validation commands while documenting workflow changes.

## Safety Rules

- This is a physical developer machine. Avoid global or user-level mutations.
- Do not modify global/user `PATH`, `PATHEXT`, `PATHEX`, registry, shell
  profiles, credentials, services, package-manager globals, or machine-wide
  toolchain state unless explicitly approved.
- Prefer repo-local scripts, temporary process environment changes, reversible
  setup, and documented rollback.
- Keep host safety, ease of use, and predictable rollback as first-class
  requirements.

## Expected Agent Behavior

- Write confident, working code.
- Do not leave speculative TODO-only work when an item can be implemented.
- Validate locally before every push batch.
- Review your own work critically before committing and before pushing.
- Keep commits small enough to review from history.
- Keep branches and worktrees scoped to one task when they are used.
- Push after every 10 commits or sooner when useful.
- Never add agent attribution, `Co-authored-by` trailers, or similar AI
  attribution to commit messages.
- Keep changes aligned with TranslateR's core priority: lossless `.po`
  preservation and translator-safe workflows.
