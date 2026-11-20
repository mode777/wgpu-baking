//! The scene upstream's GPU test bakes (ground, a grey box, a red wall, a narrow strip), shared by
//! `tests/bake.rs`, `tests/scenes.rs` and the `export-upstream-basic` example that writes
//! `test-scenes/upstream-basic/`.
#![allow(dead_code)] // each includer uses part of it

use glam::Vec3;
use lightbaker::Scene;
use lightbaker::scene::*;

pub fn quad(a: Vec3, b: Vec3, c: Vec3, d: Vec3) -> Vec<Vec3> {
    vec![a, b, c, a, c, d]
}

/// A closed box (faces outward, counter-clockwise seen from outside).
pub fn cube(min: Vec3, max: Vec3) -> Vec<Vec3> {
    let p = |x: f32, y: f32, z: f32| Vec3::new(if x > 0.0 { max.x } else { min.x }, if y > 0.0 { max.y } else { min.y }, if z > 0.0 { max.z } else { min.z });
    [
        quad(p(0., 1., 0.), p(0., 1., 1.), p(1., 1., 1.), p(1., 1., 0.)),
        quad(p(0., 0., 0.), p(1., 0., 0.), p(1., 0., 1.), p(0., 0., 1.)),
        quad(p(0., 0., 1.), p(1., 0., 1.), p(1., 1., 1.), p(0., 1., 1.)),
        quad(p(1., 0., 0.), p(0., 0., 0.), p(0., 1., 0.), p(1., 1., 0.)),
        quad(p(1., 0., 1.), p(1., 0., 0.), p(1., 1., 0.), p(1., 1., 1.)),
        quad(p(0., 0., 0.), p(0., 0., 1.), p(0., 1., 1.), p(0., 1., 0.)),
    ]
    .concat()
}

/// Ground 64 x 64 m in a grid of 4 m quads, a grey box at (`bx`, 0, 10), a red wall along x = 40.
pub fn scene(bx: f32) -> Scene {
    Scene::read(&scene_bytes(bx)).unwrap()
}

/// The scene file of [scene].
pub fn scene_bytes(bx: f32) -> Vec<u8> {
    let mut ground = Vec::new();
    for i in 0..16 {
        for j in 0..16 {
            let (x, z) = (i as f32 * 4.0, j as f32 * 4.0);
            ground.extend(quad(Vec3::new(x, 0., z), Vec3::new(x, 0., z + 4.), Vec3::new(x + 4., 0., z + 4.), Vec3::new(x + 4., 0., z)));
        }
    }
    let boxes = cube(Vec3::new(bx, 0., 10.), Vec3::new(bx + 2., 2., 12.));
    let wall = cube(Vec3::new(40., 0., 30.), Vec3::new(40.5, 4., 50.));
    // a strip narrower than a texel standing in the open (a sheet's fold, a plank's edge), facing -x
    let strip = quad(Vec3::new(30., 0.2, 20.), Vec3::new(30., 2.6, 20.), Vec3::new(30., 2.6, 20.05), Vec3::new(30., 0.2, 20.05));
    let meshes = [("ground", 0, &ground), ("box", 1, &boxes), ("wall", 2, &wall), ("strip", 1, &strip)];
    let header = SceneHeader {
        // low in the west (-x), so a box's shadow falls to its east
        sun: Sun { direction: [-1.0, 0.8, 0.0], radius_deg: 0.5, irradiance: [3.0, 3.0, 3.0] },
        sky: Sky { zenith: [0.2, 0.25, 0.35], horizon: [0.3, 0.3, 0.32], ground: [0.1, 0.1, 0.1] },
        materials: vec![
            Material { name: "ground".into(), albedo: [0.5, 0.5, 0.5], receives: true, casts: true, opacity: 1.0 },
            Material { name: "grey".into(), albedo: [0.5, 0.5, 0.5], receives: true, casts: true, opacity: 1.0 },
            Material { name: "red".into(), albedo: [0.9, 0.05, 0.05], receives: true, casts: true, opacity: 1.0 },
        ],
        meshes: meshes.iter().map(|(n, m, p)| MeshInfo { name: n.to_string(), material: *m, triangles: (p.len() / 3) as u32 }).collect(),
        lightmap: LightmapSettings { texels_per_meter: 4.0, tile_size: 16.0, chart_angle_deg: 40.0, padding: 2, atlas_width: 1024, indirect_downscale: 2 },
        bake: BakeSettings {
            sun_samples: 16,
            ao_rays: 64,
            ao_reach: 0.8,
            ao_strength: 1.0,
            ao_floor: 0.25,
            ao_contrast: 1.0,
            sdf_radius: 2.0,
            gi_rays: 256,
            gi_bounces: 2,
            gi_reach: 12.0,
            inside_share: 0.25,
            gi_filter: 1,
        },
        volume: Some(VolumeSettings { min: [0., 0.5, 0.], max: [64., 4.5, 64.], step: 2.0, rays: 128 }),
        user: serde_json::json!({ "test": true }),
    };
    let positions: Vec<Vec<Vec3>> = meshes.iter().map(|(_, _, p)| (*p).clone()).collect();
    Scene::write(&header, &positions).unwrap()
}
