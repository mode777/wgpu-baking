//! Watching and stopping a bake: progress after every GPU batch and stage, and a cancel question asked
//! before every batch and after every readback (ADR-0002: the worker cancels within one batch). Hooks are
//! called only from the bake's sequential code, so they need not be `Sync` (JS callbacks are not).

use std::fmt;

/// What the bake is doing (INTERFACES.md §7 `BakePhase`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    /// layout, BVH, scene upload, tile hashes
    Layout,
    /// the sun and occlusion per texel
    Direct,
    /// a bounce's gather (`bounce` counts from 1)
    Bounce,
    /// the irradiance volume's probes
    Volume,
}

impl Phase {
    pub fn name(self) -> &'static str {
        match self {
            Phase::Layout => "layout",
            Phase::Direct => "direct",
            Phase::Bounce => "bounce",
            Phase::Volume => "volume",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub phase: Phase,
    /// which bounce (from 1) for `Bounce`, else 0
    pub bounce: u32,
    /// items (texels, probes) done in this phase, of `total`
    pub done: usize,
    pub total: usize,
}

/// The bake was cancelled by its caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cancelled;

impl fmt::Display for Cancelled {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("bake cancelled")
    }
}

impl std::error::Error for Cancelled {}

/// A bake's hooks, as the GPU passes see them.
#[derive(Clone, Copy, Default)]
pub struct Hooks<'a> {
    pub progress: Option<&'a dyn Fn(Progress)>,
    pub cancel: Option<&'a dyn Fn() -> bool>,
}

impl Hooks<'_> {
    pub fn report(&self, phase: Phase, bounce: u32, done: usize, total: usize) {
        if let Some(p) = self.progress {
            p(Progress { phase, bounce, done, total });
        }
    }

    /// `Err(Cancelled)` once the caller asked to stop.
    pub fn check(&self) -> anyhow::Result<()> {
        match self.cancel {
            Some(c) if c() => Err(Cancelled.into()),
            _ => Ok(()),
        }
    }
}
