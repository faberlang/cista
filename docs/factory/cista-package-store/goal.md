# Goal: Cista Package Store Model

**Status**: discovery goal
**Created**: 2026-06-21
**Updated**: 2026-06-21
**Target Repo**: `/Users/ianzepp/work/faberlang/cista`
**Factory Artifact Dir**: `docs/factory/cista-package-store/`
**Note**: Implementation lives in public sibling `faberlang/cista`.
**Primary Goal**: define Faber's shared package artifact store, installed package layout, bundled Norma package shape, and target runtime binding contract

## Summary

Define a coherent cista package model for Faber so standard and third-party
packages can be consumed without copying dependency files into each project.
The first goal is not to implement a public package registry. The first goal is
to establish the package-store shape, manifest contract, resolver output, and
validation rules needed for local package installation and demo builds.

The current direction is artifact-first but not artifact-only. An installed
cista package must contain Faber interfaces needed for typechecking, and it may
either contain compiled target artifacts or contain source plus enough compile
policy to build a target artifact locally. Project directories contain
manifests, lockfiles, and project source only.

## Problem

- Current `faber` package compilation resolves built-in library interfaces from
  a repository-local `stdlib/` tree.
- Generated Rust packages depend on repo-local `crates/norma`.
- That works for in-repo development, but it does not describe how an installed
  `faber` binary finds bundled Norma interfaces, target runtime
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

## Goals

- Define `$CISTAE_HOME`, defaulting to `~/.faber/cistae`, as the shared
  versioned package artifact store used by builds.
- Define installed cista package layout for artifact-distributed and
  source-distributed packages.
- Define how a package exposes Faber interfaces and target runtime bindings.
- Define how target-native symbol mappings live outside Faber interface files.
- Define how bundled Norma fits the general package model as a bundled standard
  package.
- Define how future non-Norma packages can provide Faber interfaces and
  per-target artifacts.
- Define binding policy variants for generated target symbols versus manifest
  mappings to hand-written target implementations.
- Define a minimal local-source install flow that builds a package for a target
  and installs only the consumable artifact bundle into `$CISTAE_HOME`.
- Treat `cista.dev` as the planned canonical domain for future package
  publishing and retrieval.
- Produce enough design clarity for a delivery spec to implement the first
  milestone safely.

## Non-goals

- Do not implement a public package registry in this goal.
- Do not build remote publishing, authentication, semantic version solving, or
  full dependency graph resolution.
- Do not copy installed dependencies into project directories.
- Do not require installed dependencies to contain full source trees unless the
  package is intentionally source-distributed.
- Do not add target-specific annotations to Faber interface files.
- Do not move `stdlib/norma` or `crates/norma` during discovery.
- Do not add new target runtimes such as Wasm or Go as part of the first
  artifact.
- Do not solve every external package layout. Define the minimum contract future
  packages must satisfy.

## Ground Truth Researched

- `crates/faber/src/library.rs`: built-in library resolution returns `.fab`
  interface files under a filesystem `stdlib` root derived from the repository
  layout.
- `crates/faber/src/package.rs`: package compilation uses
  `Config.stdlib_path` when provided, otherwise the default resolver.
- `crates/faber/src/package.rs`: generated Rust crates depend on
  `norma = { path = ... }`, where the path is computed from repo-local
  `crates/norma`.
- `crates/faber/src/library.rs`: the data model already distinguishes built-in
  library providers from future package-backed providers, but only `norma` is
  currently implemented.
- `stdlib/norma/*.fab`: Norma interfaces are Faber source contracts parsed and
  typechecked with package code.
- `crates/norma`: Norma's Rust runtime implementation is a separate Rust crate,
  not embedded into the `faber` binary.
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

- `crates/cista/src/manifest.rs`: current cista manifest schema.
- `crates/cista/src/commands.rs`: current `cista check` validation behavior.
- `examples/cista-lab/source/mathesis/`: current source-distributed package
  fixture.
- `crates/faber/src/library.rs`: current provider resolver, built-in module
  discovery, and future package-provider placeholders.
- `crates/faber/src/package.rs`: package loading, generated Cargo layout,
  library provenance attachment, and generated `norma` dependency path.
- `crates/radix/src/driver/session.rs`: `Config.stdlib_path` and how compiler
  sessions carry library configuration.
- `stdlib/norma/`: current target-neutral Faber interfaces.
- `crates/norma/`: current Rust-native Norma runtime implementation.
- `docs/factory/library-import-provenance/plan.md`: current
  provider-qualified import model and provenance invariants.
- `docs/factory/intrinsics-innatum-residue/plan.md`: related native symbol
  mapping and `@ verte` cleanup pressure.
- `docs/factory/target-support-matrix/goal.md`: target metadata precedent and
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
- Development mode may continue using repo-local `stdlib/` and `crates/norma`
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
5. development fallback for repo-local `stdlib/` and `crates/norma`

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

### Phase 1: Store And Manifest Contract

- Keep `$CISTAE_HOME` as the canonical dependency artifact store root.
- Keep `~/.faber/cistae` as the default store path.
- Keep `interfaces/` and `targets/<language>/` as canonical package layout
  directories.
- Treat bundled Norma as a bundled cista package using the same package/version
  layout.
- Decide whether the project dependency intent file remains separate from
  `faber.toml` and whether the historical `requirit.toml` idea survives.

### Phase 2: Local Source Install

- Implement `cista install --path <package> --target-language rust`.
- Reuse `cista check` validation during install.
- Build or verify the Rust target implementation.
- Install only interfaces, selected target manifest, and selected target
  artifact into `$CISTAE_HOME`.
- Do not copy dependency contents into the consuming project.

### Phase 3: Installed Package Inspection

- Implement `cista package show <package>`.
- Implement `cista package interfaces <package>`.
- Implement `cista runtime bindings <package> --target rust`.
- Ensure these commands can inspect both `$CISTAE_HOME` packages and bundled
  packages.

### Phase 4: Demo Consumer Build

- Add `examples/cista-lab/demo/`.
- Add a minimal project manifest and `cista.lock` for `mathesis`.
- Teach `faber check` or `faber build` enough resolver behavior to find the
  installed `mathesis` interface from `$CISTAE_HOME`.
- Preserve provider provenance for imported package interfaces.
- Link generated Rust against the installed or locally compiled Mathesis target
  artifact.

### Phase 5: Norma As Bundled Package

- Add a Norma package manifest using `mode = "compile"` and
  `binding_policy = "manifest"`.
- Move Norma Rust binding facts out of Faber interfaces and into manifest
  bindings.
- Prove bundled Norma and installed third-party packages use the same resolver
  contract.

## Exit Strategy

- Any first implementation should preserve a development fallback that uses the
  existing repo-local `stdlib/` and `crates/norma` paths.
- If package store discovery fails, diagnostics must explain the searched paths
  and how to set the explicit path.
- If target runtime binding metadata is missing, package compilation should
  fail with a provider/target diagnostic rather than silently generating wrong
  target calls.
- A later implementation should be removable by reverting resolver/config
  routing without changing the Faber language grammar.

## Acceptance Criteria

- The goal defines `cista` as the package concept and identifies how it relates
  to the `faber` command.
- The goal defines `$CISTAE_HOME` as the shared dependency artifact store.
- The goal defines canonical source package, installed package, and bundled
  package layouts using `interfaces/` and `targets/`.
- The goal distinguishes Faber interface contracts from target-native runtime
  implementation metadata.
- The goal defines an installed artifact manifest shape with `[source]`,
  `[target]`, and `[[bindings]]` using structured `source_module`,
  `source_symbol`, and `target` binding keys.
- The goal defines source-distributed package mode with target compile policy
  and distinguishes generated bindings from manifest bindings.
- The goal identifies Norma as the larger source-distributed package example
  using `mode = "compile"` and `binding_policy = "manifest"`.
- The goal explains how Norma becomes a bundled package instance of the general
  package model.
- The goal identifies a local-source install and demo-build slice small enough
  for delivery.
- The goal records open decisions that must be answered before implementation.

## Validation

- `rg -n "default_stdlib_root|norma_runtime_path|stdlib_path|LibraryResolver" crates/faber/src crates/radix/src`
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
  `stdlib/norma/*.fab` to add a new target runtime such as Wasm.

## Open Questions

- Should `cista` appear in the project manifest format, or remain package-unit
  terminology plus a low-level crate/tool?
- Is `CISTAE_HOME` the final canonical environment variable for the shared
  package artifact store?
- Should `$CISTAE_HOME` point directly at the cista store root
  (`~/.faber/cistae`) or should a broader `FABER_HOME` own `cistae/`?
- Should the generated Rust crate depend on a bundled Norma crate path, a
  published `norma` crate version, or either depending on build mode?
- Where should target-native binding manifests live in the repo before bundled
  package layout exists?
- Should one target-specific `cista.toml` live beside each compiled target
  artifact, or should the package-version root contain an index of available
  target artifacts?
- What exact compatibility facts must Rust target artifacts record beyond
  language, triple, rustc version, artifact path, and crate name?
- Should source-distributed packages always install source under `$CISTAE_HOME`,
  or should source be retained in a separate source cache and compiled artifacts
  be stored in a build cache?
- How much of package dependency resolution belongs in the first cista milestone
  versus a later package-manager goal?
- Does the historical `requirit.toml` dependency-intent design remain useful,
  or should package dependency intent move under a newer Cista-shaped project
  manifest contract?
- What URL and API shape should `cista.dev` expose once registry work begins?

## Stop Conditions

- Stop if implementation starts before deciding the package store discovery
  order.
- Stop if the design requires target-specific annotations in Faber interface
  files for every supported backend.
- Stop if the package model only works for Norma and cannot describe a future
  non-Norma provider.
- Stop if installed binary behavior still depends exclusively on repository
  layout.
- Stop if adding Wasm or another target would require editing existing
  target-neutral Faber interfaces.
