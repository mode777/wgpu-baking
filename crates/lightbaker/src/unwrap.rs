//! The lightmap unwrap contract (ADR-0004, ADR-0006): what an unwrapper (xatlas natively in `lightbaker-uv`,
//! `xatlas-wasm` in the browser, upstream's planar layout) is given per object tile and what it returns, and the
//! UV validity checks every unwrap must pass (docs/TESTING.md "UV generation tests"). The engine depends on no
//! unwrapper; they depend on this.

use anyhow::Result;
use glam::Vec3;

/// One mesh of an object tile.
#[derive(Clone, Copy, Debug)]
pub struct MeshInput<'a> {
    /// per vertex (world space, metres)
    pub positions: &'a [[f32; 3]],
    /// three per triangle; none: three vertices a triangle
    pub indices: Option<&'a [u32]>,
    pub normals: Option<&'a [[f32; 3]]>,
    /// per triangle: charts never span materials
    pub material_ids: Option<&'a [u32]>,
}

/// xatlas' chart options (0 / unset: xatlas' defaults).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ChartOptions {
    pub max_chart_area: Option<f32>,
    pub max_boundary_length: Option<f32>,
    pub normal_deviation_weight: Option<f32>,
    pub roundness_weight: Option<f32>,
    pub straightness_weight: Option<f32>,
    pub max_iterations: Option<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnwrapOptions {
    pub texels_per_meter: f32,
    /// empty texels round each chart
    pub padding: u32,
    pub chart: ChartOptions,
    /// slower, tighter packing
    pub brute_force: bool,
}

/// One mesh's unwrap: output vertices (seams split input vertices), each with its tile UV and input vertex.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UnwrappedMesh {
    /// per output vertex, in tile texels (0 … width, 0 … height)
    pub uv: Vec<[f32; 2]>,
    /// per output vertex: the input vertex it came from
    pub xref: Vec<u32>,
    /// three per triangle into the output vertices, in the input's triangle order
    pub indices: Vec<u32>,
    /// per output vertex: its chart in the tile
    pub chart: Vec<i32>,
    pub charts: u32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UnwrappedTile {
    pub width: u32,
    pub height: u32,
    pub meshes: Vec<UnwrappedMesh>,
    pub charts: u32,
    /// share of the tile covered by charts
    pub utilisation: f32,
}

/// Lightmap UVs for one object tile: all its meshes share the tile's rectangle.
pub trait Unwrapper {
    fn unwrap(&self, meshes: &[MeshInput], options: &UnwrapOptions) -> Result<UnwrappedTile>;
}

/// How an unwrap of one mesh measures up (docs/TESTING.md).
#[derive(Clone, Debug, PartialEq)]
pub struct UvReport {
    pub triangles: usize,
    /// UVs outside the tile
    pub out_of_range: usize,
    /// triangles with 3D area but (near) zero UV area
    pub degenerate: usize,
    /// triangles whose UV winding disagrees with their chart's majority
    pub flipped: usize,
    /// output corners whose `xref` is not where the input corner is
    pub bad_xref: usize,
    /// charts holding faces of more than one material
    pub mixed_charts: usize,
    /// charts' texel density relative to the request: min, median, max (charts under 4 × 4 texels excluded; 0 if none)
    pub density: [f32; 3],
}

impl UvReport {
    pub fn valid(&self) -> bool {
        self.out_of_range + self.degenerate + self.flipped + self.bad_xref + self.mixed_charts == 0
    }
}

/// Checks one mesh's unwrap against its input (same rules as `app/src/bake/uvValidity.ts`).
pub fn check_uvs(input: &MeshInput, out: &UnwrappedMesh, width: u32, height: u32, texels_per_meter: f32) -> UvReport {
    let eps = 1e-3;
    let out_of_range = out.uv.iter().filter(|[u, v]| !(*u >= -eps && *u <= width as f32 + eps && *v >= -eps && *v <= height as f32 + eps)).count();
    let pos = |i: u32| Vec3::from(input.positions[i as usize]);
    let input_corner = |t: usize, k: usize| input.indices.map_or((t * 3 + k) as u32, |i| i[t * 3 + k]);
    let tris = out.indices.len() / 3;
    let (mut degenerate, mut bad_xref) = (0, 0);
    // per chart: (positive, negative) UV windings, (UV area, 3D area), materials
    let mut charts: std::collections::BTreeMap<i32, ([usize; 2], [f32; 2], std::collections::BTreeSet<u32>)> = Default::default();
    let mut signs = Vec::with_capacity(tris);
    for t in 0..tris {
        let c = [0, 1, 2].map(|k| out.indices[t * 3 + k]);
        for (k, &v) in c.iter().enumerate() {
            let x = out.xref[v as usize];
            if x as usize >= input.positions.len() || (pos(x) - pos(input_corner(t, k))).abs().max_element() > 1e-6 {
                bad_xref += 1;
            }
        }
        let p = c.map(|v| pos(out.xref[v as usize].min(input.positions.len() as u32 - 1)));
        let area3 = (p[1] - p[0]).cross(p[2] - p[0]).length() / 2.0;
        let u = c.map(|v| out.uv[v as usize]);
        let ua = (u[1][0] - u[0][0]) * (u[2][1] - u[0][1]) - (u[2][0] - u[0][0]) * (u[1][1] - u[0][1]);
        if area3 > 1e-10 && ua.abs() / 2.0 < 1e-6 {
            degenerate += 1;
        }
        let e = charts.entry(out.chart[c[0] as usize]).or_default();
        if ua > 0.0 {
            e.0[0] += 1;
        } else if ua < 0.0 {
            e.0[1] += 1;
        }
        e.1[0] += ua.abs() / 2.0;
        e.1[1] += area3;
        if let Some(m) = input.material_ids {
            e.2.insert(m[t]);
        }
        signs.push((out.chart[c[0] as usize], ua.signum() as i32));
    }
    let flipped = signs.iter().filter(|(c, s)| {
        let w = charts[c].0;
        let majority = if w[0] >= w[1] { 1 } else { -1 };
        *s != 0 && *s != majority
    });
    let flipped = flipped.count();
    let mixed_charts = charts.values().filter(|c| c.2.len() > 1).count();
    let mut ratios: Vec<f32> = charts.values().filter(|c| c.1[1] * texels_per_meter * texels_per_meter >= 16.0).map(|c| (c.1[0] / c.1[1]).sqrt() / texels_per_meter).collect();
    ratios.sort_by(f32::total_cmp);
    let density = if ratios.is_empty() { [0.0; 3] } else { [ratios[0], ratios[ratios.len() / 2], ratios[ratios.len() - 1]] };
    UvReport { triangles: tris, out_of_range, degenerate, flipped, bad_xref, mixed_charts, density }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A unit quad laid out by hand: two triangles in one chart.
    fn quad() -> (Vec<[f32; 3]>, UnwrappedMesh) {
        let positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 0.0, 1.0], [0.0, 0.0, 1.0]];
        let out = UnwrappedMesh { uv: vec![[1.0, 1.0], [9.0, 1.0], [9.0, 9.0], [1.0, 9.0]], xref: vec![0, 1, 2, 3], indices: vec![0, 2, 1, 0, 3, 2], chart: vec![0; 4], charts: 1 };
        (positions, out)
    }

    #[test]
    fn a_good_unwrap_is_valid() {
        let (p, out) = quad();
        let input = MeshInput { positions: &p, indices: Some(&[0, 2, 1, 0, 3, 2]), normals: None, material_ids: None };
        let r = check_uvs(&input, &out, 10, 10, 8.0);
        assert!(r.valid(), "{r:?}");
        assert!((r.density[1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn faults_are_counted() {
        let (p, mut out) = quad();
        let input = MeshInput { positions: &p, indices: Some(&[0, 2, 1, 0, 3, 2]), normals: None, material_ids: Some(&[0, 1]) };
        out.uv[2] = [11.0, 9.0]; // outside a 10-wide tile
        out.xref[3] = 1; // corner 3 claims to be vertex 1
        let r = check_uvs(&input, &out, 10, 10, 8.0);
        assert_eq!((r.out_of_range, r.bad_xref, r.mixed_charts), (1, 1, 1));
        assert!(!r.valid());
        // a triangle mirrored against its chart
        let (_, mut out) = quad();
        out.indices = vec![0, 2, 1, 0, 2, 3];
        let r = check_uvs(&MeshInput { positions: &p, indices: Some(&[0, 2, 1, 0, 2, 3]), normals: None, material_ids: None }, &out, 10, 10, 8.0);
        assert_eq!(r.flipped, 1);
    }
}
