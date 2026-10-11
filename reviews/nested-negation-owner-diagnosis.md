# Nested negation creates invocation-local evidence owners

Source diagnosis only. No build, test, timing, process change or source edit.
The observed four W3C failures are retained in
`recovery-native-final-gates-environment.log` around line 22183. The separate
ENF structural-clone assertion is outside this review.

## Actual cause and path

`QueryEvidence::owns/combine` (`evidence_exec.rs:1304–1340`) correctly refuses
records from different dictionaries. The ownership fork occurs earlier:
`RowEvidence::native_existential` at line 1487 calls `QueryEvidence::new` for
**each empty invocation** whose search record has no owner. `native_negative`
at line 1510 has the same fallback for a virtual input. Neither fallback is
bound to the original evaluator query.

The actual conformance route installs the nonempty `harness_relations()` registry
through `sparql-conformance/src/run.rs:745`, including for queries which invoke
none of those relations. Consequently `EvalCtx::may_have_evidence`
(`eval.rs:3352`) is true, `vm/mod.rs:1057` requests `exists_evidence`, and
`EvalCtx::existential_evidence` (`eval.rs:3367`, no explicit evidence source)
uses that native fallback.

For a missing member, `QueryEvidence::empty_existential` materializes the
Neither record in a fresh dictionary. `vm/evidence.rs:99–107` negates through
that owner. The retained true `NOT EXISTS` result still carries the dictionary.
Another absent-member invocation creates another dictionary, although both
are part of the same outer query. `binop.rs:3177` combines compatible MINUS
witnesses with Alternatives and rejects their different owners. Sequential
filters can meet the same error at `expr.rs:196`, which combines a row's prior
record with the next computed record.

`eval_correlated` (`binop.rs:409`) passes the existing mutable context into
substitution; it is not itself creating a new evidence query. Worker contexts
do exist in the filter path (`expr.rs:263`), so the repair must also survive a
fork made before the first lazy record exists.

Exact fixture files are under
`crates/sparql-conformance/suite/w3c-sparql11/negation/`:
`set-equals-1.rq`, `subset-01.rq`, `subset-02.rq`, `subset-03.rq`.
They combine MINUS with member-test NOT EXISTS; subset-02 additionally has
two consecutive NOT EXISTS filters, and subset-03 nests MINUS operands.
The log is an observed failure; individual throw-site attribution above follows
the source path rather than a newly collected stack trace.

## Smallest coherent repair boundary

Evaluator-generated empty/negative records must resolve through one lazily
admitted original-query owner, carried across correlated windows, worker forks
and child contexts. Reuse the existing query-evidence/admission home and its
native producer adopt/import boundary. Do not add a second dictionary per
expression, weaken `owns`, import arbitrary foreign records inside `combine`,
or disable the harness registry to hide the failure.

A plain `Option<QueryEvidence>` cloned while still None is insufficient:
workers would independently initialize it again. Lazy ownership must be shared
before those forks can create records. Preserve the no-payload ordinary path;
do not eagerly allocate a dictionary for every ordinary query. Existing native
producer owners must be adopted or explicitly imported once at the admitted
query boundary, as `EvidenceInput::bind_native_evidence` already specifies,
rather than becoming incompatible with the new evaluator owner. Genuine
independent query dictionaries must still fail direct combination.

Focused verification after repair: the complete existing W3C negation manifest
with the same harness registry (all four observed failures included); existing
`native_table_rejects_foreign_query_ids` and
`default_rows_do_not_materialize_a_table`; nested/sequential empty EXISTS with
ordinary and actual native evidence inputs; a worker fork before first evidence
materialization; bounded admission failure and last-owner release. Preserve
semantic false/true, Neither/negative polarity and incomplete-search refusal,
not merely the absence of the internal error. The mandatory full gate still
needs its normal repaired-source completion.
