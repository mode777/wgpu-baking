//! Writes `test-scenes/upstream-basic/`: the scene upstream's GPU test bakes (`upstream-basic.lbscene`) and
//! its expectations (`expect.json`): upstream's assertions as checks. With `--bake`, also bakes it on this
//! machine's GPU and records the values found as the reference other adapters are compared against.
//!
//! `cargo run -p lightbaker --release --example export-upstream-basic [-- --bake]`

#[path = "../tests/common/upstream_basic.rs"]
mod upstream_basic;

use std::path::PathBuf;

use anyhow::Result;
use lightbaker::expect::{Check, Expect, Map, Op, Operand, Probe, Reference};
use lightbaker::scene::Scene;
use lightbaker::{Gpu, Options, bake};

pub fn expectations() -> Expect {
    let probe = |name: &str, map: Map, mesh: u32, point: [f32; 3]| Probe { name: name.into(), map, mesh, point };
    let check = |name: &str, a: &str, op: Op, b: Operand| Check { name: name.into(), a: a.into(), op, b, scale: 1.0, offset: 0.0 };
    let mut probes = vec![
        probe("shade", Map::Direct, 0, [11.0, 0.0, 11.0]),
        probe("open", Map::Direct, 0, [4.0, 0.0, 11.0]),
        probe("foot", Map::Direct, 0, [7.85, 0.0, 11.0]),
        probe("near", Map::Indirect, 0, [38.5, 0.0, 40.0]),
        probe("far", Map::Indirect, 0, [20.0, 0.0, 40.0]),
        probe("volumeOpen", Map::Volume, 0, [20.0, 0.5, 40.0]),
        probe("volumeShade", Map::Volume, 0, [42.0, 0.5, 40.0]),
    ];
    let mut checks = vec![
        check("box shadow east of the box", "shade[0]", Op::Lt, Operand::Value(0.4)),
        check("sunlit ground west of the box", "open[0]", Op::Gt, Operand::Value(0.6)),
        Check { offset: -0.1, ..check("occlusion at the box's foot", "foot[1]", Op::Lt, Operand::Expr("open[1]".into())) },
        Check { scale: 1.15, ..check("red wall bleeds onto the ground", "near[0]/near[1]", Op::Gt, Operand::Expr("far[0]/far[1]".into())) },
        check("sky lights the open ground", "far[2]", Op::Gt, Operand::Value(0.0)),
        check("volume sees the sun in the open", "volumeOpen[3]", Op::Gt, Operand::Value(0.9)),
        check("volume in the wall's shadow", "volumeShade[3]", Op::Lt, Operand::Value(0.1)),
    ];
    // the narrow strip has light of its own all along it
    for k in 0..12 {
        let name = format!("strip{k}");
        probes.push(probe(&name, Map::Indirect, 3, [30.0, 0.3 + k as f32 * 0.19, 20.025]));
        checks.push(check(&format!("light along the strip ({k})"), &format!("{name}[3]"), Op::Gt, Operand::Value(0.1)));
    }
    Expect {
        scene: "upstream-basic.lbscene".into(),
        description: "Upstream lightbaker's GPU test scene (tests/bake.rs at 4fe85da): 64 m ground, grey box at x 8, red wall at x 40, a strip narrower than a texel.".into(),
        probes,
        checks,
        reference: None,
    }
}

fn main() -> Result<()> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../test-scenes/upstream-basic");
    std::fs::create_dir_all(&dir)?;
    let scene = upstream_basic::scene(8.0);
    let bytes = upstream_basic::scene_bytes(8.0);
    std::fs::write(dir.join("upstream-basic.lbscene"), &bytes)?;
    let mut expect = expectations();
    if std::env::args().any(|a| a == "--bake") {
        let gpu = Gpu::new()?;
        let log = |_: &str| {};
        let baked = pollster::block_on(bake(&Scene::read(&bytes)?, &gpu, &Options { cache: None, log: &log }))?;
        let report = expect.evaluate(&scene, &baked)?;
        for o in &report.outcomes {
            println!("{} {}: {} vs {}", if o.passed { "ok  " } else { "FAIL" }, o.name, o.a, o.b);
        }
        anyhow::ensure!(report.passed(), "the reference bake fails its own checks");
        expect.reference = Some(Reference { adapter: gpu.adapter.clone(), baker_version: lightbaker::deps::BAKER_VERSION, values: report.values });
    } else if let Ok(old) = std::fs::read_to_string(dir.join("expect.json")) {
        // keep the recorded reference when only the scene and checks are regenerated
        expect.reference = Expect::parse(&old).ok().and_then(|e| e.reference);
    }
    std::fs::write(dir.join("expect.json"), serde_json::to_string_pretty(&expect)? + "\n")?;
    println!("wrote test-scenes/upstream-basic");
    Ok(())
}
