//! The GPU side: one device, the scene's buffers, and the three passes of trace.wgsl run in batches
//! (each batch submitted and waited on, so no dispatch runs long enough for the driver to reset it).

use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::{Arc, Mutex};

use anyhow::{Result, anyhow};
use bytemuck::{Pod, Zeroable};

mod adapter;
pub use adapter::*;

use crate::bvh::Bvh;
use crate::progress::{Hooks, Phase};
use crate::raster::Texel;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct Params {
    pub sun_dir: [f32; 4],
    pub sun_irr: [f32; 4],
    pub zenith: [f32; 4],
    pub horizon: [f32; 4],
    pub ground: [f32; 4],
    pub fallback: [f32; 4],
    pub counts: [u32; 4],
    pub reach: [f32; 4],
    pub atlas: [u32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pass {
    /// per texel: soft sun, hard sun bits, occlusion, back faces
    Direct,
    /// per texel: irradiance rgb, back faces
    Gather,
    /// per probe, four values: L1 coefficients (rgb) and the sun, back faces
    Volume,
}

impl Pass {
    fn entry(self) -> &'static str {
        match self {
            Pass::Direct => "direct",
            Pass::Gather => "gather",
            Pass::Volume => "volume",
        }
    }
    /// results an item writes
    fn width(self) -> usize {
        if self == Pass::Volume { 4 } else { 1 }
    }
    /// items a dispatch: the costlier the item, the fewer
    fn batch(self) -> usize {
        match self {
            Pass::Direct => 65536,
            Pass::Gather => 16384,
            Pass::Volume => 8192,
        }
    }
}

pub struct Gpu {
    device: wgpu::Device,
    queue: wgpu::Queue,
    layout: wgpu::BindGroupLayout,
    pipelines: [wgpu::ComputePipeline; 3],
    pub adapter: String,
    /// A software (CPU) adapter such as lavapipe/llvmpipe, SwiftShader or WARP: usable only on explicit opt-in.
    pub software: bool,
    /// the adapter, its device's granted limits, and whether it is software
    pub report: AdapterReport,
    transfers: [AtomicU64; 4],
    failure: Arc<Mutex<Failure>>,
    lost: Mutex<Option<futures_channel::oneshot::Receiver<String>>>,
}

/// What went wrong on the device outside any call: an uncaptured GPU error (taken by the next check) or its loss (for good).
#[derive(Default)]
struct Failure {
    error: Option<GpuError>,
    lost: Option<String>,
}

/// What crossed between CPU and GPU since the last reset: dispatched batches, buffers read back and their bytes, bytes uploaded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Transfers {
    pub batches: u64,
    pub readbacks: u64,
    pub readback_bytes: u64,
    pub upload_bytes: u64,
}

/// The scene as the passes read it.
pub struct SceneBuffers {
    nodes: wgpu::Buffer,
    tris: wgpu::Buffer,
    materials: wgpu::Buffer,
    charts: wgpu::Buffer,
}

impl Gpu {
    /// Opens the GPU, blocking until it is ready, refusing software adapters (native only; the web awaits [`Gpu::open`]).
    #[cfg(feature = "native")]
    pub fn new() -> Result<Gpu> {
        pollster::block_on(Self::open(&GpuOptions::default()))
    }

    /// The adapter a bake would use and its limits, without opening a device (capability checks).
    pub async fn probe() -> Result<AdapterReport> {
        let adapter = Self::adapter().await?;
        Ok(AdapterReport::new(&adapter.get_info(), &adapter.limits()))
    }

    async fn adapter() -> Result<wgpu::Adapter> {
        // the browser: WebGPU only, whatever features the workspace build unified (never WebGL: no compute)
        #[cfg(target_arch = "wasm32")]
        let desc = wgpu::InstanceDescriptor { backends: wgpu::Backends::BROWSER_WEBGPU, ..wgpu::InstanceDescriptor::new_without_display_handle() };
        #[cfg(not(target_arch = "wasm32"))]
        let desc = wgpu::InstanceDescriptor::new_without_display_handle_from_env();
        let instance = wgpu::Instance::new(desc);
        Ok(instance
            .request_adapter(&wgpu::RequestAdapterOptions { power_preference: wgpu::PowerPreference::HighPerformance, ..Default::default() })
            .await
            .map_err(|e| GpuError::NoAdapter(e.to_string()))?)
    }

    pub async fn open(options: &GpuOptions) -> Result<Gpu> {
        let adapter = Self::adapter().await?;
        let info = adapter.get_info();
        admit(&info.name, is_software_adapter(&info.name, info.device_type), options)?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor { label: Some("lightbaker"), required_limits: required_limits(&adapter.limits()), ..Default::default() })
            .await
            .map_err(|e| GpuError::NoDevice(e.to_string()))?;
        let report = AdapterReport::new(&info, &device.limits());
        let failure = Arc::new(Mutex::new(Failure::default()));
        let f = failure.clone();
        device.on_uncaptured_error(Arc::new(move |e: wgpu::Error| {
            let mut f = f.lock().unwrap();
            if f.error.is_none() {
                f.error = Some(match e {
                    wgpu::Error::OutOfMemory { .. } => GpuError::OutOfMemory(e.to_string()),
                    _ => GpuError::Gpu(e.to_string()),
                });
            }
        }));
        let (lost_tx, lost_rx) = futures_channel::oneshot::channel();
        let (f, lost_tx) = (failure.clone(), Mutex::new(Some(lost_tx)));
        device.set_device_lost_callback(move |reason, message| {
            let why = format!("{reason:?}: {message}");
            f.lock().unwrap().lost.get_or_insert(why.clone());
            if let Some(tx) = lost_tx.lock().unwrap().take() {
                let _ = tx.send(why);
            }
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("trace"), source: wgpu::ShaderSource::Wgsl(include_str!("trace.wgsl").into()) });
        let storage = |binding: u32, read_only: bool| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only }, has_dynamic_offset: false, min_binding_size: None },
            count: None,
        };
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("trace"),
            entries: &[
                storage(0, true),
                storage(1, true),
                storage(2, true),
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                storage(4, true),
                storage(5, false),
                storage(6, true),
                storage(7, true),
            ],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("trace"), bind_group_layouts: &[Some(&layout)], ..Default::default() });
        let pipeline = |entry: &str| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&pl),
                module: &module,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let pipelines = [pipeline("direct"), pipeline("gather"), pipeline("volume")];
        let software = report.software;
        let adapter = format!("{} ({:?}{})", info.name, info.backend, if software { ", software" } else { "" });
        Ok(Gpu { device, queue, layout, pipelines, adapter, software, report, transfers: Default::default(), failure, lost: Mutex::new(Some(lost_rx)) })
    }

    /// Fails if the device was lost or reported an error since the last check (the error is then cleared).
    pub fn check(&self) -> Result<(), GpuError> {
        let mut f = self.failure.lock().unwrap();
        if let Some(why) = &f.lost {
            return Err(GpuError::DeviceLost(why.clone()));
        }
        f.error.take().map_or(Ok(()), Err)
    }

    /// Resolves with the reason when the device is lost; taken once (the bridge forwards it to JS).
    pub fn take_device_lost(&self) -> Option<futures_channel::oneshot::Receiver<String>> {
        self.lost.lock().unwrap().take()
    }

    /// Destroys the device, as a lost one would be (device-loss tests).
    pub fn destroy(&self) {
        self.device.destroy();
    }

    pub fn transfers(&self) -> Transfers {
        let [batches, readbacks, readback_bytes, upload_bytes] = self.transfers.each_ref().map(|c| c.load(Relaxed));
        Transfers { batches, readbacks, readback_bytes, upload_bytes }
    }

    pub fn reset_transfers(&self) {
        self.transfers.iter().for_each(|c| c.store(0, Relaxed));
    }

    pub fn scene(&self, bvh: &Bvh, materials: &[[f32; 4]], charts: &[[[f32; 4]; 2]]) -> Result<SceneBuffers> {
        Ok(SceneBuffers {
            nodes: self.storage("nodes", bytemuck::cast_slice(&bvh.nodes))?,
            tris: self.storage("tris", bytemuck::cast_slice(&bvh.tris))?,
            materials: self.storage("materials", bytemuck::cast_slice(materials))?,
            charts: self.storage("charts", bytemuck::cast_slice(if charts.is_empty() { &[[[0f32; 4]; 2]] } else { charts }))?,
        })
    }

    /// Runs `pass` over `texels` (probes for the volume: their position, their index as the normal's w),
    /// reading `light` (per atlas texel); returns each item's results. Awaits each batch's readback.
    /// Reports progress after each batch; asks `hooks` to cancel before each batch and after each readback.
    #[allow(clippy::too_many_arguments)]
    pub async fn run(&self, pass: Pass, scene: &SceneBuffers, params: Params, texels: &[Texel], light: &[[f32; 4]], hooks: &Hooks<'_>, phase: Phase, bounce: u32) -> Result<Vec<[f32; 4]>> {
        let items = texels.len();
        let mut out = Vec::with_capacity(items * pass.width());
        hooks.check()?;
        if items == 0 {
            hooks.report(phase, bounce, 0, 0);
            return Ok(out);
        }
        let texels = self.storage("texels", bytemuck::cast_slice(texels))?;
        let light = self.storage("light", bytemuck::cast_slice(if light.is_empty() { &[[0f32; 4]] } else { light }))?;
        let pipeline = &self.pipelines[pass as usize];
        let mut first = 0;
        while first < items {
            hooks.check()?;
            self.check()?;
            let n = pass.batch().min(items - first);
            let mut p = params;
            p.counts[3] = n as u32;
            p.atlas[2] = first as u32;
            self.transfers[3].fetch_add(size_of::<Params>() as u64, Relaxed);
            let uniform = self.upload("params", bytemuck::bytes_of(&p), wgpu::BufferUsages::UNIFORM);
            let size = (n * pass.width() * 16) as u64;
            let result = self.device.create_buffer(&wgpu::BufferDescriptor { label: Some("out"), size, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false });
            let read = self.device.create_buffer(&wgpu::BufferDescriptor { label: Some("read"), size, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
            // the batch's slice of the texels (or probes)
            let (offset, tsize) = ((first * 64) as u64, std::num::NonZeroU64::new((n * 64) as u64));
            let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("trace"),
                layout: &self.layout,
                entries: &[
                    entry(0, &scene.nodes, 0, None),
                    entry(1, &scene.tris, 0, None),
                    entry(2, &scene.materials, 0, None),
                    entry(3, &uniform, 0, None),
                    entry(4, &texels, offset, tsize),
                    entry(5, &result, 0, None),
                    entry(6, &scene.charts, 0, None),
                    entry(7, &light, 0, None),
                ],
            });
            let mut enc = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("trace") });
            {
                let mut cp = enc.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some(pass.entry()), timestamp_writes: None });
                cp.set_pipeline(pipeline);
                cp.set_bind_group(0, &group, &[]);
                cp.dispatch_workgroups((n as u32).div_ceil(64), 1, 1);
            }
            enc.copy_buffer_to_buffer(&result, 0, &read, 0, size);
            self.queue.submit([enc.finish()]);
            let slice = read.slice(..);
            let (tx, rx) = futures_channel::oneshot::channel();
            slice.map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
            // natively wgpu runs the callback when polled; in the browser the promise resolves while we await
            #[cfg(not(target_arch = "wasm32"))]
            let polled = self.device.poll(wgpu::PollType::wait_indefinitely());
            let mapped = rx.await;
            self.check()?;
            #[cfg(not(target_arch = "wasm32"))]
            polled.map_err(|e| anyhow!("GPU: {e:?}"))?;
            mapped.map_err(|_| anyhow!("GPU read back: the device dropped the request"))?.map_err(|e| anyhow!("GPU read back: {e:?}"))?;
            out.extend_from_slice(bytemuck::cast_slice(&slice.get_mapped_range()?));
            self.transfers[0].fetch_add(1, Relaxed);
            self.transfers[1].fetch_add(1, Relaxed);
            self.transfers[2].fetch_add(size, Relaxed);
            read.unmap();
            first += n;
            hooks.check()?;
            hooks.report(phase, bounce, first, items);
        }
        Ok(out)
    }

    fn storage(&self, label: &str, contents: &[u8]) -> Result<wgpu::Buffer> {
        self.report.limits.check_storage(label, contents.len() as u64)?;
        self.transfers[3].fetch_add(contents.len() as u64, Relaxed);
        Ok(self.upload(label, contents, wgpu::BufferUsages::STORAGE))
    }

    /// A buffer holding `contents` (a multiple of 4 bytes), written through the queue: unlike a buffer mapped at
    /// creation, this reports a lost device as an error instead of panicking.
    fn upload(&self, label: &str, contents: &[u8], usage: wgpu::BufferUsages) -> wgpu::Buffer {
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor { label: Some(label), size: contents.len() as u64, usage: usage | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        self.queue.write_buffer(&buffer, 0, contents);
        buffer
    }
}

fn entry(binding: u32, buffer: &wgpu::Buffer, offset: u64, size: Option<std::num::NonZeroU64>) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry { binding, resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer, offset, size }) }
}
