# Spec Delta

## ADDED Requirements

### Requirement: Recovery from device loss without reload
When the GPU device is lost, the page SHALL say so, refuse to bake with the lost baker, and offer to recreate
the baker, which SHALL start a new worker with a new device without reloading the page.

#### Scenario: Device destroyed, then recreated
- **WHEN** the device is destroyed (driver reset, or `device.destroy()` in a debug session) and the user presses "Recreate baker"
- **THEN** the page shows the loss, then a new baker becomes ready and the next bake passes its checks

#### Scenario: Loss during a bake
- **WHEN** the device is lost while a bake runs
- **THEN** the bake ends with a `device-lost` error and the page offers to recreate the baker
