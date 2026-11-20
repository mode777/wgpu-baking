//! `.lbscene` v2 across languages: the Rust writer is pinned by `test-scenes/format-v2/rust.lbscene`, and the
//! file the TypeScript writer produced (`ts.lbscene`, js/test/scene.test.ts) reads back as the same scene.
//! Regenerate with `LIGHTBAKER_UPDATE_GOLDEN=1 cargo test -p lightbaker --test format_v2`.

use glam::Vec3;
use lightbaker::Scene;
use lightbaker::scene::*;

fn dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-scenes/format-v2")
}

/// The golden scene (js/test/scene.test.ts builds the same): an indexed quad with every array and two
/// materials, and a non-indexed triangle; an emissive, double-sided material and a light in the header.
fn golden() -> (SceneHeader, Vec<FileMesh>) {
    let material = |name: &str, albedo: [f32; 3], emission: [f32; 3], double_sided: bool| Material { name: name.into(), albedo, receives: true, casts: true, opacity: 1.0, emission, double_sided };
    let info = |name: &str| MeshInfo { name: name.into(), material: 0, triangles: 0, vertices: None, arrays: None };
    let header = SceneHeader {
        sun: Sun { direction: [-1.0, 2.0, 0.5], radius_deg: 0.5, irradiance: [3.0, 3.0, 3.0] },
        sky: Sky { zenith: [0.25, 0.25, 0.5], horizon: [0.5, 0.5, 0.5], ground: [0.125, 0.125, 0.125] },
        materials: vec![material("white", [0.75, 0.75, 0.75], [0.0; 3], false), material("lamp", [0.5, 0.25, 0.125], [4.0, 2.0, 1.0], true)],
        meshes: vec![info("quad"), info("triangle")],
        lightmap: LightmapSettings { texels_per_meter: 8.0, tile_size: 16.0, chart_angle_deg: 40.0, padding: 2, atlas_width: 256, indirect_downscale: 2 },
        bake: BakeSettings { sun_samples: 4, ao_rays: 16, ao_reach: 0.5, ao_strength: 1.0, ao_floor: 0.25, ao_contrast: 1.0, sdf_radius: 2.0, gi_rays: 32, gi_bounces: 2, gi_reach: 8.0, inside_share: 0.25, gi_filter: 1 },
        volume: None,
        user: serde_json::json!({ "source": "golden" }),
        lights: vec![serde_json::json!({ "type": "point", "position": [0.5, 2.0, 0.5], "color": [1.0, 0.75, 0.5], "intensity": 10.0, "range": 8.0 })],
    };
    let quad = FileMesh {
        positions: vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 1.0), Vec3::new(0.0, 0.0, 1.0)],
        normals: Some(vec![Vec3::Y; 4]),
        indices: Some(vec![0, 2, 1, 0, 3, 2]),
        material_ids: Some(vec![0, 1]),
        colors: Some(vec![[1.0, 0.5, 0.25], [0.5, 1.0, 0.25], [0.25, 0.5, 1.0], [1.0, 1.0, 1.0]]),
        lightmap_uv: Some(vec![[0.0, 0.0], [8.0, 0.0], [8.0, 8.0], [0.0, 8.0]]),
    };
    let triangle = FileMesh { positions: vec![Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 1.0, 1.0), Vec3::new(1.0, 1.0, 0.0)], ..Default::default() };
    (header, vec![quad, triangle])
}

#[test]
fn rust_writer_is_pinned() {
    let (header, meshes) = golden();
    let bytes = Scene::write_v2(&header, &meshes).unwrap();
    let path = dir().join("rust.lbscene");
    if std::env::var("LIGHTBAKER_UPDATE_GOLDEN").is_ok_and(|v| v == "1") {
        std::fs::create_dir_all(dir()).unwrap();
        std::fs::write(&path, &bytes).unwrap();
    }
    assert!(std::fs::read(&path).unwrap() == bytes, "the Rust v2 writer's output changed: rerun with LIGHTBAKER_UPDATE_GOLDEN=1 if intended");
}

#[test]
fn reads_the_typescript_writers_file() {
    let (header, meshes) = golden();
    let s = Scene::read(&std::fs::read(dir().join("ts.lbscene")).expect("generate it with UPDATE_GOLDEN=1 pnpm -C js test")).unwrap();
    assert_eq!(s.file.version, 2);
    assert_eq!(s.file.meshes, meshes);
    // numbers by value: the untyped `lights` keep JSON's 10 vs 10.0 apart
    fn floats(v: serde_json::Value) -> serde_json::Value {
        match v {
            serde_json::Value::Number(n) => serde_json::json!(n.as_f64().unwrap()),
            serde_json::Value::Array(a) => serde_json::Value::Array(a.into_iter().map(floats).collect()),
            serde_json::Value::Object(o) => serde_json::Value::Object(o.into_iter().map(|(k, v)| (k, floats(v))).collect()),
            other => other,
        }
    }
    let json = |h: &SceneHeader| {
        let mut v = serde_json::to_value(h).unwrap();
        v.as_object_mut().unwrap().remove("meshes");
        floats(v)
    };
    assert_eq!(json(&s.file.header), json(&header));
    assert_eq!(s.file.header.meshes.iter().map(|m| (m.name.as_str(), m.triangles, m.vertices)).collect::<Vec<_>>(), [("quad", 2, Some(4)), ("triangle", 1, Some(3))]);
    // the engine's view: the quad split by material, the triangle as it is
    assert_eq!(s.header.meshes.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), ["quad#0", "quad#1", "triangle"]);
    assert!(s.header.unsupported().is_err(), "the golden scene has a light, emission and doubleSided");
}
