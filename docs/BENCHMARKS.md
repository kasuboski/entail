# Benchmarks

`scripts/benchmark.py` deterministically generates workload JSON with the Python standard library and checks required top-level shape/canonical integer literal encodings. Those checks are not a substitute for schema or semantic validation; the measurement harness compiles each workload through the actual worker protocol. The generated inputs are ignored by Git; regenerate all ten files with `mise exec -- nix develop --command python3 scripts/benchmark.py` (without `--binary`, this generates and structurally validates inputs without measuring).

## Run

```sh
mise exec -- nix develop --command cargo build --release --locked --bin entail
mise exec -- nix develop --command python3 scripts/benchmark.py \
  --binary target/release/entail --runs 5 --profile release \
  --rust-version 'rustc 1.98.0 (88d9e12ae 2026-08-18)'
```

The persistent worker uses serial four-byte big-endian length-prefixed JSON. For each workload, the harness measures five compile and analysis requests in one worker process. It reports wall-clock request timings separately from engine-reported `compile_elapsed_ms` and analysis report elapsed time. It captures planned/executed obligations, statuses, cache hits, solver query counts, detached native per-obligation statistics, and process RSS. Linux `/proc/<pid>/status` `VmHWM` is polled every 2 ms while the worker remains alive. Cache probes separately measure repeated identical analysis, semantic edits, metadata edits, assumption-key separation, and one-rule edit/analyze loops. Cache-hit results have no solver statistics; the harness does not reuse stale values.

`--full` doubles the requested repetitions and includes all generated workload families; the default run already covers eight families: threshold rules (10/100/1000), overlapping guards, same-constructor payloads, coupled Booleans, a shared derived DAG, and near declared model limits. The default smoke budget is 1500 ms total, 200 ms per obligation and a 24-obligation cap. Work skipped due to these limits is inconclusive, never satisfied. Use the recorded budget and report completeness when interpreting results.

## Archived release worker smoke (historical source snapshot)

[`benchmarks/smoke-release-final-2026-10-09.json`](../benchmarks/smoke-release-final-2026-10-09.json) records five repetitions for each of the eight families against source snapshot `3da81cee5fe87544eb3a5b870706881abb07b089`. It includes the build/measurement commands, Linux x86_64 host and CPU, Rust 1.98.0, Z3 5.1.0.0, worker protocol, budgets, all per-run analysis reports and native statistics, compile timings, cache probes, and sampled RSS. It predates the final exact-invalid-contract diagnostic and Serde malformed-JSON recovery patches; these metrics remain an honest measurement of that earlier point, not a claim about every detail of current HEAD. Performance was not rerun for those error-handling-only changes.

Selected worker analysis request wall times (ms, p50/p95; includes request transport) and process sampled peak RSS (KiB):

| Workload | Analysis p50 | Analysis p95 | Engine compile p50/p95 ms | Sampled worker peak RSS KiB | Queries per report | Inconclusive obligations |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| `independent_thresholds_10` | 0.17 | 16.19 | 0/0 | 40,036 | 12 | 0 |
| `independent_thresholds_100` | 1.13 | 23.84 | 1/2 | 37,204 | 24 | 1 |
| `independent_thresholds_1000` | 9.17 | 33.71 | 17/21 | 58,500 | 24 | 1 |
| `overlapping_guards` | 0.55 | 22.30 | 0/0 | 36,488 | 24 | 1 |
| `same_constructor_payload` | 0.97 | 23.00 | 1/1 | 36,752 | 24 | 1 |
| `coupled_boolean` | 0.10 | 7.36 | 0/0 | 35,408 | 3 | 0 |
| `shared_derived_dag` | 0.22 | 9.75 | 1/1 | 36,644 | 3 | 0 |
| `near_model_limits` | 34.20 | 64.63 | 46/50 | 88,520 | 24 | 1 |

The obligation cap deliberately bounds this smoke run: the 100+ rule cases include skipped/inconclusive obligations. These figures are not comparable with the historical cold CLI timings, and they do not claim completion of those larger analyses. They apply to the recorded historical source snapshot, not a full benchmark of current HEAD. Full reports include each native Z3 statistics map as detached string key/value data. `assertion_count` is only a formula-size proxy, not exact formula AST size; native memory values are kept under their original Z3 keys and are not conflated with worker RSS. `VmHWM` is a sampled process-wide high-water mark spanning the five compile/analyze repetitions for that family's worker.

### Cache and edit observations

The cache probe's first analysis executed six solver queries. Each of four identical repeats returned six cache hits and zero queries. A semantic edit changed the semantic hash and executed six fresh queries. A metadata/open-item edit preserved the semantic hash, changed the content hash, and got six semantic cache hits while reporting the new content hash. Selecting an assumption executed six queries; returning to no assumptions got six hits. The report's explicit expectations for semantic/content hash changes and assumption key separation all evaluated true. The full probe and hashes are in the JSON artifact.

## Interpretation and limits

Keep the engine's provisional 5000 ms total / 1000 ms per-obligation / 10000-obligation defaults as starting values. The benchmark's stricter bounded smoke settings are for reproducibility, not an interactive latency SLO. No unavailable metric is inferred or fabricated. Exact formula AST node count remains unavailable; detached Z3 statistics and sampled Linux RSS are measured. This is one Linux host and one release build with five repetitions, so do not generalize the results into latency or memory guarantees. This archived follow-up is worker-focused; final-revision CLI/performance timings were not collected. The separate historical CLI-versus-worker smoke below also measures an earlier revision and is not a current-HEAD claim.

## Historical CLI versus persistent-worker smoke (2026-10-22)

The removed report `smoke-release-2026-10-22.json` measured source commit `57a0f158a4636c958c7f9742a3d1cf2236c655a1`, release profile, on Linux 6.18.23 x86_64/glibc 2.44, AMD Ryzen 5 7600X (6 cores/12 threads), Rust 1.98.0 (`88d9e12ae`, 2026-08-18), Cargo 1.98.0 (`797e8a9bc`, 2026-08-05), Z3 5.1.0.0, `author-analysis/0.1`. Each timing is five repetitions; values below are p50/p95 wall milliseconds. Worker analysis timings include request transport; CLI wall timings include process startup and model compilation. Worker compile times are shown separately.

| Workload | Worker compile p50/p95 | Worker analyze p50/p95 | CLI analyze p50/p95 | Worker queries (inconclusive) | CLI queries (inconclusive) |
| --- | ---: | ---: | ---: | ---: | ---: |
| `independent_thresholds_10` | 0.23/0.42 | 0.14/16.27 | 17.41/18.77 | 12 (0) | 12 (0) |
| `independent_thresholds_100` | 2.36/2.49 | 1.30/23.27 | 83.35/83.77 | 24 (1) | 102 (0) |
| `independent_thresholds_1000` | 19.35/23.64 | 10.24/31.99 | 1146.76/1155.43 | 24 (1) | 1002 (0) |
| `overlapping_guards` | 0.89/1.14 | 0.60/23.64 | 37.89/41.73 | 24 (1) | 42 (0) |
| `same_constructor_payload` | 1.21/1.69 | 1.01/22.87 | 770.40/776.42 | 24 (1) | 822 (0) |
| `coupled_boolean` | 0.43/0.61 | 0.11/7.33 | 11.89/12.22 | 3 (0) | 3 (0) |
| `shared_derived_dag` | 1.39/1.74 | 0.34/10.44 | 15.31/16.24 | 3 (0) | 3 (0) |
| `near_model_limits` | 53.68/64.41 | 34.03/67.68 | 1774.85/1778.01 | 24 (1) | 628 (1) |

**These are not a fair like-for-like CLI/worker comparison.** Worker analyses used an explicit 1500 ms total / 200 ms per-obligation budget and a 24-obligation cap. The CLI invocation supplied `--budget-ms 1500` and did not set the per-obligation timeout or obligation cap, so large cases ran many more queries; its near-limit run timed out at the 1500 ms total budget while retaining completed results. The worker's skipped obligations are inconclusive, not satisfied. Do not sum p95 values, compare unlike-budget completion as equivalent work, or present apparent speed differences as a fair performance result.

Exact commands/configuration recorded with the measurements:

```sh
cd /home/josh/projects/entail2-integration && MISE_TRUSTED_CONFIG_PATHS=/home/josh/projects/entail2-integration/mise.toml mise exec -- nix develop --command cargo build --release --locked --bin entail
nix develop /home/josh/projects/entail2-integration --command python3 scripts/benchmark.py --binary /home/josh/projects/entail2-integration/target/release/entail --runs 5 --cli
```

The harness's CLI requests used `entail analyze benchmarks/<family>.json --checks rule_reachability,coverage,outcome_conflict --budget-ms 1500`; the persistent worker was started with obligation cap 24, and worker requests used the recorded budget object. The binary path, host/runtime and source commit above are historical environment metadata, not instructions to reproduce from this checkout. Peak process memory was not measured for this report; exact formula AST size and per-obligation native solver statistics were also unavailable. It did not measure a current checkout. The curated final-2026-10-09 and native-2026-10-22 reports remain separate historical artifacts.
