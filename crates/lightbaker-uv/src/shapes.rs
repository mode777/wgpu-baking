//! Test meshes shared by the validity suite and the equivalence fixture (`examples/export-uv-equivalence.rs`).

use std::f32::consts::TAU;

use glam::Vec3;

/// Two triangles, `a b c` and `a c d`.
pub fn quad(a: Vec3, b: Vec3, c: Vec3, d: Vec3) -> Vec<Vec3> {
    vec![a, b, c, a, c, d]
}

/// A unit box as a triangle soup, faces outward; triangles 4k … 4k + 3 are two faces.
pub fn box_faces() -> Vec<Vec3> {
    let p = Vec3::new;
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

/// A parametric surface of `u` × `v` quads, indexed (the seam rows/columns are separate vertices).
pub fn surface(u: u32, v: u32, f: impl Fn(f32, f32) -> Vec3) -> (Vec<[f32; 3]>, Vec<u32>) {
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

/// A unit UV sphere of `u` × `v` quads, poles collapsed and their zero-area triangles dropped.
pub fn uv_sphere(u: u32, v: u32) -> (Vec<[f32; 3]>, Vec<u32>) {
    let (mut p, i) = surface(u, v, |s, t| {
        let (theta, phi) = (s * TAU, t * TAU / 2.0);
        Vec3::new(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin())
    });
    for (k, q) in p.iter_mut().enumerate() {
        let row = k as u32 / (u + 1);
        if row == 0 || row == v {
            *q = [0.0, if row == 0 { 1.0 } else { -1.0 }, 0.0];
        }
    }
    let i = i
        .chunks(3)
        .filter(|t| {
            let [a, b, c] = [0, 1, 2].map(|k| Vec3::from(p[t[k] as usize]));
            (b - a).cross(c - a).length() > 1e-9
        })
        .flatten()
        .copied()
        .collect();
    (p, i)
}

/// A (2, 3) torus knot tube of `segments` × `sides` quads.
pub fn torus_knot(segments: u32, sides: u32) -> (Vec<[f32; 3]>, Vec<u32>) {
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

/// An indexed mesh as a triangle soup.
pub fn soup(positions: &[[f32; 3]], indices: &[u32]) -> Vec<Vec3> {
    indices
        .iter()
        .map(|&i| Vec3::from(positions[i as usize]))
        .collect()
}
