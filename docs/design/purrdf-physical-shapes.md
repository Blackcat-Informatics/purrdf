<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Exact physical query shapes

`purrdf_sparql_eval::shapes::ShapeProvider` supplies physical knowledge to the
existing SPARQL evaluator after logical rewriting. A provider borrows the original
subtree and its schema, registered producers, graph scope and native access costs.
It returns an exact immutable `ShapeOffer` or declines with `None`. A storage,
source or contained host failure is an error. After accepting an offer the engine
does not retry its work through another evaluator.

The first registered exact offer wins; the shipped dictionary, ordinal window
and typed calendar group shapes follow the host providers. A host registers its
provider in `ShapeRegistry`, then installs that registry through
`ExtensionEnv::with_shapes`. This changes physical selection only. It does not
register an extension-function namespace, alter parser meaning or create a
second logical planner. Every relation IRI and indexed predicate remains an
explicit caller choice.

## Original rows and execution owners

`ShapeRuntime` evaluates original logical input edges and returns opaque
invocation-local `ShapeInput` and `ShapeRow` references. `input(&[])` evaluates
the offered root literally; child inputs retain the existing logical dispatch.
The host can inspect the original schema and obtain an admitted copy of a bound
term. `emit` joins compatible original rows through the native row builder;
`emit_with` transfers admitted computed terms into the same builder. Neither API
exposes raw mutable rows, a replacement dictionary or an arbitrary evidence ID.

The original row evidence is conjoined by its owning evidence home. Reordering
and passthrough retain the original handles, and the returned bag also retains
the **whole reached search handle**, including an input that produced zero rows.
That search declaration includes reached closure independently of record
completeness. An empty physically searched interval does not declare closed-world
absence. Original EXISTS/NOT EXISTS evaluation still decides that question.

A withheld input prevents complete publication. Native allocations reserve
their actual layouts before growth and retain the original grants through
results, clones and caches. Physical estimates are comparable access work, not
memory certificates or governor fees. `record_work` reports actual visits and
transitions through the existing governor; an original stop is not success.
An opaque host implementation must honor these ownership and exactness contracts.

`EvalOptions::disable_shapes` disables physical selection for differential
execution of the same query, declarations and dataset. Direct, prepared, scoped,
governed, fallible, graph-form, correlated and UPDATE WHERE callers share the
same evaluator installation and dispatch. EXPLAIN reports the selected original
node, provider/operator, estimate and actual source/transition/output counts.
The live registry instance is available separately on `ShapeReport`; it is
process-local and is neither serialized as a semantic identity nor folded into
answer evidence. Independent executions compare canonical evidence-record
identity rather than unrelated query-local numeric IDs.

## Dictionary match

`purrdf_text::dictionary::Dictionary` compiles ordered authored dictionary rows
with the immutable text index's own resolved analyzer. Its automaton is reused
only when original entry identities, phrase spellings, all ordered payload cells
(including UNDEF), and analyzer identity match. The compiler contains no chosen
dictionary-size or phrase-length limit. Representational overflow and physical
capacity refusal are typed failures; an unsuccessful compilation never replaces
the previous successful cache entry.

`DictionaryCache` has an explicit one-entry retention policy. Disabling retention
or clearing it drops the cache's owner, while outstanding dictionary handles and
matches keep theirs. A new index with the same analyzer can reuse the automaton,
which stores no index-local document/token IDs. Each scan binds to the actual
current generation and reads each immutable token occurrence once for the whole
dictionary. `DictionaryWork` separates that scan from transition and required
output work. Hidden graph aliases retain every derivation of the original
subject/language positional join; output multiplicity can exceed physical token
volume.

`PhraseOccurrenceRelation` supplies the literal positional reference and the
dictionary capability under a caller-registered IRI. The existing
`TermOccurrenceRelation` also exposes a native capability for expanded VALUES,
adjacent token joins and UNION dictionaries. The physical recognizer retains
the authored phrase, entry/payload, intermediate token/position columns, original
argument equality checks and branch/driver order. Unsupported analyzers,
producers or logical neighbors decline before source execution. A complete
native occurrence declaration adds no argument-specific loss record; a generic
or lossy producer does not acquire that certificate by spelling the same IRI.

Longest-first non-overlap is an **explicit consumer**, not an automatic
replacement for a query's containment filter. `InternedSolutions::longest_first_spans`
returns a borrowing selection over original rows and their whole search handle.
Within each original domain it visits larger half-open spans first, then start,
end and authored ordinal. Crossing unequal spans compete; identical-span
derivations all survive; adjacent and empty spans do not overlap. It restores
original row order after selection. A strictly-longer containment query has a
different law and remains the original logical query.

## Ordinal windows and barriers

The ordered-window recognizer certifies exact integer/decimal ordinal comparisons
over an actual shared sequence binding. It prepares admitted ordered numeric
indexes in the native BGP home and sweeps each distinct driving ordinal once.
When a subject has one original exact positional cell, its admitted ordinal
index also restricts earlier subject probes before they can materialize the
pair fanout. The projected index retains that cell's original parsed owner;
it does not add bindings or change the selected BGP order. Multiple positional
cells, differing positional subjects and unknown numeric cells retain original
candidate traversal until their actual bindings reach the FILTER.
Invalid, unbound and non-exact-numeric neighbors still reach the original FILTER;
the optimization does not approximate their values or narrow them to machine
ordinals.

Output follows the native BGP's **original probe traversal**, not a new normalized
order. `DatasetView::certifies_probe_order` and its original probe comparator
expose that physical certificate. The shipped frozen dataset provides it,
including original physical, merged graph and virtual RDF streams. A view
without it declines this optimization and keeps full query support through the
literal path.

For the exact between-position barrier, the native correlated evaluator produces
the original retained inner rows and complete reached search handle once per
remaining binding/domain. A checked numeric interval index selects candidate
original rows; the original comparison program still decides their membership.
Empty selections forward the same whole search declaration. Existing ledger
push/pop, governor charges, existential evidence, memo, closure and VM negation
remain in their original owners. An eager outer conjunction still reaches its
original NAF operand even when the window comparison is false. Arbitrary/stateful
residuals cannot silently become index booleans.

An explicit evidence source can declare missing expression support even on an
ordinary false FILTER row. Without a source-provided no-gap certificate, windows
retain every original pair and the between-position barrier keeps the original
inner traversal. The expanded token-occurrence adjacency shape also declines
candidate pruning in that case. Original FILTER, EXISTS and negation owners
decide membership, closure and typed failure; a numeric or positional mismatch
does not substitute for the source's declaration. The direct phrase producer
still uses its certified native dictionary semantics.

## Typed calendar groups

The bucketed-series shape recognizes native `xsd:date` and `xsd:gYearMonth` casts
whose variables participate in GROUP. It calls the same parser, calendar cast,
renderer and term interner as literal evaluation. Exact dateTimeStamp timezone
rules, absent zones, leap and year boundaries, invalid lexical forms, unbound
keys and additional grouping columns are preserved. It neither slices lexical
strings nor approximates months by a duration.

One admitted ordering preparation enables one contiguous grouping pass.
Groups return in original first-seen order; members retain original row order.
The original aggregate body handles every aggregate, including DISTINCT,
rounding-sensitive SUM/AVG, SAMPLE, GROUP_CONCAT, FOLD and caller-defined
accumulators. Contextual folds keep global original step order and error
precedence too. Truncated input is the original opaque GROUP edge, never a
complete aggregate over a partial series.

## Scoped fused acceptance

Ordinary retained rows carry only their original cell storage. Explicit row
evidence uses admitted shared metadata and one flat original-cell owner;
changing metadata shares those cells, and unchanged metadata shares its control.
Attachment is fallible and preserves native refusal before changing ownership.
The supported default evidence record may still carry a producer rank or an
explicit dictionary owner. Those declarations remain attached; only completely
virtual metadata can use the ordinary allocation-free representation.

The explicit scoped search door materializes each original admitted producer
read once. Its receipt retains the declared output columns and the producer's
original possible-candidate rank before scope rejection or established-rank
renumbering. Ordinary search keeps its original on-demand schedule.

Unknown observations enter the same fusion frontier and contribution kernel
as established rows. They retain their original per-stratum ranks, scores,
placement and duplicate-policy checks; they do not enter the contiguous
established-rank protocol or spend the requested established top-k slots.
Scoped composition drains that admitted finite frontier to retain every unknown
candidate's certified fused score and original order. This additional native
residency and read cost belongs only to the explicit scoped door.

The retained fused answer imports original reason descriptions, search frames,
completeness and membership into one admitted query owner. Its acceptance
iterator calls the shared core accepted-merge law over original composition
positions. It preserves contributions and original evidence, adds the explicit
accepted reasons and acceptance identity, and makes no producer call, scoring
pass or fusion pass. Reason validation checks the actual returned answer.
The callback receives borrowed original records; it must inspect those records
through the iterator rather than recursively locking the same evidence owner.

## Reproducing comparisons

The `purrdf-text` `physical_shapes` native and wasm-bindgen test target exercises
the actual public APIs with shrinking dictionaries, windows, calendar groups,
barriers, host offers, scope/evidence, capacity and retained-owner controls.
Its literal comparisons include ordered rows, schemas, RDF lexicals and
serialized SPARQL bytes. The `physical_shapes` report-only bench records
dictionary size/token volume/cache hit and miss, window width/input cardinality,
and typed day/month cardinality. Native source/transition/output and allocation
counts are reported alongside the existing testkit timing estimates.
