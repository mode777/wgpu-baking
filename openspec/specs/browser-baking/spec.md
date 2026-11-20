# browser-baking Specification

## Purpose
Baking in the browser: a dedicated Web Worker runs the Rust baker on WebGPU, driven by a message protocol
from the page, which shows the adapter, progress, results and errors.

## Requirements

### Requirement: Bakes run in a dedicated worker
The page SHALL run the baker only in a dedicated module Web Worker and communicate through the worker
protocol; the main thread SHALL stay responsive during a bake.

#### Scenario: Bake from the page
- **WHEN** the user presses "Bake" on the PoC page with WebGPU available
- **THEN** the worker bakes `upstream-basic`, progress events update the page, and a `done` event delivers the results

### Requirement: Capabilities shown before baking
On load the page SHALL show whether WebGPU baking is available, the adapter name and backend, and whether the
adapter is a software/fallback adapter; without WebGPU it SHALL show the reason and disable baking.

#### Scenario: No WebGPU
- **WHEN** the page loads where `navigator.gpu` is missing
- **THEN** it shows a message containing the reason and the Bake button is disabled

#### Scenario: Software adapter
- **WHEN** the only adapter is a software adapter and `allowFallbackAdapter` is not set
- **THEN** creating the baker fails with `fallback-adapter-refused` and the page says so; with the opt-in the page labels results as from a software adapter

### Requirement: Cancel and rebake
The page SHALL offer Cancel during a bake; a cancelled bake SHALL end with a `cancelled` event, and a
following bake SHALL succeed.

#### Scenario: Cancel then rebake
- **WHEN** the user cancels a running bake and then presses "Bake" again
- **THEN** the page shows "cancelled", then a completed bake with passing checks

### Requirement: Results are shown and checked
After a bake the page SHALL show total and per-phase times, transfer counts, warnings, the outcome of every
`expect.json` check, and the direct and indirect lightmaps applied to the scene in a 3D view.

#### Scenario: Checks
- **WHEN** `upstream-basic` finishes baking
- **THEN** the page lists all 19 checks with pass/fail and an overall status

### Requirement: CI bake profile
The page SHALL offer a `ci` profile (URL `profile=ci`) that lowers sample counts without changing the layout,
such that every check in `upstream-basic/expect.json` still passes on hardware and software adapters.

#### Scenario: CI profile on a software adapter
- **WHEN** `upstream-basic` is baked with `profile=ci` on a software adapter with the opt-in
- **THEN** all 19 checks pass and the result is labelled as from a software adapter
