//! Lightmap UVs natively: xatlas (vendored `jpcy/xatlas` at `f700c77`, MIT) behind the engine's unwrap contract
//! (`lightbaker::unwrap`, ADR-0006). One xatlas atlas per object tile, auto-sized from the texel density. The
//! browser runs the same xatlas source through `xatlas-wasm` in the baker worker; on wasm32 this crate has no
//! unwrapper.

pub use lightbaker::unwrap::{
    ChartOptions, MeshInput, UnwrapOptions, UnwrappedMesh, UnwrappedTile, Unwrapper, UvReport,
    check_uvs,
};

#[cfg(not(target_arch = "wasm32"))]
mod ffi;

#[cfg(not(target_arch = "wasm32"))]
pub use native::XatlasUnwrapper;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::ffi::CStr;

    use anyhow::{Result, bail, ensure};

    use super::ffi;
    use super::*;

    /// xatlas, natively.
    #[derive(Clone, Copy, Debug, Default)]
    pub struct XatlasUnwrapper;

    /// Frees the atlas however the unwrap ends.
    struct Atlas(*mut ffi::Atlas);

    impl Drop for Atlas {
        fn drop(&mut self) {
            unsafe { ffi::xatlasDestroy(self.0) }
        }
    }

    fn set(field: &mut f32, value: Option<f32>) {
        if let Some(v) = value {
            *field = v;
        }
    }

    impl Unwrapper for XatlasUnwrapper {
        fn unwrap(&self, meshes: &[MeshInput], options: &UnwrapOptions) -> Result<UnwrappedTile> {
            ensure!(
                options.texels_per_meter > 0.0,
                "texelsPerMeter must be positive"
            );
            let atlas = Atlas(unsafe { ffi::xatlasCreate() });
            for (i, m) in meshes.iter().enumerate() {
                let corners = m.indices.map_or(m.positions.len(), <[u32]>::len);
                ensure!(corners % 3 == 0, "mesh {i}: not whole triangles");
                if let Some(n) = m.normals {
                    ensure!(
                        n.len() == m.positions.len(),
                        "mesh {i}: one normal a vertex"
                    );
                }
                if let Some(ids) = m.material_ids {
                    ensure!(
                        ids.len() == corners / 3,
                        "mesh {i}: one material id a triangle"
                    );
                }
                let mut decl: ffi::MeshDecl = unsafe { std::mem::zeroed() };
                unsafe { ffi::xatlasMeshDeclInit(&mut decl) };
                decl.vertexPositionData = m.positions.as_ptr().cast();
                decl.vertexPositionStride = 12;
                decl.vertexCount = m.positions.len() as u32;
                if let Some(n) = m.normals {
                    decl.vertexNormalData = n.as_ptr().cast();
                    decl.vertexNormalStride = 12;
                }
                if let Some(idx) = m.indices {
                    decl.indexData = idx.as_ptr().cast();
                    decl.indexCount = idx.len() as u32;
                    decl.indexFormat = ffi::INDEX_FORMAT_UINT32;
                }
                if let Some(ids) = m.material_ids {
                    decl.faceMaterialData = ids.as_ptr();
                }
                let err = unsafe { ffi::xatlasAddMesh(atlas.0, &decl, meshes.len() as u32) };
                if err != 0 {
                    let why = unsafe { CStr::from_ptr(ffi::xatlasAddMeshErrorString(err)) }
                        .to_string_lossy();
                    bail!("xatlas: mesh {i}: {why}");
                }
            }
            let mut chart: ffi::ChartOptions = unsafe { std::mem::zeroed() };
            unsafe { ffi::xatlasChartOptionsInit(&mut chart) };
            // only what is asked for overrides xatlas' own defaults
            let c = &options.chart;
            set(&mut chart.maxChartArea, c.max_chart_area);
            set(&mut chart.maxBoundaryLength, c.max_boundary_length);
            set(&mut chart.normalDeviationWeight, c.normal_deviation_weight);
            set(&mut chart.roundnessWeight, c.roundness_weight);
            set(&mut chart.straightnessWeight, c.straightness_weight);
            if let Some(n) = c.max_iterations {
                chart.maxIterations = n;
            }
            let mut pack: ffi::PackOptions = unsafe { std::mem::zeroed() };
            unsafe { ffi::xatlasPackOptionsInit(&mut pack) };
            pack.padding = options.padding;
            pack.texelsPerUnit = options.texels_per_meter;
            pack.resolution = 0;
            pack.bilinear = true;
            pack.bruteForce = options.brute_force;
            unsafe {
                ffi::xatlasComputeCharts(atlas.0, &chart);
                ffi::xatlasPackCharts(atlas.0, &pack);
            }
            let a = unsafe { &*atlas.0 };
            ensure!(
                meshes.is_empty() || a.atlasCount == 1,
                "xatlas: expected one atlas for the tile, got {}",
                a.atlasCount
            );
            let out = (0..a.meshCount as usize)
                .map(|i| {
                    let m = unsafe { &*a.meshes.add(i) };
                    let verts = unsafe {
                        std::slice::from_raw_parts(m.vertexArray, m.vertexCount as usize)
                    };
                    UnwrappedMesh {
                        uv: verts.iter().map(|v| v.uv).collect(),
                        xref: verts.iter().map(|v| v.xref).collect(),
                        chart: verts.iter().map(|v| v.chartIndex).collect(),
                        indices: unsafe {
                            std::slice::from_raw_parts(m.indexArray, m.indexCount as usize)
                        }
                        .to_vec(),
                        charts: m.chartCount,
                    }
                })
                .collect();
            let utilisation = if a.atlasCount > 0 {
                unsafe { *a.utilization }
            } else {
                0.0
            };
            Ok(UnwrappedTile {
                width: a.width,
                height: a.height,
                meshes: out,
                charts: a.chartCount,
                utilisation,
            })
        }
    }

    #[cfg(test)]
    mod tests {
        use super::ffi::*;

        #[test]
        fn ffi_layouts_match_the_cpp_compiler() {
            let sizes = [
                ("xatlasMeshDecl", size_of::<MeshDecl>()),
                ("xatlasChartOptions", size_of::<ChartOptions>()),
                ("xatlasPackOptions", size_of::<PackOptions>()),
                ("xatlasAtlas", size_of::<Atlas>()),
                ("xatlasMesh", size_of::<Mesh>()),
                ("xatlasVertex", size_of::<Vertex>()),
                ("xatlasChart", size_of::<Chart>()),
            ];
            for (i, (name, rust)) in sizes.iter().enumerate() {
                assert_eq!(unsafe { lbuv_sizeof(i as i32) }, *rust, "{name}");
            }
        }
    }
}
