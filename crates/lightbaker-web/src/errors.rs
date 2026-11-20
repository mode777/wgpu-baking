//! Engine errors as the worker protocol's `BakerError` kinds (INTERFACES.md §7), plus `cancelled`.

use lightbaker::gpu::GpuError;
use lightbaker::progress::Cancelled;
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BakerError {
    pub kind: &'static str,
    pub message: String,
}

impl BakerError {
    pub fn new(kind: &'static str, message: impl Into<String>) -> BakerError {
        BakerError {
            kind,
            message: message.into(),
        }
    }

    /// Maps an engine error by its type; anything unrecognised is `internal`.
    pub fn from_engine(e: &anyhow::Error) -> BakerError {
        let message = format!("{e:#}");
        if e.downcast_ref::<Cancelled>().is_some() {
            return BakerError::new("cancelled", message);
        }
        let kind = match e.downcast_ref::<GpuError>() {
            Some(GpuError::NoAdapter(_) | GpuError::NoDevice(_)) => "no-adapter",
            Some(GpuError::SoftwareAdapterRefused(_)) => "fallback-adapter-refused",
            Some(GpuError::LimitExceeded { .. }) => "limit-exceeded",
            Some(GpuError::DeviceLost(_)) => "device-lost",
            Some(GpuError::OutOfMemory(_)) => "out-of-memory",
            Some(GpuError::Gpu(_)) | None => "internal",
        };
        BakerError::new(kind, message)
    }

    /// A scene that could not be read or is inconsistent.
    pub fn invalid_scene(e: &anyhow::Error) -> BakerError {
        BakerError::new("invalid-scene", format!("{e:#}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds() {
        let kind = |e: anyhow::Error| BakerError::from_engine(&e).kind;
        assert_eq!(kind(Cancelled.into()), "cancelled");
        assert_eq!(kind(GpuError::NoAdapter("x".into()).into()), "no-adapter");
        assert_eq!(
            kind(GpuError::SoftwareAdapterRefused("llvmpipe".into()).into()),
            "fallback-adapter-refused"
        );
        assert_eq!(
            kind(GpuError::DeviceLost("Destroyed".into()).into()),
            "device-lost"
        );
        assert_eq!(
            kind(GpuError::OutOfMemory("x".into()).into()),
            "out-of-memory"
        );
        assert_eq!(
            kind(
                GpuError::LimitExceeded {
                    buffer: "light".into(),
                    bytes: 2,
                    limit: 1,
                    limit_name: "maxBufferSize"
                }
                .into()
            ),
            "limit-exceeded"
        );
        assert_eq!(kind(anyhow::anyhow!("boom")), "internal");
        // context added on the way up keeps the kind
        let e = anyhow::Error::from(GpuError::DeviceLost("Unknown".into())).context("bounce 1");
        assert_eq!(BakerError::from_engine(&e).kind, "device-lost");
        assert!(BakerError::from_engine(&e).message.contains("bounce 1"));
    }
}
