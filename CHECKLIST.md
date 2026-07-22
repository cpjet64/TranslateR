# TranslateR Master Checklist

Use this file to track repo-local work batches for the commit-only multiagent
workflow.

## Rules

- Keep checklist items small enough to land in one coherent atomic commit or a
  short commit range.
- Keep each branch and worktree scoped to one task when branches or worktrees
  are used.
- Keep worktrees inside this repo under
  `<repo>/.worktrees/<branch-or-task-name>`.
- Do not create sibling worktrees outside the repository.
- Do not bundle unrelated fixes in the same commit.
- Each item must include matching tests, docs, or verification evidence when
  applicable.
- Do not mark an item complete without landing evidence and validation
  evidence.
- Landing evidence must be a commit hash, commit range, branch, pushed remote
  reference, or remote commit link.
- If a checklist item spins up work in another repo, record the target repo,
  reason, commit or branch reference, and triggering item.
- Push after every 10 commits at minimum, and sooner after meaningful
  milestones, completed checklist clusters, risky refactors, or before handoff.
- Remote pipelines may run after push when configured, but PR/MR review is not
  required unless repository protection or a separate user instruction requires
  it.
- Do not create releases, tags, package publishes, or deployments from checklist
  work unless the user explicitly instructs it.

## Current Batch

Use batches of up to 10 atomic commits. Replace the placeholders with scoped
tasks before work begins.

- [ ] 1. Enforce Windows release signature status, Curt P. Software leaf
  identity, and timestamp policy. Branch/worktree: `main` in isolated rollout
  clone. Plan: `docs/plans/curtpme-publisher-timestamp-verification.md`.
  Landing evidence: implementation commit pending. Validation:
  `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  `cargo test --workspace --all-features`, `git diff --check`, and GitLab CI
  lint passed. The no-signing policy regression is wired into Windows CI, but
  local execution is blocked because nested PowerShell processes hang before
  script output on this host; the first remote Windows pipeline must supply
  executable proof.
- [ ] 2. Task title. Branch/worktree: active branch. Landing evidence: pending. Validation: pending.
- [ ] 3. Task title. Branch/worktree: active branch. Landing evidence: pending. Validation: pending.
- [ ] 4. Task title. Branch/worktree: active branch. Landing evidence: pending. Validation: pending.
- [ ] 5. Task title. Branch/worktree: active branch. Landing evidence: pending. Validation: pending.
- [ ] 6. Task title. Branch/worktree: active branch. Landing evidence: pending. Validation: pending.
- [ ] 7. Task title. Branch/worktree: active branch. Landing evidence: pending. Validation: pending.
- [ ] 8. Task title. Branch/worktree: active branch. Landing evidence: pending. Validation: pending.
- [ ] 9. Task title. Branch/worktree: active branch. Landing evidence: pending. Validation: pending.
- [ ] 10. Task title. Branch/worktree: active branch. Landing evidence: pending. Validation: pending.

## Completed

- None yet.

## External Repo Follow-Ups

- None yet.

## Validation Notes

Before every push batch, install the repo-local hooks and run the local
validation commands documented in `AGENTS.md` and `CONTRIBUTING.md`.
