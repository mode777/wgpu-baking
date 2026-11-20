## ADDED Requirements

### Requirement: Unwrap through a JS unwrapper
The bridge SHALL export `unwrapScene(scene, config, unwrapper)` that unwraps through any JS object with
`unwrapTile(meshes, options)` (the `xatlas-wasm` adapter in the worker) and returns per mesh the tile size,
charts, utilisation, UVs, `xref`, indices and the validity report; errors of the unwrapper become typed
errors.

#### Scenario: Unwrapper throws
- **WHEN** the unwrapper's `unwrapTile` throws
- **THEN** `unwrapScene` rejects with a typed `internal` error carrying the message
