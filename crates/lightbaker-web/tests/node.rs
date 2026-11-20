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
