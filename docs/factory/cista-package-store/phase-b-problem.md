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

Phase A: every third-party provider must appear in `faber.toml` deps + lock.

**Norma is special only as product default, not as resolver magic.**

Options:

| Option | Behavior | Pros | Cons |
| --- | --- | --- | --- |
| **B1 Declare** | Apps list `norma = "x.y.z"` like any dep | Uniform; honest | Every package churn |
| **B2 Implicit lock** | Toolchain install always writes Norma into `faber.lock` without faber.toml dep | Ergonomic | Hidden dep; faber must accept lock entries without declared dep |
| **B3 Hybrid** | faber.toml may omit Norma; if lock has Norma, use it; else FABER_LIBRARY_HOME fallback | Soft migration | Two resolution paths longer |

**Mandatory for problem lock:** pick migration posture before coding.

Recommendation for first B delivery: **B3 Hybrid** with explicit exit criterion:

- With lock entry for `norma`, build works **without** library-home.
- Without lock entry, sibling `FABER_LIBRARY_HOME` still works (dev).
- Declaring `norma` in deps is allowed and preferred for new packages; not
  required for entire corpus in B.

### P6 — Versioning

- Faber tool is `0.38.0` today; goal examples used `0.36.0` for Norma.
- Norma repo has **no package version file** yet.
- Phase B needs a **version source of truth** for store path
  `norma/<version>/` (e.g. norma `VERSION` / `cista.toml` / align to faber).

Do not invent SemVer solving; exact pins only (Phase A rule).

### P7 — Annotation cleanup (re-scoped)

Parent goal treats `@ verte` cleanup as Phase B pressure. **Live Norma already
cleared that debt** for public sources.

Phase B annotation work is therefore:

- **Not** “strip @verte from norma/src” (already clean).
- **Is** “do not reintroduce target annotations; keep bindings/policy in package
  metadata if any native hooks return.”
- Residual `@verte` pressure may still exist in **radix/compiler** or historical
  docs — out of Phase B package-store scope unless it blocks install.

### P8 — Nested modules and install snapshot

Install must preserve directory structure:

```text
interfaces/solum.fab
interfaces/solum/path.fab
interfaces/json/solve.fab
```

Not a flat dump. Phase A mathesis was single-file; Norma is the first multi-module
tree install proof.

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

## Recommended problem lock (decisions)

Propose these as **decided for Phase B delivery** (confirm or amend):

1. **Norma is one package identity** `norma`, multi-module interface tree.
2. **Default policy is source + generated** (Faber bodies), not manifest bindings.
3. **Install snapshots interfaces** into the store; building `libnorma.rlib` is
   **out of B** unless a later slice proves a need.
4. **faber-runtime stays a separate always-linked runtime**; not Norma’s artifact.
5. **Lock contract** from Phase A is the only build-time Norma path for “packaged”
   mode; no cista knowledge in faber.
6. **Migration = hybrid**: lock wins when present; library-home remains fallback.
7. **Version** must be explicit in Norma’s package manifest before first install.
8. **Goal.md Norma-style TOML sample** should be rewritten to match live Norma
   (generated / source), with “manifest bindings” reserved for non-Norma native
   packages.

## Exit criteria (problem-level, not full delivery yet)

Phase B is done when **all** of the following are true:

1. There exists a Norma package layout + `cista.toml` consistent with (1–4).
2. `cista install --path <norma-package>` materializes
   `$CISTAE_HOME/norma/<ver>/interfaces/...` (tree preserved).
3. A consumer with `faber.lock` Norma entry typechecks/builds using that
   `interface_root` for `norma:*` **with library-home unavailable or unused**
   for provider `norma`.
4. Same consumer without lock entry still works via existing sibling fallback.
5. faber still has **zero** dependency on cista / `CISTAE_HOME`.
6. Docs state: Norma is a normal package instance; sibling tree is fallback.

## Open questions still needing a human call

| ID | Question | Blocks code? |
| --- | --- | --- |
| Q1 | Hybrid (B3) vs require `norma` in every `faber.toml` (B1)? | Soft — recommend B3 |
| Q2 | Norma version source of truth (file / align to faber / independent)? | Yes for install path |
| Q3 | Keep `norma/src` path in-repo and map to `interfaces/` only in store, or restructure repo? | Soft — prefer map-at-install, no force-move |
| Q4 | Should lock `artifact` be omitted for Norma v1 or point at a compile-policy placeholder? | Soft — omit or policy-only |
| Q5 | Is “codegen Norma into every app crate” acceptable mid-term, or must B start shared rlib caching? | Soft for B exit; cache is optimization later |

## Relationship to other docs

- Parent: `goal.md` Phase B section — **update after this lock** so delivery does
  not implement the stale manifest-bindings Norma sketch.
- Phase A delivery: `phase-a-delivery.md` — contract to extend, not replace.
- Sibling faber `unified-package-manifest` — overlapping library-install ideas;
  Phase B here is **cista-store + faber.lock** path, not `FABER_LIBRARY_HOME`
  clone install as the long-term consumer model.

## Next step after lock

1. Confirm or amend **Recommended problem lock** and **Q1–Q5**.
2. Rewrite parent goal’s Norma TOML / Phase B bullets to match.
3. Compile Phase B delivery spec (install tree + lock + faber resolve + demo).
4. Factory implement.

---

*This document is the problem iteration artifact. It is not a delivery spec and
not authorization to implement until the recommended lock is accepted.*
