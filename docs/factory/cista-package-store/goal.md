# Goal: Cista Package Store Model

**Status**: active — phased implementation plan (library store → Norma → bins → run → meta → registry)
**Created**: 2026-06-21
**Updated**: 2026-07-08
**Target Repo**: `/Users/ianzepp/work/faberlang/cista`
**Factory Artifact Dir**: `docs/factory/cista-package-store/`
**Note**: Implementation lives in public sibling `faberlang/cista` (no radix dep).
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
| `cista` | Package **store**: check, install, inspect, remove; later run installed apps, fetch/publish |
| `radix` | Compiler (not a cista dependency) |

**Direction:** artifact-first but not artifact-only. An installed package must
include Faber interfaces for typechecking, plus either compiled target
artifacts or source plus compile policy. Project directories hold manifests,
lockfiles, and project source only.

**Shipped today (partial):** `cista check` and `cista install --path` for Rust
**library** packages (mathesis-style). Most other CLI verbs are staged stubs.

**Sequencing principle:** close the **local library** loop and Norma packaging
before binary apps, `cista run`, meta packages, or the public registry.

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
- Target-native implementation knowledge is beginning to leak into Faber
  interface files through annotations such as `@ verte rs "..."`.
- The user-facing package concept is named `cista`. The low-level crate and CLI
  may also be named `cista`, but high-level project workflows still belong in
  the `faber` command.

## Current Invariant

`$CISTAE_HOME` is the shared cista package artifact store, defaulting to
`~/.faber/cistae`. Project directories contain manifests and lockfiles, not
installed dependency contents. Installed packages contain Faber interfaces plus
either compiled target artifacts or source compile policy. Toolchain-bundled
Norma is a bundled cista package source using the same concepts as other
packages, not a separate package category.

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
- Finish local **library** install + inspect + **faber** resolution of store
  packages (demo consumer).
- Model **Norma** as a bundled/source package on the same contract as third-party
  libs (dev fallback via `FABER_LIBRARY_HOME` until that lands).
- Extend install to **bin** packages and introduce **`cista run`** for installed
  apps (coreutils-shaped proof).
- Support **meta** packages as dependency sets only.
- Treat **`cista.dev`** as the later registry host; client fetch/publish is a
  late phase, not the first milestone.
- Keep cista free of radix/compiler linkage; call `faber`/native tools as needed
  for compile steps.

## Non-goals (global)

- Do not copy installed dependencies into project directories.
- Do not require full source trees in every install (only source-distributed
  packages).
- Do not add target-specific annotations to Faber interface files.
- Do not relocate sibling `norma/src` as a special permanent category; Norma
  becomes a normal package instance.
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
- `src/commands.rs`: current `cista check` validation behavior.
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
- Source-distributed packages with pure Faber implementations may use generated
  binding policy. Source-distributed packages with hand-written target code,
  including Norma, should use manifest binding policy.
- Target-native runtime implementation knowledge belongs with package target
  metadata, not as per-target annotations in Faber interfaces.
- Norma should be modeled as the bundled standard package using the same
  manifest concepts future packages can use.
- Installed binary behavior must not depend on source-repository-relative paths
  unless explicitly running in development mode.
- Development mode may continue using sibling `norma/src` via `FABER_LIBRARY_HOME`
  while the package-store model is introduced.
- Distribution design must account for multiple targets, including Rust, Go,
  TypeScript/JavaScript, and Wasm, even if the first implementation only proves
  Rust.
- `cista.dev` is the planned canonical package host for publishing and
  retrieval; registry protocol and hosting implementation remain deferred.
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

The package/version root owns target-neutral interfaces. Each target-specific
directory owns a target-specific `cista.toml` and the compiled artifact or
compile metadata for that target.

Toolchain-bundled packages use the same package/version shape. The only
difference is their discovery root:

```text
<toolchain-root>/
├── bin/
│   ├── faber
│   └── radix
└── cistae/
    └── norma/
        └── 0.36.0/
            ├── interfaces/
            │   ├── json.fab
            │   └── hal/
            │       └── tempus.fab
            └── targets/
                └── rust/
                    └── aarch64-apple-darwin/
                        ├── cista.toml
                        └── libnorma.rlib
```

Discovery order for package artifacts:

1. explicit CLI/config store path
2. `CISTAE_HOME`
3. default `~/.faber/cistae`
4. bundled package root relative to the installed Faber toolchain
5. development fallback for sibling `norma/src` via `FABER_LIBRARY_HOME`

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
    ├── cista.lock
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

### Norma-Style Source-Distributed Package

```toml
[source]
package = "norma"
version = "0.36.0"
faber_min = "0.36.0"
kind = "source"
interfaces = "interfaces"

[target]
language = "rust"
mode = "compile"
binding_policy = "manifest"
source = "targets/rust"
crate = "norma"

[target.compile]
emit = "library"
crate_type = "rlib"
edition = "2021"

[[bindings]]
source_module = "hal/tempus"
source_symbol = "MILLISECUNDUM"
target = "norma::hal::tempus::millisecundum"

[[bindings]]
source_module = "hal/tempus"
source_symbol = "nuncNano"
target = "norma::hal::tempus::nunc_nano"

[[bindings]]
source_module = "json"
source_symbol = "parse"
target = "norma::json::parse"
```

Norma is the larger canonical source-distributed example because it has
target-neutral Faber interfaces and hand-written Rust runtime code. Current
`@ subsidia` annotations in Norma interfaces should become target source layout
in the manifest. Current `@ verte rs "..."` method overrides should become
`[[bindings]]` rows in the manifest.

## Resolver Output

The build resolver should produce one concrete resolved package record for each
provider-qualified import:

```text
importa ex "mathesis:mathesis"
  -> package: mathesis
  -> version: 0.1.0
  -> interface path: $CISTAE_HOME/mathesis/0.1.0/interfaces/mathesis.fab
  -> target language: rust
  -> target triple: aarch64-apple-darwin
  -> target mode: artifact | compile
  -> artifact path or compile recipe
  -> binding table or generated binding policy
```

The first implementation should define this output type before growing package
install behavior. Without it, install, lockfile, codegen, and diagnostics have
no shared contract.

## Lockfile Role

Even before full dependency solving exists, a project needs resolved records so
builds are not based only on mutable global state in `$CISTAE_HOME`.

Minimal first-pass `cista.lock` shape:

```toml
[[package]]
name = "mathesis"
version = "0.1.0"
source = "path:/Users/ianzepp/work/faberlang/radix/examples/cista-lab/source/mathesis"
store = "mathesis/0.1.0"
kind = "source"
target_language = "rust"
target_triple = "aarch64-apple-darwin"
```

The first pass can require exact versions and direct dependencies only.

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

The first pass should produce clear diagnostics for:

- package not installed
- requested version not installed
- interface file missing
- selected target missing
- compiled artifact incompatible with the active target/toolchain
- binding source module or source symbol not found
- binding target symbol unavailable or unchecked
- source package requires local compile but the target compiler is unavailable
- package store not found, including searched paths and how to set
  `CISTAE_HOME`

## Implementation Shape

Phased plan. Prefer **A → B → C → D**, then **E** / **F**. Do not start
registry or bin work until the local library consumer loop works.

Legacy labels Phase 1–2 from earlier drafts map to **partially shipped** store
contract + library `install --path`; remaining work is reorganized below as
Phases A–F.

### Phase A — Finish local library store (close the loop)

**Status intent:** next implementation focus.

- Keep `$CISTAE_HOME` / `~/.faber/cistae`, `interfaces/`, `targets/<language>/`.
- Flesh **inspect / list / remove** against the store (`package show`,
  `package list`, `package files`, `remove` as needed).
- Finish **demo consumer** (`examples/cista-lab`): install `mathesis`, then
  `faber check` / `faber build` resolves interfaces + Rust artifact from the
  store (no vendored copy in the project).
- Minimum **lockfile** so the demo pin is deterministic.
- Discovery order remains: explicit path → `CISTAE_HOME` → default home →
  toolchain-bundled root → dev sibling fallback.

**Exit:** on a clean layout, `cista install --path …/mathesis` then demo
`faber build` succeeds using only store-backed package data (+ documented
fallback if any).

### Phase B — Norma as first real stdlib package

- Norma package manifest (`mode = "compile"`, `binding_policy = "manifest"` as
  appropriate).
- Same resolver contract as third-party libs; bundled vs user-store instances
  share layout concepts.
- Move target binding facts out of Faber interfaces into package metadata where
  still leaking.
- Preserve **dev fallback** (`FABER_LIBRARY_HOME` / sibling `norma`) until store
  resolution is proven; fail closed with searched-path diagnostics when store
  lookup is required and fails.

**Exit:** a Faber package typechecks/links against Norma via cista resolution,
not only a sibling checkout.

### Phase C — Binary packages (coreutils-shaped)

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

- Resolve installed package name[`@version`] → executable entry; verify host
  triple / presence.
- Arg passthrough: `cista run cat -- file.txt`.
- Fail clearly if the package is a **lib** (or meta without a default bin).

**Exit:** after install, `cista run true` / `cista run cat -- …` works without
the source tree on disk.

### Phase E — Meta packages (optional, small)

- Meta package = **identity + dependency list** (and pins as needed).
- `cista install` of a meta package installs the dependency set.
- Example shape: `coreutils` meta → individual bin packages; units of truth
  remain per-utility packages.

**Exit:** local meta install expands to the expected set of store entries.

### Phase F — Registry client (cista.dev)

- `fetch` / `install <name>@ver` from remote; then `publish` / auth as needed.
- Cista remains the **client**; hosting protocol for cista.dev can evolve in
  parallel docs.
- Still no requirement to solve every registry edge case in the first publish
  slice.

**Exit:** install a published lib or bin without `--path`.

### Explicitly not in A–D

- Static site / web application packaging.
- Compiler `Target::Web` or framework-as-compiler-target.
- Multi-language package artifacts beyond Rust until Rust lib+bin is solid.

## Exit Strategy

- Preserve development fallback to sibling `norma/src` via `FABER_LIBRARY_HOME`
  until Phase B is solid.
- Package-store discovery failures must list searched paths and how to set
  `CISTAE_HOME`.
- Missing target binding metadata must fail package compilation with a clear
  provider/target diagnostic — never silent wrong calls.
- Store/resolver routing must remain revertible without changing Faber language
  grammar.
- `cista` must not gain a radix dependency; compile steps shell out or call
  public tools (`faber`, `cargo`, etc.) as needed.

## Acceptance Criteria

### Model (doc + manifests)

- Cista is the package-store concept; `faber` remains project source workflow.
- `$CISTAE_HOME` is the shared store; layouts use `interfaces/` and `targets/`.
- Interface contracts stay separate from target-native binding metadata.
- Manifest shape remains `[source]`, `[target]`, `[[bindings]]` (extend carefully
  for bin/meta roles rather than inventing a second store).
- Source vs artifact distribution and generated vs manifest binding policies
  remain defined.
- Package roles **lib / bin / meta** are defined; meta is deps-only.
- Norma is an instance of the general package model, not a permanent special case.
- `cista run` is defined as executing an **installed bin entry**, not building
  arbitrary source trees.

### Delivery gates (by phase)

- **A:** inspect/remove + mathesis install + faber demo build from store.
- **B:** Norma resolvable via the same package contract as third-party libs.
- **C:** at least one coreutils-style bin installable to the store.
- **D:** `cista run` works for that installed bin with arg passthrough.
- **E:** meta install expands dependencies (when implemented).
- **F:** remote install without `--path` (when implemented).

## Validation

- `rg -n "default_stdlib_root|norma_runtime_path|stdlib_path|LibraryResolver" ../faber/src crates/radix/src`
  should still be reviewed before implementation to verify current path
  assumptions.
- A future delivery spec should include focused tests for explicit store
  resolution, `$CISTAE_HOME` resolution, missing store diagnostics, development
  fallback, and generated Rust dependency/artifact selection.
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
- A future delivery spec should include a Norma-oriented fixture proving that
  hand-written Rust runtime sources compile through manifest policy and that
  bindings resolve without `@ subsidia` or `@ verte` in the interface files.
- Review check: the accepted model should not require editing
  `../norma/src/*.fab` to add a new target runtime such as Wasm.

## Open Questions

- Should project dependency intent live only in `faber.toml`, only in a cista
  lock/intent file, or both? (Historical `requirit.toml` — keep or drop?)
- Do store packages keep a separate `cista.toml` forever, or does `faber.toml`
  gain installable package fields for published units?
- How is **bin** role spelled in the manifest (field on `[source]`, `[target]`,
  or install metadata)?
- Default executable entry name for bins (`package` name vs explicit `bin =`)?
- Is `CISTAE_HOME` final, or should a broader `FABER_HOME` own `cistae/`?
- Should one target-specific `cista.toml` sit beside each artifact, or should
  the package-version root index available targets?
- What compatibility facts must Rust artifacts record beyond language, triple,
  rustc version, artifact path, and crate name?
- Source cache vs artifact-only install for source-distributed packages?
- How much transitive resolution is required at Phase A vs C vs F?
- PATH shims for installed bins: in scope after Phase D, or always optional?
- What URL and API shape should `cista.dev` expose (Phase F)?

## Stop Conditions

- Stop if implementation skips Phase A (faber consuming the store) and jumps to
  registry or bin-only demos that still need sibling-repo hacks.
- Stop if the design requires target-specific annotations in Faber interface
  files for every supported backend.
- Stop if the package model only works for Norma and cannot describe a future
  non-Norma provider.
- Stop if installed **bin** behavior still depends exclusively on repository
  layout (relative monorepo imports as the only way to share helpers).
- Stop if adding Wasm or another target would require editing existing
  target-neutral Faber interfaces.
- Stop if `cista run` is redefined as “compile and run local source” (that is
  `faber run`).
- Stop if cista gains a radix/compiler crate dependency.
