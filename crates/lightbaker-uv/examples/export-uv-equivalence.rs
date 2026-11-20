//! Writes `test-scenes/uv-equivalence/`: a scene of a quad, a box, a UV sphere and a torus knot
//! (`uv-equivalence.lbscene`) and its native unwrap (`native.json`), which the native and the browser path
//! (bridge + `xatlas-wasm`) are compared with (P3.6, docs/TESTING.md).
//!
//! `cargo run -p lightbaker-uv --example export-uv-equivalence`

use anyhow::Result;
use glam::Vec3;
use lightbaker::Scene;
use lightbaker::scene::MeshInfo;
use lightbaker::unwrap::{ChartOptions, UnwrapOptions, unwrap_scene};
use lightbaker_uv::XatlasUnwrapper;
use lightbaker_uv::shapes::*;

fn main() -> Result<()> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-scenes");
    let base = Scene::read(&std::fs::read(
        dir.join("upstream-basic/upstream-basic.lbscene"),
    )?)?;
    let (sphere, sphere_i) = uv_sphere(24, 12);
    let (knot, knot_i) = torus_knot(64, 8);
    let meshes: Vec<(&str, Vec<Vec3>)> = vec![
        (
            "quad",
            quad(
                Vec3::ZERO,
                Vec3::new(0., 0., 2.),
                Vec3::new(2., 0., 2.),
                Vec3::new(2., 0., 0.),
            ),
        ),
        (
            "box",
            box_faces()
                .iter()
                .map(|p| *p + Vec3::new(4., 0., 0.))
                .collect(),
        ),
        (
            "sphere",
            soup(&sphere, &sphere_i)
                .iter()
                .map(|p| *p + Vec3::new(8., 1., 0.))
                .collect(),
        ),
        (
            "knot",
            soup(&knot, &knot_i)
                .iter()
                .map(|p| *p + Vec3::new(0., 3., 8.))
                .collect(),
        ),
    ];
    let mut header = base.header.clone();
    header.meshes = meshes
        .iter()
        .map(|(name, p)| MeshInfo {
            name: name.to_string(),
            material: 0,
            triangles: (p.len() / 3) as u32,
            vertices: None,
            arrays: None,
        })
        .collect();
    header.lightmap.texels_per_meter = 16.0;
    header.lightmap.padding = 2;
    header.volume = None;
    header.user = serde_json::json!({ "test": "uv-equivalence" });
    let positions: Vec<Vec<Vec3>> = meshes.into_iter().map(|(_, p)| p).collect();
    let bytes = Scene::write(&header, &positions)?;
    let out = dir.join("uv-equivalence");
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join("uv-equivalence.lbscene"), &bytes)?;

    let scene = Scene::read(&bytes)?;
    let options = UnwrapOptions {
        texels_per_meter: 16.0,
        padding: 2,
        chart: ChartOptions::default(),
        brute_force: false,
    };
    let unwraps = unwrap_scene(&scene, &XatlasUnwrapper, &options)?;
    let meshes: Vec<_> = unwraps
        .iter()
        .map(|m| {
            anyhow::ensure!(m.report.valid(), "{}: {:?}", m.name, m.report);
            let t = &m.tile;
            Ok(serde_json::json!({
                "name": m.name, "width": t.width, "height": t.height, "charts": t.charts, "triangles": m.report.triangles,
                "xref": t.meshes[0].xref, "indices": t.meshes[0].indices,
                "uv": t.meshes[0].uv.iter().flatten().collect::<Vec<_>>(),
            }))
        })
        .collect::<Result<_>>()?;
    let json = serde_json::json!({
        "note": "native unwrap of uv-equivalence.lbscene (cargo run -p lightbaker-uv --example export-uv-equivalence)",
        "texelsPerMeter": 16, "padding": 2, "meshes": meshes,
    });
    std::fs::write(
        out.join("native.json"),
        serde_json::to_string(&json)? + "\n",
    )?;
    for m in &unwraps {
        println!(
            "{}: {} triangles, {}x{}, {} charts, {:.1} ms",
            m.name, m.report.triangles, m.tile.width, m.tile.height, m.tile.charts, m.ms
        );
    }
    Ok(())
}
