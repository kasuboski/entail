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
| Python | 3.14.7 | Supplied by the locked Nix development shell for acceptance and benchmark scripts |

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

The Nix shell supplies `gcc`, `pkg-config`, Z3 headers/library, `libclang` (required by `z3-sys` bindgen), and Python 3 for the acceptance and benchmark scripts. `LD_LIBRARY_PATH` points to the Nix store's Z3 runtime library so test binaries and `entail` can run outside their build command. Keep development and execution inside `nix develop` (or `nix-shell shell.nix`) when using Z3.

## Workspace shape

- `author_model`: strict JSON model compiler, semantic validation, exact concrete evaluator, DAG and content/semantic hashes.
- `author_analysis`: solver-independent obligation planning, result mapping, budgets, cache and witness replay.
- `author_z3`: native SMT lowering/decoding with differential and boundary tests.
- `author_worker`: `entail` CLI, concrete evaluation and persistent framed JSON worker.

End-to-end acceptance scripts and evidence status are documented in [`docs/ACCEPTANCE.md`](docs/ACCEPTANCE.md) and [`docs/DELIVERY.md`](docs/DELIVERY.md). These components validate and analyze authored formal models; they do not establish business truth or policy completeness.
