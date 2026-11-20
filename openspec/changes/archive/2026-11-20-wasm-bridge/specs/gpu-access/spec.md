# Spec Delta

## MODIFIED Requirements

### Requirement: Non-blocking GPU waits
The bake and every GPU readback SHALL be asynchronous; in wasm32 builds no code path SHALL block the thread to
wait for the GPU and the instance SHALL use only the WebGPU backend, regardless of which Cargo features are enabled.

#### Scenario: Web build
- **WHEN** the engine is built for `wasm32-unknown-unknown`
- **THEN** it contains no blocking executor or device poll-wait and requests a `BROWSER_WEBGPU` instance

#### Scenario: Native results unchanged
- **WHEN** a native caller awaits the bake with a blocking executor
- **THEN** results are byte-identical to the synchronous engine

## ADDED Requirements

### Requirement: GPU errors become bake errors
GPU validation/out-of-memory errors and device loss during a bake SHALL fail the bake with an error describing
them instead of aborting the process, and SHALL be reported through a device-lost callback when one is set.

#### Scenario: Device lost mid-bake
- **WHEN** the device is lost while a bake awaits a batch
- **THEN** the bake fails with a device-lost error carrying the reason, and the callback receives it
