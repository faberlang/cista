# Factory documentation (cista)

Open factory tracks for the public **cista** package store / registry client.

Relocated from private Radix on 2026-07-08.

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
| Cista package store | active — phases A–F (lib store → Norma → bins → run → meta → registry) | [`cista-package-store/goal.md`](cista-package-store/goal.md) |
