# Spec Delta

## ADDED Requirements

### Requirement: Unwrap in the worker
The baker worker SHALL answer an `unwrap` request for an `.lbscene` by unwrapping each mesh as an object tile with
xatlas in the worker and replying `unwrapped` with per-mesh UV data and statistics, or an `error`.

#### Scenario: Unwrap upstream-basic
- **WHEN** the page sends `unwrap` for `upstream-basic.lbscene` at 4 texels per metre
- **THEN** the worker replies `unwrapped` with four meshes, each with valid UVs, a chart count and a tile size
