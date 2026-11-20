//! Builds the vendored xatlas (jpcy/xatlas f700c77) with its C API, single-threaded like `xatlas-wasm`
//! (ADR-0006), and a probe of its struct sizes. Nothing to build for wasm32: the browser uses `xatlas-wasm`.

fn main() {
    println!("cargo:rerun-if-changed=vendor/xatlas");
    println!("cargo:rerun-if-changed=src/probe.cpp");
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        return;
    }
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .include("vendor/xatlas")
        .file("vendor/xatlas/xatlas.cpp")
        .file("src/probe.cpp")
        .define("XATLAS_C_API", "1")
        .define("XA_MULTITHREADED", "0")
        // optimised even in debug builds: unwrapping is slow otherwise
        .opt_level(2)
        .warnings(false)
        .compile("xatlas");
}
