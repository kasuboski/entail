Entail is a rust crate that implements a specification and analysis engine for business decisions.

## Dev Tooling
Use mise for access to dev tooling. On NixOS, use nix shell for access to cc.

Common Crates
- rootcause for error handling
- itertools for extra iterator helpers
- criterion for benchmarks
- rayon for parallelism
- jiff for datetimes
- cargo-nextest for testng

## Testing
We use `cargo-nextest` for running rust tests.

## Lints
We have rather pedantic clippy rules setup. Follow them.
