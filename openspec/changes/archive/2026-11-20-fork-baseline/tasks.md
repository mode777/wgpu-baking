# Tasks

## 1. Repository

- [x] 1.1 `git init -b main`, set `core.autocrlf=false`, commit existing planning files; verify `git log` shows one commit
- [x] 1.2 Add remote `upstream` (https://github.com/jer-nc/lightbaker) and fetch; verify `upstream/main` is `4fe85da`
- [x] 1.3 Merge `upstream/main` with `--allow-unrelated-histories`, moving upstream `README.md` to `docs/UPSTREAM_README.md`; verify `git merge upstream/main` reports "Already up to date" and `git diff upstream/main -- crates js Cargo.toml Cargo.lock LICENSE` is empty

## 2. Publish and docs

- [x] 2.1 Update README.md/AGENTS.md status ("fork imported") and the ADR-0001 link to `docs/UPSTREAM_README.md`; verify links resolve
- [x] 2.2 Create public GitHub repo `mode777/wgpu-baking` with `gh`, add as `origin`, push `main`; verify `git ls-remote origin main` matches local HEAD
- [x] 2.3 Tick P1.1 in ROADMAP.md

## Workflow follow-up

- Archive the change, commit on `main`, push.
