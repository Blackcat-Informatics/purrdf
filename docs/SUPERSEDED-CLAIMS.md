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
`<X>`), or — for a shapes graph — when the closure describes `<X>` with
`sh:declare`. That last case is SHACL's `sh:prefixes/owl:imports*/sh:declare`
prefix idiom, where the import target is a node the shapes graph declares prefixes
on; the W3C `prefixes-001` vectors write it, and validate as written. A shapes document that merges the W3C SHACL 1.2 vocabularies is therefore complete as written.
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
`self_import_is_resolved` (the rule), `the_w3c_prefix_idiom_needs_no_import_flag_and_a_header_import_is_refused`
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

### A built-in constraint component given a validator is a duplicate definition

**Was stated in** `crates/shapes/src/spec/mod.rs` (the linker's outcome table),
`crates/shapes/README.md`, the "Built-in declarations and the W3C vocabularies" section
of `docs/book/src/validation/shacl.md`, and the test
`imported_native_validator_is_a_duplicate_definition`:

> A built-in component's declaration carrying a VALIDATOR is a second definition of the
> built-in, refused at load — neither silently ignored (the validator would never run
> while the author believed it did) nor silently preferred (the native semantics would
> be replaced behind the author's back).

**Why it was believed.** A validator is what gives a SPARQL-based component its
meaning, so a validator on a component the engine already implements read as a second,
competing implementation, and the only choices seemed to be refusing it, ignoring it
or running it instead.

**What changed.** The specification makes several validators on one component
ordinary. SHACL 1.2 SPARQL Extensions, "Validators": "For a given constraint, a
validator is selected from the constraint component using the following rules, in
order: For node shapes, use one of the values of sh:nodeValidator, if present. For
property shapes, use one of the values of sh:propertyValidator, if present. Otherwise,
use one of the values of sh:validator." Every value is an implementation of the same
component, and "SHACL processors may choose alternative approaches as long as the
outcome is equivalent" ("Validation with SPARQL-based Constraint Components"). The
native implementation is the approach this engine chooses, with the specification's
semantics, so an alternative is neither a competing definition nor a dropped
constraint. Real vocabularies write such declarations (DASH gives most SHACL Core
components SPARQL validators), and refusing a well-formed one is over-refusal: the
rejected input is valid.

**The rule now.** A built-in component's declared validators bind as alternatives the
native implementation supersedes. Each is checked for well-formedness and never run,
and `shapes lint` lists each one (`alternative … superseded-by-native`) without
counting it as a finding. What still changes the component is still refused: a body,
`sh:ask` or `sh:select` on the component itself, a contradicting signature, a validator
that is not a well-formed SPARQL validator of its attachment, and any other `sh:`
statement except `sh:message`, `sh:labelTemplate` and the non-validating
characteristics (`crates/shapes/src/spec/link.rs`, `BUILTIN_COMPONENT_ANNOTATIONS`).
Pinned by `a_builtin_component_given_validators_binds_natively`,
`an_ill_formed_alternative_on_a_builtin_is_refused`,
`a_semantic_statement_on_a_builtin_declaration_is_refused`,
`lint_reports_superseded_alternatives_without_findings` (`tests/spec_linker.rs`),
`imported_native_validator_binds_as_a_superseded_alternative`
(`tests/component_parameters.rs`) and
`cli_shapes_lint_reports_superseded_builtin_validators`.

### A pre-binding violation refuses the load whether or not the query executes

**Was stated in** `crates/shapes/src/validator_alternatives.rs`, `crates/shapes/README.md`,
the "Built-in declarations and the W3C vocabularies" section of
`docs/book/src/validation/shacl.md`, the rule of the entry above, and the tests
`an_ill_formed_alternative_on_a_builtin_is_refused`,
`validator_declarations_enforce_attachment_kind_and_query_datatype` and the
`sparql_function_with_*_is_rejected` family:

> A validator that is not a well-formed SPARQL validator of its attachment is refused
> at load, on a built-in or a custom component — its query grammar and its
> pre-binding restrictions alike — and a malformed `sh:SPARQLFunction` declaration is
> refused at load, because SHACL 1.2 Core says a processor "SHOULD produce a failure"
> for an ill-formed shapes graph.

**Why it was believed.** The pre-binding check ran in the same parse as the grammar
check, and its refusal read like every other ill-formedness refusal, so the two were
treated as one rule.

**What changed.** They are two rules with two different scopes. SHACL 1.2 Core,
"Handling of Ill-formed Shapes Graphs": "If the shapes graph contains ill-formed nodes,
then the result of the validation process is undefined. A SHACL processor SHOULD
produce a failure in this case." That sentence has no reachability qualifier, and the
nodes it covers are the ones that violate a syntax rule — SHACL 1.2 SPARQL Extensions,
"Summary of Syntax Rules": "Nodes that violate these rules in a shapes graph are
ill-formed". SHACL 1.2 SPARQL Extensions, Appendix A, scopes the pre-binding failure
differently: "SHACL-SPARQL processors MUST report a failure when it is operating on a
shapes graph that contains SHACL-SPARQL queries (via sh:ask, sh:construct and
sh:select) that are executed with pre-bound variables and violate any of these MUST
restrictions." A `MINUS` in a validator that never runs is not in that scope.

An intermediate reading went further, and is withdrawn. It judged every ill-formed
declaration only where a shape reached it and reported the rest in a `shapes lint`
section named `inert`, so that a shapes graph importing a library such as DASH
(`<http://datashapes.org/dash>`) would load. DASH declares an ASK validator under
`sh:nodeValidator` for `sh:HasValueConstraintComponent`, an ASK
`sh:propertyValidator` for `dash:SubSetOfConstraintComponent` and a `sh:SPARQLFunction`
parameter named `value`. Those are syntax-rule violations, and Core's sentence covers
them whether or not a shape reaches them. PurRDF treats that SHOULD as a MUST. Only
the pre-binding part of that relaxation had a basis in the specification.

**The rule now.** A SHACL-SPARQL or SHACL-AF declaration that violates a syntax rule
refuses the load whether or not anything reaches it. That covers a non-SELECT value of
`sh:nodeValidator` or `sh:propertyValidator` and a non-ASK value of `sh:validator`
(`nodeValidator-class`, `propertyValidator-class`, `validator-class`, "The values of
sh:nodeValidator must be SELECT-based validators", "The values of sh:validator must be
ASK-based validators") on built-in and custom components alike. It covers a component
parameter name the rule `parameter-name-not-in` reserves ("Parameter names must not be
one of the following: this, path, PATH, value"). It covers a `sh:SPARQLFunction`
parameter named one of those or `shapesGraph` or `currentShape`: SHACL Advanced
Features, "Function Parameters", says "the same syntax rules apply" and links the
SHACL version whose list names those two. And it covers a `sh:SPARQLFunction`
without exactly one `sh:ask` or `sh:select` (SHACL Advanced Features, "SPARQL-based
Functions": "SPARQL-based functions have exactly one value for either sh:ask or
sh:select"). The refusal is `ShapesError::IllFormed`, and it lists every violation in
the graph with its declaration and rule id. A pre-binding violation refuses the load, as
`ShapesError::Prebinding`, only where the query executes: the validator a use of a
custom component selects, and a `sh:SPARQLFunction` that a node expression or reachable
SPARQL calls. A validator declared for a built-in component never executes, because the
native implementation supersedes it. A validator no use selects and a function nothing
calls never execute either. Those load, and `purrdf shapes lint` lists each in its
`unexecuted` section as a finding. SHACL-JS reachability is unchanged. Pinned by
`an_ill_formed_alternative_on_a_builtin_is_refused` and
`a_prebinding_violation_is_refused_only_where_the_query_executes`
(`tests/spec_linker.rs`),
`ill_formed_declarations_refuse_the_load_whether_or_not_a_shape_reaches_them` and
`a_function_body_violating_prebinding_is_refused_only_where_a_call_executes_it`
(`tests/shapes_graph_wellformedness.rs`),
`validator_declarations_enforce_attachment_kind_and_query_datatype` and
`parameter_names_shacl_pre_binds_are_refused_at_load_and_near_misses_are_not`
(`tests/component_parameters.rs`), and the `sparql_function_with_*_is_rejected` tests
in `crates/shapes/src/shapes.rs`.

### Every `owl:imports` triple is an import, and a `sh:declare` description resolves one

**Was stated in** `crates/rdf-core/src/imports.rs`, `crates/shapes/src/imports.rs`, the
`owl:imports` section of `docs/book/src/validation/shacl.md`, `crates/shapes/README.md`,
`crates/cli/README.md` and the host documentation of the shapes-graph import table:

> An `owl:imports <X>` is also resolved when the closure holds a triple `X sh:declare ?d`
> ... SHACL-SPARQL collects a query's prefix declarations along
> `sh:prefixes/owl:imports*/sh:declare` WITHIN the shapes graph, so the target of such an
> `owl:imports` is a node the shapes graph describes with `sh:declare`, not a document
> that has to be fetched.

**Why it was believed.** The imports rule read every `owl:imports` triple of a graph as
an import, whatever its subject. The W3C `sparql/node/prefixes-001` test writes
`ex:TestPrefixes owl:imports <http://example.com/ns#>` for its prefix path, so under that
reading the test needed a special route that counted the `sh:declare` node as present.

**What changed.** Neither specification reads an import off an arbitrary node. OWL 2's
*Mapping to RDF Graphs* §3.1.2 extracts Imp(G) from the ontology header patterns of Table 4,
`x rdf:type owl:Ontology . x owl:imports y`, and SHACL 1.2 Core follows imports from the
shapes graph's own IRI along `^owl:versionIRI?/owl:imports`. `ex:TestPrefixes` is
neither, so its triple was never an import, and the special route answered a question the
specification never asks. The same misreading made `sparql/component/validator-001` —
whose `owl:imports <http://datashapes.org/dash>` sits on a node that is neither the test
document's IRI nor an `owl:Ontology` — demand a document its approved report is computed
without.

**The rule now.** An `owl:imports` triple is an import exactly when its subject is an
anchor of its document: an IRI the document was loaded under (for an imported document,
the IRI it was imported by), a subject the document types `owl:Ontology`, or a subject
naming one of those as its `owl:versionIRI`. Any other `owl:imports` triple is data. The
`sh:declare` route is gone; SHACL-SPARQL's prefix path walks `owl:imports` edges within
the shapes graph as prefix collection. The rule is `purrdf_core::imports::imported_iris`,
for entailment and SHACL alike. Pinned by the kernel tests in
`crates/rdf-core/src/imports.rs`, `crates/shapes/src/imports.rs` and
`crates/entail/src/entails/imports.rs`,
`the_prefixes_path_follows_version_iris_and_imports` (`tests/sparql_prefixes.rs`) and
`the_w3c_validator_001_vectors_validate_with_no_import` (the command line).

### The kernel's import rule names no vocabulary beyond OWL's own

**Was stated in** `crates/rdf-core/src/imports.rs`, the module documentation of the one
`owl:imports` rule:

> The kernel registers no such predicate and names no vocabulary beyond OWL's own; an
> `ImportMap` built with [`ImportMap::new`] applies the three routes above and nothing
> else.

The sentence left with the `sh:declare` route the entry above retires, but the rule it
described kept its shape: the anchors were the loaded IRI, OWL 2's `owl:Ontology` header
and the `^owl:versionIRI` step, and nothing SHACL names.

**Why it was believed.** `owl:imports` is an OWL term, OWL 2's *Mapping to RDF Graphs*
Table 4 reads it off the ontology header, and a kernel that stayed inside OWL's
vocabulary looked like the neutral choice for a rule both engines share.

**What changed.** OWL 2 is the floor of the rule, not its limit. SHACL 1.2 Core §6.1 names
the class a document declares a shapes graph with — "The sh:ShapesGraph class MAY be used
as an rdf:type of the IRI of a graph that typically acts in the role of a shapes graph" —
and follows a shapes graph's `owl:imports`. Under the OWL-only rule,
`<G> a sh:ShapesGraph ; owl:imports <lib>` was silently data: validation ran without
`<lib>`'s shapes and nothing was refused. SHACL 1.2 SPARQL Extensions already read the
same graph classes for its implicit prefixes, from a second, private list in
`purrdf-shapes`, so two rules classified one document two ways.

**The rule now.** The kernel names the W3C graph classes directly. `purrdf_core::graph_roles`
classifies every node as a SHACL instance (`rdf:type/rdfs:subClassOf*`) of `owl:Ontology`,
`sh:ShapesGraph` (with `sh:RulesGraph` counted in its own right) and `sh:DataGraph`. The
import rule's anchors are the loaded IRI, every ontology header, every shapes graph, and
the `^owl:versionIRI` step from any of them; a node whose only role is `sh:DataGraph` is
not one (SHACL 1.2 Core §6.2, "owl:imports in the data graph is not enacted"). The
implicit prefix collection selects every role from the same classifier. These are the
W3C's classes, not minted ones; PurRDF still mints no vocabulary. Pinned by
`graph_roles::tests`, `imports::tests::every_shapes_graph_instance_imports_and_an_unrelated_type_does_not`
and `a_data_graph_alone_anchors_nothing_and_a_data_graph_ontology_does` in
`crates/rdf-core`, the engine tests
`an_import_on_every_shapes_graph_instance_is_followed_and_an_unrelated_type_is_data`
(`crates/shapes/src/imports.rs`) and
`a_shapes_graph_node_imports_in_entailment_as_in_validation`
(`crates/entail/src/entails/imports.rs`), and
`a_document_validated_as_data_enacts_no_import_and_loaded_as_shapes_it_does`
(`crates/validate/tests/shapes_owl_imports.rs`).

### A SHACL-JS validator is inert vocabulary where it is declared

**Was stated in** `crates/shapes/src/validator_alternatives.rs`, the module
documentation of the built-in alternatives:

> A SHACL JavaScript Extensions `sh:JSValidator` is declared vocabulary, not a load
> error: libraries such as DASH declare them beside SPARQL validators, and this engine
> never runs one. On a built-in it is listed here as a `ValidatorLanguage::JavaScript`
> alternative, never parsed and never run. On a custom component it is refused only
> where a shape uses the component and the validator SHACL selects for that shape is
> JavaScript-only; a use with a SPARQL validator to select runs that one.

**Why it was believed.** SHACL-JS extends the three attachments with its own validator
class, and a validator nothing runs looked like vocabulary. So a component's
`sh:JSValidator` was exempted from the attachment's class rule, like a `sh:JSFunction`
nothing calls.

**What changed.** The class rules of SHACL 1.2 SPARQL Extensions have no such exemption:
"The values of sh:validator must be ASK-based validators" (`validator-class`), and the
values of `sh:nodeValidator` and `sh:propertyValidator` "must be SELECT-based
validators" (`nodeValidator-class`, `propertyValidator-class`). A `sh:JSValidator` is
neither, so a graph that attaches one is ill-formed. SHACL 1.2 Core's "A SHACL processor
SHOULD produce a failure in this case" has no reachability qualifier, and PurRDF treats
that SHOULD as a MUST. Under the old reading `shapes lint` reported such a graph clean.
A `sh:JSFunction` or `sh:JSLibrary` is not attached through those properties, so no
syntax rule reaches it.

**The rule now.** A value of an attachment that is not a SPARQL validator of the
attachment's query form — a `sh:JSValidator` included — refuses the load as
`ShapesError::IllFormed`, naming the rule. That holds on built-in and custom components,
whether or not a shape reaches it, and on every other subject of those properties.
`ValidatorLanguage` has no JavaScript variant. A `sh:JSFunction` or `sh:JSLibrary`
nothing reaches stays inert, and a SHACL-JS construct a shape reaches stays refused as
`ShapesError::ShaclJs`. Pinned by
`a_javascript_validator_on_a_builtin_is_ill_formed_under_every_attachment` and
`a_custom_components_javascript_validator_is_ill_formed_reached_or_not`
(`crates/shapes/tests/spec_linker.rs`), and by
`a_shacl_js_validator_is_ill_formed_and_its_ask_equivalent_validates`,
`a_validator_of_a_non_component_owner_is_judged_by_the_class_rules`,
`declared_shacl_js_no_shape_reaches_is_inert_and_ordinary_constraints_fire` and
`every_shacl_js_construct_a_shape_reaches_is_a_typed_refusal`
(`crates/shapes/tests/shapes_graph_wellformedness.rs`).

### `purrdf-jsonschema` depends on `serde_json`, `regex` and `purrdf-iri` only

**Was stated in** `crates/jsonschema/README.md`:

> It depends on `serde_json`, `regex` and `purrdf-iri` only, forbids `unsafe`, and
> builds for `wasm32-unknown-unknown` like every other release crate in the workspace.

**Why it was believed.** Schemas and instances were `serde_json::Value`s, and the
evaluator's own number, date and Unicode code needed nothing else.

**What changed.** Schemas and instances are `purrdf_lex::json::Value`s, whose numbers
keep their lexemes. Exact number order, equality and `multipleOf` are
`purrdf_xsd::json_number::JsonNumber`, the `date-time`, `date` and `time` formats read
through `purrdf_xsd::rfc3339`, `\s` and the line terminators are `purrdf_lex::terminals`,
and every map is keyed by `purrdf_hash::fixed::FixedState`.

**The rule now.** `purrdf-jsonschema` depends on `regex`, `purrdf-iri`, `purrdf-xsd`,
`purrdf-lex` and `purrdf-hash` only (`crates/jsonschema/Cargo.toml`, and its row in
`layers.toml`), and `serde_json` is banned on every edge.

### The core Turtle writer's literal escaper mirrors the canonical one exactly

**Was stated in** `crates/rdf-core/src/turtle.rs`, on its private `escape_literal`:

> Mirrors [`crate::ir::canon::write_literal_escaped`] exactly.

**Why it was believed.** Both were written from the same N-Triples literal grammar: the
readable `ECHAR`s, `UCHAR` for the rest of C0 and DEL, and C1 raw.

**What changed.** They were two bodies, and they differed: the Turtle copy spelt
BACKSPACE and FORM FEED as `\u0008` and `\u000C` where the canonical form writes `\b`
and `\f`. A claim that two bodies agree is a claim nothing checked.

**The rule now.** There is one escaper, `purrdf_lex::literal_escape`, with the carrier
(`Canonical`, `Xml`, `TurtleLong`) as its only parameter. The Turtle writers spell every
term through `purrdf_lex::term_syntax`, and the canonical N-Quads writer composes the
same pieces. The `literal-and-iri-escape` and `term-syntax` jobs of
`helpers-ledger.toml` refuse a second body.

### The GTS fold view compacts IRIs under a built-in W3C/schema.org table

**Was stated in** `crates/rdf/src/gts_view.rs`, on `GtsFoldViewConfig`:

> any extra CURIE prefix entries consulted (in order, before the built-in
> W3C/schema.org table) when compacting IRIs

**Why it was believed.** `schema:` is a common prefix in the data the fold view
renders, so a built-in entry looked like convenience.

**What changed.** schema.org is not a W3C Recommendation's vocabulary, so a built-in
`schema:` prefix was a fabricated default vocabulary, which PurRDF does not supply.

**The rule now.** The fold view builds in the W3C prefixes `rdf`, `rdfs`, `owl`, `xsd`
and `skos`, read from `purrdf_iri::vocab` and `purrdf_xsd::datatype`, and compacts under
any other namespace only when the caller supplies it in
`GtsFoldViewConfig::curie_prefixes`, under the longest matching namespace
(`purrdf_iri::contract`). Pinned by
`a_schema_org_iri_stays_a_full_iri_without_a_caller_prefix` and
`a_schema_org_iri_compacts_under_a_caller_supplied_prefix`
(`crates/rdf/src/gts_view.rs`).

### A governed CLI run starts from `QueryGovernors::UNBOUNDED`

**Was stated in** `crates/cli/src/governors.rs`, the module documentation:

> [`GovernorFlags::to_governors`] starts from [`QueryGovernors::UNBOUNDED`] and adds
> only the ceilings the operator actually named.

**Why it was believed.** A dimension no flag named was taken to need no accounting at
all, and `UNBOUNDED` is the configuration that declines every ceiling.

**What changed.** `UNBOUNDED` also declines the accounting, so a `--deadline` was polled
only between operators and a long-running operator never saw it. The C ABI, the wasm
package and the Python binding decoded their governors from `METERED`, so the CLI
disagreed with every other host.

**The rule now.** Every host decodes through `purrdf_validate::governors::from_parts`,
which starts from `QueryGovernors::METERED`: a dimension no flag names is charged
against a ceiling no run can reach, and the trip report prints it as `limit …
unbounded`. `--no-ceiling` asks for `UNBOUNDED`; it combines with `--deadline` and is
refused beside a numeric ceiling or `--explain`. Pinned by
`the_metered_default_refuses_an_over_cap_query_and_no_ceiling_answers_it` and
`no_ceiling_beside_a_ceiling_is_refused_by_name` (`crates/cli/tests/governors_cli.rs`)
and `naming_nothing_is_metered` (`crates/validate/src/governors.rs`).

### Base16 has a kernel home in `purrdf-core`, with sanctioned renderers elsewhere

**Was stated in** `crates/rdf-core/src/hex.rs`, the module documentation of
`purrdf_core::hex`:

> There is therefore **one** transcription of it, here, in the crate that is a common
> ancestor of those consumers.

followed by a list of call sites that "correctly do something else" — hot-path label
renderers, allocation-free renderers into a caller's buffer, `purrdf-gts`'s own renderer
in `wire` because it could not reach the kernel, and renderers that append into a
caller's accumulator. `crates/rdf-core/src/content_id.rs` likewise stated that its
64-digit decoder was shared with `ContentDigest::from_hex` through
`content_store::decode_hex_32` / `decode_hex_32_lower`.

**Why it was believed.** `purrdf-core` was the lowest crate most renderers reached, and
each exemption needed a shape (`Display`, append, fixed buffer) the one `lower`
function did not offer.

**What changed.** The exemptions were missing entry points, not different operations.
`purrdf-hash` is the root every crate reaches, including `purrdf-gts`.

**The rule now.** `purrdf_hash::hex` is the one base16 codec, with an entry point for
each of those shapes: `Lower`/`Upper` (`Display`), `encode`, `encode_into` (append),
`encode_to_slice` (a caller's buffer), `decode`, `decode_canonical`, `decode_32` and
`decode_32_canonical` (the content-address form), and `Digest32`, the 32-byte value
every content identity wraps. The `hex` job of `helpers-ledger.toml` is enforced, with
no variants.

### `purrdf-testkit` depends on no `purrdf-*` crate

**Was stated in** the root `Cargo.toml`, on the `purrdf-testkit` workspace entry:

> It depends on no `purrdf-*` crate, so any member's tests may use it without closing
> a cycle.

**Why it was believed.** Test support needed no workspace code, and `purrdf-hash` ran
its frozen-vector suites on testkit's runner, so testkit could not depend on it.

**What changed.** Testkit's seeded draws are the SplitMix64 and LCG streams
`purrdf_hash::mix` holds, so testkit depends on the root.

**The rule now.** `purrdf-testkit`'s one first-party dependency is `purrdf-hash`, the
zero-dependency root. `purrdf-hash` has no dev-dependency on testkit: its frozen-vector
suites, differentials and benches live in the unpublished `purrdf-hash-conformance`,
natively and on wasm32. `layers.toml` holds the edge, and no member's tests close a
cycle through testkit.

### The removed external packages are the workspace's codecs, signatures, byte search and bench harness

**Was stated in** `dependency-ledger.toml`, as retained dependencies:

> `ciborium` — CBOR (RFC 8949) encode/decode for GTS packs and envelopes
> `ed25519-dalek` — Ed25519 (RFC 8032) signatures for GTS COSE_Sign1 and RDF signing
> `roxmltree` — XML 1.0 parsing for RDF/XML, OWL/XML and SPARQL XML results
> `serde_json` — JSON (RFC 8259) encode/decode for JSON-LD, SPARQL JSON results and
> bindings
> `serde_yaml_ng` — YAML parsing for YAML-LD, SSSOM headers and shape configs
> `serde` — serialization framework behind the JSON, YAML and CBOR codecs
> `memchr` — SIMD byte search for parser line splitting and escape scanning
> `criterion` — statistical benchmark harness for every crate bench …

**Why it was believed.** Each was the codec or harness its format needed, and nothing
first-party did the same job.

**What changed.** Each job now has a first-party home, and each package was a second
implementation beside it.

**The rule now.** JSON is `purrdf_lex::json` (with `json::record` in place of serde
derives), YAML is `purrdf_lex::yaml`, CBOR is `purrdf_lex::cbor`, XML is
`purrdf_lex::xml`, Ed25519 is `purrdf-ed25519`, byte search is
`purrdf_lex::scan::find_byte`/`find_byte2`, and benchmarks run on
`purrdf_testkit::bench`. `scripts/check-banned-deps.py` refuses every one of these
packages and their closures on any edge (`memchr` as a direct dependency; `regex` is
built without the prefilter that used it). `sha2` stays: it is the workspace's one SHA-2
implementation. `dependency-ledger.toml` gives every remaining package a census verdict.

### RDF/XML documents are refused for their DTD

**Was stated in** `crates/rdf/src/nesting.rs`, in the XML nesting guard:

> those documents are refused for their DTD anyway, and this keeps THIS guard from
> pre-empting that refusal with a wrong one.

**Why it was believed.** The XML reader refused every document type declaration, and a
refused DTD cannot cause an entity fetch or an expansion bomb.

**What changed.** RDF/XML documents routinely declare internal entities for namespace
IRIs (`<!ENTITY xsd "http://www.w3.org/2001/XMLSchema#">` used as `&xsd;integer`), and
XML 1.0 §4.4 requires a non-validating processor to expand an internal entity it has
read. Refusing the DTD refused well-formed RDF/XML.

**The rule now.** The RDF/XML, TriX and RIF-XML readers read through `purrdf_lex::xml`
with the internal subset enabled: internal entities expand under the reader's expansion
budget, and an external subset, an external entity or a parameter entity is refused, so
no document causes a fetch. GraphML, DataCite and SPARQL Results XML still refuse any
DTD. Pinned by `internal_entities_expand_in_rdfxml_and_external_ones_are_refused`
(`crates/rdf/src/native_codecs/rdfxml.rs`),
`an_internal_entity_expands_and_an_external_one_is_refused`
(`crates/rdf/src/nesting.rs`) and
`an_external_entity_is_refused_and_an_internal_one_is_read`
(`crates/lex/src/xml/tests.rs`).

### `purrdf-xsd`'s one runtime dependency is `purrdf-hash`

**Was stated in** `crates/xsd/README.md`, the root `README.md` crate table and
`AGENTS.md` (`purrdf-xsd` | Foundation over `purrdf-hash` alone):

> its one runtime dependency is the zero-dependency root `purrdf-hash`

**Why it was believed.** The XSD value space needed only the hex-digit reader of the
root; its lexical checks were written in the crate.

**What changed.** The whitespace facets' chunked prechecks classify each byte with
`purrdf_lex::scan::in_runs`, the one byte-run membership test, so `purrdf-xsd`
depends on `purrdf-lex`.

**The rule now.** `purrdf-xsd`'s runtime dependencies are `purrdf-lex` and
`purrdf-hash`, both first-party and with no third-party dependency; `layers.toml`
holds the edge.

### `purrdf_cdt::TextDirection` is the one RDF 1.2 base-direction type

**Was stated in** `helpers-ledger.toml` (`home = "purrdf_cdt::TextDirection"` for the
`text-direction` job) and the `AGENTS.md` crate map:

> `purrdf-cdt` … and `TextDirection`, the one RDF 1.2 base-direction type

**Why it was believed.** `purrdf-cdt` was the lowest crate every direction-carrying
layer (the IR, the query algebra, the composite datatypes) reached.

**What changed.** The event protocol carries a directional literal's direction too,
and `purrdf-events` sits below `purrdf-cdt`. It held an enum of its own, agreeing with
the `purrdf-cdt` one variant for variant, and `purrdf-core`'s ingest mapped between
the two. One type in the lowest layer that carries a direction needs no mapping.

**The rule now.** `purrdf_events::TextDirection` is the one type (`as_str`,
`from_str_token`); `purrdf_cdt::TextDirection`, `purrdf_core::RdfTextDirection` and
`purrdf_sparql_algebra::ast::BaseDirection` re-export it, and `helpers-ledger.toml`
names it as the job's home.
