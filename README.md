# Cista

Package-store and runtime-binding layer for Faber — library and `cista` CLI.

This is the **package manager** surface for Faber: install, resolve, inspect,
cache, and (later) publish against **cista.dev**. It is intentionally free of
Radix/compiler dependencies so it can stay public and evolve on its own.

## Tool layering

| Tool | Role |
| ---- | ---- |
| `faber` | Project workflow: check, build, run, test; thin `faber install` facade |
| `cista` | Package store, resolve/fetch/install, inspect, publish, cache, doctor |
| `radix` | Private compiler (not a dependency of this crate) |

## Store model

`$CISTAE_HOME` is the shared package artifact store. When unset, the default is
`~/.faber/cistae`.

Projects declare and lock dependencies; they do not embed installed package
trees. Installed packages hold Faber interfaces plus compiled target artifacts
and/or source compile policy.

## Local layout

```text
faberlang/
  cista/      this repo (public)
  faber/      public project CLI
  norma/      public stdlib source
  examples/   public app examples
  radix/      private compiler (optional for cista development)
```

## Build

```bash
cargo build --release
./target/release/cista --help
```

## Status

The local package loop supports install, inspect, remove, binary execution,
meta packages, and a filesystem development registry. The remote client has a
hermetic HTTPS/bearer contract with `login`, `logout`, `fetch --registry-url`,
and `publish --registry-url`; live cista.dev validation remains
environment-gated.

## Factory goals

Open package-store factory work lives under
[`docs/factory/`](docs/factory/) (moved out of private Radix on 2026-07-08).
