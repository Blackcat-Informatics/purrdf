Native XPath regular expressions with operational resource refusal

Issue summary

Implement the complete additive dated XPath pattern surface requested by #406. A pattern's syntax verdict and an exhausted compiler/matcher budget must remain different outcomes through SPARQL, SHACL and ShEx. JSON Schema retains its ECMA-262 engine and accepted-pattern behavior.

Primary profile definitions: [XPath F&O 2.0 Second Edition, 2010-12-14](https://www.w3.org/TR/2010/REC-xpath-functions-20101214/#regex-syntax) and [XPath F&O 3.1, 2017-03-21](https://www.w3.org/TR/2017/REC-xpath-functions-31-20170321/#regex-syntax). Replacement behavior includes the [3.1 fn:replace error conditions](https://www.w3.org/TR/2017/REC-xpath-functions-31-20170321/#func-replace).

Baseline defaults

Read `.baseline`: adversarial review; complete the requirements on this branch; own discovered defects; no development-process references in shipped documentation. `.deficiencies` is notice-only and must remain so.

Standing constraints

Read `.goals` and AGENTS.md: original first-party Rust, one home per job, deterministic RDF 1.2 behavior, bounded typed hard failures, shared lexical terminals, no semantic Cargo feature or new dependency, Rust semantic/conformance tests, and WASM portability. Commit/merge hooks are mandatory. All toolchain use is ordinary repository build/test/lint use: no investigation, installation, selection, adapters or configuration changes. Preserve every other agent's branch/worktree and excluded PRs.

Completeness contract

| Requirement | Task and proof |
| --- | --- |
| Explicit dated XPath profiles | T1/T2: a sibling native compiled-pattern type/API names XPath F&O 2.0 Second Edition (2010-12-14) and 3.1 (2017-03-21); valid/invalid neighboring grammar vectors distinguish the profiles. T4 declares and verifies selected-profile routing and unchanged unselected legacy routing. |
| Grammar and matching, including backreferences, non-capturing groups and q | T2/T3: complete applicable XSD/XPath productions, ordered captures/backreferences and all applicable flags; native byte-exact matching/capture/replacement vectors and independent small-language enumeration. XPath 2.0 already includes backreferences; only 3.1 admits non-capturing groups and q. |
| Finite VM limits | T1/T3: finite source, parser/program storage, execution steps, active states/slots and replacement output admission; native tests demonstrate each boundary and its valid neighbor. |
| Oversized patterns are resource refusals | T1/T2/T4: distinct typed resource cause before source materialization; never invalid syntax, UNDEF, negative match or a conformance finding. |
| Operational errors through SPARQL/SHACL/ShEx | T4: additive profile selection and typed fallible APIs; direct, constant-linked, dynamic/cached and nested/negated execution tests establish propagation and precedence. Reused programs/cache entries remain bound to their dated profile and compile admission; current-request bounds are admitted before a cache hit. |
| Unicode data from generator only | T2: the existing Unicode generator emits XPath category and full-case-variant data from the admitted vendored UCD; no runtime standard-library/regex Unicode inference, copied arrays or additional generator. |
| JSON Schema behavior unchanged | T5: ECMA parser/VM and existing limits remain unchanged; every existing native JSON Schema suite and its matcher tests pass. No shared VM migration is required. |
| Additive API only, no closed-enum variants | T1/T4/T6: existing public struct literals, closed enums, CompiledPattern/as_regex and legacy entry points remain source-compatible. New error wrappers/configuration entry points carry operational refusals; only already non-exhaustive errors may gain a variant. Native downstream compile fixtures and cargo semver-checks verify the surface. |
| make check and make wasm | T6: ordinary full local gate and all release-crate WASM portability builds, with actual source-bound results. No general semantic WASM executions are added. |
| PR, review, merge and issue closure | T7: final-head CodeRabbit/CI, normal main integration, concrete squash notes, ghprsq, signed result/tree/audit/closure readback. |

Enhancement audit and decisions

Transformation adopted (M/L): treat a dated pattern as a small admitted program with separate grammar and execution verdicts, rather than rewriting backreferences into a finite automaton or hiding exhaustion as a failed match. Use a flat arena/program and explicit work lists, including construction/drop/debug, so hostile nesting does not consume the machine stack.

Reuse adopted (S/M): the existing xsd_regex module remains the XPath home; reuse XML terminal tables, source whitespace scanner, block-name definitions, replacement token rules, shared work-list structures and the one Unicode generator. Character-set semantics stay profile-specific, including XPath's non-transitive full-case-variant relation and subtraction/negation rules.

Utility adopted (M): one matching/capture primitive feeds both matches and replacement, so SPARQL REGEX and REPLACE carry the same dated law. Configuration is private-field/additive builder or sibling-entry-point based; existing options structs and closed result types keep their shape.

Robustness adopted (S/M): budget admission precedes allocations and syntax erasure; loops and byte comparisons consume work; pending states account for captures/continuations as well as state count; zero-width repetition has a declared progress rule; constant linking, cache hits, nested validation, negation and alternate branches cannot erase an operational refusal. Replacement expansion is bounded.

Compatibility and routing decision: the public legacy `CompiledPattern::as_regex() -> &regex::Regex` and `Constraint::Pattern.compiled` cache remain exactly their existing types. Introduce sibling `xsd_regex::xpath::CompiledPattern` and explicit `compile(profile, pattern, flags, limits)` in that module. Existing unselected entry points keep the compatibility law; a caller explicitly selecting a dated native profile always uses the native program and typed fallible execution. SPARQL configuration lives on private engine/context state with additive setters; SHACL/ShEx expose sibling fallible validation/preparation entry points rather than changing closed error/result/options types. Native private caches cannot put backreference programs or budget errors into the legacy compatibility cache.

Cache admission decision: native programs carry immutable profile/source/flag identity and compiler admission metadata. Source and stored program/storage bounds are checked for the current request before reuse. Native memo keys include the dated law; alternating a prepared product between 2.0/3.1 cannot reuse a program admitted under the other grammar. Current matching budgets are never inherited from an earlier successful request. Syntax failures may be memoized only under the correct dated law; resource failures remain typed request outcomes and never become `None`, a cached syntax verdict or an `Arc<str>` facet finding. Low/high source/program/storage/runtime budgets and both profiles are exercised over the same prepared shape and constant/dynamic pattern. Skipped compilation is not charged as execution work, but cached artifacts must still satisfy the current admission contract.

Declined: migrating JSON Schema's distinct ECMA engine into the XPath VM. It has different capture/lookaround/property/flag semantics and current budget guarantees; migration adds compatibility risk without being needed for this complete XPath capability. Its native behavioral corpus remains an explicit regression gate. No third-party regex implementation, new crate or runtime dependency is needed.

Task 0: isolated ownership and baseline

Refresh local branches, worktrees, live issue/comments and open PR ownership immediately before creation. If still unowned, create `paudley/406-native-xpath-regex` at current origin/main in `.worktrees/406-native-xpath-regex`; preserve all siblings and root main. Record the exact base, inspect the fresh worktree contracts and establish relevant existing native regex behavior. Post this reviewed plan to issue #406.

Task 1: additive contracts and refusal classification

Introduce dated profile, finite limits and new typed syntax/resource errors for the sibling native compiled-pattern surface in the existing XPath home. Keep every legacy closed type source-compatible; declare the legacy/native routing rule above. Specify admission units/default budgets and artifact admission metadata, with meaningful refusal/valid-neighbor coverage. Validate focused core tests/fmt/lints, then normal commit, push and issue receipt.

Task 2: grammar and generated Unicode

Implement all applicable productions and exact terminal/flag rules over a flat native representation. Validate quantity/backreference numbering, closed earlier captures, character-class subtraction, permitted escapes/property/block names, profile differences and malformed neighbors. Extend only the existing generator for native XPath Unicode categories and full-case variants; generated output has reproducible provenance and version checks. Validate grammar/generation/hygiene, then normal commit, push and issue receipt.

Task 3: native execution and replacement

Implement ordered matching/captures/backreferences and replacement over the admitted representation with explicit continuation/state storage and finite budgets. Cover UTF-8 boundaries, repeated/absent captures, greedy/reluctant order, empty alternatives/repetition, anchors including final-newline behavior, class negation/subtraction, non-transitive case variants, q interactions and XPath replacement errors, including FORX0003 refusal of a pattern that matches the empty string. The empty-match check itself preserves an operational budget error. Exhaustively enumerate independent small-language matching oracles and resource-bound neighbors. Measure the affected existing/native parse/match paths before claiming any performance result. Validate, normal commit, push and issue receipt.

Task 4: all three host engines

Add explicit dated-profile selection to NativeSparqlEngine/EvalCtx and additive fallible SHACL/ShEx validation/preparation entry points. Route native caching and constant linking through the selected law and current admission without changing the legacy public compiled-pattern/cache fields. Carry explicit configuration through prepared, bound and restored validation paths, constant-linked expressions and every private evaluator context/fork; selecting a native profile must not end in a compatibility free-function facade. Retain syntactic expression/facet semantics; resource causes use the operational channel and outrank partial results, negation and alternate branches. Native end-to-end tests cover REGEX/REPLACE, linked/dynamic/cached paths, SHACL pattern evaluation, ShExJ patterns, nested shapes and limit refusals. Reuse each prepared shape and constant/dynamic pattern under alternating 2.0/3.1 profiles and low/high admission/runtime bounds, proving that cached syntax and prior successful admissions cannot erase the current refusal. Review every context/fork and the public wiring independently. Validate, normal commit, push and issue receipt.

Task 5: independent conformance and compatibility

Run existing XSD/XPath/SPARQL/SHACL/ShEx conformance plus the full JSON Schema native suites. Add dated first-party vectors and independent oracles without changing frozen payloads/expected results or masking failures. Document profile selection, grammar/Unicode provenance, finite limits and typed error handling in their existing homes, without process references. Regenerate affected projections normally. Validate, normal commit, push and issue receipt.

Task 6: complete source qualification

Run ordinary `cargo semver-checks`, `make check` and `make wasm` against the final source, plus generated/helper/layer/terminal/non-Rust hygiene. Read exit statuses and complete task-owned logs. Run mechanical deferral and deficiency checks; independently map every requirement to actual evidence, preserving pre-integration versus final-head identities. Repair concrete findings and use normal commits/hooks.

Task 7: PR through protected integration

Open a ready PR with a plain project title, source-bound validation and `Closes #406`; attach the implementation plan. Read actual final-head CodeRabbit comments/threads and all hosted checks, fix findings, synchronize origin/main normally and qualify affected source. Every feedback/integration repair passes normal verification, is committed/pushed and receives remote readback; final qualification is bound to that pushed head. Prepare and post concrete squash notes. Acquire the root serial merge lock and run only `/home/paudley/stage/root/bin/ghprsq` from the exact clean PR worktree. Verify signed result, source-tree equality, base/head/result/evidence refs, notes, PR state and issue closure, then remove only this owned worktree/branch.

