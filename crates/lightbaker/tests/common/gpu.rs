//! The GPU the tests run on, shared by the GPU integration tests.

use lightbaker::Gpu;

/// The GPU to test on, or `None` to skip. `LIGHTBAKER_REQUIRE_GPU=1` turns a skip into a failure (CI);
/// a software adapter counts as none unless `LIGHTBAKER_ALLOW_SOFTWARE_ADAPTER=1` (ADR-0009).
pub fn gpu() -> Option<Gpu> {
    let flag = |name: &str| std::env::var(name).is_ok_and(|v| v == "1");
    let unavailable = |why: String| {
        assert!(!flag("LIGHTBAKER_REQUIRE_GPU"), "LIGHTBAKER_REQUIRE_GPU=1 but {why}");
        eprintln!("no GPU, skipped: {why}");
        None
    };
    match Gpu::new() {
        Err(e) => unavailable(e.to_string()),
        Ok(gpu) if gpu.software && !flag("LIGHTBAKER_ALLOW_SOFTWARE_ADAPTER") => unavailable(format!("only a software adapter, {}; set LIGHTBAKER_ALLOW_SOFTWARE_ADAPTER=1 to use it", gpu.adapter)),
        Ok(gpu) => {
            eprintln!("GPU adapter: {}{}", gpu.adapter, if gpu.software { " [SOFTWARE ADAPTER: results are not representative of hardware]" } else { "" });
            Some(gpu)
        }
    }
}
