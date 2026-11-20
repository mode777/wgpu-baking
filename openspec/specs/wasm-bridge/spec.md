# wasm-bridge Specification

## Purpose
The JavaScript API through which a Web Worker uses the Rust baker: capability check, opening a baker on a
WebGPU adapter, baking with progress, cancelling, checking results, and typed errors.

## Requirements

### Requirement: Capability check without side effects
`capabilities()` SHALL report whether WebGPU is available and, if so, the adapter (name, vendor, device type,
software flag) and its storage-buffer binding size, buffer size and workgroups-per-dimension limits, without
creating a device; when WebGPU is missing it SHALL return `webgpu: false` with a human-readable reason.

#### Scenario: No WebGPU
- **WHEN** `capabilities()` runs where `navigator.gpu` is undefined (e.g. Node)
- **THEN** it resolves to `{ webgpu: false, reason }` and does not throw

### Requirement: Creating a baker
`WebBaker.create(allowFallbackAdapter)` SHALL open the GPU with the engine's adapter policy and reject with
`{ kind: 'no-webgpu' | 'no-adapter' | 'fallback-adapter-refused', message }` when it cannot.

#### Scenario: No WebGPU
- **WHEN** `WebBaker.create(false)` runs where `navigator.gpu` is undefined
- **THEN** it rejects with kind `no-webgpu`

### Requirement: Baking from JavaScript
`bake(scene, configJson, onProgress)` SHALL bake an `.lbscene` with the optional config overrides, call
`onProgress(phase, done, total)` during the bake, and resolve to the `.lbake` bytes, the direct atlas (RG f32),
the indirect atlas (RGB f32) with sizes, validity codes, stats, the adapter report, transfer counts and
warnings for ignored config fields. Only one bake MAY run at a time per baker.

#### Scenario: Concurrent bake rejected
- **WHEN** `bake` is called while another bake of the same baker runs
- **THEN** the second call rejects with kind `internal`

#### Scenario: Unsupported override
- **WHEN** the config contains `interpolation.mode: "adaptive"`
- **THEN** the bake proceeds and the result lists a warning naming `interpolation.mode`

### Requirement: Checking a bake
`evaluate(expectJson)` SHALL evaluate an `expect.json` against the baker's last bake and return every probe
value and check outcome.

#### Scenario: Before any bake
- **WHEN** `evaluate` is called before a bake completed
- **THEN** it throws with kind `internal`

### Requirement: Typed errors
Every rejection SHALL be an object `{ kind, message }` with `kind` one of `no-webgpu`, `no-adapter`,
`fallback-adapter-refused`, `device-lost`, `out-of-memory`, `limit-exceeded`, `invalid-scene`, `internal` or
`cancelled`.

#### Scenario: Invalid scene
- **WHEN** `bake` receives bytes that are not an `.lbscene`
- **THEN** it rejects with kind `invalid-scene`
