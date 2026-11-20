//! `lightbaker <scene.lbscene>… [--out dir] [--cache dir]`: bakes each scene into `<out>/<name>.lbake`,
//! reusing the tiles the cache already holds. One GPU device for all of them.

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use lightbaker::cache::{FsStore, TileStore};
use lightbaker::gpu::GpuOptions;
use lightbaker::{Gpu, Options, Scene, bake, output};

#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// scene files (.lbscene)
    #[arg(required = true)]
    scenes: Vec<PathBuf>,
    /// where the bakes are written (default: beside each scene)
    #[arg(long)]
    out: Option<PathBuf>,
    /// where baked tiles are kept between runs (default: none, everything baked)
    #[arg(long)]
    cache: Option<PathBuf>,
    /// bake on a software adapter (lavapipe, SwiftShader, WARP) if it is the only one: slow, results labelled;
    /// also LIGHTBAKER_ALLOW_SOFTWARE_ADAPTER=1
    #[arg(long)]
    allow_software_adapter: bool,
    /// say less
    #[arg(long, short)]
    quiet: bool,
    /// after each bake, check it against the `expect.json` beside its scene (test-scenes/); fail if a check fails
    #[arg(long)]
    check: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let allow_software = args.allow_software_adapter || std::env::var("LIGHTBAKER_ALLOW_SOFTWARE_ADAPTER").is_ok_and(|v| v == "1");
    let gpu = pollster::block_on(Gpu::open(&GpuOptions { allow_software }))?;
    let store = args.cache.as_deref().map(FsStore::new).transpose()?;
    if !args.quiet {
        eprintln!("GPU: {}", gpu.report);
        if gpu.software {
            eprintln!("warning: baking on a SOFTWARE ADAPTER: slow; results are not representative of a hardware GPU");
        }
    }
    for path in &args.scenes {
        let started = Instant::now();
        let name = path.file_stem().context("scene file name")?.to_string_lossy().to_string();
        let scene = Scene::read(&std::fs::read(path).with_context(|| format!("reading {}", path.display()))?).with_context(|| format!("{}", path.display()))?;
        let quiet = args.quiet;
        let log = move |s: &str| {
            if !quiet {
                eprintln!("  {s}");
            }
        };
        if !args.quiet {
            eprintln!("{name}:");
        }
        let baked = pollster::block_on(bake(&scene, &gpu, &Options { cache: store.as_ref().map(|s| s as &dyn TileStore), log: &log, ..Default::default() }))?;
        let dir = args.out.clone().unwrap_or_else(|| path.parent().map(PathBuf::from).unwrap_or_default());
        std::fs::create_dir_all(&dir)?;
        let file = dir.join(format!("{name}.lbake"));
        std::fs::write(&file, output::write(&scene, &baked)?)?;
        println!("{name}: {} of {} tiles baked, {:.1} s -> {}", baked.stats.tiles_baked, baked.stats.tiles, started.elapsed().as_secs_f64(), file.display());
        if args.check {
            let path = path.with_file_name("expect.json");
            let expect = lightbaker::expect::Expect::parse(&std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?)?;
            let report = expect.evaluate(&scene, &baked)?;
            for o in &report.outcomes {
                println!("  {} {}: {} vs {}", if o.passed { "ok  " } else { "FAIL" }, o.name, o.a, o.b);
            }
            let failed = report.outcomes.iter().filter(|o| !o.passed).count();
            anyhow::ensure!(failed == 0, "{name}: {failed} of {} checks failed", report.outcomes.len());
            println!("{name}: all {} checks passed", report.outcomes.len());
        }
    }
    Ok(())
}
