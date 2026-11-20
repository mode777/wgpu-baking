//! Measures bakes for the phase reports: the adapter, each stage's time (median of 5 bakes after a warm-up,
//! no cache), what crossed between CPU and GPU, and peak memory, for `test-scenes/upstream-basic` and a
//! denser variant of it (4x the texels, 512 rays, 3 bounces).
//!
//! `cargo run -p lightbaker --release --example baseline [-- --json out.json]`

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Instant;

use anyhow::Result;
use lightbaker::gpu::Transfers;
use lightbaker::{Gpu, Options, Scene, bake};

const RUNS: usize = 5;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct Measured {
    scene: String,
    atlas: (u32, u32),
    triangles: usize,
    /// stage label (the bake's log line, numbers stripped) and its median time in ms
    stages: Vec<(String, f64)>,
    total_ms: f64,
    transfers: Transfers,
    /// light atlas (16 B per texel) + BVH nodes and triangles, as uploaded
    light_atlas_bytes: u64,
    log: Vec<String>,
}

fn measure(name: &str, scene: &Scene, gpu: &Gpu) -> Result<Measured> {
    let mut runs: Vec<Vec<(String, f64)>> = Vec::new();
    let mut totals = Vec::new();
    let mut last = (Transfers::default(), Vec::new(), (0, 0));
    for run in 0..=RUNS {
        let lines: Mutex<Vec<(Instant, String)>> = Mutex::new(Vec::new());
        let log = |s: &str| lines.lock().unwrap().push((Instant::now(), s.to_string()));
        gpu.reset_transfers();
        let start = Instant::now();
        let b = pollster::block_on(bake(scene, gpu, &Options { cache: None, log: &log }))?;
        let end = Instant::now();
        let lines = lines.into_inner().unwrap();
        let mut prev = start;
        let mut stages: Vec<(String, f64)> = lines
            .iter()
            .map(|(t, s)| {
                let ms = (*t - prev).as_secs_f64() * 1e3;
                prev = *t;
                (stage(s), ms)
            })
            .collect();
        stages.push(("atlas assembly (after the last log line)".into(), (end - prev).as_secs_f64() * 1e3));
        if run > 0 {
            runs.push(stages);
            totals.push((end - start).as_secs_f64() * 1e3);
        }
        last = (gpu.transfers(), lines.into_iter().map(|(_, s)| s).collect(), (b.layout.width, b.layout.height));
    }
    let median = |mut v: Vec<f64>| {
        v.sort_by(f64::total_cmp);
        v[v.len() / 2]
    };
    let stages = (0..runs[0].len()).map(|i| (runs[0][i].0.clone(), median(runs.iter().map(|r| r[i].1).collect()))).collect();
    let (atlas_w, atlas_h) = last.2;
    Ok(Measured {
        scene: name.into(),
        atlas: last.2,
        triangles: scene.triangle_count(),
        stages,
        total_ms: median(totals),
        transfers: last.0,
        light_atlas_bytes: 16 * atlas_w as u64 * atlas_h as u64,
        log: last.1,
    })
}

/// The stage a bake's log line ends (bake.rs logs each stage when it is done).
fn stage(line: &str) -> String {
    let s = if line.contains("triangles,") {
        "layout + BVH + GPU scene upload"
    } else if line.contains("tiles to bake") {
        "tile hashes + cache lookup"
    } else if line.starts_with("sun and occlusion") {
        "rasterise + direct pass (GPU)"
    } else if line.starts_with("charts baked") {
        "uncovered-chart check"
    } else if line.starts_with("bounce 1:") {
        "CPU per-tile direct (sun field, AO, filters) + bounce 1 gather (GPU)"
    } else if line.starts_with("bounce") {
        return format!("{} gather (GPU)", line.split(':').next().unwrap_or(line));
    } else if line.starts_with("volume") {
        "volume (GPU)"
    } else {
        line
    };
    s.to_string()
}

/// Peak resident memory of this process (Linux; elsewhere measure it from outside).
fn peak_memory_kib() -> Option<u64> {
    let s = std::fs::read_to_string("/proc/self/status").ok()?;
    s.lines().find(|l| l.starts_with("VmHWM:"))?.split_whitespace().nth(1)?.parse().ok()
}

fn main() -> Result<()> {
    let allow_software = std::env::var("LIGHTBAKER_ALLOW_SOFTWARE_ADAPTER").is_ok_and(|v| v == "1");
    let gpu = pollster::block_on(Gpu::open(&lightbaker::gpu::GpuOptions { allow_software }))?;
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-scenes/upstream-basic/upstream-basic.lbscene");
    let basic = Scene::read(&std::fs::read(&path)?)?;
    let mut dense = Scene::read(&std::fs::read(&path)?)?;
    dense.header.lightmap.texels_per_meter *= 4.0;
    dense.header.lightmap.atlas_width = 2048;
    dense.header.bake.gi_rays = 512;
    dense.header.bake.gi_bounces = 3;
    let results = vec![measure("upstream-basic", &basic, &gpu)?, measure("upstream-basic-dense", &dense, &gpu)?];

    println!("adapter: {}", gpu.report);
    for m in &results {
        let t = &m.transfers;
        println!("\n{} ({} triangles, atlas {}x{}, light atlas {:.1} MiB): median of {RUNS} bakes {:.1} ms", m.scene, m.triangles, m.atlas.0, m.atlas.1, m.light_atlas_bytes as f64 / 1048576.0, m.total_ms);
        println!("  transfers: {} batches, {} readbacks ({:.2} MiB), {:.2} MiB uploaded", t.batches, t.readbacks, t.readback_bytes as f64 / 1048576.0, t.upload_bytes as f64 / 1048576.0);
        for (label, ms) in &m.stages {
            println!("  {ms:>9.1} ms  {label}");
        }
        for l in &m.log {
            println!("  | {l}");
        }
    }
    if let Some(kib) = peak_memory_kib() {
        println!("\npeak resident memory: {:.1} MiB", kib as f64 / 1024.0);
    }
    if let Some(i) = std::env::args().position(|a| a == "--json") {
        let out = std::env::args().nth(i + 1).expect("--json <file>");
        let json = serde_json::json!({ "adapter": gpu.adapter, "software": gpu.software, "report": gpu.report, "runs": RUNS, "peakMemoryKiB": peak_memory_kib(), "scenes": results });
        std::fs::write(out, serde_json::to_string_pretty(&json)?)?;
    }
    Ok(())
}
