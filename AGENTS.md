# Cista development measurements

Follow the enclosing Faber workspace instructions and any assigned lane scope.

## Measured local validation times

Measured 2026-10-03 on burgus, in an isolated `cis1` checkout with its own
on-disk Cargo target. These are focused development checks, not release gates.

| Command | Wall time | Evidence |
| --- | --- | --- |
| `cargo build` | 1.98s | Passed; dependencies warm from the focused tests |
| `cargo test --lib changed_incoming_archive_is_rejected_before_cargo -- --nocapture` | 1.94s | Passed; incremental compile and archive scenarios |
| `cargo test --lib commands::install::tests:: -- --skip install_real_norma_platform_default_builds_nested_import_without_dependency` | 0.95s | 26 passed; nested Faber CLI build explicitly excluded |
| `cargo test --lib commands::rust_target::tests::` | 0.09s | 9 passed |
| `cargo clippy --lib --tests -- -D warnings` | 1.30s | Passed; warm dependencies |
| `cargo fmt --check` | 0.14s | Passed |

The first cold test compile took 5.89s. That first invocation failed during
fixture setup, so it is a compile measurement, not a passing-test baseline.
Full-suite and nested Faber CLI build times were not measured in this unit.
