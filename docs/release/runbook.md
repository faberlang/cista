# Cista component release runbook (thin)

**Date-stamped:** 2026-08-07
**Owner:** cista (component surface) — see
[`faber/docs/release/authority.md`](../../../faber/docs/release/authority.md)
for the authority roles (tagger/publisher are operator-authorized).
**Shared contract:** the coordinated release process lives in
[`faber/docs/release/`](../../../faber/docs/release/) — `release-runbook.md`
(the coordinated operator runbook, Stage 2), plus the Stage-1 decisions:
[`release-contract.md`](../../../faber/docs/release/release-contract.md),
[`release-manifest-schema.md`](../../../faber/docs/release/release-manifest-schema.md),
[`process-local-first.md`](../../../faber/docs/release/process-local-first.md),
and [`authority.md`](../../../faber/docs/release/authority.md). This thin
runbook names only the cista-local path; the shared contract is the authority
on channels, immutability, and publication.

Cista is an **independent component** release unit: `0.Y.Z`, a **binary-only**
component release (CLI archives + checksums) published as `cista-vX.Y.Z` on
`faberlang/releases` (`release-contract.md` §9; crates.io library publication
is explicitly deferred with owner). It never advances the shared repo's
global `Latest` (`release-contract.md` §4.2). A Faber product release pins the
**cista source revision** in the release manifest — a source pin, not a binary
prerequisite.

## Cista release path (bump → regen-lock → tag → workflow_dispatch → publish)

Cista-local script (cista scripta, stdlib-only python3 — created by the
component-release-streamline campaign):

- **`./scripta/regen-lock`** — regenerate `Cargo.lock` (`cargo update
  --offline`, registry-cache caveat documented in the script) and verify
  freshness versus the manifests (F2 — a stale lockfile breaks `--locked`).
  `./scripta/regen-lock --check` verifies without writing.

Operator steps:

1. Bump `Cargo.toml` `[package] version` (cista has no bulk bump; single
   manifest). If a version alignment check is needed, the shared
   `faber/scripta/release-doctor` preflight applies the same rules.
2. `./scripta/regen-lock` — regenerate and verify the lockfile.
3. `cargo build --locked --release --bin cista` — locked release build
   (**local** proof).
4. **Single commit** containing the version bump + regenerated lockfile.
5. `git tag vX.Y.Z` — local tag bookkeeping; push is **network**.
6. `git push origin main && git push origin vX.Y.Z` — **network**; the tag
   push triggers `cista/.github/workflows/release.yml`, or use
   `workflow_dispatch` with the existing source tag
   (`.github/workflows/release.yml:4-13`).
7. Publish: the workflow builds the matrix (linux x64 + macOS x86_64 +
   macOS arm64) and uploads to `faberlang/releases` as `cista-vX.Y.Z`
   (`--latest=false`). Check the `.sha256` content names **only the downloaded
   archive basename** (F7; `release-contract.md` §5.1). **Never** `--clobber`
   a stable asset (`release-contract.md` §5.3).

## Routed residual (F5 — not fixed here)

The `cista-v0.1.0` public release is **unfulfilled**: the cista release note
(`cista/docs/release/v0.1.0.md`) claims artifacts publish to
`faberlang/releases` as `cista-v0.1.0`, but no such release is observed on the
shared surface (stage0-baseline.md §5 F5). That claim is **routed to the cista
owner** to reconcile (publish or correct the note) before the next cista
release. This runbook records the residual; it does not execute the publish.

## References

- `cista/docs/release/v0.1.0.md` — historical release note (unfulfilled
  publish claim, see above).
- `cista/.github/workflows/release.yml` — the tag/workflow_dispatch publish
  workflow (observed; CI thinning is a later campaign stage).
- `faber/docs/release/release-contract.md` — cista surface (§9), channels,
  immutability.
- `faber/docs/release/release-manifest-schema.md` — how a Faber product
  release pins the cista source revision.
