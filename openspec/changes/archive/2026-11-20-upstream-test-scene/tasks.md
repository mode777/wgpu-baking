# Tasks

## 1. Scene and expectations

- [x] 1.1 Move `quad`/`cube`/`scene` to `crates/lightbaker/tests/common/upstream_basic.rs`, include it from `tests/bake.rs`; verify `cargo test -p lightbaker --release --test bake` still passes on the GPU
- [x] 1.2 Add `lightbaker::expect` (types, operand parser, point→texel sampling, evaluation) with unit tests for parsing and a probe off the mesh; verify `cargo test -p lightbaker --lib expect`
- [x] 1.3 Add example `export-upstream-basic` writing `.lbscene` and `expect.json` (upstream assertions as checks; `--bake` adds reference values); generate `test-scenes/upstream-basic/`; verify the files exist and the example is deterministic (run twice, identical)
- [x] 1.4 Add integration test `tests/scenes.rs`: committed scene equals generator output; bake twice → identical `.lbake`; all checks pass; verify on the Intel GPU
- [x] 1.5 Verify the wasm32 build status of `lightbaker` is unchanged (still blocked by zstd-sys until P2.1; the new module adds no native-only code)

## 2. CLI

- [x] 2.1 Add `--check` to `lightbaker-cli`; verify `lightbaker test-scenes/upstream-basic/upstream-basic.lbscene --check` passes, and two CLI runs give byte-identical `.lbake` files

## 3. Docs

- [x] 3.1 Document `test-scenes/`, the `expect.json` format and regeneration in TESTING.md and BUILD.md; tick P1.4 in ROADMAP.md

## Workflow follow-up

- Archive (sync `test-scenes` spec), commit on `main`, push, confirm CI green.
