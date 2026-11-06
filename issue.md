# Issue #1177: Admit caller-selected JSON Schema reference chains with truthful typed limits before v4

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

Campaign: #1174.

## Problem

JSON Schema evaluation rejects acyclic chains longer than 250 references at one instance location, even when a caller is willing to admit the required work. Existing configurable limits cover regex matching only. The current result is a typed evaluation error, not an incorrect valid/invalid verdict; the gap is caller control over an explicitly bounded capability.

## Evidence

- [`MAX_REF_CHAIN` and public limits methods](https://github.com/Blackcat-Informatics/purrdf/blob/5d25c3a7d8e76064aa716e3e168d9f443aba7b13/crates/jsonschema/src/validate.rs#L47) fix reference depth at 250 while accepting only regex `MatchLimits` from callers.
- [`follow`](https://github.com/Blackcat-Informatics/purrdf/blob/5d25c3a7d8e76064aa716e3e168d9f443aba7b13/crates/jsonschema/src/validate.rs#L1668) detects a true same-target/same-instance cycle separately from the consecutive chain ceiling.
- [Existing API cases](https://github.com/Blackcat-Informatics/purrdf/blob/5d25c3a7d8e76064aa716e3e168d9f443aba7b13/crates/jsonschema/tests/api.rs#L594) cover acceptance at 250 and refusal at 251.
- [`EvaluationCause::ReferenceChain`](https://github.com/Blackcat-Informatics/purrdf/blob/5d25c3a7d8e76064aa716e3e168d9f443aba7b13/crates/jsonschema/src/error.rs#L72) is a unit variant in an exhaustive public enum; its structured visitor and display hardcode the global constant. The internal validation refusal already carries `maximum`.

## Required change

Introduce an explicit evaluation policy containing regex `MatchLimits` and a reference-chain maximum. Expose caller-selected policy through clearly named entrypoints, with existing convenience methods using the documented default. Retain the existing regex-only methods as default-reference-policy wrappers if practical; otherwise provide a concrete call migration before the API freeze.

Replace the unit evaluation cause with actual attempted and maximum counts, preserving keyword and instance locations in `EvaluationError` and exposing the actual values through structured inspection. Update internal refusal output consistently. A caller-raised limit must not disable true-cycle detection or turn resource exhaustion into a boolean verdict. Descending into a new instance location must retain its current chain-reset semantics.

This issue is scoped to reference-chain policy and its native error payload; it is not a mandate to introduce a general evaluator governor.

## Acceptance

- A valid chain above 250 succeeds under an explicit higher limit and refuses under a lower limit with the actual attempted/maximum values.
- An invalid instance under the same admitted chain yields an invalid verdict rather than an evaluation error.
- True cycles retain their existing behavior independently of the chosen chain maximum.
- `$ref`, `$dynamicRef`, `$recursiveRef`, descent resets and all output formats follow the same policy.
- Existing regex exhaustion retains its original cause and budget behavior.
- Publish the old-to-new cause-pattern migration and any limits-method migration; compile actual public API consumers.
- Report reference visits, cycle-membership work, allocations and retained/peak state as admitted chain length grows. Do not claim raising the limit itself improves algorithmic complexity.

Classification: necessary pre-v4 error-payload break. Attach to #1104 and reconcile the native error surface with #1115. Additive policy entrypoints alone do not remove the need to migrate the existing exhaustive cause variant.

## Verification

Source audit at `5d25c3a7d8e76064aa716e3e168d9f443aba7b13`; existing boundary cases were inspected but not executed. Extend those focused native API cases and compile actual affected consumers under the required repository profile. Preserve mandatory gates and WASM test ownership. Acceptance uses work and resource evidence, without elapsed-time targets or binary-size goals.


## Comments (0)

