# Spec Delta

## Purpose

Committed, deterministic test scenes with machine-checkable expectations, so the same light-transport
assertions can be run against any bake (native CLI, tests, and later the browser).

## ADDED Requirements

### Requirement: Upstream basic scene is committed
The repository SHALL contain `test-scenes/upstream-basic/upstream-basic.lbscene`, byte-identical to the
scene upstream's GPU integration test builds, regenerable by a documented command.

#### Scenario: Committed scene matches the generator
- **WHEN** the test suite runs
- **THEN** a test confirms the committed file equals the generator's output byte for byte

### Requirement: Expectations file per scene
Each test scene SHALL have an `expect.json` naming world-space probe points on the direct, indirect or volume
output and checks of the form `a op b·scale + offset` over probe components, where `op` is `<` or `>`.

#### Scenario: Checks encode upstream assertions
- **WHEN** `upstream-basic/expect.json` is read
- **THEN** it contains checks for the box shadow, sunlit ground, occlusion at the box's foot, red bleed near the wall, sky light, volume sun visibility and light along the narrow strip

### Requirement: Bakes can be checked against expectations
The CLI SHALL, with `--check`, evaluate the `expect.json` beside each scene after baking, print every
probe value and check result, and exit non-zero if any check fails.

#### Scenario: Passing bake
- **WHEN** `lightbaker test-scenes/upstream-basic/upstream-basic.lbscene --check` runs on a GPU
- **THEN** every check is reported as passed and the exit code is 0

#### Scenario: Probe off the mesh
- **WHEN** a probe point lies on no triangle of its mesh
- **THEN** evaluation fails with an error naming the probe

### Requirement: Scene bakes are reproducible
Baking a committed test scene twice on the same adapter without a cache SHALL produce identical `.lbake` bytes.

#### Scenario: Two CLI runs
- **WHEN** the CLI bakes `upstream-basic` twice into different directories
- **THEN** both `.lbake` files are byte-identical
