<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Volatile oracle witness diagnosis

Read-only independent diagnosis of the genuine rdflib 7.6 files under `/home/paudley/Active/purrdf/bindings/python/.venv/lib/python3.13/site-packages/rdflib/plugins/sparql/`. No execution, installation, source/environment mutation or forge call. The root reports zero callback invocations with both `counter()` and `counter(?value)` inside an OPTIONAL RHS consisting only of a BIND. The retained `raw/graph-values-group-volatile-oracle.json` explicitly records zero for the empty-argument cases. The variable-argument observation was supplied by the root; this analyst did not execute it.

## Two distinct pre-call failures

**Empty argument list:** `parser.py:1013` admits NIL as ArgList. NIL is not a named `expr` ParamList; `Comp` ignores unlabelled tokens (`parserutils.py:232`). Consequently a Function node for `counter()` has no expr member and `CompValue.__getattr__` returns None for it. `operators.py:643` Function finds the registered function but its normal raw=False branch executes `func(*e.expr)`. Expanding None raises TypeError before invoking the callback. That TypeError is translated to SPARQLError in Function; `Expr.eval` returns the error, and `evalExtend` preserves the old row on SPARQLError. Zero invocations therefore does not establish successful RHS evaluation or absence of duplicated evaluation.

**Incoming variable argument:** `evalLeftJoin` thaws each left row into the RHS context. With a RHS BIND over an empty BGP, the empty BGP returns that incoming context's solution. `evalExtend` evaluates its expression over `c.forget(ctx, _except=extend._vars)` (`evaluate.py:131`). `_addVars` excludes Extend.expr from the bound-variable summary (`algebra.py:526`): for this BIND-only RHS, the assignment target belongs to `_vars`, but ?value belongs only to the expression. `FrozenBindings.forget` (`sparql.py:214`) removes ?value because it was already bound in the before-context, is not in `_except`, and is not an initBinding key. While Function evaluates `e.expr`, `parserutils.value` recursively looks up list arguments and raises NotBoundError for the removed variable. The callback is again never reached. An initBinding for this unrelated ?this does not retain ?value.

These failures occur before callback invocation, and both are converted into an expression error whose Extend answer is the original row. The observed unbound ?call is consistent with that route. Registration failure is not required to explain either result.

## Exact replacement witness

Use a literal argument and a callback that explicitly accepts it:

`SELECT * WHERE { ?s <http://example.org/p> ?value OPTIONAL { BIND(<http://example.org/counter>(1) AS ?call) } }`

Register raw=False with a one-argument callback that increments a counter and returns the new count as an RDF Literal. Literal 1 is unaffected by forget and produces a real expr argument list. Under the inspected `evalLeftJoin` successful RHS route, each of the three left rows should invoke it once and bind ?call; the remembered-scope retry should not run because the TrueFilter matches. This is a source-derived prediction requiring the root's actual execution, not a pass. Reset the counter per query and record final count, all three bound callback values and complete output rows. A test that only sees three output rows could still have taken the preserved-error route.

For an assignment target that is also an initBinding, repeat the same literal-argument query with `AS ?this` and initBindings this=a. The returned mapping can restore a from the left; callback_invocations remains the witness that the RHS executed successfully. Do not infer invocation from the final restored value alone.

An alternative independent control is raw=True with a callback accepting `(expression, context)`: Function calls it before reading `e.expr`, so empty-argument syntax avoids the raw=False expansion failure. The callback must use the raw signature and can return a term without reading an unbound variable. Literal-argument raw=False is the smaller repair to the current witness and directly exercises normal argument evaluation.

## Verdict

ROOT CAUSE CORROBORATED BY SOURCE; replacement witness execution NOT RUN by this analyst. The zero-call receipts remain honest failed witnesses and must not be relabeled as successful volatility qualification. Retain them and link the actual successful literal-argument receipt when produced.
