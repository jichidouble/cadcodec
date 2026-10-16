# On-Demand Test Workflow Design

## Goal

Make the common FFI development loop run only the relevant test target while
keeping explicit commands for full release and all-features regression.

## Current Finding

The release FFI suite runs the tests quickly after compilation, but compiling
the optimized library and every integration target dominates the wall-clock
time. The repository already has useful target boundaries (`--lib`,
`--test abi_test`, and `--test ffi_setter_inventory`) and Cargo name filters.

## Design

Add Cargo aliases in `.cargo/config.toml` without changing test behavior or
moving existing tests:

```toml
tffi-lib = ["test", "--release", "--features", "ffi", "--lib"]
tffi-abi = ["test", "--release", "--features", "ffi", "--test", "abi_test"]
tffi-inventory = ["test", "--release", "--features", "ffi", "--test", "ffi_setter_inventory"]
tffi-all = ["test", "--release", "--features", "ffi"]
tall = ["test", "--all-features"]
```

The aliases accept Cargo filters after the alias name. For example,
`cargo tffi-abi phase4b1_ -- --nocapture` runs only the phase-specific ABI
tests, while `cargo tffi-all` remains the complete FFI regression command.

The release profile remains unchanged because it is the shipped artifact
profile. No new dependency, test runner, or test-target migration is added.

## Verification

Verify each alias with `--list` or one focused test, then run the existing full
FFI and all-features commands before a release or integration handoff.
