# T7 triage: the six decimal-spelling cases

Six approved `sht:EvalNodeExpr` cases of the SHACL 1.2 test suite
(`w3c/data-shapes`, `shacl12-test-suite/tests/node-expr/`, checkout
`4cd469867a1cc6d2fff80e8a40723b26c695d67a`) were graded under an amended
expectation: the approved `mf:result` spells a computed `xsd:decimal` in a
lexical form PurRDF did not produce, and the harness compared against the XSD 1.1
canonical spelling instead. Each section is written so it can be lifted into a
Working Group discussion item with minimal editing.

**Result: all six are engine defects, fixed. None meets G1 (1), so none enters
`SPEC_CONTRADICTED`.** No text mandates a lexical form for a computed numeric
literal that is incompatible with the approved spelling, so a conforming processor
can produce each approved term exactly, and PurRDF now does. The amended-grade
path (`NON_CANONICAL_EXPECTATIONS`, the `non-canonical-expected-decimal` label) is
removed; the six are graded against the unamended `mf:result`, term for term.
`w3c12_conformance`: `W3C12 TOTAL: passed 591, xfailed 0, ledger 0`.

## The six cases

| Case (under `node-expr/`) | Expression | Approved term | PurRDF before | PurRDF now |
|---|---|---|---|---|
| `shnex-sparql/ceil-example` | `sparql:ceil ( 3.2 )` | `"4.0"^^xsd:decimal` | `"4"` | `"4.0"` |
| `shnex-sparql/floor-example` | `sparql:floor ( 3.8 )` | `"3.0"^^xsd:decimal` | `"3"` | `"3.0"` |
| `shnex-sparql/round-example` | `sparql:round ( 3.7 )` | `"4.0"^^xsd:decimal` | `"4"` | `"4.0"` |
| `shnex-sparql/divide-example` | `sparql:divide ( 84 2 )` | `"42.0"^^xsd:decimal` | `"42"` | `"42.0"` |
| `shnex-sparql/seconds-example` | `sparql:seconds ( "2023-12-25T10:30:00"^^xsd:dateTime )` | `"00"^^xsd:decimal` | `"0"` | `"00"` |
| `shnex/sum-totalRevenue` | `shnex:sum` over revenues `4.2` and `37.8` | `"42.0"^^xsd:decimal` | `"42"` | `"42.0"` |

In every case the value was already right; only the spelling differed.

## What the texts say about a computed literal's lexical form

1. **SPARQL 1.2 Query, "Filter Evaluation" (function and operator mapping).**
   "RDF literals passed as arguments to these functions and operators are mapped to
   XML Schema typed values with a string value of the lexical form and an atomic
   datatype corresponding to the datatype IRI. The returned typed values are mapped
   back to RDF literals the same way." A typed value (a number in the value space)
   has no lexical form; the mapping back does not say which of the lexical
   representations of the value the literal carries.
2. **SPARQL 1.2 Query, the operator table.** "A / B | numeric numeric |
   op:numeric-divide(A, B) | numeric; but xsd:decimal if both operands are
   xsd:integer". It fixes the datatype of `84 / 2` (decimal) and nothing about its
   spelling.
3. **SPARQL 1.2 Query, numeric functions (the dated text's own examples).**
   `ROUND(2.5)` is shown as `3.0`, `ROUND(-2.5)` as `-2.0`, `CEIL(10.5)` as `11.0`,
   `CEIL(-10.5)` as `-10.0`, `FLOOR(10.5)` as `10.0`, `FLOOR(-10.5)` as `-11.0`.
   The specification's own results for the decimal inputs keep a point and a
   fractional digit, which is exactly the spelling the approved tests have.
4. **SPARQL 1.2 Query, `SECONDS`.** "Returns the seconds part of the lexical form
   of arg." and the example `SECONDS("2011-01-10T14:45:13.815-05:00"^^xsd:dateTime)`
   is `13.815`. The seconds field of the lexical form `2023-12-25T10:30:00` is
   `00`, so the approved `"00"^^xsd:decimal` is what the function's own definition
   yields. (`00` is a valid `xsd:decimal` lexical representation.)
5. **SPARQL 1.2 Query, aggregate `Sum`.** `SumList(L) = op:numeric-add(L1,
   SumList(L2..n))`: defined over values; no lexical form.
6. **SHACL 1.2 Node Expressions, SPARQL function expressions
   (`#sparql-function-evaluation`).** "The evaluation follows the SPARQL semantics
   for the corresponding SPARQL function." No further statement about the output
   literal.
7. **SHACL 1.2 Node Expressions, sum expression.** "The output nodes of the sum
   expression is the list consisting of exactly one node that is computed as the sum
   of all nodes from N, see SPARQL SUM." Its worked example says: "For the
   following data graph, the sum will be `42.0`." (with revenues `2.1`, `37.8`,
   `2.1`). The informative text spells the sum with a point and a fractional digit,
   the same spelling the approved test `sum-totalRevenue` expects.
8. **XSD 1.1 Part 2, section 2.3.** "Canonical representations are provided where
   feasible for the use of other applications; they are not required for schema
   processing itself. A conforming schema processor implementation is not required
   to implement canonical mappings." A canonical mapping exists, but nothing makes a
   computed result use it. The same Part 2 says an `xsd:decimal` value has many
   lexical representations (`4`, `4.0`, `04`) that all denote the one value.
9. **Comparison rule of the suites.** The node-expression suite README says the
   test harness must "verify that the output nodes are equal to the list mentioned
   in mf:result" and says nothing of numeric comparison. SHACL Node
   Expressions state "Nodes are compared using term equality, i.e. `"01"^^xsd:integer`
   is distinct from `"1"^^xsd:integer`" for their own operations. The SPARQL 1.1 /
   1.2 suites' expected results spell whole decimals both ways (`"3"` in
   `functions#ceil01`, `"2.0"` elsewhere), and PurRDF's SPARQL harness compares
   numeric result cells by value for that reason. A harness comparing term for term
   (the only rule the README text and the Node Expressions term-equality language support)
   makes the approved spelling the only passing one.

## Conclusion per case (G1 (1))

Does a text fix the computed literal's lexical form to something other than the
approved spelling? No, for all six.

- **ceil, floor, round, divide, sum:** the texts leave the form to the processor,
  and where they show a form (SPARQL examples in item 3; the node-expression sum
  example in item 7) it is the approved one. A processor that spells a computed
  decimal with a point and a fractional digit conforms and passes. PurRDF's earlier
  XSD 1.1 canonical spelling was a choice, not a requirement (item 8).
- **seconds:** item 4 yields `00` directly.

So the cases are fixed, not tabled. The SPARQL 1.1 `ceil01`-style expectations
that spell `"3"` do not conflict: that suite is graded by value, and its own
results spell whole decimals inconsistently.

## What changed in the engine

One rule, applied wherever the engine computes a number, in `purrdf-xsd` and the
SPARQL evaluator that node expressions run through:

- A **computed `xsd:decimal`** (the results of arithmetic operators, unary minus,
  `CEIL`/`FLOOR`/`ROUND`/`ABS`, `SUM`, `AVG`, and the statistical extension
  aggregates) is spelled with a decimal point and at least one fractional digit:
  whole values `4.0`, `0.0`; values with a fraction unchanged (`2.5`, `-0.025`).
  Implemented as `PreparedDecimalText::with_required_point` and
  `purrdf_xsd::numeric::prepare_computed_numeric_text`, selected by the
  evaluator's one producer (`xsd_workspace_value`), so lexical sizing and
  admission accounting stay exact.
- **`SECONDS`** returns the seconds field of the argument's lexical form (at least
  two integer digits, fraction kept): `purrdf_xsd::numeric::prepare_seconds_field_text`.
- Unchanged: casts and constructors (`xsd:decimal(...)`) and unary plus keep the XSD
  1.1 canonical text, `xsd:integer`, `xsd:double`/`xsd:float` spelling, the
  canonical text of a stored value, and the cast-to-string text of
  `prepare_xpath_text` (XPath `fn:string` of a decimal remains `4`).
- Pinned consequences, regenerated with the repository's own procedures: the
  evaluator-trace goldens (computed whole decimals and the scratch-byte cost of the
  extra two characters), the governor corpus vectors for `SECONDS` (`05.123...`
  now keeps its leading zero, one more scratch byte) with its freeze manifest and
  `GOVERNOR_CORPUS_DIGEST`, the numeric transcript digest, and the unit and
  integration expectations of computed decimals.

## Candidate editorial clarifications

For the Working Group (SHACL 1.2 Node Expressions / test suite):

1. **State the comparison rule in the node-expression test README.** It says the
   output must be "equal" to `mf:result` and does not say whether that is RDF term
   equality or value equality. With term equality, six tests pin down a spelling
   that no normative text fixes; with value equality they pass for any spelling.
   Either is workable; the README should say which.
2. **If term equality is intended, say how a processor learns the computed
   spelling.** The SPARQL text maps typed values "back to RDF literals the same
   way" without choosing a lexical form. A sentence in the node-expression
   function-evaluation text ("a computed numeric literal uses the lexical form of
   the examples in SPARQL Query") or a note in the test README would make
   `42.0` and `4.0` an explicit requirement rather than a convention that happens
   to match the examples.
3. **`seconds-example`: `"00"^^xsd:decimal` is surprising.** It is what "the
   seconds part of the lexical form" yields, but it is not a form any decimal
   canonical mapping produces. Consider noting this in the test or the SPARQL
   text, since an implementation computing `fn:seconds-from-dateTime` as a decimal
   value gets `0`.
4. **SPARQL Query examples vs. Node Expressions examples** already agree (`11.0`,
   `42.0`); the SPARQL 1.1/1.2 test suites' expected `"3"` for `CEIL(2.5)` does
   not. A single stated convention would remove the inconsistency.

Corners for #384's proposed-test candidates: a node-expression test of
`sparql:ceil ( 3 )` over an `xsd:integer` (result stays `xsd:integer`, spelling
`3`), one over `xsd:double` (`4.0E0`), and a `sparql:divide ( 1 3 )` whose
decimal result has a fraction, would pin the rule's boundary.

## Addendum: two failures found on the branch

1. **`cdt_lexical_divergence`.** The scanned-file pin moved 4122 to 4161 because the
   re-vendor of `vectors/shacl12` raised the count of `.rq/.ru/.ttl/.trig/.nt/.nq/.n3`
   tracked files there from 442 (base `410a927d9`) to 481 at HEAD: exactly +39. No
   file under `vectors/shacl12` mentions the SEP-0009 namespace, so the new files add
   no composite literal; the test's substantive assertions (959 recovered literals,
   949 well formed, 13 lexer refusals, all inside SEP-0009) pass unchanged. Only
   `EXPECTED_TOKENIZED_FILES` was re-pinned.
2. **`shapes_tools_cli::cli_rules`.** The fixture `nesting.srl` is
   `RULE { ?x ex:n <<( ?x ex:n ?y )>> } WHERE { ?x ex:n ?y }`. SPARQL 1.2 RL: "rules
   with a template that involves a triple term containing variables are run-once
   rules", run "after all the rules that could produce data that they depend on";
   a closed dependency "ensures that rule R2 has generated all its possible output
   before rule R1 is executed". The rule's body matches its own head predicate, so it
   depends on itself through a run-once rule: a cycle through a closed dependency,
   correctly refused as not stratifiable. This is case (a): the engine is right and
   the test was stale. The CLI test now asserts the refusal (exit 1, "not
   stratifiable") and keeps a stratifiable neighbour (`ex:m` head, same body) that the
   CLI accepts under the same `--max-term-generating-rounds`.

## Addendum (Stage 3, Task 18): corrections to the text above

- **SECONDS.** It returns the seconds field of the argument's lexical form
  exactly as written (`05.500`, `00.250`, `00`), read by
  `purrdf_xsd::temporal::seconds_field_text`. This term conflicts with the SPARQL
  1.1 test `functions/seconds-01`, which expects `"1"`, `"2"`, `"0"`, `"3"`. That
  test grades by value, so both suites pass. No term-exact rule satisfies both
  `seconds-01` and the SHACL `seconds-example` (`"00"`). This is recorded in the
  WG findings draft, item 2.
- **Unary plus** keeps its operand's spelling: XPath `op:numeric-unary-plus` is
  the identity, and the W3C `result-unplus-2.srx` keeps `3`. The earlier
  statement that unary plus "keeps the XSD 1.1 canonical text" is superseded.
- **Scope of the spelling rule.** The ".0 for a computed whole decimal" rule
  covers the arithmetic in `purrdf-xsd` and `purrdf-sparql-eval`. The scores
  from `purrdf-text` and `purrdf-retrieval` are `xsd:decimal` values with their
  own fixed 12-digit fraction, so "one rule wherever the engine computes a
  number" overstated it.
