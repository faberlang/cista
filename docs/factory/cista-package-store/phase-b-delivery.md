# Phase B Delivery — Norma Platform Default Foundation

**Status**: foundation milestone delivered (2026-07-09)
**Parent goal**: [`goal.md`](goal.md)
**Problem lock**: [`phase-b-problem.md`](phase-b-problem.md)

## Scope

This slice proves the package-store foundation for Norma as a platform default
source package. It does not enter Phase C bins, Phase D `cista run`, Phase E
meta packages, or Phase F registry work.

## Locked Contract

- Norma package identity is `norma`.
- Norma is source-distributed for this phase: `source.kind = "source"`,
  `target.mode = "compile"`, `target.binding_policy = "generated"`.
- Norma source keeps its repo layout under `src/`; cista install maps the
  package-declared interface path into the store's canonical `interfaces/`
  directory.
- Norma is a hard platform default. `cista install --path <norma> --project
  <app>` may write a `faber.lock` record even when the app's `faber.toml` does
  not list `norma` in `[dependencies]`.
- No `libnorma.rlib`, `[[bindings]]`, or Rust `norma` crate is invented for the
  default story.
- `faber-runtime` remains a separate generated-code runtime dependency; it is
  not modeled as Norma's target artifact in Phase B.

## Implementation Evidence

Live implementation already carries the foundational path:

- `src/commands/install.rs` recognizes `norma` as a platform default for lock
  rewrite.
- Generated/interfaces-only packages are detected by
  `binding_policy = "generated"`, `target.mode = "compile"`, and no
  `target.source`.
- Interfaces are copied from the package-declared source path into
  `$CISTAE_HOME/<package>/<version>/interfaces/`.
- Interfaces-only installs write a thin target `cista.toml` with no artifact and
  no bindings.
- `../norma/cista.toml` declares the live Norma package shape:
  `interfaces = "src"`, `binding_policy = "generated"`, no target source, and
  version `0.1.0`.

This delivery adds a regression proof:

- `install_norma_platform_default_snapshots_src_interfaces_without_artifact`
  constructs a Norma-shaped source package with a nested module, installs it
  into a temp store, rewrites a project lock without a `[dependencies]` entry,
  verifies the store `interfaces/` tree, and asserts the lock/target manifest
  have no artifact or binding rows.

## Remaining Phase B Work

- Prove sibling `faber` consumes a locked `norma` platform-default record before
  falling back to `FABER_LIBRARY_HOME` for provider `norma`.
- Decide the exact provisioning mechanism for installed toolchains:
  cista/bootstrap lock injection vs a toolchain-owned default lock record.
- Add an end-to-end packaged-path build/check proof using real
  `../norma/cista.toml` and a real app that imports at least one nested
  `norma:*` module.

## Validation

```bash
timeout 300 cargo test install_norma_platform_default_snapshots_src_interfaces_without_artifact -- --format terse
```
