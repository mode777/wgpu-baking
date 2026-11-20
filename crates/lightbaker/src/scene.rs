//! The scene a bake reads (`.lbscene`): what an engine exports, and all the baker knows of it.
//!
//! A file is the magic `LBSC`, the format version (u32), the header's length (u32), the header (JSON,
//! [`SceneHeader`]), then the meshes' positions (f32, x y z per vertex, three vertices a triangle, not
//! indexed), each mesh's array starting on a 4-byte boundary in header order. Y is up, units are metres.
//!
//! Version 2 (docs/FORMAT.md) keeps the framing; each mesh names its vertex count and typed arrays
//! (`positions:f32x3` and optionally `normals:f32x3`, `indices:u32`, `materialIds:u16`, `colors:f32x3`,
//! `lightmapUv:f32x2`), which follow the header in mesh order, then array order, each 4-byte aligned. The engine
//! sees every version as upstream's non-indexed triangles, one material a mesh: a v2 mesh with several material
//! ids becomes one engine mesh per material, remembering where its triangles came from ([`MeshOrigin`]).

use anyhow::{Context, Result, bail, ensure};
use glam::Vec3;
use serde::{Deserialize, Serialize};

pub const SCENE_MAGIC: &[u8; 4] = b"LBSC";
pub const SCENE_VERSION: u32 = 1;
/// indexed meshes with optional normals, material ids, colours and lightmap UVs
pub const SCENE_VERSION_2: u32 = 2;

/// Linear RGB.
pub type Rgb = [f32; 3];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneHeader {
    pub sun: Sun,
    pub sky: Sky,
    pub materials: Vec<Material>,
    pub meshes: Vec<MeshInfo>,
    pub lightmap: LightmapSettings,
    pub bake: BakeSettings,
    /// the irradiance volume over the play area; none: no volume baked
    #[serde(default)]
    pub volume: Option<VolumeSettings>,
    /// anything the engine wants back with the result, copied untouched (its own geometry hash, say)
    #[serde(default)]
    pub user: serde_json::Value,
    /// point, spot and area lights (v2; INTERFACES.md §3; baked from P4.1)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lights: Vec<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Sun {
    /// towards the sun (normalised by the baker)
    pub direction: [f32; 3],
    /// the disc's angular radius (degrees): the penumbra's width
    pub radius_deg: f32,
    /// irradiance on a surface square to it (linear RGB, the engine's units)
    pub irradiance: Rgb,
}

/// The sky's radiance, a gradient from the horizon to the zenith, and below the horizon the ground's.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Sky {
    pub zenith: Rgb,
    pub horizon: Rgb,
    pub ground: Rgb,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Material {
    pub name: String,
    /// diffuse reflectance (linear RGB): the colour bounced light takes from it
    pub albedo: Rgb,
    /// laid out in the lightmap and baked
    pub receives: bool,
    /// blocks light (the sun's, the sky's, bounces)
    pub casts: bool,
    /// the share of light it stops where it casts (leaves, nets: below one)
    #[serde(default = "one")]
    pub opacity: f32,
    /// emitted radiance (linear RGB; v2, baked from P4.2)
    #[serde(default, skip_serializing_if = "is_black")]
    pub emission: Rgb,
    /// lit and casting from both sides (v2, baked from P4.3)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub double_sided: bool,
}

fn is_black(c: &Rgb) -> bool {
    *c == [0.0; 3]
}

fn one() -> f32 {
    1.0
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeshInfo {
    pub name: String,
    /// its material (v2: of triangles without a material id)
    pub material: u32,
    pub triangles: u32,
    /// v2: vertices in each per-vertex array
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vertices: Option<u32>,
    /// v2: the arrays that follow, `name:type`, in order
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arrays: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LightmapSettings {
    pub texels_per_meter: f32,
    /// side of the square tiles the ground plan is cut into (m): what is baked again when something changes
    pub tile_size: f32,
    /// neighbouring triangles join a chart while they face within this of its first (degrees)
    pub chart_angle_deg: f32,
    /// empty texels round each chart
    pub padding: u32,
    /// the atlas' width; its height is what the tiles need
    pub atlas_width: u32,
    /// the bounced light's texture is this many times smaller per side
    pub indirect_downscale: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BakeSettings {
    /// sun rays a texel, a square number (a grid over the texel, each to its own part of the disc)
    pub sun_samples: u32,
    pub ao_rays: u32,
    /// how far ambient occlusion looks (m)
    pub ao_reach: f32,
    pub ao_strength: f32,
    pub ao_floor: f32,
    pub ao_contrast: f32,
    /// the sun's distance field reaches this many texels each side of a shadow's edge
    pub sdf_radius: f32,
    pub gi_rays: u32,
    pub gi_bounces: u32,
    /// how far bounced light is gathered from (m); beyond, the sky
    pub gi_reach: f32,
    /// a texel whose rays meet this share of back faces lies inside a solid
    pub inside_share: f32,
    /// the denoiser's radius on bounced light (texels)
    pub gi_filter: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VolumeSettings {
    pub min: [f32; 3],
    pub max: [f32; 3],
    /// between probes (m)
    pub step: f32,
    pub rays: u32,
}

/// A loaded scene: its header and each (engine) mesh's triangles.
pub struct Scene {
    /// the engine's header: as in the file, but v2 meshes with several material ids split one per material
    pub header: SceneHeader,
    /// per mesh, three vertices a triangle
    pub positions: Vec<Vec<Vec3>>,
    /// of the whole file: what the bake was made from
    pub hash: blake3::Hash,
    /// per mesh, per triangle corner (as `positions`): vertex normals, if the file has them
    pub normals: Vec<Option<Vec<Vec3>>>,
    /// per mesh, per triangle corner: vertex colours (linear RGB albedo multipliers), if any
    pub colors: Vec<Option<Vec<[f32; 3]>>>,
    /// per mesh, per triangle corner: lightmap UVs given by the file, if any
    pub lightmap_uvs: Vec<Option<Vec<[f32; 2]>>>,
    /// per mesh: the file mesh and its triangles it was made of
    pub origin: Vec<MeshOrigin>,
    /// the file as written (indexed v2 data), to rewrite it and to map results back per file vertex
    pub file: SceneFile,
}

/// Where an engine mesh's triangles come from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeshOrigin {
    pub file_mesh: u32,
    /// its triangles' indices in the file mesh, in order
    pub triangles: Vec<u32>,
}

/// A scene file's meshes as written.
#[derive(Clone, Debug)]
pub struct SceneFile {
    pub version: u32,
    pub header: SceneHeader,
    pub meshes: Vec<FileMesh>,
}

/// A mesh as a v2 file holds it (a v1 mesh: positions only, three vertices a triangle).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FileMesh {
    pub positions: Vec<Vec3>,
    pub normals: Option<Vec<Vec3>>,
    pub indices: Option<Vec<u32>>,
    /// per triangle
    pub material_ids: Option<Vec<u16>>,
    pub colors: Option<Vec<[f32; 3]>>,
    pub lightmap_uv: Option<Vec<[f32; 2]>>,
}

impl FileMesh {
    pub fn triangle_count(&self) -> usize {
        self.indices.as_ref().map_or(self.positions.len() / 3, |i| i.len() / 3)
    }
    /// The vertex of triangle `t`'s corner `k`.
    pub fn corner(&self, t: usize, k: usize) -> usize {
        self.indices.as_ref().map_or(t * 3 + k, |i| i[t * 3 + k] as usize)
    }
    /// The arrays it holds, as named in the header.
    fn array_names(&self) -> Vec<String> {
        let mut a = vec!["positions:f32x3".to_string()];
        let mut add = |present: bool, name: &str| {
            if present {
                a.push(name.into());
            }
        };
        add(self.normals.is_some(), "normals:f32x3");
        add(self.indices.is_some(), "indices:u32");
        add(self.material_ids.is_some(), "materialIds:u16");
        add(self.colors.is_some(), "colors:f32x3");
        add(self.lightmap_uv.is_some(), "lightmapUv:f32x2");
        a
    }
}

/// Reads a little-endian array of `n` items of `T` at `*at`, then moves `*at` past it (4-byte aligned).
fn take<T: bytemuck::Pod>(bytes: &[u8], at: &mut usize, n: usize, what: &str) -> Result<Vec<T>> {
    let len = n * size_of::<T>();
    let raw = bytes.get(*at..*at + len).with_context(|| format!("{what}: cut short"))?;
    *at = align4(*at + len);
    Ok(bytemuck::pod_collect_to_vec(raw))
}

fn vec3s(f: Vec<f32>) -> Vec<Vec3> {
    f.chunks_exact(3).map(|p| Vec3::new(p[0], p[1], p[2])).collect()
}

/// A per-vertex array as three values a triangle, for the triangles `tris` of `m`.
fn per_corner<T: Copy>(m: &FileMesh, tris: &[u32], per_vertex: &[T]) -> Vec<T> {
    tris.iter().flat_map(|&t| (0..3).map(move |k| per_vertex[m.corner(t as usize, k)])).collect()
}

impl Scene {
    pub fn read(bytes: &[u8]) -> Result<Scene> {
        ensure!(bytes.len() >= 12 && &bytes[0..4] == SCENE_MAGIC, "not a lightbaker scene");
        let version = u32::from_le_bytes(bytes[4..8].try_into()?);
        ensure!(version == SCENE_VERSION || version == SCENE_VERSION_2, "scene format {version}, this baker reads {SCENE_VERSION} and {SCENE_VERSION_2}");
        let len = u32::from_le_bytes(bytes[8..12].try_into()?) as usize;
        let header: SceneHeader = serde_json::from_slice(bytes.get(12..12 + len).context("scene header cut short")?).context("scene header")?;
        let mut at = align4(12 + len);
        let mut meshes = Vec::with_capacity(header.meshes.len());
        for m in &header.meshes {
            ensure!((m.material as usize) < header.materials.len(), "mesh {}: no material {}", m.name, m.material);
            let mesh = if version == SCENE_VERSION {
                ensure!(m.vertices.is_none() && m.arrays.is_none(), "mesh {}: vertices/arrays are v2 fields", m.name);
                FileMesh { positions: vec3s(take(bytes, &mut at, m.triangles as usize * 9, &format!("mesh {}: positions", m.name))?), ..Default::default() }
            } else {
                read_v2_mesh(bytes, &mut at, m, header.materials.len())?
            };
            ensure!(mesh.triangle_count() == m.triangles as usize, "mesh {}: {} triangles in its arrays, the header says {}", m.name, mesh.triangle_count(), m.triangles);
            meshes.push(mesh);
        }
        if at != bytes.len() {
            bail!("scene: {} bytes after the last mesh", bytes.len() - at);
        }
        header.validate()?;
        Ok(Scene::from_file(SceneFile { version, header, meshes }, blake3::hash(bytes)))
    }

    /// The engine's view of a file: non-indexed triangles, one material a mesh.
    fn from_file(file: SceneFile, hash: blake3::Hash) -> Scene {
        let mut header = file.header.clone();
        header.meshes.clear();
        let (mut positions, mut normals, mut colors, mut lightmap_uvs, mut origin) = (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for (fi, (info, m)) in file.header.meshes.iter().zip(&file.meshes).enumerate() {
            // triangles by material, in material order (one group without material ids)
            let mut groups: std::collections::BTreeMap<u32, Vec<u32>> = std::collections::BTreeMap::new();
            for t in 0..m.triangle_count() {
                let mat = m.material_ids.as_ref().map_or(info.material, |ids| ids[t] as u32);
                groups.entry(mat).or_default().push(t as u32);
            }
            let split = groups.len() > 1;
            for (mat, tris) in groups {
                positions.push(per_corner(m, &tris, &m.positions));
                normals.push(m.normals.as_ref().map(|n| per_corner(m, &tris, n)));
                colors.push(m.colors.as_ref().map(|c| per_corner(m, &tris, c)));
                lightmap_uvs.push(m.lightmap_uv.as_ref().map(|u| per_corner(m, &tris, u)));
                let name = if split { format!("{}#{}", info.name, mat) } else { info.name.clone() };
                header.meshes.push(MeshInfo { name, material: mat, triangles: tris.len() as u32, vertices: None, arrays: None });
                origin.push(MeshOrigin { file_mesh: fi as u32, triangles: tris });
            }
        }
        Scene { header, positions, hash, normals, colors, lightmap_uvs, origin, file }
    }

    /// The file's bytes (what an engine writes; the tests build scenes with it).
    pub fn write(header: &SceneHeader, positions: &[Vec<Vec3>]) -> Result<Vec<u8>> {
        ensure!(header.meshes.len() == positions.len(), "one positions array a mesh");
        let head = serde_json::to_vec(header)?;
        let mut out = Vec::new();
        out.extend_from_slice(SCENE_MAGIC);
        out.extend_from_slice(&SCENE_VERSION.to_le_bytes());
        out.extend_from_slice(&(head.len() as u32).to_le_bytes());
        out.extend_from_slice(&head);
        for (m, p) in header.meshes.iter().zip(positions) {
            ensure!(p.len() == m.triangles as usize * 3, "mesh {}: {} vertices for {} triangles", m.name, p.len(), m.triangles);
            out.resize(align4(out.len()), 0);
            for v in p {
                out.extend_from_slice(bytemuck::bytes_of(&v.to_array()));
            }
        }
        out.resize(align4(out.len()), 0);
        Ok(out)
    }

    /// A v2 file. The header's meshes give names and default materials; their `triangles`, `vertices` and
    /// `arrays` are set from `meshes`.
    pub fn write_v2(header: &SceneHeader, meshes: &[FileMesh]) -> Result<Vec<u8>> {
        ensure!(header.meshes.len() == meshes.len(), "one mesh a header mesh");
        let mut header = header.clone();
        for (info, m) in header.meshes.iter_mut().zip(meshes) {
            let n = m.positions.len();
            for (len, what) in [(m.normals.as_ref().map(Vec::len), "normals"), (m.colors.as_ref().map(Vec::len), "colors"), (m.lightmap_uv.as_ref().map(Vec::len), "lightmapUv")] {
                ensure!(len.is_none_or(|l| l == n), "mesh {}: {what} has {} entries for {n} vertices", info.name, len.unwrap_or(0));
            }
            ensure!(m.indices.as_ref().map_or(n % 3 == 0, |i| i.len() % 3 == 0), "mesh {}: not whole triangles", info.name);
            info.triangles = m.triangle_count() as u32;
            info.vertices = Some(n as u32);
            info.arrays = Some(m.array_names());
        }
        let head = serde_json::to_vec(&header)?;
        let mut out = Vec::new();
        out.extend_from_slice(SCENE_MAGIC);
        out.extend_from_slice(&SCENE_VERSION_2.to_le_bytes());
        out.extend_from_slice(&(head.len() as u32).to_le_bytes());
        out.extend_from_slice(&head);
        let mut put = |b: &[u8]| {
            out.resize(align4(out.len()), 0);
            out.extend_from_slice(b);
        };
        for m in meshes {
            let flat: Vec<f32> = m.positions.iter().flat_map(|v| v.to_array()).collect();
            put(bytemuck::cast_slice(&flat));
            if let Some(n) = &m.normals {
                let flat: Vec<f32> = n.iter().flat_map(|v| v.to_array()).collect();
                put(bytemuck::cast_slice(&flat));
            }
            if let Some(i) = &m.indices {
                put(bytemuck::cast_slice(i));
            }
            if let Some(ids) = &m.material_ids {
                put(bytemuck::cast_slice(ids));
            }
            if let Some(c) = &m.colors {
                put(bytemuck::cast_slice(c));
            }
            if let Some(u) = &m.lightmap_uv {
                put(bytemuck::cast_slice(u));
            }
        }
        out.resize(align4(out.len()), 0);
        Ok(out)
    }

    /// The scene rewritten in its own version with this scene's settings (sun, sky, materials, lights, lightmap,
    /// bake, volume, user): what a bake with changed settings reads.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut h = self.header.clone();
        h.meshes = self.file.header.meshes.clone();
        if self.file.version == SCENE_VERSION {
            Scene::write(&h, &self.file.meshes.iter().map(|m| m.positions.clone()).collect::<Vec<_>>())
        } else {
            Scene::write_v2(&h, &self.file.meshes)
        }
    }

    pub fn triangle_count(&self) -> usize {
        self.positions.iter().map(|p| p.len() / 3).sum()
    }
}

fn read_v2_mesh(bytes: &[u8], at: &mut usize, m: &MeshInfo, materials: usize) -> Result<FileMesh> {
    let n = m.vertices.with_context(|| format!("mesh {}: v2 meshes need `vertices`", m.name))? as usize;
    let arrays = m.arrays.as_ref().with_context(|| format!("mesh {}: v2 meshes need `arrays`", m.name))?;
    let tris = m.triangles as usize;
    let mut mesh = FileMesh::default();
    let mut seen = std::collections::BTreeSet::new();
    for a in arrays {
        ensure!(seen.insert(a.as_str()), "mesh {}: array {a} twice", m.name);
        let what = format!("mesh {}: {a}", m.name);
        match a.as_str() {
            "positions:f32x3" => mesh.positions = vec3s(take(bytes, at, n * 3, &what)?),
            "normals:f32x3" => mesh.normals = Some(vec3s(take(bytes, at, n * 3, &what)?)),
            "indices:u32" => {
                let i: Vec<u32> = take(bytes, at, tris * 3, &what)?;
                if let Some(bad) = i.iter().find(|&&v| v as usize >= n) {
                    bail!("mesh {}: index {bad} out of range (it has {n} vertices)", m.name);
                }
                mesh.indices = Some(i);
            }
            "materialIds:u16" => {
                let ids: Vec<u16> = take(bytes, at, tris, &what)?;
                if let Some(bad) = ids.iter().find(|&&v| v as usize >= materials) {
                    bail!("mesh {}: material id {bad} has no material", m.name);
                }
                mesh.material_ids = Some(ids);
            }
            "colors:f32x3" => mesh.colors = Some(take::<f32>(bytes, at, n * 3, &what)?.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect()),
            "lightmapUv:f32x2" => mesh.lightmap_uv = Some(take::<f32>(bytes, at, n * 2, &what)?.chunks_exact(2).map(|c| [c[0], c[1]]).collect()),
            other => bail!("mesh {}: unknown array `{other}`", m.name),
        }
    }
    ensure!(seen.contains("positions:f32x3"), "mesh {}: no positions", m.name);
    if !seen.contains("indices:u32") {
        ensure!(n == tris * 3, "mesh {}: {n} vertices without indices is not {tris} triangles", m.name);
    }
    Ok(mesh)
}
impl SceneHeader {
    /// What the scene asks for that this engine cannot bake yet: refused, never silently ignored (AGENTS.md rule 3).
    pub fn unsupported(&self) -> Result<()> {
        ensure!(self.lights.is_empty(), "the scene has {} lights: point, spot and area lights are not supported yet (P4.1)", self.lights.len());
        for m in &self.materials {
            ensure!(m.emission == [0.0; 3], "material {}: emission is not supported yet (P4.2)", m.name);
            ensure!(!m.double_sided, "material {}: doubleSided is not supported yet (P4.3)", m.name);
        }
        Ok(())
    }

    fn validate(&self) -> Result<()> {
        let l = &self.lightmap;
        ensure!(l.texels_per_meter > 0.0 && l.tile_size > 0.0, "lightmap: texelsPerMeter and tileSize must be positive");
        ensure!(l.atlas_width >= 64 && l.atlas_width % 4 == 0, "lightmap: atlasWidth must be a multiple of 4, at least 64");
        ensure!(matches!(l.indirect_downscale, 1 | 2 | 4), "lightmap: indirectDownscale 1, 2 or 4 (the tiles' grid)");
        let s = (self.bake.sun_samples as f32).sqrt() as u32;
        ensure!(s * s == self.bake.sun_samples && s >= 1, "bake: sunSamples must be a square number");
        ensure!(self.bake.ao_rays >= 1 && self.bake.gi_rays >= 1, "bake: aoRays and giRays at least one");
        if let Some(v) = &self.volume {
            ensure!(v.step > 0.0 && (0..3).all(|i| v.max[i] >= v.min[i]), "volume: bad bounds or step");
        }
        Ok(())
    }
}

pub fn align4(n: usize) -> usize {
    (n + 3) & !3
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(meshes: Vec<MeshInfo>, materials: usize) -> SceneHeader {
        SceneHeader {
            sun: Sun { direction: [0.0, 1.0, 0.0], radius_deg: 0.5, irradiance: [1.0; 3] },
            sky: Sky { zenith: [0.1; 3], horizon: [0.1; 3], ground: [0.1; 3] },
            materials: (0..materials).map(|i| Material { name: format!("m{i}"), albedo: [0.5; 3], receives: true, casts: true, opacity: 1.0, emission: [0.0; 3], double_sided: false }).collect(),
            meshes,
            lightmap: LightmapSettings { texels_per_meter: 4.0, tile_size: 16.0, chart_angle_deg: 40.0, padding: 2, atlas_width: 256, indirect_downscale: 2 },
            bake: BakeSettings { sun_samples: 4, ao_rays: 8, ao_reach: 0.8, ao_strength: 1.0, ao_floor: 0.25, ao_contrast: 1.0, sdf_radius: 2.0, gi_rays: 8, gi_bounces: 1, gi_reach: 4.0, inside_share: 0.25, gi_filter: 1 },
            volume: None,
            user: serde_json::Value::Null,
            lights: vec![],
        }
    }

    fn info(name: &str) -> MeshInfo {
        MeshInfo { name: name.into(), material: 0, triangles: 0, vertices: None, arrays: None }
    }

    /// A unit quad as two indexed triangles, with every optional array.
    fn quad() -> FileMesh {
        FileMesh {
            positions: vec![Vec3::ZERO, Vec3::X, Vec3::new(1.0, 0.0, 1.0), Vec3::Z],
            normals: Some(vec![Vec3::Y; 4]),
            indices: Some(vec![0, 2, 1, 0, 3, 2]),
            material_ids: Some(vec![0, 1]),
            colors: Some(vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 1.0, 1.0]]),
            lightmap_uv: Some(vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]]),
        }
    }

    #[test]
    fn v2_round_trip_and_material_split() {
        let bytes = Scene::write_v2(&header(vec![info("quad")], 2), &[quad()]).unwrap();
        let s = Scene::read(&bytes).unwrap();
        assert_eq!(s.file.version, SCENE_VERSION_2);
        assert_eq!(s.file.meshes, vec![quad()]);
        assert_eq!(s.file.header.meshes[0].arrays.as_ref().unwrap().len(), 6);
        // two materials: two engine meshes, one triangle each, per-corner data expanded
        assert_eq!(s.header.meshes.iter().map(|m| (m.name.as_str(), m.material, m.triangles)).collect::<Vec<_>>(), [("quad#0", 0, 1), ("quad#1", 1, 1)]);
        assert_eq!(s.positions[0], vec![Vec3::ZERO, Vec3::new(1.0, 0.0, 1.0), Vec3::X]);
        assert_eq!(s.colors[1].as_ref().unwrap(), &vec![[1.0, 0.0, 0.0], [1.0, 1.0, 1.0], [0.0, 0.0, 1.0]]);
        assert_eq!(s.lightmap_uvs[0].as_ref().unwrap()[1], [1.0, 1.0]);
        assert_eq!(s.normals[0].as_ref().unwrap().len(), 3);
        assert_eq!(s.origin, vec![MeshOrigin { file_mesh: 0, triangles: vec![0] }, MeshOrigin { file_mesh: 0, triangles: vec![1] }]);
        // rewriting keeps the version and the arrays
        assert_eq!(Scene::read(&s.to_bytes().unwrap()).unwrap().file.meshes, vec![quad()]);
    }

    #[test]
    fn v1_unchanged() {
        let positions = vec![vec![Vec3::ZERO, Vec3::X, Vec3::Z]];
        let mut h = header(vec![info("tri")], 1);
        h.meshes[0].triangles = 1;
        let bytes = Scene::write(&h, &positions).unwrap();
        let s = Scene::read(&bytes).unwrap();
        assert_eq!(s.file.version, SCENE_VERSION);
        assert_eq!(s.positions, positions);
        assert!(s.normals[0].is_none() && s.colors[0].is_none() && s.lightmap_uvs[0].is_none());
        assert_eq!(s.to_bytes().unwrap(), bytes, "a v1 scene rewrites to the same bytes");
        assert!(!String::from_utf8_lossy(&bytes).contains("doubleSided"), "v2 defaults are not written");
    }

    #[test]
    fn v2_errors() {
        let write = |m: FileMesh, materials: usize| Scene::write_v2(&header(vec![info("bad")], materials), &[m]);
        let err = |bytes: Vec<u8>| format!("{:#}", Scene::read(&bytes).err().expect("must fail"));
        let mut out_of_range = quad();
        out_of_range.indices = Some(vec![0, 2, 1, 0, 3, 4]);
        assert!(err(write(out_of_range, 2).unwrap()).contains("index 4 out of range"));
        assert!(err(write(quad(), 1).unwrap()).contains("material id 1 has no material"));
        let mut short = quad();
        short.colors = Some(vec![[0.0; 3]; 3]);
        assert!(write(short, 2).is_err(), "colors must have one entry a vertex");
        // an unknown array in the header is an error, not skipped
        let mut bytes = write(FileMesh { indices: None, positions: vec![Vec3::ZERO, Vec3::X, Vec3::Z], ..Default::default() }, 1).unwrap();
        let at = bytes.windows(15).position(|w| w == b"positions:f32x3").unwrap();
        bytes[at..at + 15].copy_from_slice(b"position5:f32x3");
        assert!(err(bytes).contains("unknown array"));
    }

    #[test]
    fn unsupported_inputs_are_refused() {
        let mut h = header(vec![], 1);
        assert!(h.unsupported().is_ok());
        h.materials[0].emission = [1.0, 0.0, 0.0];
        assert!(h.unsupported().unwrap_err().to_string().contains("emission is not supported yet"));
        h.materials[0].emission = [0.0; 3];
        h.materials[0].double_sided = true;
        assert!(h.unsupported().unwrap_err().to_string().contains("doubleSided"));
        h.materials[0].double_sided = false;
        h.lights.push(serde_json::json!({ "type": "point" }));
        assert!(h.unsupported().unwrap_err().to_string().contains("lights"));
    }
}