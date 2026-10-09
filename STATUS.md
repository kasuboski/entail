# Status

## Current Goal
- Deliver feature-complete Author decision engine v0 in entail2 through delegated implementation/validation; remove handwritten worker JSON recovery in favor of Serde as requested.

## Steps & Progress
- [x] Read full spec/plan/schema/fixtures; initialize Git on main, commit baseline and isolate component worktrees.
- [x] Establish pinned Rust/Nix/Z3 workspace and native SAT/UNSAT smoke build (`47746e4`).
- [x] Implement model/compiler/evaluator and repair schema, date, outcome-scope and preconstruction-limit bugs.
- [x] Complete model diagnostic/serialization follow-up (`1d1c761`): precise context, metadata roundtrips; 23 model tests, Clippy, format and MSRV pass.
- [x] Implement analysis catalog, feasibility/prerequisites/vacuity, budgets, witness replay, conditional/proposed/open-item reporting and completed-answer caching (`041e443`, `eddcee4`).
- [x] Repair coverage rule provenance (`cc81705`), including metadata-cache and skipped-result tests.
- [x] Implement Z3 exact lowering/decoding; native statistics and finite/generated differential checks (`45cfab6` through `8f79b9a`); 14 Z3 tests including 29 expressions/58 fixed-assignment comparisons.
- [x] Implement CLI/persistent worker; timeout/default clamping, null rejection, capacity precheck, compile timing, active-analysis kill/reap/restart and truncation tests (`98ea437`, `b2a2260`).
- [x] Protect library invariants (`a007a1e`): immutable compiler-only CompiledModel, opt-in namespaced/type-separated caching; 55 workspace tests plus compile-fail doctest pass at that checkpoint.
- [x] External integrated acceptance: 20 CLI groups, real worker lifecycle/protocol suite and 8 Python regression tests pass (`a254f0e`).
- [x] Measure release workload/cache/edit/assumption behavior; native stats/compile timing/Linux VmHWM measured in five-run evidence (`4359d3f`, `3f7ee1b`); historical evidence preserved.
- [x] Independent semantic/API reviews; all reported semantic/cache/provenance defects assigned and committed.
- [ ] Worker recovery follow-up: cancelled handwritten JSON scanner approach; agent replacing it with a minimal Serde top-level Visitor and IgnoredAny, with malformed-frame routing tests.
- [ ] Fresh final integration agent: combine final patches, clean docs/lock/ignores, and run complete nextest/doctest/MSRV/format/Clippy/external acceptance gates.
- [ ] Merge final validated integration on main and record final delivery evidence.

## Unknowns
- Final all-patches test count and clean main commit await integration gates.
- Malformed request IDs are recoverable only when Serde parses the root ID before the error; otherwise return null, never infer nested/string IDs.
- No latency guarantee: measured smoke supports retaining provisional 5s total / 1s query / 10,000-obligation defaults, not a performance SLO.

## Discovered Issues
- Initial checkout contained spec/tooling only; Git and application foundation established.
- Earlier tracker omission/restoration corrected; parent-owned tracker is committed on main.
- Earlier model scope/date/schema/resource/diagnostic and analysis replay/cache/vacuity/provenance defects repaired; final integrated retest pending.
- Earlier public CompiledModel mutability could poison UNSAT cache identity; immutable accessors and compiler-only construction now committed.
- Earlier worker omitted-budget defaults bypassed caps and capacity was checked too late; both repaired.
- Best-effort malformed-frame ID recovery originally matched nested IDs; my follow-up instruction led to needless handwritten JSON parsing. User objection accepted; scanner cancelled and Serde-based replacement delegated.
- External CLI native-backend-fault injection is unavailable; backend-error mapping is covered through analysis seams and documented honestly.
