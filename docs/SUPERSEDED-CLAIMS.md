<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Superseded claims

A claim this repository once made in its own source, and no longer makes.

## Why this file exists

A doc comment that states an invariant is a conclusion someone reached with the code
in front of them. When the code moves, the honest edit is to rewrite the comment —
and a rewritten comment leaves no trace that the old reading was ever the rule. The
next reader who finds the old claim quoted in a design note, a downstream project, or
a review comment has no way to tell whether it was wrong, whether it changed, or
whether they are looking at a bug.

So a reversal gets recorded here as well as fixed in place. Each entry names the claim
as it stood, why it was believed, what changed, and where the new rule lives. This is
not a changelog: it holds only claims that were once *stated as invariants* and are now
false.

It is **not** an authorization surface, and nothing here excuses undone work. An entry
means a claim was corrected, never that a defect is accepted.

## The claims

### A prepared shapes product cannot carry host relations

**Was stated in** `crates/shapes/src/product/mod.rs`, on `ShapesProfile::CORE`:

> What the profile genuinely excludes is the one binding no shapes graph can describe:
> a product of this profile is prepared against the EMPTY property-function registry,
> because `sh:sparql` bodies that call host relations depend on wiring the shapes graph
> cannot state and a product therefore cannot carry.

**Why it was believed.** A relation registry is host wiring. Nothing in a shapes graph
declares it, so a restoring process cannot rebuild it, and the profile took that to
mean a product could not be *bound* to one either.

**What changed.** The premise conflated two different things: what a shapes graph can
describe, and what a product can state as a requirement. The identity rows exist
precisely for the bindings a shapes graph cannot describe — that is what the
implementation-identity row already did for native functions. A product binds the
environment it was prepared against, and `PreparedShapes::to_product_for_host` is how a
host that wired a relation or declared a namespace says which.

**The rule now.** A product binds the environment it was prepared against.
`to_product` was prepared against nothing, so restoring it under a host that wired a
relation is still refused — a fact about which writer a caller used, not a capability
the profile lacks. Pinned by
`product::tests::a_product_written_for_a_host_restores_under_that_host_s_relations` and
its empty-registry sibling.

### A SPARQL-bodied function's body is deliberately not folded into its identity

**Was stated in** `crates/sparql-eval/src/user_fn.rs`, on `content_fingerprint`:

> a SPARQL-bodied function's parsed body is deliberately not folded either: the only
> stable byte form available for it is the algebra serializer's rendering, and making
> that load-bearing would turn a cosmetic wording change in an unrelated module into a
> silent invalidation of every persisted artifact.

**Why it was believed.** `UserFunction::body` held parsed algebra, and the only way to
get bytes out of algebra was the serializer — whose output is a rendering choice, not a
fact about the function.

**What changed.** The objection was sound and is answered rather than overruled.
`UserFunction::body` is now the declaration's own SPARQL **text**, which no serializer
can reword. Folding it closes a real hole: two registries whose functions declared
identical parameters, arities and return types but carried different bodies digested
identically, so a persisted artifact bound to one would open against the other.

**The rule now.** The `Declared` population folds the body text, canonicalized. It
still does **not** fold the environment a body was later bound against, because a
`Declared` entry must stay rebuildable at restore from the shapes graph alone — folding
host wiring in would refuse a valid restore. Pinned by
`the_declared_digest_separates_two_identical_declarations_with_different_bodies` and
`binding_does_not_move_the_declared_digest`.

### A custom function IRI is resolved dynamically, never at parse time

**Was stated in** `crates/sparql-eval/src/engine.rs`, justifying why the user-function
registry is excluded from `check_plan_matches_relations`:

> `Function::Custom` is resolved dynamically at evaluation time (never at parse time,
> unlike a property-function predicate or a `Custom` aggregate's admission).

**Why it was believed.** It was true of every function kind at the time: a call-position
IRI was looked up in the registry while an expression was being evaluated, so no plan
could depend on the table.

**What changed.** A SPARQL-bodied function's body is now parsed and feasibility-ordered
against an extension environment before evaluation begins, so that half of the registry
*is* parse-time-bound.

**The rule now.** The two resolutions are distinguished rather than merged:
expression-position `Function::Custom` dispatch is still dynamic, while a SPARQL body's
parse binding is static and carries the environment identity it was bound against.

### The SPARQL engine holds the parse-time configuration for every query it runs

**Was stated in** `crates/sparql-eval/src/engine.rs`, on `NativeSparqlEngine`'s
`parser_options` field and its `with_parser_options` setter:

> Parse-time configuration (the extension-function namespace set), applied to every
> query and update this engine parses.

**Why it was believed.** It was true when written, and it was the only home available:
a `ParserOptions` had to live somewhere the parse could reach, and the engine is what
performs the parse.

**What changed.** An environment gained base options of its own, and for a while both
existed. That is not a redundancy, it is a fork: `prepare_for` derived from the
engine's field, so every ordinary query — every `sh:sparql` body, target, rule and
node expression — read the engine's configuration, while the environment's reached
exactly one door, the function-body bind. A host could declare a relation namespace,
watch it hard-error an unregistered IRI inside a `sh:SPARQLFunction` body, and watch
the identical IRI in a `sh:sparql` body on the same host silently become an ordinary
triple pattern and conform green. Two parse configurations wearing one name, and the
symptom was this repository's own silent-reclassification bug at a sibling door.

Several hosts had already been bitten without anyone noticing: the conformance
harness, the text search tests and the geo rewrite tests each set engine options AND
built a separate environment, so the declaration they thought they had made was
already being dropped.

**The rule now.** The engine holds no parse configuration. `parser_options`,
`with_parser_options` and `parser_options_for` are gone, and an `ExtensionEnv` is the
only thing that says how a SPARQL text is read — on the query lane, the UPDATE lane,
the function-body bind and the product identity alike. Deleting the field rather than
leaving it unread is deliberate: a setter that silently stopped taking effect would be
the same class of defect one level further down. Pinned by
`function_body_relation::a_declared_namespace_reaches_a_sparql_constraint_body` with
its two valid neighbours, and by
`engine::tests::every_chunk_worker_sees_the_declared_parser_options`.

### A `sh:SPARQLFunction` body is opaque to the footprint walk because the walk does not read it

**Was stated in** `crates/shapes/src/footprint.rs`:

> A `sh:SPARQLFunction` body is a SELECT that may carry one, and this walk does not
> read it.

**Why it was believed.** It is a true description of the walk, and `OPAQUE_QUERY_TEXT`
is the conservative answer, so the reason looked sufficient.

**What changed.** Nothing in the code — this entry records a *strengthened* reason, not
a reversal of behaviour, because the weak reason invited a change that would have been
unsound. Reading the body would make the footprint environment-dependent, and a
footprint is derived at plan time from a `Shapes` value with no environment in scope,
cached on a `PreparedShapes`, and reused across validations that each install a
different registry. A precise footprint would therefore be consumed under an
environment it was not derived under.

**The rule now.** The body stays opaque because opaque is TOP, and TOP is the only
answer correct under every environment simultaneously. Pinned by
`plan::tests::a_function_body_s_footprint_does_not_depend_on_the_environment`.

### The executor's rung is not a cursor, and `execute` is the only stage that runs a query

**Was stated in** `docs/design/purrdf-retrieval-ladder.md`, §4 on the seam a
caller stops at, and §1 on the composition:

> That rung is not a cursor and stopping there buys no laziness: each stratum's
> result is fully materialized before its first row is readable (§7).

> Only `execute` runs a query, and every higher-level entry point is *defined as*
> the composition.

**Why it was believed.** It was a true description of the value. `execute` handed
back a `VecDeque` of rows the evaluator had already materialized, behind a
`RankedStreamImpl` that owned nothing but those rows. There was no handle in it
that could reach a store, so the rung really was a list with a receipt, and the
only query in the pipeline really was the one `execute` ran per unit.

**What changed.** A producer may now declare an **exclusion basis**, and fusion
settles finality by asking it — per candidate, against the producer's own index.
Which candidates anyone needs an answer for is decided from a frontier that does
not exist until the rows are being merged, so the lookup cannot be precomputed:
pre-answering for candidates nobody will ask about is the drain the mechanism
exists to remove. `execute` therefore compiles and prepares the lookup and puts
it *inside* the stream, which now borrows the caller's dataset — that is where
`RankedStreamImpl<'d>`'s lifetime parameter comes from — and the lookup query
runs during `fuse`.

**The rule now.** The two halves are stated apart. Under `execute` the **rows**
are still fully materialized before the first one is readable, so stopping there
buys no laziness in the rows and a consumer that reads one has paid for all of
them; the read `search` takes is lazy in its rows, and the entry *The evaluator
exposes no cursor* below records that reversal. The **stream** is a live handle: `execute` is the only stage that runs a
ranked read and the only stage that compiles a query, but it is not the only
place a query runs. Pinned by
`multimodal_read_bound::membership_lookups_stop_the_drain_where_the_threshold_licenses`,
which drives the whole ladder over a dataset and measures lookups served by the
producers after `execute` returned, and by
`exclusion_lookup::a_failed_lookup_fails_the_request_rather_than_answering_possible`,
which shows the lookup is a real measurement whose failure fails the request
rather than a value that was computed up front.

### The engine has no random access, so only a domain declaration can settle what a stream will not name

**Was stated in** `docs/design/purrdf-retrieval-ladder.md` §7, and in the same
words in `crates/retrieval/src/fuse.rs`, `crates/retrieval/src/ranked_stream.rs`
on `StreamContract::domains`, and `crates/retrieval/PRODUCER-CONTRACT.md` under
A15:

> **The engine has no random access.** A ranked stream offers "next row" and
> "how did you end", so the only way to learn that a same-domain stream does
> *not* hold a candidate is to read it until it names the candidate or ends.
> […] Exact scores and a `k`-bounded read over non-overlapping strata are
> jointly achievable only if the producers say which candidates they can name.

**Why it was believed.** It was an exact reading of the trait. `RankedStream` had
`next` and `receipt` and nothing else, so a consumer holding one had no way to
put a question to a producer — only to keep pulling. Given that surface, a
standing declaration about whole blocks was the only channel a promise could
arrive through, and the "only if" followed.

**What changed.** The protocol gained a question. `RankedStream::exclusion` puts
one named candidate to a producer that declared a basis for answering, and
`ExclusionVerdict::Excluded` retires that stream's claim on that candidate. So
there are two routes to the same licence, not one, and they are not
interchangeable: a domain declaration settles finality where the blocks separate
the producers, and answers nothing where two producers share a block and merely
happen not to overlap — the configuration where both declarations are true and
neither says the thing that lets a candidate certify. The lookup reaches exactly
that case, because it is a measurement about one candidate rather than a promise
about a class of them.

**The rule now.** The engine has random access where a producer sold it, and only
there. Against a producer that declares nothing and answers nothing the old
sentence still holds word for word, which is why it is conditioned rather than
deleted: exact scores and a `k`-bounded read over strata that do not overlap are
jointly achievable by *either* route and by neither if the producers offer
neither. The two halves are carried separately all the way down — a stream that
names a candidate outside its declared blocks broke a *declaration*
(`ProtocolError::OutsideDeclaredDomain`), while one that names a candidate it
excluded broke an *observation* (`ProtocolError::ExclusionContradicted`) — so a
refusal blames the promise that was actually broken. Pinned by
`fusion::an_undeclared_read_that_may_ask_bounds_itself_where_the_threshold_falls`
and by
`multimodal_read_bound::the_finality_licence_is_spelled_once_and_enforcement_keeps_its_own`,
which holds the licence and the enforcement apart at every site that reads them.

### An undeclared-domain run drains

**Was stated in** `crates/retrieval/src/fusion_stream.rs`, on the licence, and in
the same words in `crates/retrieval/src/lib.rs`, `crates/retrieval/src/fuse.rs`
and `crates/retrieval/PRODUCER-CONTRACT.md` under A15:

> What the licence buys is the reading: without it, strata whose candidate sets
> do not overlap are read to their ends however small the caller's top-k, because
> no confirmation is ever coming.

**Why it was believed.** It was measured, repeatedly, and it was true of every
fixture that measured it. It is also the reference point three oracles rested
on — two in the Python suite and one in Rust — each of which produced a drain by
withholding the domain declaration and then compared a declared run against it.
That is what made it invisible: the claim held for every run those fixtures could
produce, because every producer in them also declared
`ExclusionBasis::Unavailable`, and a withheld declaration was therefore the whole
of what they varied.

**What changed.** The basis stopped being unavailable. With domains undeclared
*and* a membership basis answered, the identical streams do not drain: fusion
stops where the fused threshold licenses it to, which is a property of the decay
law, the weights and `K`, and is flat in the streams' length. A claim that held
only because a fixture never exercised the alternative is not a claim about the
engine, and this one held for exactly that reason.

**The rule now.** A drain needs both halves withheld: nothing declared **and**
nothing askable. Withholding the declaration alone is not enough, so a fixture
that produces a drain that way must say — and now does say — that its producers
also answer no lookup, rather than attributing the drain to the missing
declaration alone. Pinned by
`fusion::an_undeclared_read_that_may_ask_bounds_itself_where_the_threshold_falls`,
which fuses one fixture three ways — told nothing and asked nothing, told nothing
but answering, and declaring its own blocks — and asserts one answer, one set of
rows, and three different reading costs, with the stopping rank derived from the
crate's own `crossing_rank_at` rather than written down as a literal.

### The request's bound narrows every stratum's depth or none

**Was stated in** `docs/design/purrdf-retrieval-ladder.md` §7, and in the same
shape in `crates/retrieval/src/planner.rs`, `crates/retrieval/src/lib.rs` and
`crates/retrieval/PRODUCER-CONTRACT.md` under A15:

> the request states its own bound, and where **every** stratum's declared blocks
> are pairwise disjoint the planner derives each depth from that bound. […] Any
> overlap, or any unrestricted stratum, and the declared-or-measured bound
> stands.

**Why it was believed.** The proof was written over the whole surviving set. It
opened by writing the strata `s = 1..m` with pairwise-disjoint block sets and
`Unique` declared by all of them, and derived the prefix from that. Read back, a
proof whose premises quantify over every stratum looks like a licence that every
stratum has to earn together, and the rule was implemented as the proof was
written.

**What changed.** The premises were read again for what each one is actually
about. The disjointness a candidate `x` of stratum `s` needs is that no *other*
surviving stratum can name `x` — which says nothing about whether two of those
others share a block with each other. The `Unique` premise is read off one
stream and is a fact about that stream's own candidates, so a neighbour
declaring `Allowed` repeats candidates `s` cannot receive. Both premises are
therefore about `s`, and charging `s` for a neighbour's shape was an
over-refusal: it read exactly like correct strictness, nothing looked broken, and
the only thing that moved was how deep a read nobody was watching went.

**The rule now.** The verdict is **per stratum**. A stratum `s` is licensed the
request's bound `k` when the request states one, `s` declares `Unique`, and `s`'s
blocks are disjoint from the union of every other surviving stratum's; `s`'s
depth is then `min(declared, statistics-narrowed, k)` whatever the strata beside
it declared, and a stratum that fails a condition keeps the declared-or-measured
bound it always had without taking a neighbour's prefix with it. One exclusion
stays request-wide, and the reason is that the declaration is: `Unrestricted`
among two or more strata promises nothing about which candidates it will not
name, so it meets every other stratum's blocks and costs everyone the prefix.
Pinned by `multimodal_read_bound::a_stratum_disjoint_from_a_sharing_pair_narrows_while_the_pair_does_not`
and `multimodal_read_bound::a_unique_stratum_narrows_beside_a_disjoint_allowed_one`,
with `multimodal_read_bound::a_lone_allowed_stratum_is_not_licensed_by_being_alone`
as the neighbour that shows the per-stratum reading did not become no reading at
all.

### The probe row is counted nowhere

**Was stated in** `crates/retrieval/src/lib.rs`, and in the same words in
`crates/retrieval/src/compile.rs`, `crates/retrieval/src/execute.rs`,
`crates/retrieval/PRODUCER-CONTRACT.md` and §9 of
`docs/design/purrdf-retrieval-ladder.md`:

> The probe is a read and never a value: no plan field, identity or resolution
> number moves by one because of it.

**Why it was believed.** It was the whole point of the probe. `compile` emits
`LIMIT depth + 1` so that the arrival of the extra row can distinguish a cut read
from an exhausted one, and every number the answer carried was a number about the
*answer* — a planned depth, a rank, a contribution, an identity. A probe row that
moved any of those would have made the layer's bookkeeping disagree with the plan
by one, which is the off-by-one the sentence was written to forbid.

**What changed.** A number appeared that is not about the answer. The trailer
gained `StratumResolution::rows_materialised`: how many rows a stratum's read
actually returned, beside how far the fusion walked it. The two are routinely far
apart — a plan whose depth no licence narrowed materializes its declared length
and hands a handful of ranks to a fusion that certifies immediately — and the gap
is the whole subject, because a narrowing judged by ranks pulled alone is judged
by the counter it was built to lower. The probe row is a row the read paid for,
so a figure that excluded it would report a read as cheaper than it was by
exactly the row that makes its ending observable.

**The rule now.** The probe row moves no plan field, no identity and no
arithmetic of the answer, and it is counted in exactly one number: the rows a
read materialized. The exception is principled rather than an erosion — that
number asks what the read *cost* rather than what the answer is made of, and it
is the only number in the trailer that does. Pinned by
`multimodal_read_bound::the_work_never_exceeds_the_materialised_control_in_any_configuration`,
whose materialized control for the licensed configuration reads its five rows
*and the probe row*, six, while the planned depth and the ranks pulled stay where
they were; by
`multimodal_read_bound::every_stratum_is_read_once_and_produces_exactly_the_rows_its_fusion_pulled`,
which counts the probe row in an on-demand read exactly where the fusion read past
the planned depth and nowhere else; and by
`multimodal_read_bound::the_rendered_observed_resolution_carries_every_counter_a_caller_pays_for`,
which holds the rendered trailer against a figure assembled from the producers'
own counters rather than from the trailer.

### A read that is not read to its end cannot carry a verifiable receipt

**Was stated in** `crates/retrieval/src/search.rs`, the module header on the read
`search` takes, and in the same words in §7 of
`docs/design/purrdf-retrieval-ladder.md`:

> Discarding rather than resuming is what keeps the answer verifiable. A
> [`PfAttestation`] exists only on a *completed* governed run, the fusion reads
> it before it pulls a row, and two runs of one stratum attest separately — so
> an answer spliced from a narrow prefix and a deeper continuation would be an
> answer whose evidence describes neither read.

and, as the reason a lazily driven cursor was set aside, that a witness exists
only on a completed `GovernedOutcome`, so a cursor pulled on demand has no run
behind it and no attestation to announce.

**Why it was believed.** It was true of the one channel the executor read
evidence from. The witness a unit's attestation is read off travelled on the
governed lane's `GovernedOutcome`, which exists only once the evaluator has
drained every invocation into a bag; the fusion reads a stream's attestation
before its first row; so the only read with evidence to announce at that
instant was one that had already finished. Given that channel, a read stopped
early — or taken in two parts — had nothing a receipt could be about, and
throwing a short read away whole and taking the planned one looked like the
price of verifiability.

**What changed.** The premise conflated where the witness was *carried* with
when its two facts are *known*. The evaluator's own witness rule reads the
generation the instant a cursor opens and the service level when the
invocation ends; neither needs the bag. `NativeSparqlEngine::open_call_cursor`
opens a rendered unit's one call as an invocation held open, and its
`CallCursor` announces what the invocation attested at the open and builds its
witness when the consumer stops (`CallCursor::settle`): the generation pinned at
open, the service level read then, and a second generation if the cursor now
reports one. The fusion reads the announcement before the first row, exactly as
before, and at the trailer every stream settles
(`RankedStream::settle`): the witness is read under the same sole-witness rule a
finished run is read under and held to the announcement, and a settlement that
disagrees is refused (`ProtocolError::AttestationMoved`). The receipt therefore
covers exactly the read that produced the rows — however far the fusion took it
— and is pinned to one generation across the whole of it.

**The rule now.** `search` reads every stratum it rendered on demand: one
invocation per stratum, opened at the planned depth, read a row per pull and
never re-opened, with its receipt taken when the read stops. Nothing is
discarded and nothing is read twice; a fusion that needs more reads on in the
same invocation. Pinned by
`on_demand_receipt::a_read_stopped_mid_invocation_carries_a_verified_receipt_identical_to_the_materialised_one`,
which stops a four-hundred-row read at its sixth row and holds its
attestations, evidence identity, exactness and rows to the materialized read's;
by `on_demand_receipt::an_index_that_moves_under_the_read_is_refused_when_the_read_settles`
and `on_demand_receipt::a_forged_announcement_is_refused_at_the_settlement_and_the_true_one_is_admitted`,
each beside its admitted neighbour; and by
`multimodal_read_bound::a_stopping_rank_the_rows_overrun_is_read_past_in_the_same_read`,
which reads past the compiled plan's own prediction and pays for the planned total once.

### The evaluator exposes no cursor, and incremental enumeration is separate, larger work

**Was stated in** `docs/design/purrdf-retrieval-ladder.md` §7, on the unfused
rung:

> There is no windowed or incremental execution to rest a claim on: the
> evaluator exposes no cursor, stream or iterator surface at all, and is
> materialized at every operator. […] (Making enumeration incremental is
> separate, larger work; this paragraph records what ships.)

**Why it was believed.** Every query entry the evaluator offered returned a
finished answer, and every algebra node hands its parent a bag. A per-row
evaluator for SPARQL in general is a rewrite of every operator, and the
sentence read that as the only way to read anything incrementally.

**What changed.** The read this layer needs incrementally is not SPARQL in
general. A unit this layer renders is one property-function call under
row-for-row operators — projections, `OFFSET`-free `LIMIT`s and renaming
`BIND`s — and a relation's `PfCursor` is already per-row at the seam. So the
evaluator reads exactly that shape on demand, through the same admission,
unification and containment the governed lane uses, and refuses every other
shape by name rather than materializing it behind a cursor's back. A `FILTER`
over the call is one more such operator: it drops rows and never reorders them,
so the read evaluates it per pulled row with the engine's own expression
evaluator, and never offers the relation a `LIMIT` that stands above it.

**The rule now.** `execute` still materializes each stratum before its first
row is readable; `execute_within` at `ReadSchedule::OnDemand` — the read
`search` takes — produces each row when its stream is pulled. A consumer that
stops at the sixth rank of a four-hundred-row plan has caused six rows to be
produced. Pinned by
`multimodal_read_bound::every_stratum_is_read_once_and_produces_exactly_the_rows_its_fusion_pulled`
and `multimodal_read_bound::the_work_never_exceeds_the_materialised_control_in_any_configuration`,
which hold every configuration's rows and producer-reported work exactly, and
never above the materialized control's.

### A stream's ending is fixed before its first row, or a caller that stopped early would be told something different

**Was stated in** `crates/retrieval/src/execute.rs`, on `StreamEnding`:

> It is fixed at construction rather than computed in
> [`RankedStreamImpl::receipt`] because the fact is about the evaluator's
> answer, not about how much of the stream a consumer chose to pull: a stream
> whose ending were derived at the end would say something different to a
> caller that stopped early, which is precisely the falsifiable status the
> ranked-stream protocol forbids.

**Why it was believed.** For a materialized read the ending is a fact about the
evaluator's finished answer, knowable before the first row; deriving it later
looked like letting a consumer's stop change a status.

**What changed.** A read produced on demand learns its ending only when it gets
there — the probe row arriving, or the producer running out — so its ending is
decided at that pull. The feared failure cannot happen, and could not before:
`RankedStreamImpl::receipt` refuses to answer until the stream has returned its
last row (`ProtocolError::NeverEndingSource`), so a caller that stopped early is
told no ending at all rather than a different one, and a caller that read to the
end is told the ending the same rows reach under either schedule.

**The rule now.** A materialized read's ending is fixed when its stream is
built; an on-demand read's is decided at the pull that reaches it, and is the
same ending for the same rows. Pinned by
`multimodal_read_bound::the_on_demand_read_returns_the_answer_the_materialised_read_returns`,
which holds every configuration's terminal statuses, and its rows, to the
materialized read's.

### An unresolved shapes-graph import warns and validates the shapes graph alone

**Was stated in** `crates/cli/src/shapes_source.rs`, the module documentation:

> Naming no `--import` at all leaves the imports UNRESOLVED but does not refuse them: it
> reports each one on stderr and the caller proceeds with the shapes graph alone. That
> asymmetry is deliberate and load-bearing.

**Why it was believed.** Some shapes documents, including two in the vendored W3C
SHACL corpus, carry an `owl:Ontology` header whose imports do not affect their shapes.
Refusing every unresolved import looked like it would reject valid input. The check
also had no rule for telling a missing ontology from one already in hand: every
`owl:imports` object counted as unresolved.

**What changed.** The over-refusal risk came from that missing rule, not from
refusing. An import is now resolved when it names a document already read (the
shapes document's own retrieval IRI, `--shapes-base`, `shacl pack --base` or
`@base`, or an `--import` document), or when the closure already holds the
ontology it names (`<X> a owl:Ontology`, or an ontology whose `owl:versionIRI` is
`<X>`). The first case is SHACL's `sh:prefixes/owl:imports*` idiom, which points
at the document's own IRI. A shapes document that merges the W3C SHACL 1.2 vocabularies is therefore complete as written.
With that rule in place, the warn-and-continue path could only mean one thing: a
verdict about a smaller shapes graph than the one named, printed next to a warning
that does not undo it.

**The rule now.** `purrdf_core::imports` decides, for entailment and SHACL alike.
Every shapes-graph entry point on every host — every `Shapes` constructor, and so
validation, the change path, rules, node expressions, lint and the prepared product,
through the Rust API, the command line, Python, WebAssembly and C — resolves the
closure through `purrdf_shapes::imports::resolve_shapes_imports` against the caller's
import table and refuses an incomplete one with the typed
`ShapesError::Imports(ShapesImportError::Unresolved)`: exit 1 on the command line,
naming each IRI and its `--import IRI=FILE` pair; `ShapesImportError` in Python;
`ShaclImportError` in JavaScript; `PURRDF_STATUS_SHAPES_IMPORT_ERROR` in C. `entails`
refuses it with `EntailError::UnresolvedImport`. Pinned by
`merged_vocabulary_needs_no_import_flag` and `unresolved_import_is_refused` (CLI),
`merged_vocabulary_packs` and `unresolved_import_refused` (product),
`import_present_in_graph_is_resolved`, `absent_import_is_unresolved` and
`self_import_is_resolved` (the rule), `a_self_imported_prefix_document_needs_no_import_flag`
(the W3C `prefixes-001` vector), and the cross-host verdict tests
(`crates/validate/tests/shapes_owl_imports.rs`,
`every_shapes_lane_gives_the_same_owl_imports_verdict`,
`every_shapes_entry_point_gives_the_same_owl_imports_verdict`,
`test_shacl_owl_imports.py`, `shacl-owl-imports.test.mjs`).

### A shapes graph's `owl:imports` rule is applied by every host

**Was stated in** the `owl:imports` section of `docs/book/src/validation/shacl.md`
and `crates/cli/src/shapes_source.rs`:

> The prepared-product packer that the WebAssembly and C-ABI hosts call applies the
> same rule, so every host agrees on which shapes graphs are complete.

**Why it was believed.** The command line and the product packer both called the
one resolution rule, and the product packer is what the three binding hosts reach
when they pack.

**What changed.** Packing is one entry point of many. The engine's own
constructors, the SARIF, rules, node-expression and lint boundaries, and therefore
every Python, WebAssembly and C function except pack, never applied the rule: the
same shapes graph was refused on the command line and validated, given an empty
inference graph, or certified clean on the other hosts.

**The rule now.** The enforcement is at the shapes engine boundary itself, as the
entry above states: a `Shapes` value is complete by construction, and every host
takes the import table in its own spelling and raises the same typed refusal.
