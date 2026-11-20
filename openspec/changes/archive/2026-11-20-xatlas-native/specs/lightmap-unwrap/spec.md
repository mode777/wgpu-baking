# Spec Delta

## ADDED Requirements

### Requirement: Native unwrap matches the contract
The native unwrapper SHALL implement the engine's unwrap contract with xatlas built from the same pinned commit as
the web module, and its output SHALL pass the UV validity checks on the test meshes.

#### Scenario: Test meshes natively
- **WHEN** the quad, the 3-material box, the `upstream-basic` meshes, a UV sphere and a torus knot are unwrapped natively
- **THEN** every validity check passes and the median chart density is within ±20 % of the request

### Requirement: Deterministic unwrap
Unwrapping the same input with the same options SHALL give identical output on the same target.

#### Scenario: Twice
- **WHEN** a torus knot is unwrapped twice natively
- **THEN** both results are identical
