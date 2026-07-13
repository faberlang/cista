# Factory documentation (cista)

Open factory tracks for the public **cista** package store / registry client.

Relocated from private Radix on 2026-07-08.

## Active contract

The current unblocked cista contract is the local/package-store client:

- `$CISTAE_HOME`, defaulting to `~/.faber/cistae`, is the shared package store.
- Project repositories keep manifests and lockfiles, not installed dependency
  trees.
- `cista` owns install, inspect, remove, run, cache, publish, fetch, and
  credential handling for package artifacts.
- `faber` consumes documented manifests and lockfiles, and must not depend on
  the `cista` crate or discover the cista store directly.
- Package roles `lib`, `bin`, and `meta` share the same store shape.

Phase G and the bounded path-safety theme are closed. The client has hermetic
local/filesystem registry proof, authenticated HTTP transport proof, credential
proof, remote CLI routing proof, staged cache extraction, and path containment
proof through `64a567d`.

The remaining live `cista.dev` proof is operator-gated. It requires explicit
live registry environment variables, isolated credentials and cache roots, a
disposable package identity, and evidence recorded without tokens or headers.
No further unblocked `cista.dev` implementation unit is named here.

## Layout (current)

```text
cista/
  src/                 library + CLI
  docs/factory/        this control plane
# siblings
  ../faber             project tool (library resolution, package build)
  ../norma             public stdlib source
  ../faber-runtime     generated Rust runtime dependency
  ../radix             private compiler session/config surfaces
```

```bash
cargo build --release
./target/release/cista --help
```

Each `goal.md` owns its **Status** line.

## Open goals

| Goal | Status | Entry |
| ---- | ------ | ----- |
| Cista package store | Phase G and path-safety theme v1 closed; live cista.dev proof operator-gated | [`cista-package-store/goal.md`](cista-package-store/goal.md) |

## Archived phase evidence

Closed phase details remain in the package-store goal and delivery notes:
Phase A local library install and Faber lock consumption, Phase B Norma package
problem/delivery notes, Phase C binary packages, Phase D `cista run`, Phase E
meta packages, Phase F local/dev registry, Phase G HTTP/auth transport, and the
post-Phase-G path-safety closeout.

The draft public-registry campaign under
[`cista-dev-registry/`](cista-dev-registry/) is design discovery for a future
deployed service. It is not live service proof and does not reopen the closed
client contract.
