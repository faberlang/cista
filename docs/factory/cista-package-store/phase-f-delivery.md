# Phase F Delivery — Local/Dev Registry Client

**Status:** local/dev loop closed; live transport residual
**Closed:** 2026-07-10

The first registry client slice uses an explicit filesystem registry configured
with `--registry` or `CISTA_REGISTRY`. Its stable local layout is
`<registry>/<name>/<version>/`, with published versions immutable.

- `cista publish --path <source> --registry <root>` validates and snapshots a
  source package.
- `cista fetch name@version` requires an exact pin and copies the snapshot into
  `$CISTAE_HOME/.cache/registry/<name>/<version>`.
- `cista install name@version` fetches that snapshot and runs the existing
  package installation pipeline without `--path`.

## Proof

- `publish_and_fetch_exact_package_snapshot` proves publication, exact fetch,
  source independence, immutability, and rejection of unpinned fetches.
- A CLI proof publishes `true`, deletes its source, fetches and installs
  `true@0.1.0` without `--path`, then runs the installed executable.
- Phase D/E focused regression tests continue to pass.

## Honest residual

This slice does not claim a live cista.dev wire protocol. HTTPS transport,
content integrity/signatures, authentication, publish authorization, yank, and
server-side index semantics require a registry contract and remain future work.
The local transport establishes the client workflow and cache/install boundary
without inventing those external decisions.
