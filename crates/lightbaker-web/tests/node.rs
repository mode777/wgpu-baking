//! The bridge without WebGPU (Node has `navigator` but no `navigator.gpu`): capabilities say why, and a baker
//! cannot be created. Run with `wasm-pack test --node crates/lightbaker-web`.
#![cfg(target_arch = "wasm32")]

use js_sys::Reflect;
use lightbaker_web::{WebBaker, capabilities};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;

fn get(v: &JsValue, k: &str) -> JsValue {
    Reflect::get(v, &k.into()).unwrap()
}

#[wasm_bindgen_test]
async fn capabilities_without_webgpu() {
    let caps = capabilities().await.unwrap();
    assert_eq!(get(&caps, "webgpu").as_bool(), Some(false));
    let reason = get(&caps, "reason").as_string().unwrap();
    assert!(reason.contains("navigator.gpu"), "{reason}");
    assert!(get(&caps, "adapter").is_undefined());
}

#[wasm_bindgen_test]
async fn create_without_webgpu_rejects_with_kind() {
    let err = WebBaker::create(false)
        .await
        .err()
        .expect("no baker without WebGPU");
    assert_eq!(get(&err, "kind").as_string().as_deref(), Some("no-webgpu"));
    assert!(
        get(&err, "message")
            .as_string()
            .is_some_and(|m| !m.is_empty())
    );
}

/// An unwrapper laying out each triangle on its own, like the xatlas adapter's output shape.
fn fake_unwrapper(body: &str) -> lightbaker_web::unwrap::JsUnwrapper {
    use wasm_bindgen::JsCast;
    let o = js_sys::Object::new();
    let f = js_sys::Function::new_with_args("meshes, options", body);
    Reflect::set(&o, &"unwrapTile".into(), &f).unwrap();
    o.unchecked_into()
}

const PER_TRIANGLE: &str = r#"
    if (options.texelsPerMeter !== 4 || options.padding !== 2 || options.chart.maxIterations !== 2 || 'maxCost' in options.chart) throw new Error('options ' + JSON.stringify(options));
    const n = meshes[0].positions.length / 3;
    const uv = new Float32Array(n * 2);
    for (let c = 0; c < n; c++) { const x = Math.floor(c / 3) * 4, k = c % 3; uv[c * 2] = k === 2 ? x + 2 : x; uv[c * 2 + 1] = k === 1 ? 2 : 0; }
    const seq = Uint32Array.from({ length: n }, (_, i) => i);
    return { width: (n / 3) * 4, height: 3, charts: n / 3, utilisation: 0.5,
             meshes: [{ uv, xref: seq, indices: seq.slice(), chart: Int32Array.from(seq, (i) => Math.floor(i / 3)), charts: n / 3 }] };
"#;

const SCENE: &[u8] = include_bytes!("../../../test-scenes/upstream-basic/upstream-basic.lbscene");

fn config() -> JsValue {
    js_sys::JSON::parse(r#"{"texelsPerMeter":4,"padding":2,"xatlas":{"maxIterations":2}}"#).unwrap()
}

#[wasm_bindgen_test]
fn unwrap_scene_through_a_js_unwrapper() {
    let out =
        lightbaker_web::unwrap::unwrap_scene_js(SCENE, config(), &fake_unwrapper(PER_TRIANGLE))
            .unwrap();
    let meshes: js_sys::Array = out.into();
    let names: Vec<String> = meshes
        .iter()
        .map(|m| get(&m, "name").as_string().unwrap())
        .collect();
    assert_eq!(names, ["ground", "box", "wall", "strip"]);
    for m in meshes.iter() {
        assert_eq!(
            get(&m, "valid").as_bool(),
            Some(true),
            "{:?}",
            get(&m, "report")
        );
        let tris = get(&m, "triangles").as_f64().unwrap();
        assert_eq!(
            js_sys::Float32Array::from(get(&m, "uv")).length() as f64,
            tris * 6.0
        );
    }
}

#[wasm_bindgen_test]
fn unwrapper_errors_are_internal() {
    let err = lightbaker_web::unwrap::unwrap_scene_js(
        SCENE,
        config(),
        &fake_unwrapper("throw new Error('xatlas: boom')"),
    )
    .unwrap_err();
    assert_eq!(get(&err, "kind").as_string().as_deref(), Some("internal"));
    assert!(
        get(&err, "message")
            .as_string()
            .unwrap()
            .contains("xatlas: boom")
    );
    let err =
        lightbaker_web::unwrap::unwrap_scene_js(b"nope", config(), &fake_unwrapper(PER_TRIANGLE))
            .unwrap_err();
    assert_eq!(
        get(&err, "kind").as_string().as_deref(),
        Some("invalid-scene")
    );
}
