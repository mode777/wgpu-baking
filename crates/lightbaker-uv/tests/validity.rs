//! UV validity suite (docs/TESTING.md "UV generation tests") for the native xatlas unwrapper.
#![cfg(not(target_arch = "wasm32"))]

use std::f32::consts::TAU;

use glam::Vec3;
use lightbaker_uv::*;

fn options(texels_per_meter: f32) -> UnwrapOptions {
    UnwrapOptions {
        texels_per_meter,
        padding: 2,
        chart: ChartOptions::default(),
        brute_force: false,
    }
}

/// Unwraps the meshes as one tile, asserts every check passes and the median density is within ±20 %.
fn unwrap_valid(name: &str, meshes: &[MeshInput], texels_per_meter: f32) -> UnwrappedTile {
    let tile = XatlasUnwrapper
        .unwrap(meshes, &options(texels_per_meter))
        .unwrap();
    assert_eq!(tile.meshes.len(), meshes.len(), "{name}");
    assert!(
        tile.width > 0 && tile.height > 0,
        "{name}: {}x{}",
        tile.width,
        tile.height
    );
    for (i, (m, out)) in meshes.iter().zip(&tile.meshes).enumerate() {
        let r = check_uvs(m, out, tile.width, tile.height, texels_per_meter);
        let triangles = m.indices.map_or(m.positions.len(), <[u32]>::len) / 3;
        assert_eq!(r.triangles, triangles, "{name} mesh {i}");
        assert!(r.valid(), "{name} mesh {i}: {r:?}");
        // 0: no chart big enough to measure, allowed for a mesh too small to have one (upstream-basic's strip)
        let tiny = mesh_area(m) * texels_per_meter * texels_per_meter < 64.0;
        let density_ok = if r.density == [0.0; 3] {
            tiny
        } else {
            (0.8..=1.2).contains(&r.density[1])
        };
        assert!(density_ok, "{name} mesh {i}: density {:?}", r.density);
    }
    tile
}

fn mesh_area(m: &MeshInput) -> f32 {
    let corner = |t: usize, k: usize| {
        Vec3::from(m.positions[m.indices.map_or(t * 3 + k, |i| i[t * 3 + k] as usize)])
    };
    let triangles = m.indices.map_or(m.positions.len(), <[u32]>::len) / 3;
    (0..triangles)
        .map(|t| {
            (corner(t, 1) - corner(t, 0))
                .cross(corner(t, 2) - corner(t, 0))
                .length()
                / 2.0
        })
        .sum()
}

fn soup(input: &[Vec3]) -> Vec<[f32; 3]> {
    input.iter().map(|p| p.to_array()).collect()
}

fn quad(a: Vec3, b: Vec3, c: Vec3, d: Vec3) -> Vec<Vec3> {
    vec![a, b, c, a, c, d]
}

/// A parametric surface, `u` × `v` quads, indexed, wrapping in both directions.
fn surface(u: u32, v: u32, f: impl Fn(f32, f32) -> Vec3) -> (Vec<[f32; 3]>, Vec<u32>) {
    let mut positions = Vec::new();
    for j in 0..=v {
        for i in 0..=u {
            positions.push(f(i as f32 / u as f32, j as f32 / v as f32).to_array());
        }
    }
    let at = |i: u32, j: u32| j * (u + 1) + i;
    let mut indices = Vec::new();
    for j in 0..v {
        for i in 0..u {
            indices.extend([
                at(i, j),
                at(i + 1, j),
                at(i + 1, j + 1),
                at(i, j),
                at(i + 1, j + 1),
                at(i, j + 1),
            ]);
        }
    }
    (positions, indices)
}

fn uv_sphere() -> (Vec<[f32; 3]>, Vec<u32>) {
    let (mut p, i) = surface(48, 24, |u, v| {
        let (theta, phi) = (u * TAU, v * TAU / 2.0);
        Vec3::new(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin())
    });
    // collapse each pole row onto its pole exactly
    for (k, q) in p.iter_mut().enumerate() {
        let row = k / 49;
        if row == 0 || row == 24 {
            *q = [0.0, if row == 0 { 1.0 } else { -1.0 }, 0.0];
        }
    }
    // drop the zero-area pole triangles
    let i = i.chunks(3).filter(|t| {
        let [a, b, c] = [0, 1, 2].map(|k| Vec3::from(p[t[k] as usize]));
        (b - a).cross(c - a).length() > 1e-9
    });
    (p.clone(), i.flatten().copied().collect())
}

/// A (2, 3) torus knot tube, `segments` × `sides` quads.
fn torus_knot(segments: u32, sides: u32) -> (Vec<[f32; 3]>, Vec<u32>) {
    let curve = |t: f32| {
        let (p, q) = (2.0 * t * TAU, 3.0 * t * TAU);
        let r = 2.0 + q.cos();
        Vec3::new(r * p.cos(), q.sin(), r * p.sin())
    };
    surface(segments, sides, |s, a| {
        let (c, next) = (curve(s), curve(s + 1e-3));
        let tangent = (next - c).normalize();
        let n = tangent.cross(Vec3::Y).normalize();
        let b = tangent.cross(n);
        let angle = a * TAU;
        c + 0.4 * (n * angle.cos() + b * angle.sin())
    })
}

#[test]
fn a_quad_tile_is_sized_by_the_density() {
    let p = soup(&quad(
        Vec3::ZERO,
        Vec3::new(0., 0., 2.),
        Vec3::new(2., 0., 2.),
        Vec3::new(2., 0., 0.),
    ));
    let m = MeshInput {
        positions: &p,
        indices: None,
        normals: None,
        material_ids: None,
    };
    let tile = unwrap_valid("quad", &[m], 16.0);
    assert_eq!(tile.charts, 1);
    // 2 m at 16 texels a metre, plus padding
    for side in [tile.width, tile.height] {
        assert!((32..=40).contains(&side), "{}x{}", tile.width, tile.height);
    }
}

#[test]
fn charts_never_span_materials() {
    // a box whose faces are three materials, two faces each
    let p = soup(&box_faces());
    let materials: Vec<u32> = (0..12).map(|t| t / 4).collect();
    let m = MeshInput {
        positions: &p,
        indices: None,
        normals: None,
        material_ids: Some(&materials),
    };
    let tile = unwrap_valid("3-material box", &[m], 16.0);
    assert!(tile.charts >= 3);
}

fn box_faces() -> Vec<Vec3> {
    let p = |x: f32, y: f32, z: f32| Vec3::new(x, y, z);
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

#[test]
fn the_upstream_basic_meshes() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-scenes/upstream-basic/upstream-basic.lbscene"
    );
    let scene = lightbaker::Scene::read(&std::fs::read(path).unwrap()).unwrap();
    let positions: Vec<Vec<[f32; 3]>> = scene.positions.iter().map(|p| soup(p)).collect();
    for (info, p) in scene.header.meshes.iter().zip(&positions) {
        let m = MeshInput {
            positions: p,
            indices: None,
            normals: None,
            material_ids: None,
        };
        unwrap_valid(&info.name, &[m], scene.header.lightmap.texels_per_meter);
    }
    // all of them in one tile, too
    let all: Vec<MeshInput> = positions
        .iter()
        .map(|p| MeshInput {
            positions: p,
            indices: None,
            normals: None,
            material_ids: None,
        })
        .collect();
    unwrap_valid(
        "upstream-basic",
        &all,
        scene.header.lightmap.texels_per_meter,
    );
}

#[test]
fn a_uv_sphere() {
    let (p, i) = uv_sphere();
    let m = MeshInput {
        positions: &p,
        indices: Some(&i),
        normals: None,
        material_ids: None,
    };
    unwrap_valid("sphere", &[m], 32.0);
}

#[test]
fn a_torus_knot_deterministically() {
    let (p, i) = torus_knot(256, 32);
    assert_eq!(i.len() / 3, 16384);
    let m = MeshInput {
        positions: &p,
        indices: Some(&i),
        normals: None,
        material_ids: None,
    };
    let first = unwrap_valid("torus knot", &[m], 16.0);
    let second = XatlasUnwrapper.unwrap(&[m], &options(16.0)).unwrap();
    assert!(first == second, "a second unwrap differs");
}

#[test]
fn bad_input_is_an_error() {
    let p = [[0.0f32; 3]; 4];
    let m = MeshInput {
        positions: &p,
        indices: None,
        normals: None,
        material_ids: None,
    };
    assert!(XatlasUnwrapper.unwrap(&[m], &options(4.0)).is_err());
    let p = [[0.0f32; 3]; 3];
    let m = MeshInput {
        positions: &p,
        indices: None,
        normals: None,
        material_ids: None,
    };
    assert!(XatlasUnwrapper.unwrap(&[m], &options(0.0)).is_err());
}
