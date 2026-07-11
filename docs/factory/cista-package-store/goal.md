# Goal: Cista Package Store Model

**Status**: active — Phases A–F local/dev loop closed; next: cista.dev HTTP/auth transport
**Created**: 2026-06-21
**Updated**: 2026-07-10
**Target Repo**: `/Users/ianzepp/work/faberlang/cista`
**Factory Artifact Dir**: `docs/factory/cista-package-store/`
**Related**: `phase-a-delivery.md` (shipped), `phase-b-problem.md` (problem lock), `phase-b-delivery.md` (Phase B closed)
**Note**: Implementation lives in public sibling `faberlang/cista` (no radix dep;
no crate dependency on sibling `faber`).
**Primary Goal**: ship Faber's shared package artifact store, install lifecycle,
faber consumption of installed packages, package roles (lib / bin / meta),
`cista run` for installed apps, and later cista.dev client surfaces.

## Summary

Define and implement a coherent **cista** package model so standard and
third-party packages are installed into a shared store and consumed without
copying dependency trees into each project.

**Tool split (stable):**

| Tool | Role |
| ---- | ---- |
| `faber` | Project workflow: check, build, run, test on **source**; thin install facade later |
| `cista` | Package **store**: check, install, inspect, remove; may spawn `faber` / native tools for verification and build steps; later run installed apps, fetch/publish |
| `radix` | Compiler (not a cista dependency) |

**Direction:** artifact-first but not artifact-only. An installed package must
include Faber interfaces for typechecking, plus either compiled target
artifacts or source plus compile policy. Project directories hold manifests,
lockfiles, and project source only.

**Shipped (Phase A):** `cista check`, `install --path` (with project-root
`faber.lock` rewrite), store `package list|show|files`, `inspect`, `remove`;
`faber` consumes `faber.toml` `[dependencies]` + `faber.lock` for third-party
providers. Registry and several other verbs remain staged.

**Sequencing principle:** close the **local library** loop (A) and Norma as
platform-default package (B) before binary apps, `cista run`, meta packages, or
the public registry.

## Problem

- Current `faber` package compilation resolves built-in library interfaces from
  sibling Norma source via `FABER_LIBRARY_HOME` / local `faberlang/norma` layout.
- Generated Rust packages depend on sibling **`faber-runtime`** (crate name
  `faber`), not a Rust `norma` crate.
- That works for multi-repo local development, but it does not describe how an
  installed `faber` binary finds bundled Norma interfaces, target runtime
  implementations, target metadata, or future third-party packages.
- User projects should declare dependencies, but dependency files should not be
  copied under each project. Installing the same dependency for multiple
  projects should materialize one versioned package artifact in a shared
  location.
- Published or installed third-party packages should not require consumers to
  pull a full source repository. Consumers need Faber API/interface files,
  compatible target artifacts or compile policy, and binding metadata that
  connects those two surfaces.
- Some packages should be installable as source-distributed packages. Open
  source packages, simple Faber-only libraries, and target-portable libraries
  may ship source plus enough manifest policy for the consumer's local toolchain
  to compile the package.
- Live Norma is pure Faber source under `norma/src` (no `@ verte` / `@ subsidia`
  in public sources). Host I/O uses `ad` routes; language carriers live in
  **`faber-runtime`**, not a Rust `norma` crate. Packaging Norma must not invent
  a hand-written `libnorma.rlib` as the default story.
- The user-facing package concept is named `cista`. The low-level crate and CLI
  may also be named `cista`, but high-level project workflows still belong in
  the `faber` command.

## Current Invariant

`$CISTAE_HOME` is the shared cista package artifact store, defaulting to
`~/.faber/cistae`. Project directories contain manifests and lockfiles, not
installed dependency contents. Installed packages contain Faber interfaces plus
either compiled target artifacts or source compile policy. Toolchain-bundled
Norma uses the **same store package shape** as other packages. Product-wise
Norma is a **hard platform default** (always available; not listed in app
`faber.toml` `[dependencies]`), not a second store category.

## Repo Separation Invariant

`faber` and `cista` remain independently buildable sibling repositories. Neither
repo may add a Rust crate dependency on the other, directly or through a shared
workspace-only helper crate. Cross-repo integration must use stable file
formats, documented store paths, environment variables, command-line flags, exit
codes, and spawned processes.

`faber` is lower in the toolchain. It owns Faber project manifests, Faber's
build lockfile, package source loading, typechecking, code generation, and build
semantics. It must not know that `cista` exists: no `cista` crate dependency, no
`cista` process dependency, no `cista.lock`, and no cista-specific store
discovery in ordinary builds.

`cista` owns package-store installation, inspection, removal, cache/registry
operations, and installed artifact lifecycle. When `cista` needs Faber-language
validation or compilation, it invokes the `faber` executable. When `cista`
changes a project's installed dependency set, it updates the project-owned
`faber.lock` file that `faber` can consume without knowing which package manager
produced it.

## Package roles

Installable units share one store layout but differ by role:

| Role | Meaning | Primary consumer |
| ---- | ------- | ---------------- |
| **lib** | Interfaces + target artifact (or compile policy); linked/typechecked by builds | `faber check` / `faber build` |
| **bin** | Executable application entry in the store | `cista run <name>` |
| **meta** | Identity + dependency list only (e.g. suite install) | `cista install` expands deps |

- Real packages are **individual units** (e.g. `cat`, `mathesis`), not a forced
  monorepo blob.
- A suite such as coreutils is **one bin package per utility**, optional shared
  **lib** for helpers, optional **meta** that only lists those bins.
- Project packages use **`faber.toml`** today; store packages use **`cista.toml`**.
  Whether those merge is an open decision; meta-as-deps does not require merging
  first.

## Goals

- Keep `$CISTAE_HOME` (default `~/.faber/cistae`) as the shared versioned store.
- Keep installed layout for artifact-distributed and source-distributed packages.
- Keep Faber interfaces target-neutral; target bindings live in package
  metadata, not `@ verte` / per-target annotations in API files.
- **Phase A (shipped):** local library install + inspect + faber consumption of
  `faber.lock` for third-party deps (mathesis demo).
- **Phase B:** model **Norma** as a source package on the same store/`interfaces/`
  contract; hard platform default (not in app `faber.toml`); independent Norma
  versioning; package/codegen from locked interface roots; dev fallback via
  `FABER_LIBRARY_HOME` until packaged path is proven. See `phase-b-problem.md`.
- Extend install to **bin** packages and introduce **`cista run`** for installed
  apps (coreutils-shaped proof).
- Support **meta** packages as dependency sets only.
- Treat **`cista.dev`** as the later registry host; client fetch/publish is a
  late phase, not the first milestone.
- Keep cista free of radix/compiler linkage and free of a crate dependency on
  sibling `faber`; call `faber`/native tools as needed for compile steps.

## Non-goals (global)

- Do not copy installed dependencies into project directories.
- Do not require full source trees in every install (only source-distributed
  packages).
- Do not add target-specific annotations to Faber interface files.
- Do not keep sibling `norma/src` as the only permanent production discovery path;
  Norma becomes a package instance in the store (repo may keep `src/`; install
  maps into store `interfaces/`).
- Do not require apps to list `norma` in `faber.toml` `[dependencies]` (platform
  default). Do not merge Norma into `faber-runtime`.
- Do not add Wasm/Go/TS package targets before Rust **lib + bin** is solid.
- Do not put application packaging (e.g. static site / “web target”) into the
  compiler or into early cista phases.
- Do not treat `cista run` as “run a framework against a source tree”; it runs
  an **installed package entry** (bin/tool). Site/framework build CLIs are
  separate product tools that may be installed as bins.

### Deferred to later phases (not forever out of scope)

- Public registry protocol, auth, publish/yank (Phase F).
- Rich SemVer solving (start with direct exact deps + lockfile).
- Full transitive resolution beyond what each phase explicitly needs.
- PATH shims for installed bins (nice-to-have after `cista run`).

## Ground Truth Researched

- `../faber/src/library.rs` (sibling `faber`): library resolution returns `.fab`
  interface files under `$FABER_LIBRARY_HOME/norma/src` (local sibling layout).
- `../faber/src/package/` (sibling `faber`): package compilation uses
  library-home resolution; generated Rust crates depend on
  `faber = { package = "faber-runtime", path = ... }` (sibling `faber-runtime`).
- `../faber/src/library.rs`: the data model already distinguishes built-in
  library providers from future package-backed providers; `norma` is the first
  provider.
- `../norma/src/*.fab`: Norma interfaces are Faber source contracts parsed and
  typechecked with package code.
- There is **no** residual Rust `norma` crate. Runtime carriers live in sibling
  `faber-runtime` (`use faber::…`).
- User clarification: avoid project-local dependency trees. Multiple Faber
  projects should share installed dependency artifacts from one versioned store.
- User clarification: use `~/.faber/cistae` or `$CISTAE_HOME` as the shared
  installed package location.
- User clarification: installed dependencies should contain the Faber API or
  interface files plus compiled target artifacts or source compile policy.
- User clarification: the package manifest should have a `[source]` section for
  Faber-facing requirements, a `[target]` section for the selected target, and
  `[[bindings]]` entries with `source_module`, `source_symbol`, and `target`.
- User clarification: some packages should be publishable and installable as
  source. These packages should include source code plus manifest flags that let
  the package be compiled into the selected target language.
- User clarification: Norma is a concrete source-distributed package example:
  it has Faber interface files plus hand-written Rust implementation code, so it
  should use manifest bindings rather than generated binding policy.

## Reference Packet

Before lowering this goal into delivery, inspect:

- `src/manifest.rs`: current cista manifest schema.
- `src/commands/`: current `cista check` validation behavior.
- `../examples/cista-lab/source/mathesis/`: current source-distributed package
  fixture.
- `../faber/src/library.rs`: current provider resolver, built-in module
  discovery, and future package-provider placeholders.
- `../faber/src/package.rs`: package loading, generated Cargo layout,
  library provenance attachment, and generated `faber-runtime` dependency path.
- sibling radix `crates/radix/src/driver/session.rs`: `Config.stdlib_path` and how compiler
  sessions carry library configuration.
- `../norma/src/`: current target-neutral Faber interfaces.
- sibling radix `docs/factory/library-import-provenance/plan.md`: current
  provider-qualified import model and provenance invariants.
- sibling radix `docs/factory/intrinsics-innatum-residue/plan.md`: related native symbol
  mapping and `@ verte` cleanup pressure.
- sibling radix `docs/factory/target-support-matrix/goal.md`: target metadata precedent and
  validation posture.

## Constraints And Invariants

- `faber` remains the user-facing project/build/package tool unless a later
  explicit naming decision changes it.
- `radix` remains the compiler/developer CLI.
- `cista` names a Faber package unit and the low-level package crate/tool.
- Faber `.fab` interfaces for package APIs should remain target-neutral.
- Faber `.fab` files remain the source of truth for signatures, effects,
  cursor/generator contracts, return types, and other Faber-facing facts.
- Installed cista artifacts live in a shared store rooted at `$CISTAE_HOME`,
  defaulting to `~/.faber/cistae`.
- Faber project directories contain manifests and lockfiles describing
  dependency requirements and resolved pins, not installed dependency contents.
- A published or installed cista may be distributed in artifact mode or source
  mode. Artifact mode contains Faber-facing interface files, target-specific
  compiled artifacts, and binding metadata. Source mode contains Faber-facing
  interface files, included source code, and enough target policy to compile the
  package locally.
- The installed artifact manifest should describe source/API requirements,
  target artifact identity, and source-to-target bindings. It should not
  duplicate type facts the compiler can read from `.fab` interface files.
- `[[bindings]]` entries use structured `source_module` and `source_symbol`
  fields for the Faber API symbol and `target` for the target-language symbol.
  Compact symbol strings are reserved for diagnostics and internal display.
- Source-distributed packages with pure Faber implementations use generated
  binding policy. Live **Norma** is in this class. Packages with hand-written
  target code (e.g. lab mathesis) use manifest binding policy.
- Target-native runtime implementation knowledge belongs with package target
  metadata, not as per-target annotations in Faber interfaces.
- Norma is modeled as a store package instance (same layout concepts) and as a
  **platform default** (not an app `faber.toml` dependency).
- Installed binary behavior must not depend on source-repository-relative paths
  unless explicitly running in development mode.
- Development mode may continue using sibling `norma/src` via `FABER_LIBRARY_HOME`
  while the package-store model is introduced.
- Distribution design must account for multiple targets, including Rust, Go,
  TypeScript/JavaScript, and Wasm, even if the first implementation only proves
  Rust.
- `cista.dev` is the planned canonical package host for publishing and
  retrieval; registry protocol and hosting implementation remain deferred.
- `faber` and `cista` must not share Rust implementation types. Their shared
  contract is the documented TOML schema, store layout, environment variables,
  CLI behavior, and process exit semantics.
- Do not weaken current package provenance checks to make a design fit.

## Store Layout

`$CISTAE_HOME` points directly at the shared cista package artifact store:

```text
$CISTAE_HOME/
└── mathesis/
    └── 0.1.0/
        ├── interfaces/
        │   └── mathesis.fab
        └── targets/
            └── rust/
                └── aarch64-apple-darwin/
                    ├── cista.toml
                    └── libmathesis.rlib
```

The package/version root owns target-neutral **interfaces** (Faber `.fab` API
tree). Each target-specific directory owns a target-specific `cista.toml` and
optional compiled artifact or compile metadata.

**`interfaces/` is store vocabulary for every package.** At install, cista
always materializes the Faber API tree under
`$CISTAE_HOME/<package>/<version>/interfaces/`, copying from whatever path the
source package’s `cista.toml` declares (`interfaces = "interfaces"`,
`interfaces = "src"`, etc.). Source repos need not rename their trees.

Toolchain-bundled packages use the same package/version shape. The only
difference is their discovery root (cista/toolchain-owned; not ordinary faber
build discovery):

```text
<toolchain-root>/
├── bin/
│   ├── faber
│   └── radix
└── cistae/
    └── norma/
        └── <norma_version>/          # independent of faber crate version
            ├── interfaces/           # snapshot of norma/src (tree preserved)
            │   ├── solum.fab
            │   ├── solum/
            │   │   └── path.fab
            │   ├── json.fab
            │   └── …
            └── targets/              # optional / thin for Norma v1
                └── rust/
                    └── <triple>/
                        └── cista.toml
```

Norma Phase B does **not** require a prebuilt `libnorma.rlib`. Live Norma is
pure Faber (+ host `ad` routes); `faber-runtime` remains a separate always-linked
language runtime for generated crates.

### Store discovery (cista-owned)

Package-manager discovery order for store roots and installed packages. This
list is used by `cista` only. `faber` does not walk this list during normal
builds; it consumes absolute paths from `faber.lock`.

1. explicit CLI/config store path
2. `CISTAE_HOME`
3. default `~/.faber/cistae`
4. bundled package root relative to the installed Faber toolchain
5. development fallback for sibling `norma/src` via `FABER_LIBRARY_HOME`
   (package-manager / toolchain tooling only; not ordinary `faber build`)

## Source Package Fixture Layout

The canonical local source package fixture uses the same vocabulary as installed
packages:

```text
examples/cista-lab/
├── source/
│   └── mathesis/
│       ├── cista.toml
│       ├── interfaces/
│       │   └── mathesis.fab
│       └── targets/
│           └── rust/
│               ├── Cargo.toml
│               └── src/
│                   └── lib.rs
└── demo/
    ├── faber.toml
    ├── faber.lock
    └── src/
        └── main.fab
```

The demo project contains only its own manifest, lockfile, and source. It does
not contain copied dependency package files.

## Manifest Shapes

### Artifact-Distributed Package

```toml
[source]
package = "mathesis"
version = "0.1.0"
faber_min = "0.36.0"
kind = "artifact"
interfaces = "../../../interfaces"

[target]
language = "rust"
mode = "artifact"
binding_policy = "manifest"
triple = "aarch64-apple-darwin"
artifact = "libmathesis.rlib"
crate = "mathesis"
rustc = "1.88.0"

[target.flags]
edition = "2021"

[[bindings]]
source_module = "mathesis"
source_symbol = "quadratum"
target = "mathesis::mathesis::quadratum"
```

- `[source]` describes the Faber-facing contract: package identity, interface
  location, minimum Faber version, and optional feature requirements.
- `[target]` describes the selected target artifact: target language, target
  triple or platform identity, artifact path, and language/toolchain flags.
- `[[bindings]]` maps one Faber API symbol to one target-language symbol using
  structured source identity fields plus a target-language symbol string.
- Async, cursor/generator, return type, and parameter facts should be validated
  from referenced `.fab` files, not copied into the manifest unless a future
  adapter field is needed to explain a native mismatch.

### Source-Distributed Faber Package

```toml
[source]
package = "mathesis"
version = "0.1.0"
faber_min = "0.36.0"
kind = "source"
interfaces = "interfaces"
sources = "src"

[target]
language = "rust"
mode = "compile"
binding_policy = "generated"
crate = "mathesis"

[target.compile]
emit = "library"
crate_type = "rlib"
edition = "2021"
```

This form is for packages whose implementation is Faber source and whose target
symbols are produced by Faber codegen. No `[[bindings]]` rows are required
unless the package needs hand-written target implementation hooks.

### Norma Source Package (live model — Phase B)

Live Norma is pure Faber under `norma/src` (multi-module tree). Public sources
do not use `@ verte` / `@ subsidia`. Host I/O is `ad` routes; language carriers
are **`faber-runtime`**, not a Norma Rust crate.

```toml
[source]
package = "norma"
version = "0.1.0"          # owned by Norma; independent of faber crate version
faber_min = "0.38.0"       # minimum Faber tool that can consume this package
kind = "source"
interfaces = "src"         # source-repo path; install maps → store interfaces/

[target]
language = "rust"
mode = "compile"
binding_policy = "generated"   # Faber bodies; not manifest bindings
# No [[bindings]] required for default Norma.
# No libnorma.rlib required for Phase B exit.
```

**Platform default:** apps do **not** list `norma` in `faber.toml`
`[dependencies]`. Norma is always available. Provisioning (toolchain install,
`cista install` of Norma, or dev sibling layout) supplies absolute interface
paths faber can use without discovering `$CISTAE_HOME`.

**Manifest bindings** remain the right model for packages with hand-written
target code (e.g. lab mathesis), not for default Norma.

See `phase-b-problem.md` for the full problem lock.

## Resolver Output

For Phase A, `faber` owns the build-time resolved package record internally.
`cista` owns its own installed-package/store view internally. The two repos must
not share Rust types; both implementations conform to the same documented
file/process contract.

The build resolver inside `faber` should produce one concrete resolved package
record for each provider-qualified import:

```text
importa ex "mathesis:mathesis"
  -> package: mathesis
  -> version: 0.1.0
  -> interface path: /absolute/path/to/mathesis/0.1.0/interfaces/mathesis.fab
  -> target language: rust
  -> target triple: aarch64-apple-darwin
  -> target mode: artifact | compile
  -> artifact path or compile recipe
  -> binding table or generated binding policy
```

The first implementation should define this internal `faber` output shape before
growing package install behavior further. The cross-repo contract is still the
on-disk store layout, `cista.toml`, `faber.toml`, and `faber.lock`, not a shared
library type.

## Project Dependency Intent

Phase A locks project dependency intent in `faber.toml`; `faber.lock` records
the resolved build inputs. `faber.lock` is a Faber build lockfile, not a cista
lockfile. The minimal project manifest syntax is:

```toml
[dependencies]
mathesis = "0.1.0"
```

Phase A dependency rules (third-party):

- Dependency keys are package/provider names used by provider-qualified imports
  such as `mathesis:mathesis`.
- Versions are exact strings only; no SemVer ranges.
- Dependencies are direct only; no transitive solving.
- No registry source syntax and no path dependencies in `faber.toml`.
- Packages enter the shared store through `cista install --path ...`.
- `faber check` / `faber build` validate that third-party provider-qualified
  imports are declared in `faber.toml`, pinned in `faber.lock`, and backed by
  explicit interface/artifact paths from the lock.
- `faber` must not discover `$CISTAE_HOME`, call `cista`, or interpret
  cista-specific store roots during normal builds.

**Norma (Phase B):** not a third-party dep line. Provider `norma` is a hard
platform default — always available without `faber.toml` `[dependencies]`.
Packaged mode still uses absolute interface paths (implicit lock / toolchain
default provision / equivalent), never faber-side store discovery. Dev mode may
use `FABER_LIBRARY_HOME` / sibling `norma/src` until packaged provision is
proven.

## Lockfile Role

Even before full dependency solving exists, a project needs resolved records so
builds are not based only on mutable global state in `$CISTAE_HOME`.

Minimal first-pass `faber.lock` shape:

```toml
[[package]]
name = "mathesis"
version = "0.1.0"
source = "path:/Users/ianzepp/work/faberlang/examples/cista-lab/source/mathesis"
package_root = "/Users/ianzepp/.faber/cistae/mathesis/0.1.0"
kind = "source"
target_language = "rust"
target_triple = "aarch64-apple-darwin"
target_manifest = "/Users/ianzepp/.faber/cistae/mathesis/0.1.0/targets/rust/aarch64-apple-darwin/cista.toml"
interface_root = "/Users/ianzepp/.faber/cistae/mathesis/0.1.0/interfaces"
artifact = "/Users/ianzepp/.faber/cistae/mathesis/0.1.0/targets/rust/aarch64-apple-darwin/libmathesis.rlib"
crate = "mathesis"
rustc = "1.88.0"
```

Phase A `faber.lock` paths are explicit file-system paths so `faber` does not
need to know the package-manager store root or environment. The package manager
may rewrite the lock when a package is installed, moved, or re-resolved. The
first pass can require exact versions and direct dependencies only.

## Target Identity

`--target rust` is not enough to validate compiled artifacts. The first pass
should record at least:

- target language, such as `rust`
- platform triple, such as `aarch64-apple-darwin`
- Faber compiler version
- target compiler identity, such as Rust compiler version when relevant

Rust `.rlib` artifacts are especially toolchain-sensitive. Until the
compatibility story is proven, source-distributed `mode = "compile"` packages
are safer than prebuilt Rust artifacts.

## Build Cache

Source-distributed packages need a local compiled-output cache that is separate
from both the installed source package and the consuming project.

Candidate locations:

```text
$CISTAE_HOME/        installed package inputs
~/.faber/build/      compiled package outputs/cache
project/target/      final app build outputs
```

The first pass should avoid writing compiled dependency artifacts into the
consuming project.

## Package Validation

`cista check <package> --target-language rust` should exist before or alongside
install. Target implementation build verification should remain explicit via
`--verify-target-build` because it shells out to native target tooling.

The first validation surface only needs to prove:

- `[source]` exists and has package identity, version, and Faber requirement.
- interface directory exists.
- target mode is valid.
- source directory or artifact path exists for the selected target policy.
- every `[[bindings]].source_module` exists in the interface files.
- every `[[bindings]].source_symbol` exists in that module.
- manifest binding policy supplies bindings where required.

## Binding Source Identity

Bindings in user-authored manifests should avoid dense encoded symbol strings.
Use structured fields instead:

```toml
[[bindings]]
source_module = "hal/tempus"
source_symbol = "MILLISECUNDUM"
target = "norma::hal::tempus::millisecundum"
```

Internally, the resolver may still render canonical fully qualified identities
for diagnostics and lockfile display:

```text
norma:hal/tempus#MILLISECUNDUM
```

This keeps package identity, module path, and exported symbol distinct without
forcing users to write compact separator-heavy strings in TOML.

## Path Install Semantics

For the first pass, `cista install --path` should snapshot-copy the package into
the store. Live links to path dependencies create confusing rebuild semantics
and should be deferred until the base model is stable.

`cista install --path examples/cista-lab/source/mathesis --target-language rust`
should build the selected target artifact and install only the consumable
package artifact into `$CISTAE_HOME`.

### Project root and `faber.lock` rewrite (Phase A)

When a project root is known, install also rewrites that project's `faber.lock`
with absolute paths to the installed package. Phase A project-root rule:

- Prefer `--project <dir>` pointing at a directory that contains `faber.toml`.
- If `--project` is omitted, use the current working directory when it contains
  `faber.toml`.
- If no project root is found, install only materializes the store entry and
  does not create or rewrite a lockfile.
- Lock rewrite is only valid when the package name and exact version are
  declared in the project's `faber.toml` `[dependencies]`.
- Demo flow: from `examples/cista-lab/demo` (or with
  `--project examples/cista-lab/demo`), install mathesis so the demo
  `faber.lock` receives locked interface/artifact paths.

## Transitive Dependencies

The first pass can defer full transitive dependency resolution. It should state
the restriction explicitly:

- project dependencies must be direct and exact
- installed source packages may depend only on bundled Norma and package-local
  code unless a later delivery phase adds dependency graph resolution

## Binding Policy Behavior

The manifest policy names imply different compiler behavior:

- `binding_policy = "generated"`: Faber compiles source and owns the target
  symbol names.
- `binding_policy = "manifest"`: the compiler must call or link target symbols
  listed in `[[bindings]]`.

The first pass should implement one policy completely before treating both as
equally supported. The likely order is manifest bindings for the current
Mathesis fixture, then generated bindings for a pure Faber source package.

## Required Diagnostics

Diagnostics are tool-owned. Store-env messaging belongs to `cista`; build-time
lock and path messaging belongs to `faber`.

### cista (store / install / inspect)

- package not installed
- requested version not installed
- interface file missing from the package or store layout
- selected target missing under `targets/<language>/…`
- binding source module or source symbol not found (during `check` / install
  validation)
- binding target symbol unavailable or unchecked when verification is requested
- source package requires local compile but the target compiler is unavailable
- package store not found, including searched paths and how to set
  `CISTAE_HOME`
- project root missing `faber.toml` when `--project` or cwd lock rewrite was
  requested
- package not declared (or wrong exact version) in the project's
  `[dependencies]` when a lock rewrite was requested

### faber (build-time lock consumption)

- provider-qualified import not declared in `faber.toml` `[dependencies]`
- dependency declared in `faber.toml` but missing from `faber.lock`
- locked interface path missing or unreadable
- locked artifact path missing or unreadable
- locked target manifest path missing, unreadable, or invalid against the
  documented target-manifest schema
- compiled artifact incompatible with the active target/toolchain when faber
  validates lock facts against the active build

## Implementation Shape

Phased plan. Prefer **A → B → C → D**, then **E** / **F**. Do not start
registry or bin work until the local library consumer loop works.

Legacy labels Phase 1–2 from earlier drafts map to **partially shipped** store
contract + library `install --path`; remaining work is reorganized below as
Phases A–F.

### Phase A — Finish local library store (close the loop)

**Status intent:** shipped (factory Phase A).

- Keep `faber` and `cista` repo-separated: no Rust dependency in either
  direction; process spawning and file contracts only.
- Keep `$CISTAE_HOME` / `~/.faber/cistae`, `interfaces/`, `targets/<language>/`.
- Flesh **inspect / list / remove** against the store (`package show`,
  `package list`, `package files`, `remove` as needed).
- Finish **demo consumer** (`examples/cista-lab/demo`): install `mathesis` with
  project root known (`--project` or cwd with `faber.toml`), rewrite
  `faber.lock`, then `faber check` / `faber build` consumes the locked interface
  and Rust artifact paths (no vendored copy in the project).
- Minimum project dependency intent in `faber.toml` (`[dependencies] mathesis =
  "0.1.0"`).
- Minimum **`faber.lock`** so the demo pin is deterministic and not based only
  on mutable global store state.
- Project-root / lock rewrite rule: `--project <dir>` preferred; else cwd when
  it contains `faber.toml`; store-only install when no project root; lock
  rewrite requires an exact `[dependencies]` match.
- `faber` owns build-time dependency consumption by reading `faber.toml` and
  `faber.lock` only; it must not know about `cista`, `CISTAE_HOME`, or
  package-manager store discovery.
- `cista install --path` may spawn `faber` and native tools for validation/build
  steps, but must not link against `faber` or `radix`.
- `cista` store discovery order remains as under **Store discovery
  (cista-owned)** above.

**Exit:** on a clean layout, `cista install --path …/mathesis` with demo project
root updates `examples/cista-lab/demo/faber.lock`, then demo `faber build`
succeeds using only locked interface/artifact paths (+ documented fallback if
any).

### Phase B — Norma as platform-default package

**Status intent:** next implementation focus (problem locked in
`phase-b-problem.md`).

- Add Norma package identity: `cista.toml` with independent **Norma version**,
  `kind = "source"`, `interfaces = "src"`, `binding_policy = "generated"`.
- Keep `norma/src` in the Norma repo; install maps into store
  `…/norma/<norma_ver>/interfaces/` (same universal install mapping as every
  package — not Norma-only).
- Preserve multi-module tree (`solum/path`, `json/solve`, …).
- **Do not** require `libnorma.rlib` or `[[bindings]]` for default Norma.
- **Do not** list `norma` in app `faber.toml` `[dependencies]` (hard platform
  default). Third-party deps stay explicit (Phase A).
- **Do not** merge Norma into `faber-runtime`; runtime remains the always-linked
  language carrier crate for generated Rust.
- Same store layout concepts as third-party packages; bundled vs user-store
  instances differ only by root (cista/toolchain discovery).
- Provision packaged Norma paths for faber without teaching faber about cista:
  implicit lock injection, toolchain default path records, or equivalent
  (delivery chooses mechanism — Q6 in `phase-b-problem.md`).
- Preserve **dev fallback** (`FABER_LIBRARY_HOME` / sibling `norma`) until the
  packaged path is proven; `cista` fails closed with searched-path diagnostics
  when store lookup is required and fails.
- `faber` still does not discover `$CISTAE_HOME` or call `cista`.

**Exit:** a Faber package typechecks/builds against `norma:*` using packaged
interface paths (not only sibling `FABER_LIBRARY_HOME`), **without** declaring
`norma` in `faber.toml`, while third-party packages continue to use explicit
deps + lock. Dev sibling fallback still works when packaged Norma is absent.

### Phase C — Binary packages (coreutils-shaped)

**Closed 2026-07-10:** manifests carry `source.role`; `role = "bin"` builds
the named Cargo binary and installs it under the host target directory. The
`examples/coreutils/packages/true` proof installs and executes without its
source path. See `phase-c-delivery.md`.

- Package role **lib vs bin** in install metadata (manifest field or equivalent).
- `cista install --path` builds and installs an **executable** into the store
  (not only `lib*.rlib`).
- Shared helpers become a real **lib** package (e.g. coreutils `common/gnu`),
  not monorepo-only relative imports, for any bin that should be installable.
- Proof: one utility end-to-end (`true` or `cat` under
  `examples/coreutils/packages/…`).

**Exit:** `cista install --path …/packages/true` (or `cat`) materializes a
runnable store entry for the host triple.

### Phase D — `cista run`

**Closed 2026-07-10:** `cista run name[@version] -- …` resolves the installed
Rust artifact for the current host, requires a `bin` role, validates the
artifact path and presence, and passes arguments through. The proof runs after
the source package has been deleted. See `phase-d-delivery.md`.

- Resolve installed package name[`@version`] → executable entry; verify host
  triple / presence.
- Arg passthrough: `cista run cat -- file.txt`.
- Fail clearly if the package is a **lib** (or meta without a default bin).

**Exit:** after install, `cista run true` / `cista run cat -- …` works without
the source tree on disk.

### Phase E — Meta packages (optional, small)

**Closed 2026-07-10:** minimal meta manifests declare identity plus exact local
dependency pins. Install validates the complete direct set before writes,
installs each normal package through the existing pipeline, and snapshots the
meta identity/pins without stale source paths. See `phase-e-delivery.md`.

- Meta package = **identity + dependency list** (and pins as needed).
- `cista install` of a meta package installs the dependency set.
- Example shape: `coreutils` meta → individual bin packages; units of truth
  remain per-utility packages.

**Exit:** local meta install expands to the expected set of store entries.

### Phase F — Registry client (cista.dev)

**Local/dev loop closed 2026-07-10:** an explicit filesystem registry supports
immutable publish, exact fetch into the store-owned cache, and install by
`name@version` without `--path`. Live cista.dev HTTP, authentication, and yank
remain residual work rather than a fabricated protocol. See
`phase-f-delivery.md`.

- `fetch` / `install <name>@ver` from remote; then `publish` / auth as needed.
- Cista remains the **client**; hosting protocol for cista.dev can evolve in
  parallel docs.
- Still no requirement to solve every registry edge case in the first publish
  slice.

**Exit:** install a published lib or bin without `--path`.

### Phase G — cista.dev HTTP/auth transport

**Active 2026-07-10:** establish the remote client transport independently of
the filesystem registry. The first slice provides an HTTP(S) client contract,
optional bearer authentication, strict success-status handling, and local
loopback contract tests. Credentials fail closed over plain HTTP. Live
`cista.dev` validation remains environment-gated and is not claimed here.

**Authenticated round-trip slice closed 2026-07-10:** exact package archive
PUT/GET requests now share the HTTPS transport and canonical package identity
validation. A hermetic registry proves bearer authorization, immutable publish,
fetch, unauthorized rejection, and path-escape rejection without weakening the
plain-HTTP credential prohibition. This is client-contract evidence, not a live
cista.dev product run.

**Credential CLI slice closed 2026-07-10:** `cista login` reads a bearer token
from an explicitly named environment variable and stores it by bare HTTPS
origin; `cista logout` removes only that origin. The owner-only TOML file is
written atomically, rejects loose Unix permissions, and never accepts token
values on the command line. Hermetic tests cover replacement, origin isolation,
removal, URL rejection, and file mode. Remote fetch/publish CLI routing still
awaits the fixed archive/server contract; no live cista.dev result is claimed.

- Remote API paths are origin-relative and must begin with exactly one `/`.
- Authenticated requests use `Authorization: Bearer <token>` only over HTTPS.
- Transport errors and non-success responses are terminal; remote operations
  must not silently fall back to the local filesystem registry.
- Remote fetch/publish CLI routing remains after the server contract is fixed.

**Exit:** a CLI fetch/publish operation can use the authenticated remote
transport against the fixed cista.dev API without weakening local/dev behavior.

### Explicitly not in A–D

- Static site / web application packaging.
- Compiler `Target::Web` or framework-as-compiler-target.
- Multi-language package artifacts beyond Rust until Rust lib+bin is solid.

## Exit Strategy

- Preserve development fallback to sibling `norma/src` via `FABER_LIBRARY_HOME`
  until Phase B is solid.
- `cista` package-store discovery failures must list searched paths and how to
  set `CISTAE_HOME`.
- Missing target binding metadata must fail package compilation with a clear
  provider/target diagnostic — never silent wrong calls.
- Store/resolver routing must remain revertible without changing Faber language
  grammar.
- `cista` must not gain a radix or `faber` crate dependency; compile steps shell
  out or call public tools (`faber`, `cargo`, etc.) as needed.

## Acceptance Criteria

### Model (doc + manifests)

- Cista is the package-store concept; `faber` remains project source workflow.
- `faber` and `cista` remain separate repositories with no crate dependency in
  either direction.
- `$CISTAE_HOME` is the shared store; layouts use `interfaces/` and `targets/`.
- `faber.toml` owns project dependency intent; `faber.lock` owns resolved build
  inputs.
- `faber` does not know about `cista`, `CISTAE_HOME`, or cista-specific store
  discovery during normal builds.
- Interface contracts stay separate from target-native binding metadata.
- Manifest shape remains `[source]`, `[target]`, `[[bindings]]` (extend carefully
  for bin/meta roles rather than inventing a second store).
- Source vs artifact distribution and generated vs manifest binding policies
  remain defined.
- Package roles **lib / bin / meta** are defined; meta is deps-only.
- Norma is a **store package instance** (same `interfaces/` layout) and a
  **platform default** (not an app dependency line; version independent of faber).
- `faber-runtime` is not the Norma package.
- `cista run` is defined as executing an **installed bin entry**, not building
  arbitrary source trees.

### Delivery gates (by phase)

- **A (shipped):** inspect/remove + mathesis install + `faber.toml` dependency
  intent + `faber.lock` pin + faber demo build from locked paths.
- **B:** Norma package installable to store; `norma:*` builds from packaged
  interface paths without `norma` in `faber.toml`; dev fallback still works;
  no faber/`CISTAE_HOME` coupling.
- **C:** at least one coreutils-style bin installable to the store.
- **D:** `cista run` works for that installed bin with arg passthrough.
- **E:** meta install expands dependencies (when implemented).
- **F:** remote install without `--path` (when implemented).

## Validation

- `rg -n "default_stdlib_root|norma_runtime_path|stdlib_path|LibraryResolver" ../faber/src crates/radix/src`
  should still be reviewed before implementation to verify current path
  assumptions.
- A future delivery spec should include focused `cista` tests for explicit store
  resolution, `$CISTAE_HOME` resolution, and missing store diagnostics, plus
  focused `faber` tests proving builds consume `faber.lock` paths without
  knowing about `cista` or `$CISTAE_HOME`.
- A future delivery spec should include a fixture proving that a package/native
  runtime mapping can resolve a Faber symbol without `@ verte` in the Faber
  interface.
- A future delivery spec should include a fixture proving that `cista install
  --path` installs interface files, compiled target artifacts, and binding
  manifests into `$CISTAE_HOME` without copying package files into the consuming
  project.
- A future delivery spec should include a fixture proving that a source package
  installed in `$CISTAE_HOME` can be compiled locally for the active target and
  cached outside the consuming project.
- A Phase B delivery spec should prove: `cista install` of Norma materializes
  `…/norma/<ver>/interfaces/` tree from `norma/src`; a consumer without
  `norma` in `faber.toml` builds `norma:*` from packaged interface paths with
  library-home unused for provider `norma`; sibling fallback still works when
  packaged Norma is absent.
- Review check: the accepted model should not require editing
  `../norma/src/*.fab` to add a new target runtime such as Wasm.
- Review check: default Norma does not require `libnorma.rlib` or reintroducing
  `@ verte` / `@ subsidia` into public Norma sources.

## Open Questions

### Explicitly decided for Phase A

- Project dependency intent lives in `faber.toml`; resolved build inputs live in
  `faber.lock`.
- Historical `requirit.toml` is not part of Phase A.
- `faber` and `cista` do not share Rust types or crate dependencies. Each repo
  owns internal representations that conform to the documented file/process
  contract.
- `CISTAE_HOME` is the Phase A package-manager store environment variable;
  `faber` does not read it during normal builds.
- Store discovery order is **cista-owned**; `faber` does not walk it.
- Target-specific manifests sit beside artifacts at
  `targets/<language>/<triple>/cista.toml`.
- The minimal `faber.lock` record includes package, version, source, package
  root, kind, target language, target triple, target manifest, interface root,
  artifact, crate, and rustc version.
- Phase A Rust artifact compatibility records language, triple, rustc version,
  artifact path, crate name, and manifest flags already present in `cista.toml`.
- Project root for lock rewrite: `--project <dir>` preferred; else cwd with
  `faber.toml`; store-only install when no project root; rewrite requires exact
  `[dependencies]` match.
- Diagnostics: store/env failures are `cista`; lock/path/import failures during
  build are `faber`.

### Explicitly decided for Phase B

- Norma is a **hard platform default**: not listed in app `faber.toml`
  `[dependencies]`; always available for `norma:*`.
- Norma is **versioned independently** of the faber crate; store path uses
  Norma’s own version.
- Live Norma is pure Faber + `ad` routes + separate **`faber-runtime`**; default
  policy is `binding_policy = "generated"`, not manifest bindings / `libnorma`.
- Store `interfaces/` is **universal**; install maps `cista.toml`’s
  `interfaces = "…"` path (e.g. Norma `src`) into `…/<pkg>/<ver>/interfaces/`.
- Keep `norma/src` in-repo; map at install (no forced restructure).
- Phase B does not require shared Norma rlib caching (codegen from interfaces).
- Full problem lock: `phase-b-problem.md`.

### Deferred

- Bootstrap/install UX for injecting the platform-default Norma lock record in
  installed toolchains. Decision is cista/bootstrap lock injection; remaining
  work is UX and real-toolchain proof, not faber-side `$CISTAE_HOME` discovery.
- Initial Norma version number string (delivery Q7).
- Do store packages keep a separate `cista.toml` forever, or does `faber.toml`
  gain installable package fields for published units? (Later packaging design.)
- How is **bin** role spelled in the manifest (field on `[source]`, `[target]`,
  or install metadata)? (Phase C.)
- Default executable entry name for bins (`package` name vs explicit `bin =`)?
  (Phase C/D.)
- Source cache vs artifact-only install for source-distributed packages beyond
  the Phase A rule that compiled dependency outputs do not live in the consuming
  project.
- How much transitive resolution is required after the Phase A direct/exact
  dependency restriction?
- Whether a broader `FABER_HOME` eventually owns `cistae/` instead of
  `CISTAE_HOME`.
- Package-version root target indexes beyond the Phase A target-specific
  manifest location.
- Additional Rust artifact compatibility facts beyond the Phase A set.
- PATH shims for installed bins: in scope after Phase D, or always optional?
- What URL and API shape should `cista.dev` expose? (Phase F.)

## Stop Conditions

- Stop if implementation skips Phase A (`faber` consuming locked package
  records) and jumps to registry or bin-only demos that still need sibling-repo
  hacks.
- Stop if either `faber` or `cista` adds a Rust crate dependency on the other,
  or on a shared workspace-only helper crate, to satisfy the package-store
  integration.
- Stop if the design requires target-specific annotations in Faber interface
  files for every supported backend.
- Stop if the package model only works for Norma and cannot describe a future
  non-Norma provider.
- Stop if Norma packaging invents a permanent special store category or forces
  apps to declare `norma` in `faber.toml` as if it were third-party.
- Stop if Norma is merged into `faber-runtime` or a fake default `libnorma.rlib`
  becomes the only supported Norma story.
- Stop if installed **bin** behavior still depends exclusively on repository
  layout (relative monorepo imports as the only way to share helpers).
- Stop if adding Wasm or another target would require editing existing
  target-neutral Faber interfaces.
- Stop if `cista run` is redefined as “compile and run local source” (that is
  `faber run`).
- Stop if cista gains a radix/compiler crate dependency.
