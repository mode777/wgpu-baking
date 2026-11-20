## ADDED Requirements

### Requirement: Native and web unwraps agree
The browser path (bridge + `xatlas-wasm`) and the native unwrapper SHALL give equivalent unwraps of the same
scene with the same options: same tile sizes, chart counts and triangle-to-vertex structure, UVs within the
documented tolerance.

#### Scenario: Equivalence fixture
- **WHEN** `test-scenes/uv-equivalence` is unwrapped natively and through the bridge with `xatlas-wasm`
- **THEN** both match the fixture's native unwrap within the tolerance

### Requirement: Scene unwrap through the contract
Unwrapping a scene SHALL give one object tile per engine mesh through the `Unwrapper` contract, each with the
UV validity report, identically natively and in the browser.

#### Scenario: Browser unwrap
- **WHEN** the page unwraps `upstream-basic` in the baker worker
- **THEN** the bridge's Rust checks report every mesh valid
