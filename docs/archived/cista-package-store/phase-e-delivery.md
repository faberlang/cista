# Phase E Delivery — Meta Packages

**Status:** closed
**Closed:** 2026-07-10

A meta package is a minimal `cista.toml` containing `[source]` identity with
`role = "meta"` and one or more exact `[[dependencies]]`. For local installation,
each dependency also names a path relative to the meta package.

`cista install --path` validates the entire direct dependency set—including
unique pins and agreement between declared and resolved name/version—before
writing any package. It then uses the normal library/binary installation path
for every dependency and stores the meta package at `<name>/<version>` with
identity and pins only. Source-relative paths are deliberately omitted from the
installed snapshot.

## Proof

- `examples/coreutils/packages/coreutils` expands to the standalone `true`
  binary package.
- `install_meta_expands_exact_local_dependencies` verifies both store entries
  and proves the installed meta snapshot has no stale source path.
- A CLI-level local install verifies the meta entry and host executable are
  both materialized.

Phase E supports direct local dependency sets. Remote retrieval and broader
dependency solving belong to Phase F.
