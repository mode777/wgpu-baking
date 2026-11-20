//! The GPU the tests run on, shared by the GPU integration tests.

use lightbaker::Gpu;
use lightbaker::gpu::GpuOptions;

/// The GPU to test on, or `None` to skip. `LIGHTBAKER_REQUIRE_GPU=1` turns a skip into a failure (CI);
/// the engine refuses a software adapter unless `LIGHTBAKER_ALLOW_SOFTWARE_ADAPTER=1` (ADR-0009).
pub fn gpu() -> Option<Gpu> {
    let flag = |name: &str| std::env::var(name).is_ok_and(|v| v == "1");
    match pollster::block_on(Gpu::open(&GpuOptions { allow_software: flag("LIGHTBAKER_ALLOW_SOFTWARE_ADAPTER") })) {
        Err(e) => {
            assert!(!flag("LIGHTBAKER_REQUIRE_GPU"), "LIGHTBAKER_REQUIRE_GPU=1 but {e:#}");
            eprintln!("no GPU, skipped: {e:#}");
            None
        }
        Ok(gpu) => {
            eprintln!("GPU adapter: {}{}", gpu.report, if gpu.software { " [SOFTWARE ADAPTER: results are not representative of hardware]" } else { "" });
            Some(gpu)
        }
    }
}
