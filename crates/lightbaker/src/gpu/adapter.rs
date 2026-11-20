//! Which adapter the engine bakes on, what it asks of it, and what it reports about it (ADR-0003 points
//! 7–8): software adapters only on explicit opt-in (AGENTS.md rule 2), WebGPU's default limits with the
//! buffer sizes raised to the adapter's, and typed errors a caller can map to its own (INTERFACES.md §7).

use std::fmt;

/// How to open the GPU.
#[derive(Clone, Copy, Debug, Default)]
pub struct GpuOptions {
    /// accept a software/fallback adapter (lavapipe, SwiftShader, WARP, `isFallbackAdapter`); results are labelled
    pub allow_software: bool,
}

/// Why the GPU could not be used. Carried inside `anyhow::Error`; callers `downcast_ref::<GpuError>()`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GpuError {
    /// no WebGPU / no adapter at all
    NoAdapter(String),
    /// the only adapter is a software one and the caller did not allow it
    SoftwareAdapterRefused(String),
    /// the adapter would not give a device with the limits asked for
    NoDevice(String),
    /// a buffer the bake needs is larger than the device allows
    LimitExceeded { buffer: String, bytes: u64, limit: u64, limit_name: &'static str },
    /// the device was lost (driver reset, `destroy()`, the browser reclaiming it); the reason as reported
    DeviceLost(String),
    /// the device ran out of memory
    OutOfMemory(String),
    /// a GPU validation or internal error
    Gpu(String),
}

impl fmt::Display for GpuError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GpuError::NoAdapter(e) => write!(f, "no GPU adapter: {e}"),
            GpuError::SoftwareAdapterRefused(name) => write!(f, "the only GPU adapter is a software one ({name}); allow software adapters to bake on it anyway (slow, results labelled)"),
            GpuError::NoDevice(e) => write!(f, "no GPU device: {e}"),
            GpuError::LimitExceeded { buffer, bytes, limit, limit_name } => write!(f, "the {buffer} buffer needs {bytes} bytes, more than the device's {limit_name} of {limit}"),
            GpuError::DeviceLost(why) => write!(f, "the GPU device was lost ({why})"),
            GpuError::OutOfMemory(e) => write!(f, "the GPU ran out of memory: {e}"),
            GpuError::Gpu(e) => write!(f, "GPU error: {e}"),
        }
    }
}

impl std::error::Error for GpuError {}

/// The adapter and device a bake runs on.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterReport {
    pub name: String,
    pub vendor: u32,
    pub device: u32,
    /// `IntegratedGpu`, `DiscreteGpu`, `VirtualGpu`, `Cpu` or `Other`
    pub device_type: String,
    /// `Vulkan`, `Dx12`, `Metal`, `BrowserWebGpu`, …
    pub backend: String,
    pub driver: String,
    pub driver_info: String,
    /// a software/fallback adapter: correct but slow, results labelled
    pub software: bool,
    pub limits: GrantedLimits,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantedLimits {
    pub max_storage_buffer_binding_size: u64,
    pub max_buffer_size: u64,
    pub max_compute_workgroups_per_dimension: u32,
}

impl GrantedLimits {
    pub fn of(l: &wgpu::Limits) -> GrantedLimits {
        GrantedLimits {
            max_storage_buffer_binding_size: l.max_storage_buffer_binding_size as u64,
            max_buffer_size: l.max_buffer_size,
            max_compute_workgroups_per_dimension: l.max_compute_workgroups_per_dimension,
        }
    }

    /// A buffer of `bytes` bound as storage fits (`LimitExceeded` otherwise).
    pub fn check_storage(&self, buffer: &str, bytes: u64) -> Result<(), GpuError> {
        if bytes > self.max_buffer_size {
            return Err(GpuError::LimitExceeded { buffer: buffer.into(), bytes, limit: self.max_buffer_size, limit_name: "maxBufferSize" });
        }
        if bytes > self.max_storage_buffer_binding_size {
            return Err(GpuError::LimitExceeded { buffer: buffer.into(), bytes, limit: self.max_storage_buffer_binding_size, limit_name: "maxStorageBufferBindingSize" });
        }
        Ok(())
    }
}

impl AdapterReport {
    /// Completes a report with what the browser tells about its adapter (`GPUAdapterInfo`), which wgpu does
    /// not pass on: a name when wgpu has none, and software detection by vendor/architecture (Chrome need
    /// not flag lavapipe or SwiftShader as `isFallbackAdapter`).
    pub fn apply_browser_info(&mut self, vendor: &str, architecture: &str, description: &str, is_fallback: bool) {
        let label = [vendor, architecture, description].iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join(" ");
        if self.name.is_empty() {
            self.name = if label.is_empty() { "unnamed WebGPU adapter".into() } else { label.clone() };
        }
        if self.driver.is_empty() {
            self.driver = label.clone();
        }
        self.software |= is_fallback || is_software_adapter(&label, wgpu::DeviceType::Other);
    }

    pub fn new(info: &wgpu::AdapterInfo, limits: &wgpu::Limits) -> AdapterReport {
        AdapterReport {
            name: info.name.clone(),
            vendor: info.vendor,
            device: info.device,
            device_type: format!("{:?}", info.device_type),
            backend: format!("{:?}", info.backend),
            driver: info.driver.clone(),
            driver_info: info.driver_info.clone(),
            software: is_software_adapter(&info.name, info.device_type),
            limits: GrantedLimits::of(limits),
        }
    }
}

impl fmt::Display for AdapterReport {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let l = &self.limits;
        write!(
            f,
            "{} ({}, {}{}), limits: storage binding {} MiB, buffer {} MiB, workgroups/dim {}",
            self.name,
            self.backend,
            self.device_type,
            if self.software { ", SOFTWARE" } else { "" },
            l.max_storage_buffer_binding_size >> 20,
            l.max_buffer_size >> 20,
            l.max_compute_workgroups_per_dimension
        )
    }
}

/// Software adapters by device type (wgpu reports WebGPU's `isFallbackAdapter` as `Cpu`) or by the names
/// they report (ADR-0009).
pub fn is_software_adapter(name: &str, device_type: wgpu::DeviceType) -> bool {
    let name = name.to_lowercase();
    device_type == wgpu::DeviceType::Cpu
        || ["llvmpipe", "lavapipe", "swiftshader", "microsoft basic render driver", "warp"].iter().any(|s| name.contains(s))
}

/// Whether an adapter may be used.
pub fn admit(name: &str, software: bool, options: &GpuOptions) -> Result<(), GpuError> {
    if software && !options.allow_software { Err(GpuError::SoftwareAdapterRefused(name.into())) } else { Ok(()) }
}

/// WebGPU's default limits (every conformant adapter grants them), with the two buffer sizes raised to
/// what the adapter allows so large atlases can be bound where possible.
pub fn required_limits(adapter: &wgpu::Limits) -> wgpu::Limits {
    let d = wgpu::Limits::defaults();
    wgpu::Limits {
        max_storage_buffer_binding_size: adapter.max_storage_buffer_binding_size.max(d.max_storage_buffer_binding_size),
        max_buffer_size: adapter.max_buffer_size.max(d.max_buffer_size),
        ..d
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn software_needs_opt_in() {
        let refuse = GpuOptions::default();
        let allow = GpuOptions { allow_software: true };
        assert_eq!(admit("llvmpipe", true, &refuse), Err(GpuError::SoftwareAdapterRefused("llvmpipe".into())));
        assert_eq!(admit("llvmpipe", true, &allow), Ok(()));
        assert_eq!(admit("Intel(R) Graphics", false, &refuse), Ok(()));
    }

    #[test]
    fn limits_raise_only_buffer_sizes() {
        let adapter = wgpu::Limits { max_storage_buffer_binding_size: 2 << 30, max_buffer_size: 4 << 30, max_compute_workgroups_per_dimension: 1 << 20, max_bind_groups: 32, ..wgpu::Limits::defaults() };
        let r = required_limits(&adapter);
        let d = wgpu::Limits::defaults();
        assert_eq!((r.max_storage_buffer_binding_size, r.max_buffer_size), (adapter.max_storage_buffer_binding_size, adapter.max_buffer_size));
        assert_eq!((r.max_compute_workgroups_per_dimension, r.max_bind_groups), (d.max_compute_workgroups_per_dimension, d.max_bind_groups));
    }

    #[test]
    fn storage_size_checked() {
        let g = GrantedLimits { max_storage_buffer_binding_size: 128 << 20, max_buffer_size: 256 << 20, max_compute_workgroups_per_dimension: 65535 };
        assert!(g.check_storage("light", 128 << 20).is_ok());
        let e = g.check_storage("light", (128 << 20) + 16).unwrap_err();
        assert!(matches!(e, GpuError::LimitExceeded { limit_name: "maxStorageBufferBindingSize", .. }), "{e}");
        assert!(matches!(g.check_storage("texels", 300 << 20), Err(GpuError::LimitExceeded { limit_name: "maxBufferSize", .. })));
    }

    #[test]
    fn browser_info_names_and_flags_software() {
        let limits = GrantedLimits { max_storage_buffer_binding_size: 1, max_buffer_size: 1, max_compute_workgroups_per_dimension: 1 };
        let blank = AdapterReport { name: String::new(), vendor: 0, device: 0, device_type: "Other".into(), backend: "BrowserWebGpu".into(), driver: String::new(), driver_info: String::new(), software: false, limits };
        let mut intel = blank.clone();
        intel.apply_browser_info("intel", "xe-3lpg", "", false);
        assert_eq!((intel.name.as_str(), intel.software), ("intel xe-3lpg", false));
        let mut lavapipe = blank.clone();
        lavapipe.apply_browser_info("mesa", "llvmpipe", "", false);
        assert!(lavapipe.software, "lavapipe without isFallbackAdapter is still software");
        let mut swiftshader = blank.clone();
        swiftshader.apply_browser_info("google", "swiftshader", "", false);
        assert!(swiftshader.software);
        let mut fallback = blank.clone();
        fallback.apply_browser_info("", "", "", true);
        assert!(fallback.software);
        assert_eq!(fallback.name, "unnamed WebGPU adapter");
    }

    #[test]
    fn software_adapters_are_recognised() {
        use wgpu::DeviceType::*;
        for name in ["llvmpipe (LLVM 15.0.7, 256 bits)", "lavapipe", "SwiftShader Device (Subzero)", "Microsoft Basic Render Driver"] {
            assert!(is_software_adapter(name, Other), "{name}");
        }
        assert!(is_software_adapter("anything", Cpu));
        assert!(!is_software_adapter("Intel(R) Graphics", IntegratedGpu));
    }
}
