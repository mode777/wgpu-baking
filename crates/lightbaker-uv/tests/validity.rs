//! UV validity suite (docs/TESTING.md "UV generation tests") for the native xatlas unwrapper.
#![cfg(not(target_arch = "wasm32"))]

use glam::Vec3;
use lightbaker_uv::shapes::*;
use lightbaker_uv::*;

fn options(texels_per_meter: f32) -> UnwrapOptions {
    UnwrapOptions {
        texels_per_meter,
        padding: 2,
        chart: ChartOptions::default(),
        brute_force: false,
    }
}

fn input<'a>(positions: &'a [[f32; 3]], indices: Option<&'a [u32]>) -> MeshInput<'a> {
    MeshInput {
        positions,
        indices,
        normals: None,
        material_ids: None,
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

fn arrays(input: &[Vec3]) -> Vec<[f32; 3]> {
    input.iter().map(|p| p.to_array()).collect()
}

#[test]
fn a_quad_tile_is_sized_by_the_density() {
    let p = arrays(&quad(
        Vec3::ZERO,
        Vec3::new(0., 0., 2.),
        Vec3::new(2., 0., 2.),
        Vec3::new(2., 0., 0.),
    ));
    let tile = unwrap_valid("quad", &[input(&p, None)], 16.0);
    assert_eq!(tile.charts, 1);
    // 2 m at 16 texels a metre, plus padding
    for side in [tile.width, tile.height] {
        assert!((32..=40).contains(&side), "{}x{}", tile.width, tile.height);
    }
}

#[test]
fn charts_never_span_materials() {
    // a box whose faces are three materials, two faces each
    let p = arrays(&box_faces());
    let materials: Vec<u32> = (0..12).map(|t| t / 4).collect();
    let m = MeshInput {
        material_ids: Some(&materials),
        ..input(&p, None)
    };
    let tile = unwrap_valid("3-material box", &[m], 16.0);
    assert!(tile.charts >= 3);
}

#[test]
fn the_upstream_basic_meshes() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../test-scenes/upstream-basic/upstream-basic.lbscene"
    );
    let scene = lightbaker::Scene::read(&std::fs::read(path).unwrap()).unwrap();
    let tpm = scene.header.lightmap.texels_per_meter;
    let positions: Vec<Vec<[f32; 3]>> = scene.positions.iter().map(|p| arrays(p)).collect();
    for (info, p) in scene.header.meshes.iter().zip(&positions) {
        unwrap_valid(&info.name, &[input(p, None)], tpm);
    }
    // all of them in one tile, too
    let all: Vec<MeshInput> = positions.iter().map(|p| input(p, None)).collect();
    unwrap_valid("upstream-basic", &all, tpm);
}

#[test]
fn a_uv_sphere() {
    let (p, i) = uv_sphere(48, 24);
    unwrap_valid("sphere", &[input(&p, Some(&i))], 32.0);
}

#[test]
fn a_torus_knot_deterministically() {
    let (p, i) = torus_knot(256, 32);
    assert_eq!(i.len() / 3, 16384);
    let m = input(&p, Some(&i));
    let first = unwrap_valid("torus knot", &[m], 16.0);
    let second = XatlasUnwrapper.unwrap(&[m], &options(16.0)).unwrap();
    assert!(first == second, "a second unwrap differs");
}

#[test]
fn bad_input_is_an_error() {
    let p = [[0.0f32; 3]; 4];
    assert!(
        XatlasUnwrapper
            .unwrap(&[input(&p, None)], &options(4.0))
            .is_err()
    );
    let p = [[0.0f32; 3]; 3];
    assert!(
        XatlasUnwrapper
            .unwrap(&[input(&p, None)], &options(0.0))
            .is_err()
    );
}
