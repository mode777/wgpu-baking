//! The structs Rust uploads and trace.wgsl reads must match byte for byte (AGENTS.md "GPU and shader rules"):
//! the shader is parsed with naga and each struct's size and member offsets compared with Rust's.

use std::mem::{offset_of, size_of};

use lightbaker::bvh::{Node, Tri};
use lightbaker::gpu::Params;
use lightbaker::raster::Texel;

/// A WGSL struct's span and its members' (name, offset).
fn wgsl(name: &str) -> (u32, Vec<(String, u32)>) {
    let module = naga::front::wgsl::parse_str(include_str!("../src/gpu/trace.wgsl")).expect("trace.wgsl parses");
    let mut layouter = naga::proc::Layouter::default();
    layouter.update(module.to_ctx()).unwrap();
    let ty = module.types.iter().find(|(_, t)| t.name.as_deref() == Some(name)).unwrap_or_else(|| panic!("no struct {name} in trace.wgsl")).1;
    match &ty.inner {
        naga::TypeInner::Struct { members, span } => (*span, members.iter().map(|m| (m.name.clone().unwrap(), m.offset)).collect()),
        other => panic!("{name} is not a struct: {other:?}"),
    }
}

fn check(name: &str, size: usize, rust: &[(&str, usize)]) {
    let (span, members) = wgsl(name);
    assert_eq!(span as usize, size, "{name}: WGSL {span} bytes, Rust {size}");
    let rust: Vec<(String, u32)> = rust.iter().map(|(n, o)| (n.to_string(), *o as u32)).collect();
    assert_eq!(members, rust, "{name}: member offsets differ");
}

#[test]
fn rust_and_wgsl_structs_agree() {
    check("Node", size_of::<Node>(), &[("min", offset_of!(Node, min)), ("first", offset_of!(Node, first)), ("max", offset_of!(Node, max)), ("count", offset_of!(Node, count))]);
    check("Tri", size_of::<Tri>(), &[("v0", offset_of!(Tri, v0)), ("e1", offset_of!(Tri, e1)), ("e2", offset_of!(Tri, e2)), ("uv01", offset_of!(Tri, uv01)), ("uv2", offset_of!(Tri, uv2))]);
    check("Texel", size_of::<Texel>(), &[("pos", offset_of!(Texel, pos)), ("normal", offset_of!(Texel, normal)), ("dx", offset_of!(Texel, dx)), ("dy", offset_of!(Texel, dy))]);
    check(
        "Params",
        size_of::<Params>(),
        &[
            ("sun_dir", offset_of!(Params, sun_dir)),
            ("sun_irr", offset_of!(Params, sun_irr)),
            ("zenith", offset_of!(Params, zenith)),
            ("horizon", offset_of!(Params, horizon)),
            ("ground", offset_of!(Params, ground)),
            ("fallback", offset_of!(Params, fallback)),
            ("counts", offset_of!(Params, counts)),
            ("reach", offset_of!(Params, reach)),
            ("atlas", offset_of!(Params, atlas)),
        ],
    );
    assert_eq!(size_of::<Tri>(), 80);
}

#[test]
fn bindings_fit_webgpu_defaults() {
    let module = naga::front::wgsl::parse_str(include_str!("../src/gpu/trace.wgsl")).unwrap();
    let storage = module.global_variables.iter().filter(|(_, v)| matches!(v.space, naga::AddressSpace::Storage { .. })).count();
    let uniform = module.global_variables.iter().filter(|(_, v)| v.space == naga::AddressSpace::Uniform).count();
    assert_eq!((storage, uniform), (6, 1));
    assert!(storage <= 8, "WebGPU's default maxStorageBuffersPerShaderStage");
}
