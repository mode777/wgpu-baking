//! Lightmap UVs in the browser (ADR-0006): xatlas runs as its own module (`xatlas-wasm`) in the worker; the
//! worker hands the bridge an object with `unwrapTile(meshes, options)` (`app/src/bake/xatlasUnwrapper.ts`),
//! and the bridge calls it through the engine's unwrap contract — the same `unwrap_scene` and UV checks as
//! natively. Calls are synchronous: the module is loaded before.

use js_sys::{Array, Float32Array, Int32Array, Object, Reflect, Uint32Array};
use lightbaker::Scene;
use lightbaker::unwrap::{
    ChartOptions, MeshInput, UnwrapOptions, UnwrappedMesh, UnwrappedTile, Unwrapper, unwrap_scene,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;

use crate::{BakerError, reject, to_js};

#[wasm_bindgen(typescript_custom_section)]
const JS_UNWRAPPER: &'static str = r#"
/** One mesh of an object tile, as the unwrapper gets it. */
export interface JsUnwrapMesh { positions: Float32Array; indices?: Uint32Array; normals?: Float32Array; materialIds?: Uint32Array }
/** The unwrap contract's options (unset chart options: xatlas' defaults). */
export interface JsUnwrapOptions {
  texelsPerMeter: number;
  padding: number;
  chart: { maxChartArea?: number; maxBoundaryLength?: number; normalDeviationWeight?: number; roundnessWeight?: number; straightnessWeight?: number; maxIterations?: number };
  bruteForce: boolean;
}
export interface JsUnwrappedTile {
  width: number;
  height: number;
  charts: number;
  utilisation: number;
  meshes: { uv: Float32Array; xref: Uint32Array; indices: Uint32Array; chart: Int32Array; charts: number }[];
}
/** What `unwrapScene` calls: one object tile per call (the xatlas-wasm adapter in the worker). */
export interface JsUnwrapper { unwrapTile(meshes: JsUnwrapMesh[], options: JsUnwrapOptions): JsUnwrappedTile }
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "JsUnwrapper")]
    pub type JsUnwrapper;

    #[wasm_bindgen(method, catch, js_name = unwrapTile)]
    fn unwrap_tile(
        this: &JsUnwrapper,
        meshes: &Array,
        options: &JsValue,
    ) -> Result<JsValue, JsValue>;
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsOptions<'a> {
    texels_per_meter: f32,
    padding: u32,
    chart: &'a ChartOptions,
    brute_force: bool,
}

fn message(e: &JsValue) -> String {
    e.dyn_ref::<js_sys::Error>()
        .map(|e| String::from(e.message()))
        .or_else(|| e.as_string())
        .unwrap_or_else(|| format!("{e:?}"))
}

fn get(v: &JsValue, k: &str) -> anyhow::Result<JsValue> {
    Reflect::get(v, &k.into())
        .map_err(|e| anyhow::anyhow!("unwrapper result: {k}: {}", message(&e)))
}

fn number(v: &JsValue, k: &str) -> anyhow::Result<f64> {
    get(v, k)?
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("unwrapper result: {k} is not a number"))
}

fn typed<T: JsCast>(v: &JsValue, k: &str) -> anyhow::Result<T> {
    get(v, k)?
        .dyn_into::<T>()
        .map_err(|_| anyhow::anyhow!("unwrapper result: {k} has the wrong type"))
}

impl Unwrapper for JsUnwrapper {
    fn unwrap(
        &self,
        meshes: &[MeshInput],
        options: &UnwrapOptions,
    ) -> anyhow::Result<UnwrappedTile> {
        let input = Array::new();
        for m in meshes {
            let o = Object::new();
            let set = |k: &str, v: &JsValue| Reflect::set(&o, &k.into(), v).map(drop);
            let f32s = |v: &[[f32; 3]]| Float32Array::from(v.as_flattened());
            set("positions", &f32s(m.positions)).map_err(|e| anyhow::anyhow!(message(&e)))?;
            if let Some(i) = m.indices {
                set("indices", &Uint32Array::from(i)).map_err(|e| anyhow::anyhow!(message(&e)))?;
            }
            if let Some(n) = m.normals {
                set("normals", &f32s(n)).map_err(|e| anyhow::anyhow!(message(&e)))?;
            }
            if let Some(ids) = m.material_ids {
                set("materialIds", &Uint32Array::from(ids))
                    .map_err(|e| anyhow::anyhow!(message(&e)))?;
            }
            input.push(&o);
        }
        let opts = to_js(&JsOptions {
            texels_per_meter: options.texels_per_meter,
            padding: options.padding,
            chart: &options.chart,
            brute_force: options.brute_force,
        });
        let out = self
            .unwrap_tile(&input, &opts)
            .map_err(|e| anyhow::anyhow!("unwrapper: {}", message(&e)))?;
        let out_meshes: Array = typed(&out, "meshes")?;
        let meshes = out_meshes
            .iter()
            .map(|m| {
                let uv = typed::<Float32Array>(&m, "uv")?.to_vec();
                Ok(UnwrappedMesh {
                    uv: uv.as_chunks::<2>().0.to_vec(),
                    xref: typed::<Uint32Array>(&m, "xref")?.to_vec(),
                    indices: typed::<Uint32Array>(&m, "indices")?.to_vec(),
                    chart: typed::<Int32Array>(&m, "chart")?.to_vec(),
                    charts: number(&m, "charts")? as u32,
                })
            })
            .collect::<anyhow::Result<_>>()?;
        Ok(UnwrappedTile {
            width: number(&out, "width")? as u32,
            height: number(&out, "height")? as u32,
            meshes,
            charts: number(&out, "charts")? as u32,
            utilisation: number(&out, "utilisation")? as f32,
        })
    }
}

/// `UnwrapConfig` of the worker protocol.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UnwrapConfig {
    texels_per_meter: f32,
    padding: u32,
    #[serde(default)]
    xatlas: XatlasConfig,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct XatlasConfig {
    #[serde(flatten)]
    chart: ChartOptions,
    brute_force_packing: bool,
}

/// Unwraps every mesh of an `.lbscene` as its own object tile through `unwrapper`, checking the UVs
/// (INTERFACES.md §8). Resolves to `UnwrappedMeshResult[]` (without `loadMs`, which the worker adds).
#[wasm_bindgen(js_name = unwrapScene)]
pub fn unwrap_scene_js(
    scene: &[u8],
    config: JsValue,
    unwrapper: &JsUnwrapper,
) -> Result<JsValue, JsValue> {
    let config: UnwrapConfig = serde_wasm_bindgen::from_value(config).map_err(|e| {
        reject(BakerError::new(
            "invalid-scene",
            format!("unwrap config: {e}"),
        ))
    })?;
    let scene = Scene::read(scene).map_err(|e| reject(BakerError::invalid_scene(&e)))?;
    let options = UnwrapOptions {
        texels_per_meter: config.texels_per_meter,
        padding: config.padding,
        chart: config.xatlas.chart,
        brute_force: config.xatlas.brute_force_packing,
    };
    let unwraps = unwrap_scene(&scene, unwrapper, &options)
        .map_err(|e| reject(BakerError::new("internal", format!("{e:#}"))))?;
    let out = Array::new();
    for m in unwraps {
        let o = Object::new();
        let set = |k: &str, v: JsValue| {
            Reflect::set(&o, &k.into(), &v).expect("setting a property of a plain object");
        };
        let mesh = &m.tile.meshes[0];
        set("name", m.name.as_str().into());
        set("width", m.tile.width.into());
        set("height", m.tile.height.into());
        set("charts", m.tile.charts.into());
        set("utilisation", m.tile.utilisation.into());
        set("triangles", (m.report.triangles as u32).into());
        set("uv", Float32Array::from(mesh.uv.as_flattened()).into());
        set("xref", Uint32Array::from(mesh.xref.as_slice()).into());
        set("indices", Uint32Array::from(mesh.indices.as_slice()).into());
        set("ms", m.ms.into());
        set("valid", m.report.valid().into());
        set("report", to_js(&m.report));
        out.push(&o);
    }
    Ok(out.into())
}
