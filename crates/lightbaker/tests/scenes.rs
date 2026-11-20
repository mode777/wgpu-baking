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
fn progress_and_cancel() {
    use lightbaker::progress::{Cancelled, Phase, Progress};
    use std::cell::{Cell, RefCell};
    let Some(gpu) = gpu() else { return };
    let dir = scenes_dir().join("upstream-basic");
    let scene = Scene::read(&std::fs::read(dir.join("upstream-basic.lbscene")).unwrap()).unwrap();

    // every phase reports, in order, and ends with done = total
    let seen: RefCell<Vec<Progress>> = RefCell::new(Vec::new());
    let progress = |p: Progress| seen.borrow_mut().push(p);
    let full = pollster::block_on(bake(&scene, &gpu, &Options { progress: Some(&progress), ..Default::default() })).unwrap();
    let seen = seen.into_inner();
    let mut ends: Vec<(Phase, u32)> = Vec::new();
    for (i, p) in seen.iter().enumerate() {
        assert!(p.done <= p.total, "{p:?}");
        let last_of_phase = seen.get(i + 1).is_none_or(|q| (q.phase, q.bounce) != (p.phase, p.bounce));
        if last_of_phase {
            assert_eq!(p.done, p.total, "{p:?} ends its phase");
            ends.push((p.phase, p.bounce));
        }
    }
    assert_eq!(ends, [(Phase::Layout, 0), (Phase::Direct, 0), (Phase::Bounce, 1), (Phase::Bounce, 2), (Phase::Volume, 0)]);

    // cancelled after the first direct batch: no further batch, a Cancelled error
    gpu.reset_transfers();
    let stop = Cell::new(false);
    let progress = |p: Progress| {
        if p.phase == Phase::Direct {
            stop.set(true);
        }
    };
    let cancel = || stop.get();
    let err = pollster::block_on(bake(&scene, &gpu, &Options { progress: Some(&progress), cancel: Some(&cancel), ..Default::default() })).err().expect("the bake was cancelled");
    assert!(err.downcast_ref::<Cancelled>().is_some(), "{err:#}");
    assert_eq!(gpu.transfers().batches, 1, "cancelled within one batch");

    // the next bake is unaffected
    let again = pollster::block_on(bake(&scene, &gpu, &Options::default())).unwrap();
    assert!(output::write(&scene, &again).unwrap() == output::write(&scene, &full).unwrap());
}

#[test]
fn cancelled_bake_leaves_the_store_unchanged() {
    use lightbaker::progress::{Cancelled, Phase, Progress};
    use std::cell::Cell;
    let Some(gpu) = gpu() else { return };
    let scene = Scene::read(&std::fs::read(scenes_dir().join("upstream-basic/upstream-basic.lbscene")).unwrap()).unwrap();
    let store = MemoryStore::new();
    let stop = Cell::new(false);
    let progress = |p: Progress| {
        if p.phase == Phase::Bounce {
            stop.set(true);
        }
    };
    let cancel = || stop.get();
    let err = pollster::block_on(bake(&scene, &gpu, &Options { cache: Some(&store), progress: Some(&progress), cancel: Some(&cancel), ..Default::default() })).err().expect("the bake was cancelled");
    assert!(err.downcast_ref::<Cancelled>().is_some(), "{err:#}");
    assert!(store.is_empty(), "a cancelled bake wrote {} entries", store.len());
    let next = pollster::block_on(bake(&scene, &gpu, &Options { cache: Some(&store), ..Default::default() })).unwrap();
    assert_eq!(next.stats.tiles_baked, next.stats.tiles);
    let fresh = pollster::block_on(bake(&scene, &gpu, &Options::default())).unwrap();
    assert!(output::write(&scene, &next).unwrap() == output::write(&scene, &fresh).unwrap());
}

#[test]
fn device_lost_mid_bake() {
    use lightbaker::gpu::GpuError;
    use lightbaker::progress::{Phase, Progress};
    let Some(gpu) = gpu() else { return };
    let scene = Scene::read(&std::fs::read(scenes_dir().join("upstream-basic/upstream-basic.lbscene")).unwrap()).unwrap();
    let lost = gpu.take_device_lost().expect("the receiver is taken once");
    assert!(gpu.take_device_lost().is_none());
    let progress = |p: Progress| {
        if p.phase == Phase::Direct {
            gpu.destroy();
        }
    };
    let err = pollster::block_on(bake(&scene, &gpu, &Options { progress: Some(&progress), ..Default::default() })).err().expect("the bake fails");
    assert!(matches!(err.downcast_ref::<GpuError>(), Some(GpuError::DeviceLost(_))), "{err:#}");
    let why = pollster::block_on(lost).expect("the lost callback fired");
    assert!(why.contains("Destroyed"), "{why}");
    // the device stays lost
    assert!(matches!(gpu.check(), Err(GpuError::DeviceLost(_))));
}

#[test]
fn indexed_v2_bakes_like_v1() {
    use lightbaker::scene::FileMesh;
    let Some(gpu) = gpu() else { return };
    let v1 = Scene::read(&std::fs::read(scenes_dir().join("upstream-basic/upstream-basic.lbscene")).unwrap()).unwrap();
    // the same triangles, indexed: shared corners welded by exact position
    let meshes: Vec<FileMesh> = v1
        .positions
        .iter()
        .map(|p| {
            let mut seen = std::collections::HashMap::new();
            let mut positions = Vec::new();
            let indices = p.iter().map(|v| *seen.entry(v.to_array().map(f32::to_bits)).or_insert_with(|| { positions.push(*v); positions.len() as u32 - 1 })).collect();
            FileMesh { positions, indices: Some(indices), ..Default::default() }
        })
        .collect();
    let v2 = Scene::read(&Scene::write_v2(&v1.header, &meshes).unwrap()).unwrap();
    assert_eq!(v2.file.version, 2);
    assert!(v2.file.meshes[0].positions.len() < v1.positions[0].len(), "the ground's corners are shared");
    assert_eq!(v2.positions, v1.positions);
    let a = pollster::block_on(bake(&v1, &gpu, &Options::default())).unwrap();
    let b = pollster::block_on(bake(&v2, &gpu, &Options::default())).unwrap();
    assert!(a.direct == b.direct && a.indirect == b.indirect && a.validity == b.validity, "v1 and indexed v2 bake to different light");
}

#[test]
fn upstream_basic_bakes_reproducibly_and_meets_its_expectations() {
    let Some(gpu) = gpu() else { return };
    let dir = scenes_dir().join("upstream-basic");
    let expect = Expect::parse(&std::fs::read_to_string(dir.join("expect.json")).unwrap()).unwrap();
    let scene = Scene::read(&std::fs::read(dir.join(&expect.scene)).unwrap()).unwrap();
    let log = |_: &str| {};
    let opts = Options { cache: None, log: &log, ..Default::default() };
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
    let fresh = pollster::block_on(bake(&scene, &gpu, &Options { cache: Some(&store), log: &log, ..Default::default() })).unwrap();
    assert!(!store.is_empty());
    let again = pollster::block_on(bake(&scene, &gpu, &Options { cache: Some(&store), log: &log, ..Default::default() })).unwrap();
    assert_eq!((again.stats.tiles_baked, again.stats.chunks_baked), (0, 0));
    assert!(output::write(&scene, &again).unwrap() == output::write(&scene, &first).unwrap());
    assert!(output::write(&scene, &fresh).unwrap() == output::write(&scene, &first).unwrap());
}
