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

Local library store loop (Phase A): `check`, `install --path` (with optional
`--project` / cwd `faber.lock` rewrite), store `package list|show|files`,
`inspect`, and `remove` are implemented. Registry and several other verbs remain
staged.

## Factory goals

Open package-store factory work lives under
[`docs/factory/`](docs/factory/) (moved out of private Radix on 2026-07-08).
