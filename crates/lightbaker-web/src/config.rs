//! A bake's configuration from the worker (a partial `BakeConfig`, INTERFACES.md §4) applied to a scene's own
//! settings. What the engine cannot do yet is reported as a warning, never dropped silently.

use anyhow::{Context, Result, bail};
use lightbaker::scene::SceneHeader;
use serde_json::Value;

/// The overrides' effect: warnings for fields not (yet) supported, and whether cached tiles may be reused.
#[derive(Debug, PartialEq)]
pub struct Applied {
    pub warnings: Vec<String>,
    pub reuse_tiles: bool,
}

/// Applies `config` (JSON object, possibly empty) to `header`.
pub fn apply(header: &mut SceneHeader, config: &str) -> Result<Applied> {
    let mut applied = Applied {
        warnings: Vec::new(),
        reuse_tiles: true,
    };
    let config = config.trim();
    if config.is_empty() {
        return Ok(applied);
    }
    let v: Value = serde_json::from_str(config).context("bake config is not JSON")?;
    let Value::Object(root) = v else {
        bail!("bake config must be a JSON object")
    };
    for (section, value) in &root {
        match (section.as_str(), value) {
            (
                "lightmap" | "sampling" | "gi" | "ao" | "incremental" | "interpolation" | "denoise"
                | "output",
                Value::Object(fields),
            ) => {
                for (field, v) in fields {
                    set(header, &mut applied, section, field, v)?;
                }
            }
            // used when the baker is created, not per bake
            ("allowFallbackAdapter", _) => {}
            ("preset", Value::String(p)) if p == "custom" => {}
            ("preset", _) => applied
                .warnings
                .push("preset: presets are resolved by the app; only explicit values apply".into()),
            _ => applied
                .warnings
                .push(format!("{section}: not supported by this baker")),
        }
    }
    Ok(applied)
}

fn set(
    h: &mut SceneHeader,
    applied: &mut Applied,
    section: &str,
    field: &str,
    v: &Value,
) -> Result<()> {
    let num = || {
        v.as_f64()
            .with_context(|| format!("{section}.{field} must be a number"))
    };
    let int = || -> Result<u32> {
        let n = num()?;
        if n < 0.0 || n.fract() != 0.0 || n > u32::MAX as f64 {
            bail!("{section}.{field} must be a whole number ≥ 0");
        }
        Ok(n as u32)
    };
    let (l, b) = (&mut h.lightmap, &mut h.bake);
    match (section, field) {
        ("lightmap", "texelsPerMeter") => l.texels_per_meter = num()? as f32,
        ("lightmap", "padding") => l.padding = int()?,
        ("lightmap", "indirectDownscale") => l.indirect_downscale = int()?,
        ("lightmap", "chartAngleDeg") => l.chart_angle_deg = num()? as f32,
        ("lightmap", "atlasMaxSize") => l.atlas_width = int()?,
        ("lightmap", "unwrap") if v.as_str() == Some("planar") => {}
        ("sampling", "sunSamples") => b.sun_samples = int()?,
        ("sampling", "aoRays") => b.ao_rays = int()?,
        ("sampling", "giRays") => b.gi_rays = int()?,
        ("sampling", "volumeRays") => match h.volume.as_mut() {
            Some(vol) => vol.rays = int()?,
            None => applied
                .warnings
                .push("sampling.volumeRays: the scene has no volume".into()),
        },
        ("gi", "bounces") => b.gi_bounces = int()?,
        ("gi", "reach") => b.gi_reach = num()? as f32,
        ("gi", "insideShare") => b.inside_share = num()? as f32,
        ("ao", "strength") => b.ao_strength = num()? as f32,
        ("ao", "reach") => b.ao_reach = num()? as f32,
        ("ao", "floor") => b.ao_floor = num()? as f32,
        ("ao", "contrast") => b.ao_contrast = num()? as f32,
        ("incremental", "reuseTiles") => {
            applied.reuse_tiles = v
                .as_bool()
                .with_context(|| "incremental.reuseTiles must be a boolean")?
        }
        ("incremental", "progressivePreview") => {}
        _ => applied.warnings.push(format!(
            "{section}.{field}: not supported by this baker yet"
        )),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lightbaker::scene::*;

    fn json(h: &SceneHeader) -> serde_json::Value {
        serde_json::to_value(h).unwrap()
    }

    fn header() -> SceneHeader {
        SceneHeader {
            sun: Sun {
                direction: [0.0, 1.0, 0.0],
                radius_deg: 0.5,
                irradiance: [1.0; 3],
            },
            sky: Sky {
                zenith: [0.1; 3],
                horizon: [0.1; 3],
                ground: [0.1; 3],
            },
            materials: vec![],
            meshes: vec![],
            lightmap: LightmapSettings {
                texels_per_meter: 4.0,
                tile_size: 16.0,
                chart_angle_deg: 40.0,
                padding: 2,
                atlas_width: 1024,
                indirect_downscale: 2,
            },
            bake: BakeSettings {
                sun_samples: 16,
                ao_rays: 64,
                ao_reach: 0.8,
                ao_strength: 1.0,
                ao_floor: 0.25,
                ao_contrast: 1.0,
                sdf_radius: 2.0,
                gi_rays: 256,
                gi_bounces: 2,
                gi_reach: 12.0,
                inside_share: 0.25,
                gi_filter: 1,
            },
            volume: Some(VolumeSettings {
                min: [0.0; 3],
                max: [1.0; 3],
                step: 1.0,
                rays: 128,
            }),
            user: serde_json::Value::Null,
            lights: vec![],
        }
    }

    #[test]
    fn empty_config_changes_nothing() {
        let mut h = header();
        assert_eq!(
            apply(&mut h, "").unwrap(),
            Applied {
                warnings: vec![],
                reuse_tiles: true
            }
        );
        assert_eq!(apply(&mut h, "{}").unwrap().warnings.len(), 0);
        assert_eq!(json(&h), json(&header()));
    }

    #[test]
    fn known_fields_apply() {
        let mut h = header();
        let a = apply(&mut h, r#"{"lightmap":{"texelsPerMeter":2,"padding":3,"unwrap":"planar"},"sampling":{"giRays":64,"volumeRays":32},"gi":{"bounces":1},"ao":{"strength":0.5},"incremental":{"reuseTiles":false},"allowFallbackAdapter":true,"preset":"custom"}"#).unwrap();
        assert!(a.warnings.is_empty(), "{:?}", a.warnings);
        assert!(!a.reuse_tiles);
        assert_eq!((h.lightmap.texels_per_meter, h.lightmap.padding), (2.0, 3));
        assert_eq!(
            (h.bake.gi_rays, h.bake.gi_bounces, h.bake.ao_strength),
            (64, 1, 0.5)
        );
        assert_eq!(h.volume.unwrap().rays, 32);
    }

    #[test]
    fn unsupported_fields_warn() {
        let mut h = header();
        let a = apply(&mut h, r#"{"interpolation":{"mode":"adaptive"},"lightmap":{"unwrap":"xatlas"},"preset":"balanced","lights":[]}"#).unwrap();
        assert!(
            a.warnings
                .iter()
                .any(|w| w.starts_with("interpolation.mode")),
            "{:?}",
            a.warnings
        );
        assert!(a.warnings.iter().any(|w| w.starts_with("lightmap.unwrap")));
        assert!(a.warnings.iter().any(|w| w.starts_with("preset")));
        assert!(a.warnings.iter().any(|w| w.starts_with("lights")));
        assert_eq!(json(&h), json(&header()));
    }

    #[test]
    fn bad_values_fail() {
        assert!(apply(&mut header(), "[1]").is_err());
        assert!(apply(&mut header(), "{not json").is_err());
        assert!(apply(&mut header(), r#"{"sampling":{"giRays":-1}}"#).is_err());
        assert!(apply(&mut header(), r#"{"sampling":{"giRays":1.5}}"#).is_err());
        assert!(apply(&mut header(), r#"{"incremental":{"reuseTiles":1}}"#).is_err());
    }
}
