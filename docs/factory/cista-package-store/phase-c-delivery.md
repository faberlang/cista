# Phase C Delivery — Binary Packages

**Status:** closed
**Closed:** 2026-07-10

Phase C extends the existing package manifest and store shape rather than
creating a separate binary store. `source.role` identifies `lib`, `bin`, or
future `meta` packages; omitted roles remain `lib` so shipped Phase A/B
manifests retain their meaning.

For a Rust source package with `role = "bin"`, `cista install --path` builds the
binary named by `target.crate`, copies it into
`targets/rust/<host-triple>/`, and records that executable as the installed
target artifact. Phase D owns name/version resolution and execution through
`cista run`.

## Proof

- `examples/coreutils/packages/true` is a coreutils-shaped standalone binary
  package.
- `install_binary_materializes_runnable_host_entry` installs the fixture shape,
  verifies the installed manifest retains `role = "bin"`, and directly executes
  the installed artifact successfully.
- Focused binary and interfaces-only library install tests pass.

The packet's full library test run additionally requires pinned `norma` and
`radix` siblings. This worktree has neither; the real-Norma integration test
therefore remains unavailable in the packet, while its self-contained sibling
test passes.
