# bake-control Specification

## Purpose
Progress reporting and cancellation of a running bake, so a caller can show progress and stop a bake
within one GPU batch.

## Requirements

### Requirement: Progress per GPU batch
A bake SHALL report progress as (phase, items done, items in the phase) after layout and after every GPU batch,
with phases `layout`, `direct`, `bounce`, `volume`; `done` SHALL reach `total` at the end of each phase.

#### Scenario: Phases in order
- **WHEN** `upstream-basic` is baked with a progress hook
- **THEN** the hook sees `layout`, `direct`, `bounce` (twice to completion) and `volume`, in that order, each ending with done = total

### Requirement: Cancellation within one batch
A bake SHALL ask the cancel hook before every GPU batch and after every readback and stop with a `Cancelled`
error as soon as it returns true, without writing to the tile store afterwards.

#### Scenario: Cancel during the direct pass
- **WHEN** the cancel hook starts returning true after the first progress report of phase `direct`
- **THEN** the bake fails with `Cancelled` and no further batch is dispatched

#### Scenario: Next bake after a cancel
- **WHEN** a bake is cancelled and the same GPU bakes the scene again without cancelling
- **THEN** the second bake succeeds with the same bytes as an uncancelled bake
