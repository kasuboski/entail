# Worked example: from a JSON decision model to a Z3 finding

How does Entail find that a proposed access grant can outlast a contractor's contract? This walkthrough follows the existing [unsafe-grant fixture](examples/models/unsafe-grant.json), from authored JSON to a reported counterexample. It also shows the coverage query for the same model.

No solver background is needed. A **symbolic input** is a placeholder for a value not yet chosen. A **query** asks whether values exist that make all its conditions true. Z3 is the solver that answers that question:

- **SAT** (satisfiable): there is such an assignment; a **witness** is one example.
- **UNSAT** (unsatisfiable): no assignment satisfies these conditions.
- **UNKNOWN**, timeout, or cancellation: no conclusive answer; none means success.

The important question is *what conditions did we ask about?* For compliance, Entail searches for a violation, so SAT is a finding, not approval of the policy.

## 1. Start with the authored model

The fixture describes `ContractorAccess` at one snapshot in time:

| Declaration | Meaning in this fixture |
| --- | --- |
| `ApprovalStatus` | Enum variants, in order: `Approved`, `NotApproved`, `Unknown` |
| `sponsor_approval` | Input of type `ApprovalStatus` |
| `contract_end`, `requested_expiry` | Date inputs |
| `Grant` | Outcome constructor with a Date field named `expiry` |
| `Deny` | Declared outcome with a `DenialReason` field named `reason`; no rule produces it here |
| `GrantApprovedRequest` | If approved, produce `Grant` with the requested expiry |
| `ApprovalRequired` | A Grant must have sponsor approval |
| `ExpiryWithinContract` | A Grant's expiry must be no later than the contract end |

Here are the operative rule and expiry constraint, copied without their `origin` metadata for readability (these are excerpts, not a complete model):

```json
{
  "id": "GrantApprovedRequest",
  "when": {
    "kind": "eq",
    "left": { "kind": "ref", "path": "input.sponsor_approval" },
    "right": { "kind": "enum", "enum": "ApprovalStatus", "variant": "Approved" }
  },
  "then": {
    "outcome": "Grant",
    "fields": {
      "expiry": { "kind": "ref", "path": "input.requested_expiry" }
    }
  }
}
```

```json
{
  "id": "ExpiryWithinContract",
  "for_outcome": "Grant",
  "must": {
    "kind": "le",
    "left": { "kind": "ref", "path": "outcome.expiry" },
    "right": { "kind": "ref", "path": "input.contract_end" }
  }
}
```

`eq` means equality; `le` means less than or equal. The rule guard checks **only approval**. It does not check the expiry date. The requirement is separate, so the engine can discover that the proposal violates it.

**IDs name declarations; expression trees define behavior.** `GrantApprovedRequest` is used to look up the rule, identify obligations, and report which rule matched. Its spelling does not instruct Z3 to approve anything. Similarly, `ExpiryWithinContract` is a lookup/reporting label: the `le` expression is the requirement. Renaming a rule consistently does not change its guard or output, although IDs are included in semantic hashes and obligation/cache identity. Input reference paths and outcome constructor/field names resolve actual declared inputs and outputs; they are not natural-language instructions either.

The source records `S1` and `S2` contain synthetic policy sentences. Origins link declarations to those records and mark review status. They do not compile the sentences into logic or establish that the interpretation is correct. Proposed rules still participate. Open items about other prerequisites, denial policy, and actual expiry enforcement remain visible but add no solver conditions.

The fixture has `domain: []` and `derived: []`. It declares the assumption `SponsorsAlreadyApproved`, whose expression is the same approval equality, but **declaration is not selection**. We select no assumptions in the main walkthrough.

## 2. Compile before solving

In [`author_model`](../crates/author_model/src/lib.rs), `compile_json` first runs structural/resource preflight, then deserializes into `RawModel` with tagged `Expr` and `Type` nodes. It performs semantic validation; JSON Schema alone is not enough.

For these expressions, the checks are concrete:

1. `input.sponsor_approval` resolves to the declared `ApprovalStatus` input. The literal `ApprovalStatus.Approved` names a real variant of the same enum. Their equality has type Bool, as required for a rule guard.
2. The rule's `then.outcome` resolves to `Grant`. Its fields must contain exactly the required `expiry` field. `input.requested_expiry` is a Date, matching `Grant.expiry`.
3. The constraint's `for_outcome` resolves to `Grant`. `outcome.expiry` is checked in that constructor's field scope, not against arbitrary outcomes. Both operands of `le` are Dates, and its result is Bool.
4. IDs, enum declarations, source references, and origin restrictions are validated. In general the compiler also checks literal dates, integer units, derived dependency cycles, and expression/declaration limits. This fixture has no derived dependencies to sort.

An undeclared reference, wrong field type, missing payload field, or unsupported expression fails compilation before Z3 is called. Diagnostics carry codes and declaration/JSON Pointer/source context where available. Successful compilation returns an immutable `CompiledModel`: validated declarations, derived order (empty here), and content/semantic hashes. It does **not** certify that the rules comply; this unsafe fixture compiles successfully.

See the [language and expression specification](SPEC.md#2-model-structure) for the full contract.

## 3. Plan the question

Suppose analysis requests `coverage` and `constraint_compliance`, with `assumption_ids: []`. In [`author_analysis`](../crates/author_analysis/src/lib.rs), `check_selected`, `next_plan`, and `analyze` produce this sequence, assuming sufficient budget:

| Obligation ID | Question |
| --- | --- |
| `domain_feasibility` | Are any typed inputs in scope? Always first. |
| `coverage` | Is there an in-scope input where no rule applies? |
| `rule_reachability:GrantApprovedRequest` | Can the rule apply? Automatically included immediately before compliance when not already requested. |
| `constraint_compliance:GrantApprovedRequest:ApprovalRequired` | Can this rule violate the approval requirement? |
| `constraint_compliance:GrantApprovedRequest:ExpiryWithinContract` | Can this rule violate the expiry requirement? |

Rules and constraints are sorted by ID for planning. Compliance pairs are generated only when the rule's output constructor matches the constraint's `for_outcome`. The expiry obligation itself is solver-independent data:

```json
{
  "kind": "constraint_compliance",
  "rule_id": "GrantApprovedRequest",
  "constraint_id": "ExpiryWithinContract"
}
```

There is no expression hidden inside those IDs: the backend uses them to retrieve the rule and constraint above. An empty domain would skip dependent obligations rather than claim compliance. Budgets can also leave work unfinished. A completed cached answer may avoid a new solver call; the following describes a cache miss.

## 4. Create symbolic inputs and the base conditions

In [`author_z3`](../crates/author_z3/src/lib.rs), `solve_query` creates a **fresh, isolated solver per obligation**, sets timeout/resource parameters, and creates an environment mapping expression paths to native Z3 terms:

| Expression path | Z3 symbol | Encoding |
| --- | --- | --- |
| `input.sponsor_approval` | `input_sponsor_approval` | Integer: `Approved = 0`, `NotApproved = 1`, `Unknown = 2` |
| `input.contract_end` | `input_contract_end` | Integer date ordinal |
| `input.requested_expiry` | `input_requested_expiry` | Integer date ordinal |

An ordinal counts days from `0001-01-01`, which is 0. `9999-12-31` is 3652058. Gregorian leap years are accounted for, so integer order agrees with date order. `Unknown` is an ordinary enum value (2), not a solver's UNKNOWN answer.

`add_type_bounds` asserts membership in the enum and bounds on both dates. There are no independent symbolic outcome variables for this query, and declaring `Deny` does not add a denial rule. In general Booleans use Z3 Bool terms; integers, dates, and enums use Int terms. Compiler type checks distinguish these business types even when the backend uses the same integer representation. Derived expressions, if present, get named terms equated to their lowered expressions in dependency order; none are needed here.

Next the backend asserts every domain predicate and only the selected assumptions. Define the base conditions as:

```text
T = (approval is 0, 1, or 2)
    AND (0 <= contract_end <= 3652058)
    AND (0 <= requested_expiry <= 3652058)
D = true                 -- no domain predicates
A = true                 -- no selected assumptions
B = T AND D AND A
```

These are type bounds, not an inferred policy that dates must be sensible for a particular contractor. Requirements are **not** added to `B` or to rule guards; that would hide the very violation we are searching for. See [decision semantics](SPEC.md#3-decision-semantics).

## 5. Lower the expressions and ask the exact query

`lower_expr` maps `ref` to its environment term, the enum literal to its zero-based ordinal, `eq` to Z3 equality, and `le` to integer `<=`. Thus:

```text
g = (input_sponsor_approval = 0)
rule payload expiry = input_requested_expiry
```

For compliance, `lower_outcome_fields` lowers the rule's payload expressions. `solve_query` extends a copy of the environment with:

```text
outcome.expiry -> input_requested_expiry
```

It then lowers the constraint **using that rule's payload**, giving:

```text
c = (input_requested_expiry <= input_contract_end)
```

The question is `B AND g AND NOT c`: “Can an approved request produce an expiry after the contract ends?” It is not `B AND g AND c`, which would merely find a compliant example.

Here is the exact logical content of the expiry query in SMT-LIB notation, an ordinary text notation for solver expressions. Parentheses put the operator first: `(= a 0)` means `a = 0`, and `assert` requires a condition to hold. **This is a readable equivalent of the native Z3 API assertions, not an SMT text file emitted by Entail**; solver parameters and internal printing are not shown.

```smt2
(declare-const input_sponsor_approval Int)
(declare-const input_contract_end Int)
(declare-const input_requested_expiry Int)
(assert (or (= input_sponsor_approval 0)
            (= input_sponsor_approval 1)
            (= input_sponsor_approval 2)))
(assert (>= input_contract_end 0))
(assert (<= input_contract_end 3652058))
(assert (>= input_requested_expiry 0))
(assert (<= input_requested_expiry 3652058))
(assert (and (= input_sponsor_approval 0)
             (not (<= input_requested_expiry input_contract_end))))
(check-sat)
```

For **coverage**, keep the same declarations and five type-bound assertions, but replace the final assertion with:

```smt2
(assert (not (or (= input_sponsor_approval 0))))
(check-sat)
```

This is `B AND NOT OR(all rule guards)`; the one-element `or` reflects the backend's aggregation. `NotApproved` and `Unknown` both give uncovered inputs. No applicable rule means *unspecified*, not Deny.

The mandatory feasibility query adds `true` to the base assertions and is SAT here. Reachability adds `g` and is SAT. `ApprovalRequired` compliance adds `g AND NOT g`, which is UNSAT: no approval violation exists under this rule. Expiry compliance and coverage are both SAT. These are the [check catalog's](SPEC.md#4-check-catalog) distinct questions, not one overall business-verification result.

## 6. Decode a witness and replay it without Z3

After SAT, `decode_inputs` evaluates **every declared input** in the Z3 model with model completion enabled (`eval(term, true)`). Enum ordinals are converted back to declared variant names, date ordinals to canonical date strings, and values to typed `InputValues`. Integers in other models remain exact decimal strings with their units.

For a human-friendly example, use the existing [unsafe-expiry scenario](examples/scenarios/unsafe-expiry.json): approval 0, contract-end ordinal 739980, requested-expiry ordinal 739981. It satisfies every assertion above. Its decoded input shape is:

```json
{
  "sponsor_approval": { "kind": "enum", "enum": "ApprovalStatus", "variant": "Approved" },
  "contract_end": { "kind": "date", "value": "2026-12-31" },
  "requested_expiry": { "kind": "date", "value": "2027-01-01" }
}
```

**This is an illustrative valid witness, not a recorded or guaranteed Z3 choice.** The solver may choose other dates, including dates near the allowed bounds; witnesses are not promised to be realistic, minimal, or unique.

Before accepting a SAT answer, `map_answer` calls `replay_witness` in `author_analysis`. The ordinary `author_model::evaluate` function, which uses no solver:

1. Validates the complete typed input assignment and computes derived expressions (none here).
2. Checks domain predicates and selected assumptions; this scenario is inside the domain.
3. Evaluates the guard: `Approved == Approved` is true. `matched_rule_ids` contains `GrantApprovedRequest`.
4. Constructs `Grant` with `expiry = 2027-01-01`. There is one distinct outcome, so classification is `unique`.
5. Evaluates Grant-targeted constraints: approval passes; `2027-01-01 <= 2026-12-31` is false. `violated_constraint_ids` contains `ExpiryWithinContract`.

The compliance replay additionally checks that the **queried rule** matched, the constraint targets its constructor, and the constraint evaluates false on **that rule's own computed payload**. It does not just trust a violation belonging to some other rule. Coverage replay instead requires an in-domain assignment with an empty matched-rule set. For example, changing approval to `NotApproved` gives classification `unspecified`, no outcomes, and no violated constraints.

A failed or erroneous SAT replay yields result `status: "error"` with `diagnostic_code: "witness_mismatch"`, not a finding. This is a check on counterexamples through the typed evaluator, not formal proof certification of the translation or of UNSAT results.

## 7. Map the answer into the report

For expiry compliance, the result's key fields are the following (a **partial result excerpt**, omitting the witness and timing/statistics fields):

```json
{
  "obligation_id": "constraint_compliance:GrantApprovedRequest:ExpiryWithinContract",
  "obligation": {
    "kind": "constraint_compliance",
    "rule_id": "GrantApprovedRequest",
    "constraint_id": "ExpiryWithinContract"
  },
  "declaration_ids": ["GrantApprovedRequest", "ExpiryWithinContract"],
  "status": "finding",
  "finding_category": "violation",
  "source_ids": ["S1", "S2"],
  "conditional": false,
  "labels": [],
  "rule_reachability_status": "satisfied"
}
```

`witness` contains the solver's decoded inputs, not the evaluator's whole output. `rule_reachability_status` is the status of the separate reachability result (whose category here is `reachable`). `source_ids` includes provenance from relevant declarations **and** common model declarations/domain/selected assumptions; it is not a minimal explanation or a claim that the source text was proved. Coverage reports `status: "finding"`, category `uncovered`, and `declaration_ids: ["GrantApprovedRequest"]`.

The enclosing `AnalysisReport` records language/engine/solver versions, `model_id`, content and semantic hashes, effective `checks` and `budget`, `selected_assumption_ids: []`, `domain_predicate_ids: []`, participating proposed declaration IDs, and all three open items. It also reports completion, elapsed time, obligation/query counts, cache hits, and remaining work. Per-obligation timing and statistics are included when available; fresh Z3 answers include assertion count and solver statistics. `complete` is about analysis execution, not policy correctness. See [results and provenance](SPEC.md#5-results-and-provenance) and the `AnalysisReport`/`ObligationResult` definitions in `author_analysis` for all fields.

## 8. Try it and see the scope change

From the repository root, after the [documented build setup](../README.md#build-and-run):

```sh
nix develop --command target/debug/entail validate docs/examples/models/unsafe-grant.json
nix develop --command target/debug/entail analyze docs/examples/models/unsafe-grant.json --checks coverage,constraint_compliance
nix develop --command target/debug/entail evaluate docs/examples/models/unsafe-grant.json docs/examples/scenarios/unsafe-expiry.json
```

Analysis finds some counterexample; evaluation deterministically replays the supplied human-friendly scenario. Both emit JSON, but their result shapes differ. Findings are normal completed command results, not invalid-input errors.

Now explicitly select the declared assumption:

```sh
nix develop --command target/debug/entail analyze docs/examples/models/unsafe-grant.json --checks coverage,constraint_compliance --assumptions SponsorsAlreadyApproved
```

The base conditions gain `(= input_sponsor_approval 0)`. Coverage becomes UNSAT (`covered`) because every **in-scope** input now satisfies the only rule's guard. Expiry compliance remains SAT because the assumption imposes no date relationship. The report lists `SponsorsAlreadyApproved`, sets results' `conditional: true`, and adds `conditional_on_assumptions`. This is narrowed scope, not repaired behavior for unapproved contractors.

The fixture is synthetic and still has unresolved policy and lifecycle questions. Even an UNSAT violation query only rules out that violation under this encoding, model, and assumption context. Entail trusts its translation and Z3 for UNSAT; it emits no independently checked proof certificates. A snapshot expiry comparison cannot establish that access will actually be stopped on time.
