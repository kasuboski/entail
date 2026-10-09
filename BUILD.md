# Rust workspace build foundation

This repository currently targets `x86_64-linux` (NixOS in the tested environment). Z3 is linked dynamically from the pinned Nix development shell; the build is intentionally not a portable/static binary build.

## Recorded toolchain and native versions

Observed and tested on 2026-10-09:

| Component | Version / pin | Notes |
| --- | --- | --- |
| Rust development toolchain | 1.98.0 | Selected by `mise.toml` |
| Tested minimum Rust toolchain | 1.85.0 | `cargo check --workspace --all-targets --all-features --locked` and the native SAT/UNSAT smoke test both pass |
| Cargo | 1.98.0 (dev environment) | Rust 1.85 Cargo also used for MSRV checks |
| `z3` crate | 0.19.4 | Exact version in workspace dependencies |
| `z3-sys` crate | 0.10.9 | Exact resolution in `Cargo.lock` |
| Native Z3 | 5.1.0 | Provided by Nixpkgs revision `e7439b6b14ad3cc35d05608ebca9bce01a25f5f8`; dynamically linked |
| Nixpkgs | `e7439b6b14ad3cc35d05608ebca9bce01a25f5f8` | Locked in `flake.lock` (2026-10-08 snapshot) |
| Nix | 2.34.6 | Tested local Nix version |
| mise | 2026.4.6 | `mise.toml` pins Rust 1.98.0 and cargo-nextest 0.9.117 |
| cargo-nextest | 0.9.117 | Configured by `mise.toml` |

Rust 1.85 remains the declared workspace MSRV. This is supported by the locked dependency graph and the checks above; reevaluate the declaration whenever dependencies are upgraded.

## Reproducible local build

Install the tools declared by mise and run the checks from the repository root:

```sh
mise install
mise run format
mise run lint
mise run msrv
mise run test
```

The tasks enter the locked Nix dev shell. The equivalent direct commands are:

```sh
nix develop --command cargo fmt --all --check
nix develop --command cargo clippy --workspace --all-targets --all-features -- -D warnings
nix develop --command cargo nextest run --workspace --all-features --no-tests=pass
mise x rust@1.85.0 -- nix develop --command cargo check --workspace --all-targets --all-features --locked
```

For the native Z3 SAT/UNSAT smoke test:

```sh
nix develop --command cargo test -p author_z3 --locked
mise x rust@1.85.0 -- nix develop --command cargo test -p author_z3 --locked
```

`nix develop` uses `flake.nix` and `flake.lock`; `nix-shell shell.nix` is also supported and uses the same pinned Nixpkgs revision. Cargo dependencies are locked by `Cargo.lock` and commands above use `--locked` where dependency resolution matters.

The Nix shell supplies `gcc`, `pkg-config`, Z3 headers/library, and `libclang` (required by `z3-sys` bindgen). `LD_LIBRARY_PATH` points to the Nix store's Z3 runtime library so test binaries and `entail` can run outside their build command. Keep development and execution inside `nix develop` (or `nix-shell shell.nix`) when using Z3.

## Workspace shape

- `author_model`: model representation foundation.
- `author_analysis`: solver-independent analysis foundation.
- `author_z3`: native solver integration foundation and SAT/UNSAT smoke test.
- `author_worker`: minimal `entail` binary foundation.

These are intentionally stubs. No decision semantics or worker protocol are implemented yet.
