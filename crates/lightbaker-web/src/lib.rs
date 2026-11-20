//! lightbaker in the browser: the wasm-bindgen API a dedicated Web Worker uses (ADR-0002, INTERFACES.md §8).
//! `capabilities()` checks WebGPU without opening a device; `WebBaker` owns one GPU device, a tile store kept
//! between bakes, and the flags `cancel()` sets while a bake awaits the GPU. Every rejection is a
//! `{ kind, message }` object (`BakerError` kinds, plus `cancelled`).

pub mod config;
pub mod errors;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use js_sys::{Float32Array, Function, Object, Promise, Reflect, Uint8Array};
use lightbaker::cache::{MemoryStore, TileStore};
use lightbaker::expect::Expect;
use lightbaker::gpu::{AdapterReport, GpuOptions};
use lightbaker::progress::Progress;
use lightbaker::{Bake, Gpu, Options, Scene, bake, output};
use serde::Serialize;
use wasm_bindgen::prelude::*;

pub use errors::BakerError;

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn console_log(s: &str);
}

fn log_line(s: &str) {
    console_log(&format!("lightbaker: {s}"));
}

/// Plain JS values: objects for structs and maps, numbers (not BigInt) for 64-bit integers.
fn to_js<T: Serialize + ?Sized>(v: &T) -> JsValue {
    v.serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .unwrap_or_else(|e| JsValue::from_str(&e.to_string()))
}

fn reject(e: BakerError) -> JsValue {
    to_js(&e)
}

/// Why WebGPU is unusable here, if it is (`navigator.gpu` missing: an old browser, a non-secure context, Node).
fn no_webgpu() -> Option<String> {
    let navigator = Reflect::get(&js_sys::global(), &"navigator".into())
        .ok()
        .filter(|n| !n.is_undefined() && !n.is_null());
    let Some(navigator) = navigator else {
        return Some("no navigator: not a browser or worker".into());
    };
    let gpu = Reflect::get(&navigator, &"gpu".into())
        .ok()
        .filter(|g| !g.is_undefined() && !g.is_null());
    if gpu.is_none() {
        return Some("navigator.gpu is undefined: this browser, context or worker has no WebGPU (Chrome/Edge 113+ over https or localhost)".into());
    }
    None
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Capabilities {
    webgpu: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    adapter: Option<AdapterCaps>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AdapterCaps {
    #[serde(flatten)]
    report: AdapterReport,
    /// WebGPU's `isFallbackAdapter` (wgpu reports it as device type `Cpu`)
    is_fallback_adapter: bool,
}

impl AdapterCaps {
    fn new(report: AdapterReport) -> AdapterCaps {
        AdapterCaps {
            is_fallback_adapter: report.device_type == "Cpu",
            report,
        }
    }
}

/// Whether this context can bake: WebGPU present, the adapter it would use and its limits. Opens no device.
#[wasm_bindgen]
pub async fn capabilities() -> Result<JsValue, JsValue> {
    if let Some(reason) = no_webgpu() {
        return Ok(to_js(&Capabilities {
            webgpu: false,
            adapter: None,
            reason: Some(reason),
        }));
    }
    Ok(to_js(&match Gpu::probe().await {
        Ok(report) => Capabilities {
            webgpu: true,
            adapter: Some(AdapterCaps::new(report)),
            reason: None,
        },
        Err(e) => Capabilities {
            webgpu: false,
            adapter: None,
            reason: Some(format!("{e:#}")),
        },
    }))
}

struct Inner {
    gpu: Gpu,
    store: MemoryStore,
    cancel: Cell<bool>,
    busy: Cell<bool>,
    lost: RefCell<Option<String>>,
    on_lost: RefCell<Option<Function>>,
    last: RefCell<Option<(Scene, Bake)>>,
}

/// One baker: a GPU device and the tiles it baked, one bake at a time.
#[wasm_bindgen]
pub struct WebBaker {
    inner: Rc<Inner>,
}

#[wasm_bindgen]
impl WebBaker {
    /// Opens the GPU. A software/fallback adapter is refused unless `allow_fallback_adapter` (results are labelled).
    pub async fn create(allow_fallback_adapter: bool) -> Result<WebBaker, JsValue> {
        if let Some(reason) = no_webgpu() {
            return Err(reject(BakerError::new("no-webgpu", reason)));
        }
        let gpu = Gpu::open(&GpuOptions {
            allow_software: allow_fallback_adapter,
        })
        .await
        .map_err(|e| reject(BakerError::from_engine(&e)))?;
        let lost = gpu.take_device_lost();
        let inner = Rc::new(Inner {
            gpu,
            store: MemoryStore::new(),
            cancel: Cell::new(false),
            busy: Cell::new(false),
            lost: RefCell::new(None),
            on_lost: RefCell::new(None),
            last: RefCell::new(None),
        });
        if let Some(lost) = lost {
            let weak = Rc::downgrade(&inner);
            wasm_bindgen_futures::spawn_local(async move {
                let Ok(why) = lost.await else { return };
                let Some(inner) = weak.upgrade() else { return };
                inner.lost.replace(Some(why.clone()));
                inner.cancel.set(true);
                if let Some(f) = inner.on_lost.borrow().as_ref() {
                    let _ = f.call1(&JsValue::NULL, &JsValue::from_str(&why));
                }
            });
        }
        Ok(WebBaker { inner })
    }

    /// The adapter and the device's granted limits (`software`: a fallback adapter, results labelled).
    pub fn adapter(&self) -> JsValue {
        to_js(&AdapterCaps::new(self.inner.gpu.report.clone()))
    }

    /// Bakes an `.lbscene` with `config_json` overrides (a partial `BakeConfig`; "" for none). `on_progress`
    /// receives `{ phase, bounce, done, total }`. Resolves to the bake's outputs; rejects with `{ kind, message }`.
    pub fn bake(
        &self,
        scene: Vec<u8>,
        config_json: String,
        on_progress: Option<Function>,
    ) -> Promise {
        let inner = self.inner.clone();
        wasm_bindgen_futures::future_to_promise(async move {
            if inner.busy.replace(true) {
                return Err(reject(BakerError::new(
                    "internal",
                    "a bake is already running on this baker: cancel it first",
                )));
            }
            inner.cancel.set(false);
            let result = run(&inner, &scene, &config_json, on_progress.as_ref()).await;
            inner.busy.set(false);
            result.map_err(reject)
        })
    }

    /// Stops the running bake at its next GPU batch (it rejects with kind `cancelled`).
    pub fn cancel(&self) {
        self.inner.cancel.set(true);
    }

    /// Checks the last bake against an `expect.json` (docs/TESTING.md): every probe value and check outcome.
    pub fn evaluate(&self, expect_json: &str) -> Result<JsValue, JsValue> {
        let expect = Expect::parse(expect_json)
            .map_err(|e| reject(BakerError::new("internal", format!("expect.json: {e:#}"))))?;
        let last = self.inner.last.borrow();
        let (scene, baked) = last
            .as_ref()
            .ok_or_else(|| reject(BakerError::new("internal", "no bake to evaluate yet")))?;
        let report = expect
            .evaluate(scene, baked)
            .map_err(|e| reject(BakerError::new("internal", format!("{e:#}"))))?;
        Ok(to_js(
            &serde_json::json!({ "passed": report.passed(), "values": report.values, "outcomes": report.outcomes }),
        ))
    }

    /// Forgets every baked tile (the next bake bakes everything).
    #[wasm_bindgen(js_name = clearCache)]
    pub fn clear_cache(&self) {
        self.inner.store.clear();
    }

    /// Bytes held by the tile store.
    #[wasm_bindgen(js_name = cacheBytes)]
    pub fn cache_bytes(&self) -> f64 {
        self.inner.store.bytes() as f64
    }

    /// `cb(reason)` when the device is lost (then this baker is unusable: create a new one).
    #[wasm_bindgen(js_name = setDeviceLostCallback)]
    pub fn set_device_lost_callback(&self, cb: Function) {
        if let Some(why) = self.inner.lost.borrow().as_ref() {
            let _ = cb.call1(&JsValue::NULL, &JsValue::from_str(why));
        }
        self.inner.on_lost.replace(Some(cb));
    }

    /// Destroys the GPU device as a driver reset would (manual device-loss checks, docs/TESTING.md).
    #[wasm_bindgen(js_name = loseDeviceForTesting)]
    pub fn lose_device_for_testing(&self) {
        self.inner.gpu.destroy();
    }
}

async fn run(
    inner: &Inner,
    bytes: &[u8],
    config_json: &str,
    on_progress: Option<&Function>,
) -> Result<JsValue, BakerError> {
    if let Some(why) = inner.lost.borrow().as_ref() {
        return Err(BakerError::new(
            "device-lost",
            format!("the GPU device was lost ({why}): create a new baker"),
        ));
    }
    let mut scene = Scene::read(bytes).map_err(|e| BakerError::invalid_scene(&e))?;
    let original = scene.header.clone();
    let applied = config::apply(&mut scene.header, config_json)
        .map_err(|e| BakerError::new("invalid-scene", format!("bake config: {e:#}")))?;
    if serde_json::to_value(&scene.header).ok() != serde_json::to_value(&original).ok() {
        // the scene's hash (and so its tiles' hashes) must follow the settings actually baked
        let rewritten = Scene::write(&scene.header, &scene.positions)
            .map_err(|e| BakerError::invalid_scene(&e))?;
        scene = Scene::read(&rewritten).map_err(|e| BakerError::invalid_scene(&e))?;
    }
    let progress = |p: Progress| {
        if let Some(f) = on_progress {
            let _ = f.call1(&JsValue::NULL, &to_js(&p));
        }
    };
    let cancel = || inner.cancel.get();
    let store = applied
        .reuse_tiles
        .then_some(&inner.store as &dyn TileStore);
    inner.gpu.reset_transfers();
    let options = Options {
        cache: store,
        log: &log_line,
        progress: Some(&progress),
        cancel: Some(&cancel),
    };
    let baked = bake(&scene, &inner.gpu, &options)
        .await
        .map_err(|e| BakerError::from_engine(&e))?;
    let lbake = output::write(&scene, &baked).map_err(|e| BakerError::from_engine(&e))?;
    let out = outputs(inner, &scene, &baked, &lbake, &applied.warnings);
    inner.last.replace(Some((scene, baked)));
    Ok(out)
}

/// The bake as the worker hands it on: typed arrays (copies, transferable) and plain metadata.
fn outputs(inner: &Inner, scene: &Scene, b: &Bake, lbake: &[u8], warnings: &[String]) -> JsValue {
    let o = Object::new();
    let set = |k: &str, v: &JsValue| {
        let _ = Reflect::set(&o, &k.into(), v);
    };
    let l = &b.layout;
    set("lbake", &Uint8Array::from(lbake).into());
    set(
        "atlas",
        &to_js(&serde_json::json!({ "width": l.width, "height": l.height })),
    );
    set(
        "direct",
        &Float32Array::from(bytemuck_flat(&b.direct)).into(),
    );
    set("validity", &Uint8Array::from(&b.validity[..]).into());
    let indirect = Object::new();
    let _ = Reflect::set(&indirect, &"width".into(), &b.indirect_size.0.into());
    let _ = Reflect::set(&indirect, &"height".into(), &b.indirect_size.1.into());
    let _ = Reflect::set(
        &indirect,
        &"data".into(),
        &Float32Array::from(bytemuck_flat(&b.indirect)).into(),
    );
    set("indirect", &indirect.into());
    if let Some(v) = &b.volume {
        let vol = to_js(&serde_json::json!({ "origin": v.origin, "step": v.step, "size": v.size }));
        let flat: Vec<f32> = v
            .probes
            .iter()
            .flat_map(|p| p.iter().flatten().copied())
            .collect();
        let _ = Reflect::set(
            &vol,
            &"probes".into(),
            &Float32Array::from(&flat[..]).into(),
        );
        set("volume", &vol);
    }
    let gpu = &inner.gpu;
    set(
        "stats",
        &to_js(&serde_json::json!({
            "triangles": scene.triangle_count(),
            "charts": l.charts.len(),
            "bake": b.stats,
            "adapter": gpu.adapter,
            "software": gpu.software,
            "transfers": gpu.transfers(),
            "cacheBytes": inner.store.bytes(),
        })),
    );
    set("adapter", &to_js(&AdapterCaps::new(gpu.report.clone())));
    set("warnings", &to_js(warnings));
    o.into()
}

fn bytemuck_flat<const N: usize>(v: &[[f32; N]]) -> &[f32] {
    v.as_flattened()
}
