# Phase D Delivery — Installed Binary Execution

**Status:** closed
**Closed:** 2026-07-10

`cista run name[@version] -- <args>` resolves an installed package using the
existing store rules, selects only `targets/rust/<host-triple>/cista.toml`, and
executes the artifact named there. It fails closed when the package role is not
`bin`, host metadata does not match, the artifact is absent, or its manifest
path is not a single file name within the target directory.

## Proof

- `run_installed_binary_with_passthrough_argument` installs a binary package,
  deletes its complete source tree, and runs the stored executable with an
  argument the executable verifies.
- `executable_path_rejects_library_packages` proves a library artifact is not
  executable through this command.
- A CLI-level install/delete/run proof succeeds with
  `examples/coreutils/packages/true`.

Process failures are reported as command errors. Phase D does not introduce
PATH shims, source-tree execution, or cross-target fallback.
