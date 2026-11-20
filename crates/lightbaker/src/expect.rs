//! Expectations for a test scene (`test-scenes/<name>/expect.json`): named points where a bake is sampled
//! and checks on what it must show there, e.g. "the ground east of the box is in shadow". The same file
//! checks a bake made natively or in the browser. Format: docs/TESTING.md.

use std::collections::BTreeMap;

use anyhow::{Context, Result, anyhow, bail};
use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::bake::Bake;
use crate::scene::Scene;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Expect {
    /// the scene file, beside this one
    pub scene: String,
    pub description: String,
    pub probes: Vec<Probe>,
    pub checks: Vec<Check>,
    /// values measured on a reference bake (informative: other adapters are compared against them)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<Reference>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Map {
    /// `[sun field, occlusion, 0, 0]`
    Direct,
    /// `[r, g, b, r + g + b]` of the bounced and sky light
    Indirect,
    /// the irradiance volume's probe at or below the point: `[L0 r, L0 g, L0 b, sun]`
    Volume,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Probe {
    pub name: String,
    pub map: Map,
    /// the mesh the point lies on (lightmap maps only)
    #[serde(default)]
    pub mesh: u32,
    pub point: [f32; 3],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Check {
    pub name: String,
    /// `name[i]` or `name[i]/name[j]`
    pub a: String,
    pub op: Op,
    /// a number, `name[i]` or `name[i]/name[j]`
    pub b: Operand,
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub scale: f32,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub offset: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Op {
    #[serde(rename = "<")]
    Lt,
    #[serde(rename = ">")]
    Gt,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Operand {
    Value(f32),
    Expr(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Reference {
    pub adapter: String,
    pub baker_version: u32,
    pub values: BTreeMap<String, [f32; 4]>,
}

fn one() -> f32 {
    1.0
}
fn is_one(v: &f32) -> bool {
    *v == 1.0
}
fn is_zero(v: &f32) -> bool {
    *v == 0.0
}

/// One check's outcome.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub name: String,
    pub a: f32,
    pub b: f32,
    pub passed: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub values: BTreeMap<String, [f32; 4]>,
    pub outcomes: Vec<Outcome>,
}

impl Report {
    pub fn passed(&self) -> bool {
        self.outcomes.iter().all(|o| o.passed)
    }
}

impl Expect {
    pub fn parse(json: &str) -> Result<Expect> {
        Ok(serde_json::from_str(json)?)
    }

    /// Samples every probe and evaluates every check.
    pub fn evaluate(&self, scene: &Scene, bake: &Bake) -> Result<Report> {
        let mut values = BTreeMap::new();
        for p in &self.probes {
            let v = sample(scene, bake, p).with_context(|| format!("probe {}", p.name))?;
            values.insert(p.name.clone(), v);
        }
        let outcomes = self
            .checks
            .iter()
            .map(|c| {
                let a = operand(&c.a, &values).with_context(|| format!("check {}", c.name))?;
                let b = match &c.b {
                    Operand::Value(v) => *v,
                    Operand::Expr(e) => operand(e, &values).with_context(|| format!("check {}", c.name))?,
                } * c.scale
                    + c.offset;
                let passed = match c.op {
                    Op::Lt => a < b,
                    Op::Gt => a > b,
                };
                Ok(Outcome { name: c.name.clone(), a, b, passed })
            })
            .collect::<Result<_>>()?;
        Ok(Report { values, outcomes })
    }
}

/// `name[i]` or `name[i]/name[j]`.
fn operand(expr: &str, values: &BTreeMap<String, [f32; 4]>) -> Result<f32> {
    let term = |t: &str| -> Result<f32> {
        let t = t.trim();
        let (name, rest) = t.split_once('[').ok_or_else(|| anyhow!("`{t}`: expected name[i]"))?;
        let i: usize = rest.strip_suffix(']').ok_or_else(|| anyhow!("`{t}`: expected name[i]"))?.trim().parse()?;
        let v = values.get(name.trim()).ok_or_else(|| anyhow!("`{t}`: no probe {name}"))?;
        v.get(i).copied().ok_or_else(|| anyhow!("`{t}`: component {i} out of 0..4"))
    };
    match expr.split_once('/') {
        Some((a, b)) => Ok(term(a)? / term(b)?),
        None => term(expr),
    }
}

/// The bake's value at a probe point.
pub fn sample(scene: &Scene, bake: &Bake, probe: &Probe) -> Result<[f32; 4]> {
    let p = Vec3::from(probe.point);
    if probe.map == Map::Volume {
        let v = bake.volume.as_ref().context("the bake has no volume")?;
        let cell = |a: usize| ((p[a] - v.origin[a]) / v.step).floor();
        let (i, j, k) = (cell(0), cell(1), cell(2));
        let [nx, ny, nz] = v.size.map(|n| n as f32);
        if i < 0.0 || j < 0.0 || k < 0.0 || i >= nx || j >= ny || k >= nz {
            bail!("{p} is outside the volume");
        }
        let c = v.probes[(j as usize * nz as usize + k as usize) * nx as usize + i as usize][0];
        return Ok(c);
    }
    let (x, y) = atlas_texel(scene, bake, probe.mesh as usize, p)?;
    let l = &bake.layout;
    Ok(match probe.map {
        Map::Direct => {
            let d = bake.direct[y * l.width as usize + x];
            [d[0], d[1], 0.0, 0.0]
        }
        _ => {
            let (iw, _) = bake.indirect_size;
            let ds = (l.width / iw) as usize;
            let e = bake.indirect[(y / ds) * iw as usize + x / ds];
            [e[0], e[1], e[2], e[0] + e[1] + e[2]]
        }
    })
}

/// The atlas texel a point on a mesh falls in, through its triangle's chart.
fn atlas_texel(scene: &Scene, bake: &Bake, mesh: usize, p: Vec3) -> Result<(usize, usize)> {
    let pos = scene.positions.get(mesh).with_context(|| format!("no mesh {mesh}"))?;
    let tri = (0..pos.len() / 3).find(|&t| on_triangle(p, pos[t * 3], pos[t * 3 + 1], pos[t * 3 + 2])).with_context(|| format!("{p} lies on no triangle of mesh {mesh}"))?;
    let l = &bake.layout;
    let c = l.tri_chart[mesh][tri];
    if c == u32::MAX {
        bail!("triangle {tri} of mesh {mesh} is not in the lightmap");
    }
    let tile = &l.tiles[l.chart_tile()[c as usize] as usize];
    let a = l.charts[c as usize].affine(tile, l.density, l.padding);
    let dot = |r: [f32; 4]| r[0] * p.x + r[1] * p.y + r[2] * p.z + r[3];
    let (x, y) = (dot(a[0]).floor(), dot(a[1]).floor());
    if x < 0.0 || y < 0.0 || x >= l.width as f32 || y >= l.height as f32 {
        bail!("{p} maps outside the atlas");
    }
    Ok((x as usize, y as usize))
}

/// Within 1 mm of the triangle's plane and inside it.
fn on_triangle(p: Vec3, a: Vec3, b: Vec3, c: Vec3) -> bool {
    let n = (b - a).cross(c - a);
    let area2 = n.length();
    if area2 == 0.0 || (p - a).dot(n / area2).abs() > 1e-3 {
        return false;
    }
    let w = |u: Vec3, v: Vec3| (v - u).cross(p - u).dot(n) / (area2 * area2);
    let eps = -1e-4;
    w(a, b) >= eps && w(b, c) >= eps && w(c, a) >= eps
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values() -> BTreeMap<String, [f32; 4]> {
        BTreeMap::from([("near".into(), [0.6, 0.2, 0.1, 0.9]), ("far".into(), [0.3, 0.3, 0.3, 0.9])])
    }

    #[test]
    fn operands() {
        let v = values();
        assert_eq!(operand("near[0]", &v).unwrap(), 0.6);
        assert!((operand("near[0] / near[1]", &v).unwrap() - 3.0).abs() < 1e-6);
        assert!(operand("near[4]", &v).is_err());
        assert!(operand("nope[0]", &v).is_err());
        assert!(operand("near", &v).is_err());
    }

    #[test]
    fn parses_checks() {
        let e = Expect::parse(
            r#"{"scene":"s.lbscene","description":"d","probes":[{"name":"near","map":"indirect","mesh":0,"point":[1,0,2]}],
                "checks":[{"name":"bleed","a":"near[0]/near[1]","op":">","b":"far[0]/far[1]","scale":1.15},
                          {"name":"lit","a":"near[0]","op":"<","b":0.4}]}"#,
        )
        .unwrap();
        assert_eq!(e.checks[0].op, Op::Gt);
        assert_eq!(e.checks[0].scale, 1.15);
        assert!(matches!(e.checks[1].b, Operand::Value(v) if v == 0.4));
        assert_eq!(e.checks[1].scale, 1.0);
        assert!(Expect::parse(r#"{"scene":"s","description":"d","probes":[],"checks":[],"extra":1}"#).is_err());
    }

    #[test]
    fn points_on_triangles() {
        let (a, b, c) = (Vec3::ZERO, Vec3::new(0., 0., 4.), Vec3::new(4., 0., 4.));
        assert!(on_triangle(Vec3::new(1., 0., 3.), a, b, c));
        assert!(on_triangle(Vec3::new(0., 0., 2.), a, b, c), "on an edge");
        assert!(!on_triangle(Vec3::new(3., 0., 1.), a, b, c), "outside");
        assert!(!on_triangle(Vec3::new(1., 0.01, 3.), a, b, c), "off the plane");
    }
}
