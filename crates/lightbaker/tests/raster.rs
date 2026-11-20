//! Texels on triangles (raster.rs, ADR-0004 decision 2): points and normals from the triangles' UVs, vertex
//! normals interpolated when the mesh has them.

use glam::Vec3;
use lightbaker::Scene;
use lightbaker::layout::Layout;
use lightbaker::raster::rasterize;
use lightbaker::scene::*;

fn scene(normals: Option<Vec<Vec3>>) -> Scene {
    let header = SceneHeader {
        sun: Sun { direction: [0.0, 1.0, 0.0], radius_deg: 0.5, irradiance: [1.0; 3] },
        sky: Sky { zenith: [0.1; 3], horizon: [0.1; 3], ground: [0.1; 3] },
        materials: vec![Material { name: "m".into(), albedo: [0.5; 3], receives: true, casts: true, opacity: 1.0, emission: [0.0; 3], double_sided: false }],
        meshes: vec![MeshInfo { name: "quad".into(), material: 0, triangles: 0, vertices: None, arrays: None }],
        lightmap: LightmapSettings { texels_per_meter: 8.0, tile_size: 16.0, chart_angle_deg: 40.0, padding: 2, atlas_width: 256, indirect_downscale: 2 },
        bake: BakeSettings { sun_samples: 4, ao_rays: 8, ao_reach: 0.8, ao_strength: 1.0, ao_floor: 0.25, ao_contrast: 1.0, sdf_radius: 2.0, gi_rays: 8, gi_bounces: 1, gi_reach: 4.0, inside_share: 0.25, gi_filter: 1 },
        volume: None,
        user: serde_json::Value::Null,
        lights: vec![],
    };
    // a 4 m x 4 m quad on the ground, facing up
    let mesh = FileMesh { positions: vec![Vec3::ZERO, Vec3::new(4.0, 0.0, 0.0), Vec3::new(4.0, 0.0, 4.0), Vec3::new(0.0, 0.0, 4.0)], normals, indices: Some(vec![0, 2, 1, 0, 3, 2]), ..Default::default() };
    Scene::read(&Scene::write_v2(&header, &[mesh]).unwrap()).unwrap()
}

/// Every covered texel's normal.
fn normals(s: &Scene) -> Vec<Vec3> {
    let layout = Layout::build(s).unwrap();
    layout.tiles.iter().flat_map(|t| rasterize(s, &layout, t)).filter(|t| t.covered()).map(|t| t.n()).collect()
}

#[test]
fn faces_without_vertex_normals_keep_the_face_normal() {
    let n = normals(&scene(None));
    assert!(n.len() > 100);
    assert!(n.iter().all(|n| (*n - Vec3::Y).length() < 1e-6));
}

#[test]
fn vertex_normals_are_interpolated() {
    // tilted ±30° about z across x: a quad shaded as if it bulged
    let (s, c) = (30f32.to_radians().sin(), 30f32.to_radians().cos());
    let (left, right) = (Vec3::new(-s, c, 0.0), Vec3::new(s, c, 0.0));
    let n = normals(&scene(Some(vec![left, right, right, left])));
    assert!(n.iter().all(|n| (n.length() - 1.0).abs() < 1e-4), "unit length");
    assert!(n.iter().all(|n| n.x >= -s - 1e-4 && n.x <= s + 1e-4), "between the corners' normals");
    let (min, max) = n.iter().fold((f32::MAX, f32::MIN), |(a, b), n| (a.min(n.x), b.max(n.x)));
    assert!(min < -0.3 && max > 0.3, "varies across the quad: {min} … {max}");
}

#[test]
fn normals_turned_away_fall_back_to_the_face() {
    let n = normals(&scene(Some(vec![Vec3::Y, Vec3::Y, Vec3::Y, -Vec3::Y])));
    assert!(n.iter().all(|n| n.dot(Vec3::Y) >= 0.1), "no texel normal turns away from its face");
    assert!(n.iter().any(|n| (*n - Vec3::Y).length() < 1e-6), "the face normal is used where the interpolation turns away");
}
