# Phase B Problem Iteration — Norma as a package

**Status**: problem lock (iterate before delivery)  
**Parent goal**: `goal.md`  
**Depends on**: Phase A shipped (store + `faber.lock` + faber lock consume)  
**Date**: 2026-07-08

## One-sentence problem

**Make `norma:*` resolve and build from package-manager lock paths (same
contract as mathesis), while Norma’s real implementation model is pure Faber
source + host `ad` routes + always-on `faber-runtime` carriers — not a
hand-written `libnorma.rlib` with `[[bindings]]`.**

## Why this iteration exists

The parent goal still sketches “Norma-style” packages as:

- `binding_policy = "manifest"`
- hand-written `targets/rust` + `[[bindings]]`
- installed `libnorma.rlib`
- cleanup of `@ verte` / `@ subsidia` in interfaces

**Live Norma (2026-07) contradicts that sketch.**

| Claim in older goal text | Live evidence |
| --- | --- |
| Norma has hand-written Rust target + manifest bindings | `norma/src/**/*.fab` is pure Faber; AGENTS bans `@ externa` / `@ subsidia` |
| `@ verte` / `@ subsidia` leak in Norma interfaces | **Zero** matches under `norma/src` today |
| Norma runtime is a Rust `norma` crate | No such crate; generated packages depend on **`faber-runtime`** (`use faber::…`) |
| Stdlib discovery is forever special | Today: `$FABER_LIBRARY_HOME/norma/src/*.fab` (sibling layout), separate from cista store |

Phase B must **re-lock the problem against live reality**, not implement the
stale sketch.

## What Phase A already solved (inputs to B)

1. Shared store layout under `$CISTAE_HOME` (cista-owned discovery).
2. `cista install --path` for Rust **source** libs → interfaces + artifact + target `cista.toml`.
3. Project `faber.toml` `[dependencies]` + `faber.lock` with **absolute** paths.
4. `faber` resolves provider imports from locked `interface_root` without knowing cista.
5. Repo separation: no faber↔cista crate dependency.

**Mathesis proof** is a third-party-shaped package with Faber interface bodies
(and a small Rust rlib). Norma is larger and different in *implementation
layering*, not just size.

## Live architecture (ground truth)

```text
┌─────────────────────────────────────────────────────────────┐
│ Application package (faber.toml + faber.lock)               │
│   importa ex "norma:solum" …                                │
└───────────────────────────┬─────────────────────────────────┘
                            │ today: LibraryResolver →
                            │   $FABER_LIBRARY_HOME/norma/src/solum.fab
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ Norma source (public repo)                                  │
│   norma/src/**/*.fab                                        │
│   - pure Faber bodies                                       │
│   - pure math/text (mathesis, chorda, valor, json/*)        │
│   - host I/O via ad 'solum:lege' / 'tempus:…' routes        │
│   - nested modules: solum/path, json/solve, caelum/…        │
└───────────────────────────┬─────────────────────────────────┘
                            │ package build path today:
                            │ re-codegen library .fab into app crate
                            ▼
┌─────────────────────────────────────────────────────────────┐
│ Generated Rust app crate                                    │
│   + always: path dep on faber-runtime (Valor, Ascii, …)     │
│   + host frame / ad dispatch (runtime + platform)           │
│   NOT: link libnorma.rlib as the stdlib body                │
└─────────────────────────────────────────────────────────────┘
```

### Inventory (approx.)

- **~32** top-level `.fab` modules under `norma/src` (plus nested dirs:
  `solum/`, `json/`, `caelum/`, `llm/`, …).
- **Implementation styles mixed in one package identity `norma`:**
  - pure catalog (e.g. `mathesis`, large parts of `chorda`)
  - pure Faber codecs (`json/*`, `valor`)
  - host-routed devices (`solum`, `tempus`, `processus`, `consolum`, …) via `ad`
- **faber-runtime** = language carriers and display/frame helpers, not “Norma
  stdlib as a Rust package.”

### Two different “mathesis” names (do not conflate)

| Name | Role |
| --- | --- |
| `examples/cista-lab/source/mathesis` | **Third-party cista package** used in Phase A |
| `norma:mathesis` | **Stdlib math catalog** inside Norma source |

Phase B is about packaging **Norma** (`norma:*`), not renaming the lab fixture.

## Real problem decomposition

### P1 — Identity and layout

**Question:** What is the installable unit called and how do files map?

Proposed lock (aligned with Phase A + live source):

```text
$CISTAE_HOME/norma/<version>/
  interfaces/          # copy or snapshot of norma/src tree
    solum.fab
    solum/path.fab
    json.fab
    json/solve.fab
    …
  targets/rust/<triple>/
    cista.toml         # package identity + policy (may be thin)
    # artifact: TBD — see P3
```

Import `norma:solum/path` →  
`interface_root/solum/path.fab`  
(same relative module path as today under `norma/src`).

**Today’s resolver layout** is `library_home/norma/src/...`.  
**Lock layout** is `interface_root/...` (no extra `src` segment).  
Phase A already uses the lock layout for mathesis. Norma must use the **same**
lock layout so faber stays store-agnostic.

### P2 — What “install Norma” means when there is no libnorma

Phase A install **builds a Rust library** and copies an `.rlib`. That fit
mathesis’s hand-written `targets/rust`.

For Norma:

| If we force `cargo build` of a fake norma crate | Cost |
| --- | --- |
| Invents a Rust package that does not exist as product | High |
| Duplicates what codegen already does from `.fab` | Waste |
| Couples version of Norma sources to a rustc rlib ABI | Painful |

**Problem statement:** Phase B install must snapshot **Faber interface/source
tree** into the store and write lock fields faber already understands. Building
a dedicated `libnorma.rlib` is **not** required for the first honest exit if
faber continues to **codegen Norma modules from locked interfaces** (current
behavior for library packages with bodies).

That is still a real package: versioned, shared, no project-local copy, lock
pins absolute paths.

### P3 — faber-runtime vs Norma package (mandatory separation)

These are **two different products**:

| Artifact | Owns | Consumer |
| --- | --- | --- |
| **Norma package** | `norma:*` Faber APIs and bodies | import resolution + typecheck + codegen of library modules |
| **faber-runtime** | Language-level Rust carriers (`Valor`, …) | every generated Rust crate’s Cargo.toml |

**Do not** merge them into one cista package in Phase B.

- Generated crates already path-depend on `faber-runtime` via faber package
  builder (`package/cargo.rs`).
- Locking Norma does not replace that dependency.
- Later: toolchain install may pin **both** Norma package version and
  faber-runtime version; that is distribution, not one store entry.

**Phase B success does not require** “Norma’s artifact field points at
faber-runtime.” Prefer:

- `interface_root` required and used
- `artifact` optional / absent / or a compile-policy stub for Norma v1
- or `kind = "source"` with `mode = "compile"` meaning “consumer/toolchain
  compiles Faber sources,” not “prebuilt rlib”

### P4 — Manifest policy for Norma

**Recommended default for Phase B (re-lock):**

```toml
[source]
package = "norma"
version = "<align with faber or norma release>"
faber_min = "…"
kind = "source"
interfaces = "interfaces"   # or "src" mapped at install

[target]
language = "rust"
mode = "compile"
binding_policy = "generated"   # NOT manifest — bodies are Faber
# no [[bindings]] required for pure Faber + ad routes
```

The goal’s `binding_policy = "manifest"` Norma example is **wrong for live
Norma** unless we reintroduce hand-written Rust targets (explicitly not current
Norma policy).

Reserve `manifest` bindings for packages like lab mathesis / future native
shims — not for default Norma.

### P5 — Declaration and default injection

Phase A: third-party providers appear in `faber.toml` deps + lock.

**Decided (user, 2026-07-08):** Norma is a **hard platform default**, not an
application dependency declaration.

- Apps **do not** list `norma` in `faber.toml` `[dependencies]`.
- Norma **just is** available to any Faber package build (like a language stdlib).
- That does **not** reintroduce faber walking `$CISTAE_HOME`. Provisioning still
  produces paths faber can consume:
  - **Dev:** sibling / `FABER_LIBRARY_HOME` layout (unchanged fallback).
  - **Packaged:** toolchain or `cista` ensures Norma is installed and the build
    sees Norma via **implicit lock injection** or an equivalent default lock
    record (absolute interface paths), not via a user-authored dep line.
- Third-party packages still use explicit `faber.toml` deps + lock rewrite.

So Norma is special as **product default / always-on provider**, not as a
second store category or a cista-aware faber.

### P6 — Versioning

**Decided:** Norma is **versioned independently of Faber**.

- Store path: `$CISTAE_HOME/norma/<norma_version>/…`
- Norma’s own package manifest (or VERSION) is the source of truth for that
  version string — not `faber`’s crate version.
- Faber may declare a minimum compatible Norma (`faber_min` inverted, or
  toolchain pins both) later; Phase B only requires Norma to carry its own
  version.

Do not invent SemVer solving; exact pins only where pins exist (Phase A rule).

### P7 — Annotation cleanup (re-scoped)

Parent goal treats `@ verte` cleanup as Phase B pressure. **Live Norma already
cleared that debt** for public sources.

Phase B annotation work is therefore:

- **Not** “strip @verte from norma/src” (already clean).
- **Is** “do not reintroduce target annotations; keep bindings/policy in package
  metadata if any native hooks return.”
- Residual `@verte` pressure may still exist in **radix/compiler** or historical
  docs — out of Phase B package-store scope unless it blocks install.

### P8 — Nested modules, `interfaces/`, and install mapping

#### What `interfaces/` means

In the **installed store**, every package version root has a target-neutral
Faber API tree:

```text
$CISTAE_HOME/<package>/<version>/
  interfaces/     ← Faber .fab contracts the compiler typechecks against
  targets/…       ← optional: compiled artifacts or compile policy
```

That directory name is the **store vocabulary**, not “a special Norma idea.”
Phase A mathesis already installs into `interfaces/mathesis.fab`.

#### Source layout vs store layout

A **source package** (git tree before install) may put Faber files anywhere its
`cista.toml` declares:

```toml
# mathesis lab (already)
interfaces = "interfaces"   # source tree path → copied into store interfaces/

# Norma today (likely)
interfaces = "src"          # source tree path norma/src → store interfaces/
```

At **install**, cista always materializes the store as:

```text
…/norma/<ver>/interfaces/solum.fab
…/norma/<ver>/interfaces/solum/path.fab
…/norma/<ver>/interfaces/json/solve.fab
```

So “remap” means: **read from whatever path the package author used; write the
canonical store tree.** It does **not** mean rewrite every import or rename
every repo on disk.

#### Does every cista install do this?

**Yes — for the Faber-facing API tree.** That is the package-store contract:

| Source repo | cista.toml field | Store after install |
| --- | --- | --- |
| mathesis | `interfaces = "interfaces"` | `…/mathesis/0.1.0/interfaces/` |
| norma | `interfaces = "src"` (proposed) | `…/norma/<ver>/interfaces/` |
| future lib | whatever path they choose | always `…/<pkg>/<ver>/interfaces/` |

faber only ever sees the **store (or lock) path** to that tree
(`interface_root` in `faber.lock`). It never needs to know the source-repo
spelling (`src` vs `interfaces`).

**Decided lean:** keep `norma/src` in the Norma repo; map to store `interfaces/`
at install. No forced in-repo restructure for Phase B.

Nested modules must preserve directory structure under that tree (not a flat dump).

### P9 — Fallback and fail-closed

| Mode | Behavior |
| --- | --- |
| Dev without install | `FABER_LIBRARY_HOME` / sibling `norma/src` (today) |
| Project with Norma in lock | use `interface_root` only for `norma` provider |
| cista store miss when install required | searched-path diagnostics (cista-owned) |
| faber missing lock path | path-missing diagnostics (faber-owned; Phase A) |

**Do not** teach faber to walk `$CISTAE_HOME` for Norma. That would re-break
separation.

## What Phase B is not

- Not merging Norma into faber-runtime.
- Not reintroducing a Rust `norma` crate as the public stdlib.
- Not registry / cista.dev.
- Not bins / `cista run`.
- Not forcing every corpus package to declare Norma on day one.
- Not multi-language Norma targets.
- Not implementing the stale goal sample that assumes `libnorma.rlib` +
  `binding_policy = "manifest"` as the default Norma shape.

## Problem lock (decided 2026-07-08)

1. **Norma is one package identity** `norma`, multi-module Faber tree.
2. **Default policy is source + generated** (Faber bodies), not manifest bindings.
3. **Install snapshots the Faber API tree** into the store under canonical
   `interfaces/`; building `libnorma.rlib` is **out of B** unless proven later.
4. **faber-runtime stays a separate always-linked runtime**; not Norma’s artifact.
5. **No cista knowledge in faber**; packaged builds use lock (or equivalent
   absolute path records), not `$CISTAE_HOME` discovery.
6. **Norma is a hard platform default** — not listed in app `faber.toml`
   `[dependencies]`. It is always available; third-party deps stay explicit.
7. **Norma version is independent of Faber**; Norma package manifest owns the
   version string used in the store path.
8. **Source path mapping is universal:** every cista install copies the path
   named by `cista.toml` `interfaces = "…"` into store `…/interfaces/`. Norma
   keeps `src/` in-repo; install maps `src` → store `interfaces/`. No special
   remap just for Norma.
9. **Artifact field (Q4 default):** omit or leave compile-policy-only for Norma
   v1; do not invent a fake rlib.
10. **Shared rlib cache (Q5 default):** out of B; keep codegen-from-locked-
    interfaces (same as today’s library path). Cache is a later optimization.
11. **Goal.md** Norma TOML / Phase B bullets must be rewritten to match this lock.

### Q1–Q5 resolution table

| ID | Decision |
| --- | --- |
| **Q1** | Norma is hard default: **not** in `faber.toml`. Always available. Provision via toolchain/install + lock (or dev library-home), not app dep lines. |
| **Q2** | Norma versioned **independently** of Faber. |
| **Q3** | `interfaces/` = **store** name for Faber API tree. Install **always** maps package-declared path → store `interfaces/`. Keep `norma/src` in repo. |
| **Q4** | No user preference → **omit artifact / policy-only** for Norma v1. |
| **Q5** | No user preference → **no shared rlib cache in B**; codegen from interfaces. |

## Exit criteria (problem-level)

Phase B is done when **all** of the following are true:

1. Norma package layout + `cista.toml` (independent version; `interfaces = "src"`
   or equivalent map).
2. `cista install --path <norma>` materializes
   `$CISTAE_HOME/norma/<norma_ver>/interfaces/...` (tree preserved).
3. A consumer **without** `norma` in `faber.toml` typechecks/builds `norma:*`
   from packaged paths (implicit lock / default provision) with library-home
   unused for provider `norma`.
4. Dev fallback without package install still works via sibling /
   `FABER_LIBRARY_HOME`.
5. faber still has **zero** dependency on cista / `CISTAE_HOME`.
6. Docs: Norma is the default package instance; not an app dependency line.

## Open questions remaining

| ID | Question | Notes |
| --- | --- | --- |
| Q6 | Exact mechanism for “hard default”: inject Norma into every `faber.lock` on install/bootstrap, vs faber built-in default path table written by toolchain? | Implementation shape; product intent is clear (always available, not in faber.toml) |
| Q7 | Initial Norma version string (`0.1.0` vs track faber minors independently)? | Needs a first number in norma `cista.toml` |

## Relationship to other docs

- Parent: `goal.md` Phase B section — **rewrite next** to match this lock.
- Phase A delivery: `phase-a-delivery.md` — third-party dep rules stay; Norma is
  the exception as platform default.
- Sibling faber `unified-package-manifest` — do not conflate with cista-store path.

## Next step

1. ~~Confirm Q1–Q5~~ (done).
2. Rewrite parent goal’s Norma / Phase B sections.
3. Resolve Q6 (default provision mechanism) in delivery spec.
4. Phase B delivery spec + factory implement.

---

*Problem lock accepted for Q1–Q5. Q6 is delivery design, not product ambiguity.*
