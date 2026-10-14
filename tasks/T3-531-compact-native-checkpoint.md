# #531 compact continuation: focused native checkpoint

The current uncommitted two-file candidate is `crates/sparql-eval/src/{expr.rs,binop.rs}` on `330c55f8c`. Source SHA-256 at this checkpoint: `expr.rs de15e29c07f3da9dd409a70a8d2a41be25828391206c3657e35ed13d2f462fe8`; `binop.rs 25107e43405d67ed5c4695c8ef367b31ae43a89686fb2f751ca33d86857e73b3`. The code moves retained FILTER and LATERAL state into existing admitted `WorkspaceBox` owners constructed out of line. The FILTER consuming iterator is retained behind the box while its predicate recurses, then released before output finish. No test, budget, helper ledger or tooling was changed.

Direct standard Cargo, four build jobs and four test threads:

- `cargo clippy --locked -p purrdf-sparql-eval --lib -- -D warnings`: PASS (`T3-531-compact-strict-final.log`). The first two compiler receipts show corrected borrow/iteration diagnostics; they are not passes.
- `cargo test --locked -p purrdf-sparql-eval --lib filter_`: 39/39 PASS (`T3-531-compact-filter-native.log`).
- `cargo test --locked -p purrdf-sparql-eval --lib lateral_`: 18/18 PASS (`T3-531-compact-lateral-native.log`).
- `cargo test --locked -p purrdf-sparql-eval --test lateral_e2e`: 20/20 PASS, including truncation and correlated EXISTS (`T3-531-compact-lateral-e2e.log`).
- `cargo test --locked -p purrdf-sparql-eval --test evaluation_stack`: 10/10 PASS, including the existing thousand-LATERAL allocation ceiling (`T3-531-compact-stack-native.log`). Its prior one-test focused run also passed (`T3-531-compact-lateral-allocation.log`).
- `cargo test --locked -p purrdf-sparql-eval --lib exists_`: 84/84 PASS, including definition/deferred/forced strategy, cache, evidence, admission and stateful controls (`T3-531-compact-exists-native.log`).

The independently reviewed source has no remaining identified ownership or first-error-order blocker (`../reviews/issue-531-compact-continuation-design-review.md`). These native tests do **not** establish a stack-floor improvement or acceptable default allocation/work cost. One new admitted box per ordinary FILTER/LATERAL is real. Fresh standard optimized WASM packaging, unchanged three original floors and stack-byte controls, and a fixed-input allocation/instruction comparison remain required. The original latest failed floors are synchronous SELECT 135 < 181, LATERAL 234 < 281 and FILTER NOT EXISTS 133 < 180. Do not claim #531 fixed or commit this candidate from this checkpoint.
