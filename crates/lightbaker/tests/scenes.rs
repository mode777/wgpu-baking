//! The committed test scenes (`test-scenes/`): each is what its generator writes, bakes to the same bytes
//! twice, and shows what its `expect.json` says it must.

#[path = "common/upstream_basic.rs"]
mod upstream_basic;
#[path = "common/gpu.rs"]
mod gpu;

use std::path::PathBuf;

use gpu::gpu;
use lightbaker::cache::MemoryStore;
use lightbaker::expect::{Expect, Map, Probe};
use lightbaker::{Options, Scene, bake, output};

fn scenes_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-scenes")
}

#[test]
fn upstream_basic_is_the_upstream_test_scene() {
    let committed = std::fs::read(scenes_dir().join("upstream-basic/upstream-basic.lbscene")).expect("run the export-upstream-basic example");
    assert!(committed == upstream_basic::scene_bytes(8.0), "test-scenes/upstream-basic differs from the generator: rerun `cargo run -p lightbaker --example export-upstream-basic`");
    let expect = Expect::parse(&std::fs::read_to_string(scenes_dir().join("upstream-basic/expect.json")).unwrap()).unwrap();
    assert_eq!(expect.scene, "upstream-basic.lbscene");
    assert_eq!(expect.checks.len(), 19);
}

#[test]
fn upstream_basic_bakes_reproducibly_and_meets_its_expectations() {
    let Some(gpu) = gpu() else { return };
    let dir = scenes_dir().join("upstream-basic");
    let expect = Expect::parse(&std::fs::read_to_string(dir.join("expect.json")).unwrap()).unwrap();
    let scene = Scene::read(&std::fs::read(dir.join(&expect.scene)).unwrap()).unwrap();
    let log = |_: &str| {};
    let opts = Options { cache: None, log: &log };
    gpu.reset_transfers();
    let first = pollster::block_on(bake(&scene, &gpu, &opts)).unwrap();
    let t = gpu.transfers();
    assert!(t.batches > 0 && t.readbacks == t.batches && t.readback_bytes > 0 && t.upload_bytes > 0, "{t:?}");
    let second = pollster::block_on(bake(&scene, &gpu, &opts)).unwrap();
    assert!(output::write(&scene, &first).unwrap() == output::write(&scene, &second).unwrap(), "two bakes of the same scene differ");

    let report = expect.evaluate(&scene, &first).unwrap();
    for o in &report.outcomes {
        eprintln!("{} {}: {} vs {}", if o.passed { "ok  " } else { "FAIL" }, o.name, o.a, o.b);
    }
    assert!(report.passed());

    // a probe that is not on its mesh is an error naming it
    let mut bad = expect.clone();
    bad.probes.push(Probe { name: "nowhere".into(), map: Map::Direct, mesh: 0, point: [10.0, 5.0, 10.0] });
    let err = format!("{:#}", bad.evaluate(&scene, &first).unwrap_err());
    assert!(err.contains("probe nowhere"), "{err}");

    // the in-memory store (the browser's) reuses every tile and probe chunk of an unchanged scene
    let store = MemoryStore::new();
    let fresh = pollster::block_on(bake(&scene, &gpu, &Options { cache: Some(&store), log: &log })).unwrap();
    assert!(!store.is_empty());
    let again = pollster::block_on(bake(&scene, &gpu, &Options { cache: Some(&store), log: &log })).unwrap();
    assert_eq!((again.stats.tiles_baked, again.stats.chunks_baked), (0, 0));
    assert!(output::write(&scene, &again).unwrap() == output::write(&scene, &first).unwrap());
    assert!(output::write(&scene, &fresh).unwrap() == output::write(&scene, &first).unwrap());
}
