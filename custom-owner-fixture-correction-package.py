from pathlib import Path
import difflib
root = Path('/home/paudley/Active/purrdf/.worktrees/508-sparql-eval-complete-bounded-workspace')
stage = root/'.stage/sparql-eval-complete-bounded-workspace'
target = root/'crates/sparql-eval/tests/custom_aggregate_ownership.rs'
base = target.read_text()
text = (stage/'custom-aggregate-owned-entry-fixtures.rs').read_text()
(stage/'custom-owner-fixture-correction.patch').write_text(''.join(difflib.unified_diff(base.splitlines(True), text.splitlines(True), 'a/crates/sparql-eval/tests/custom_aggregate_ownership.rs', 'b/crates/sparql-eval/tests/custom_aggregate_ownership.rs')))

target = root/'crates/sparql-eval/src/modifier.rs'
base = target.read_text()
text = base.replace('fn volatility(&self) -> Volatility {\n            Volatility::Volatile\n        }\n        fn init(&self, _: &[(String, TermValue)])', 'fn volatility(&self) -> Volatility {\n            Volatility::Volatile\n        }\n        fn algebraic_class(&self) -> crate::agg_fn::AlgebraicClass {\n            crate::agg_fn::AlgebraicClass::Commutative\n        }\n        fn state_bound(&self) -> u64 { 0 }\n        fn init(&self, _: &[(String, TermValue)])')
assert text != base, 'the contextual fixture supplement must be applied before this correction'
text = text.replace('Literal::new_simple_literal(value)', 'Literal::new_simple(value)')
(stage/'modifier-custom-fixture-fixed.rs').write_text(text)
(stage/'modifier-custom-fixture-baseline.rs').write_text(base)
