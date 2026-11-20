# Design: cancel-and-device-loss

## Context

`bakerCore` posts `error{kind:'device-lost'}` from the bridge's lost callback (with the running job's id, if
any), then the running bake rejects with `device-lost` too; `BakerClient` rejects the job and calls
`onError`. The bridge exposes `loseDeviceForTesting()`. Engine writes tiles to the store only after all
passes (`bake.rs`), so a cancel error before that leaves the store untouched — not yet tested.

## Goals / Non-Goals

**Goals:** recovery without reload; one clear device-lost signal per baker; manual real-GPU proof.
**Non-Goals:** keeping baked tiles across workers (the store dies with the worker; documented).

## Decisions

- **`BakerService`** wraps `BakerClient` with a `createWorker` factory (DI for tests) and a small state
  machine: `starting → ready | unavailable | refused → (lost) → starting`. `recreate()` disposes the old
  client (terminates the worker) and runs `init` again. The page binds buttons to the service state.
- **One loss report per baker:** the core posts the global `device-lost` error (no `jobId`) once when the
  callback fires and drops its baker; a bake running at that moment additionally rejects with its own
  `device-lost` job error. The service reacts only to the global one (idempotent).
- **Debug request** `{ type: 'debug-lose-device' }`: only acted on by the core; the page sends it only with
  `?debug=1`. Kept in the protocol because the manual device-loss check needs a real `device.destroy()`
  inside the worker's device, which the page cannot reach otherwise.
- **Rust test** uses a `MemoryStore`, a progress hook that cancels at the first `bounce` report, and asserts
  `store.is_empty()` and that the next bake bakes every tile.

## Risks / Trade-offs

- [Browser fires `lost` asynchronously after `destroy()`] → UI waits on the event, not on the call.

GPU-to-CPU readbacks, blocking calls, native-only dependencies: none. ADRs: ADR-0002 status updated
(confirmed by the Phase 2 spike).
