//! Bakes small hand-built scenes on the GPU and checks what the light must show: a box's shadow on
//! the ground where the sun puts it, darker corners at its foot, a red wall's colour bounced onto the
//! ground beside it, and a second bake of a scene changed in one place baking only the tiles it reaches.

#[path = "common/upstream_basic.rs"]
mod upstream_basic;
#[path = "common/gpu.rs"]
mod gpu;

use lightbaker::layout::Layout;
use lightbaker::{Options, Scene, bake};
use gpu::gpu;
use upstream_basic::scene;

/// The atlas texel a ground point falls in.
fn ground_texel(s: &Scene, l: &Layout, x: f32, z: f32) -> usize {
    let tri = (0..s.positions[0].len() / 3)
        .find(|&t| {
            let p = &s.positions[0][t * 3..t * 3 + 3];
            let (lo, hi) = (p[0].min(p[1]).min(p[2]), p[0].max(p[1]).max(p[2]));
            x >= lo.x && x <= hi.x && z >= lo.z && z <= hi.z
        })
        .unwrap();
    let c = l.tri_chart[0][tri] as usize;
    let tile = &l.tiles[l.chart_tile()[c] as usize];
    let a = l.charts[c].affine(tile, l.density, l.padding);
    let p = [x, 0.0, z, 1.0];
    let dot = |r: [f32; 4]| r.iter().zip(p).map(|(a, b)| a * b).sum::<f32>();
    dot(a[1]).floor() as usize * l.width as usize + dot(a[0]).floor() as usize
}

#[test]
fn shadows_occlusion_bounces_and_reuse() {
    let Some(gpu) = gpu() else { return };
    let dir = std::env::temp_dir().join(format!("lightbaker-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let log = |s: &str| eprintln!("{s}");
    let store = lightbaker::cache::FsStore::new(&dir).unwrap();
    let opts = Options { cache: Some(&store), log: &log };
    let s = scene(8.0);
    let b = pollster::block_on(bake(&s, &gpu, &opts)).unwrap();
    let l = &b.layout;
    assert_eq!(b.stats.tiles_baked, b.stats.tiles);

    // the box (x 8 … 10, z 10 … 12) shades the ground east of it; west of it is lit
    let shade = b.direct[ground_texel(&s, l, 11.0, 11.0)];
    let open = b.direct[ground_texel(&s, l, 4.0, 11.0)];
    assert!(shade[0] < 0.4, "in the box's shadow: field {}", shade[0]);
    assert!(open[0] > 0.6, "in the sun: field {}", open[0]);
    // its foot is darker than the open ground
    let foot = b.direct[ground_texel(&s, l, 7.85, 11.0)];
    assert!(foot[1] < open[1] - 0.1, "occlusion at the box's foot {} vs open {}", foot[1], open[1]);

    // the red wall (x 40 … 40.5, sunlit on its west face) reddens the ground west of it
    let small = |i: usize| (i / l.width as usize / 2) * b.indirect_size.0 as usize + (i % l.width as usize) / 2;
    let near = b.indirect[small(ground_texel(&s, l, 38.5, 40.0))];
    let far = b.indirect[small(ground_texel(&s, l, 20.0, 40.0))];
    assert!(near[0] / near[1] > far[0] / far[1] * 1.15, "bounced red near the wall {near:?} vs far {far:?}");
    assert!(far[2] > 0.0, "the sky lights the open ground {far:?}");

    // the volume: above the open ground it sees the sun, beside the wall's east face (in its shadow) less of it
    let v = b.volume.as_ref().unwrap();
    let probe = |x: f32, y: f32, z: f32| {
        let [nx, _, nz] = v.size.map(|n| n as usize);
        let (i, j, k) = (((x - v.origin[0]) / v.step) as usize, ((y - v.origin[1]) / v.step) as usize, ((z - v.origin[2]) / v.step) as usize);
        v.probes[(j * nz + k) * nx + i]
    };
    assert!(probe(20.0, 0.5, 40.0)[0][3] > 0.9);
    assert!(probe(42.0, 0.5, 40.0)[0][3] < 0.1);

    // the narrow strip has light of its own all along it, not only near where its light was gathered
    let ds = l.width as usize / b.indirect_size.0 as usize;
    for k in 0..12 {
        let y = 0.3 + k as f32 * 0.19;
        let c = l.tri_chart[3][0] as usize;
        let a = l.charts[c].affine(&l.tiles[l.chart_tile()[c] as usize], l.density, l.padding);
        let p = [30.0, y, 20.025, 1.0];
        let dot = |r: [f32; 4]| r.iter().zip(p).map(|(a, b)| a * b).sum::<f32>();
        let (x, yy) = (dot(a[0]).floor() as usize / ds, dot(a[1]).floor() as usize / ds);
        let e = b.indirect[yy * b.indirect_size.0 as usize + x];
        assert!(e[0] + e[1] + e[2] > 0.1, "strip at {y} m: {e:?}");
    }

    // baked again unchanged: nothing to bake; with the box moved, only the tiles it reaches
    let again = pollster::block_on(bake(&s, &gpu, &opts)).unwrap();
    assert_eq!(again.stats.tiles_baked, 0);
    assert_eq!(again.stats.chunks_baked, 0);
    assert_eq!(again.direct, b.direct);
    // the same light gives the same file, byte for byte: a pipeline that hashes it publishes nothing new
    assert!(lightbaker::output::write(&s, &again).unwrap() == lightbaker::output::write(&s, &b).unwrap(), "the .lbake differs between a bake and its cached rerun");
    let moved = pollster::block_on(bake(&scene(6.0), &gpu, &opts)).unwrap();
    assert!(moved.stats.tiles_baked > 0 && moved.stats.tiles_baked < moved.stats.tiles, "{} of {}", moved.stats.tiles_baked, moved.stats.tiles);
    let _ = std::fs::remove_dir_all(&dir);
}
