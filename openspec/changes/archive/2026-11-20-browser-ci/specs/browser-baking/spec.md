# Spec Delta

## ADDED Requirements

### Requirement: CI bake profile
The page SHALL offer a `ci` profile (URL `profile=ci`) that lowers sample counts without changing the layout,
such that every check in `upstream-basic/expect.json` still passes on hardware and software adapters.

#### Scenario: CI profile on a software adapter
- **WHEN** `upstream-basic` is baked with `profile=ci` on a software adapter with the opt-in
- **THEN** all 19 checks pass and the result is labelled as from a software adapter
