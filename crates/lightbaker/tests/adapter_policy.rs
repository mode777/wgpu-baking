//! Which adapters count as software (ADR-0009): they bake only on explicit opt-in.

use lightbaker::gpu::is_software_adapter;
use wgpu::DeviceType::*;

#[test]
fn software_adapters_are_recognised() {
    for name in ["llvmpipe (LLVM 15.0.7, 256 bits)", "lavapipe", "SwiftShader Device (Subzero)", "Microsoft Basic Render Driver"] {
        assert!(is_software_adapter(name, Other), "{name}");
    }
    assert!(is_software_adapter("anything", Cpu));
    assert!(!is_software_adapter("Intel(R) Graphics", IntegratedGpu));
    assert!(!is_software_adapter("NVIDIA GeForce RTX 4070", DiscreteGpu));
}
