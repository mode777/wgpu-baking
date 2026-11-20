//! Native/web equivalence (P3.6): the native unwrap of `test-scenes/uv-equivalence` matches its fixture
//! (`native.json`), which the browser path (bridge + `xatlas-wasm`, `app/test/xatlas.test.ts`) matches too.
#![cfg(not(target_arch = "wasm32"))]

use lightbaker::Scene;
use lightbaker::unwrap::{ChartOptions, UnwrapOptions, unwrap_scene};
use lightbaker_uv::XatlasUnwrapper;
use serde_json::Value;

/// UVs may differ by this many texels between compilers (ADR-0006, P3.6).
const UV_TOLERANCE: f64 = 0.01;

#[test]
fn the_native_unwrap_matches_the_fixture() {
    let dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-scenes/uv-equivalence");
    let scene = Scene::read(&std::fs::read(dir.join("uv-equivalence.lbscene")).unwrap()).unwrap();
    let fixture: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("native.json")).unwrap()).unwrap();
    let options = UnwrapOptions {
        texels_per_meter: 16.0,
        padding: 2,
        chart: ChartOptions::default(),
        brute_force: false,
    };
    let unwraps = unwrap_scene(&scene, &XatlasUnwrapper, &options).unwrap();
    let expected = fixture["meshes"].as_array().unwrap();
    assert_eq!(unwraps.len(), expected.len());
    for (m, e) in unwraps.iter().zip(expected) {
        let name = &m.name;
        assert!(m.report.valid(), "{name}: {:?}", m.report);
        let (t, out) = (&m.tile, &m.tile.meshes[0]);
        let ints = |k: &str| {
            e[k].as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u32)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            (name.as_str(), t.width, t.height, t.charts),
            (
                e["name"].as_str().unwrap(),
                ints_of(e, "width"),
                ints_of(e, "height"),
                ints_of(e, "charts")
            ),
            "{name}: tile"
        );
        assert!(out.xref == ints("xref"), "{name}: xref differs");
        assert!(out.indices == ints("indices"), "{name}: indices differ");
        let uv: Vec<f64> = e["uv"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        let worst = out
            .uv
            .iter()
            .flatten()
            .zip(&uv)
            .map(|(a, b)| (*a as f64 - b).abs())
            .fold(0.0, f64::max);
        assert!(
            worst <= UV_TOLERANCE,
            "{name}: UVs differ by up to {worst} texels"
        );
    }
}

fn ints_of(e: &Value, k: &str) -> u32 {
    e[k].as_u64().unwrap() as u32
}
