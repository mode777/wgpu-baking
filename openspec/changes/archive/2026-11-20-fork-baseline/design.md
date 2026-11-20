# Design: fork-baseline

## Context

The working folder already contains our docs, `openspec/`, `.github/` and `.opencode/`. Upstream
(jer-nc/lightbaker, `main` = `4fe85da`) has its own `README.md` and a `docs/` folder (`DESIGN.md`,
`FORMAT.md`). ADR-0001 fixes the strategy: full history, remote `upstream`, our work on `main`.

## Goals / Non-Goals

**Goals:** upstream history reachable from `main`; `git merge upstream/main` is a no-op afterwards; no
upstream file modified except the README move.

**Non-Goals:** building or testing the code (P1.2/P1.5); branches (all work lands on `main`).

## Decisions

- **Merge, not rebase or squash.** `git merge --allow-unrelated-histories upstream/main` keeps upstream
  commits intact, so later `git merge upstream/main` works incrementally. A squash would break that.
- **Order: commit our docs first, then merge.** The only path conflict is `README.md`. Resolve it in the
  merge commit: upstream's version goes to `docs/UPSTREAM_README.md` (`git show upstream/main:README.md`),
  ours stays at `README.md`. Doing the move inside the merge commit means `git log --follow` shows it
  clearly and no upstream file is otherwise touched.
- **Public origin via `gh repo create mode777/wgpu-baking --public`**, as the project owner decided.
  Upstream is MIT, so publishing the fork is permitted as long as the licence text is kept (it is).

## Risks / Trade-offs

- [Upstream `docs/` collides with our `docs/`] → upstream only adds `DESIGN.md`/`FORMAT.md`; verified
  with `git merge` output; any other collision is resolved in favour of keeping both files.
- [Line-ending churn on Windows] → set `core.autocrlf=false` locally before the merge so upstream files
  stay byte-identical.

GPU-to-CPU readbacks, blocking calls, native-only dependencies: none (no code changes).
ADRs: none created or updated (implements ADR-0001).
