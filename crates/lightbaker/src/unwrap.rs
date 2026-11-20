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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_chart_area: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_boundary_length: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normal_deviation_weight: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roundness_weight: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub straightness_weight: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
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

/// One scene mesh's unwrap: its object tile and how its UVs measure up.
#[derive(Clone, Debug)]
pub struct SceneMeshUnwrap {
    pub name: String,
    pub tile: UnwrappedTile,
    pub report: UvReport,
    pub ms: f64,
}

/// Unwraps every engine mesh of a scene (v2 meshes split per material) as its own object tile: its triangle
/// soup and, if the scene has them, its vertex normals. The same natively and in the browser.
pub fn unwrap_scene(scene: &crate::Scene, unwrapper: &dyn Unwrapper, options: &UnwrapOptions) -> Result<Vec<SceneMeshUnwrap>> {
    let to_arrays = |v: &[Vec3]| v.iter().map(|p| p.to_array()).collect::<Vec<_>>();
    scene
        .positions
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let started = web_time::Instant::now();
            let positions = to_arrays(p);
            let normals = scene.normals.get(i).and_then(|n| n.as_deref()).map(to_arrays);
            let input = MeshInput { positions: &positions, indices: None, normals: normals.as_deref(), material_ids: None };
            let name = &scene.header.meshes[i].name;
            let tile = unwrapper.unwrap(&[input], options).map_err(|e| e.context(format!("unwrapping {name}")))?;
            anyhow::ensure!(tile.meshes.len() == 1, "unwrapping {name}: {} meshes back for one", tile.meshes.len());
            let report = check_uvs(&input, &tile.meshes[0], tile.width, tile.height, options.texels_per_meter);
            Ok(SceneMeshUnwrap { name: name.clone(), tile, report, ms: started.elapsed().as_secs_f64() * 1000.0 })
        })
        .collect()
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

    /// Lays each triangle out on its own: corner k of triangle t at (4t + k, k).
    struct PerTriangle;

    impl Unwrapper for PerTriangle {
        fn unwrap(&self, meshes: &[MeshInput], _: &UnwrapOptions) -> Result<UnwrappedTile> {
            let n = meshes[0].positions.len();
            let uv = (0..n)
                .map(|c| {
                    let x = (c / 3 * 4) as f32;
                    [[x, 0.0], [x, 2.0], [x + 2.0, 0.0]][c % 3]
                })
                .collect();
            let mesh = UnwrappedMesh { uv, xref: (0..n as u32).collect(), indices: (0..n as u32).collect(), chart: (0..n as i32).map(|c| c / 3).collect(), charts: n as u32 / 3 };
            Ok(UnwrappedTile { width: (n / 3 * 4) as u32, height: 3, meshes: vec![mesh], charts: n as u32 / 3, utilisation: 0.5 })
        }
    }

    #[test]
    fn a_scene_is_unwrapped_one_tile_a_mesh() {
        use crate::scene::*;
        let header = SceneHeader {
            meshes: vec![
                MeshInfo { name: "a".into(), material: 0, triangles: 1, vertices: None, arrays: None },
                MeshInfo { name: "b".into(), material: 0, triangles: 2, vertices: None, arrays: None },
            ],
            materials: vec![Material { name: "m".into(), albedo: [0.5; 3], receives: true, casts: true, opacity: 1.0, emission: [0.0; 3], double_sided: false }],
            ..serde_json::from_value(serde_json::json!({
                "sun": { "direction": [0, 1, 0], "radiusDeg": 0.5, "irradiance": [1, 1, 1] },
                "sky": { "zenith": [0, 0, 0], "horizon": [0, 0, 0], "ground": [0, 0, 0] },
                "materials": [], "meshes": [],
                "lightmap": { "texelsPerMeter": 4, "tileSize": 16, "chartAngleDeg": 40, "padding": 2, "atlasWidth": 1024, "indirectDownscale": 2 },
                "bake": { "sunSamples": 1, "aoRays": 1, "aoReach": 1, "aoStrength": 1, "aoFloor": 0, "aoContrast": 1, "sdfRadius": 1, "giRays": 1, "giBounces": 1, "giReach": 1, "insideShare": 0, "giFilter": 0 }
            }))
            .unwrap()
        };
        let v = |x: f32, z: f32| Vec3::new(x, 0.0, z);
        let tri = |o: f32| [v(o, 0.0), v(o, 1.0), v(o + 1.0, 0.0)];
        let positions = vec![tri(0.0).to_vec(), [tri(2.0), tri(4.0)].concat()];
        let scene = crate::Scene::read(&crate::Scene::write(&header, &positions).unwrap()).unwrap();
        let options = UnwrapOptions { texels_per_meter: 1.0, padding: 0, chart: ChartOptions::default(), brute_force: false };
        let out = unwrap_scene(&scene, &PerTriangle, &options).unwrap();
        assert_eq!(out.iter().map(|m| (m.name.as_str(), m.tile.width, m.report.triangles)).collect::<Vec<_>>(), [("a", 4, 1), ("b", 8, 2)]);
    }
}
