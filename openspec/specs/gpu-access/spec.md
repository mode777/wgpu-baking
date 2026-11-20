# gpu-access Specification

## Purpose
How the engine opens a GPU: which adapters it accepts, which device limits it requests and reports, and that
waiting for GPU work never blocks the calling thread (a requirement in the browser).

## Requirements

### Requirement: Software adapters only on explicit opt-in
Opening a GPU SHALL fail with a "software adapter refused" error naming the adapter when the chosen adapter is
a software or fallback adapter and the caller did not allow software adapters.

#### Scenario: Refused
- **WHEN** the only adapter is llvmpipe and software adapters are not allowed
- **THEN** opening fails with the software-adapter-refused error naming llvmpipe

#### Scenario: Allowed
- **WHEN** software adapters are allowed
- **THEN** opening succeeds and the adapter report marks it as software

### Requirement: Adapter report
An opened GPU SHALL report the adapter name, vendor, device type, backend, driver, whether it is a software
adapter, and the granted limits for storage-buffer binding size, buffer size and compute workgroups per dimension.

#### Scenario: CLI shows the adapter
- **WHEN** the CLI bakes a scene
- **THEN** it prints the adapter's name, backend and granted limits before baking

### Requirement: Explicit device limits
The engine SHALL request the WebGPU default limits, raising only the maximum storage-buffer binding size and
maximum buffer size to the adapter's values, and SHALL reject a buffer exceeding the granted limits with a
limit-exceeded error before allocating it.

#### Scenario: Oversized buffer
- **WHEN** a bake needs a storage buffer larger than the granted binding size
- **THEN** it fails with a limit-exceeded error naming the buffer and both sizes, without a GPU validation panic

### Requirement: Non-blocking GPU waits
The bake and every GPU readback SHALL be asynchronous; under the `web` feature no code path SHALL block the
thread to wait for the GPU, and the browser build SHALL select only the WebGPU backend.

#### Scenario: Web build
- **WHEN** the engine is built with the `web` feature
- **THEN** it contains no blocking executor or device poll-wait and requests a `BROWSER_WEBGPU` instance

#### Scenario: Native results unchanged
- **WHEN** a native caller awaits the bake with a blocking executor
- **THEN** results are byte-identical to the synchronous engine
