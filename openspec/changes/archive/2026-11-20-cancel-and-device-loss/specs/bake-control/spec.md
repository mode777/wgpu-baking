# Spec Delta

## ADDED Requirements

### Requirement: Cancelled bakes leave the tile store unchanged
A bake cancelled at any GPU batch SHALL write no tile or probe chunk to its tile store.

#### Scenario: Cancel during a bounce
- **WHEN** a bake with an empty in-memory store is cancelled during its first bounce
- **THEN** the store is still empty and a following bake with the same store bakes every tile
