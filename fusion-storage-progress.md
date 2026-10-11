# Fusion native ownership source progress

Root is the sole source writer. Production RankedStreamImpl carries the same
scope_workspace through materialized and on-demand reads; the actual adapter
forwards it into fusion. Caller-built streams declare no native account.
Frontier and nested signature/rank selection use the single lower ordered arena;
candidate namer/asked/excluded and duplicate-allowed sets use its membership
wrapper. Emitted storage uses the existing fixed-hash AdmittedMap. Wide bitsets
and contribution arrays now retain AdmittedVec ownership; immutable output clones
share the admitted control rather than copying the contribution vector.
Original variant/word ordering and borrowed RankOrder selection are preserved.

Independent read-only review identified remaining test adaptations and the native
protocol-to-fusion boxing defect. Both were corrected. A later native storage
refusal is latched before returning to the public caller; subsequent next/trailer
return the first native cause before any producer callback. Successful trailer
then next continues as before. Wide membership/word-order oracle source was added.

No compiler, runtime acceptance or full gate has run against this source batch.
Constructor arrays now use the shared admitted iterator factory and its fallible
constructor propagates through the real fuse caller. Degradation arrays and its
shared control, ordered stratum lookup and pending exclusion array also retain
native owners. Validated IRI copying preserves original component spans through
the existing IRI home, with before-copy admission and a shared WorkspaceIri owner;
fusion's owned contribution/status keys now use this native factory. The copy
fixture checks independent re-recognition, exact capacity/refund and refusal.
Fusion now borrows caller-owned profile configuration through the same constructor
used by owned profiles. Fused/Search result arrays retain their admitted buffer
and shared control. Duplicate-stratum and threshold scratch borrow existing keys
inside admitted membership arenas. The diagnostic strata formatter walks the
original registration slice without collecting a temporary vector. Settlement
scratch is admitted before producer callbacks. Per-row perturbed sets and report
deficit/inflation/unbounded sets retain immutable shared native membership; the
two identical loss directions share one arena. Existing native/portable semantic
oracles were adapted, and original-reservation fixtures now require retained
arena/control grants after workspace drop, allocation-free sharing, final-owner
release and typed first-insert refusal before publication. These fixtures are
written but unrun. The observed-resolution report now retains an admitted ordered
map and control; its renderer borrows that original map. The production evidence
identity drives the same canonical encoder directly into the original BLAKE3
record hasher, using the shared framing home and original domain prefix, without
materializing bytes. Public re-derivation still writes the original bytes through
that encoder. Existing evidence replay assertions require equality of those two
paths when the consolidated acceptance runs. Pinned attestation and final
fidelity/domain/exclusion report maps now retain admitted ordered arenas and
shared controls; the canonical encoder borrows their original iterators. The
one OwnedIri representation serves domain tags and retrieval IRIs, removing the
second ownership body. Candidate domain sets use shared admitted membership;
clones share their original immutable control. Projected row blocks parse through
the original admitted IRI home; grammar errors preserve row context and physical
refusals preserve their typed cause. Original-reservation fixture source requires
retained IRI component spans, sorted membership/intersection semantics, shared
ownership without allocation, grants after scope drop and final-owner release.
These newer sources are uncompiled and unrun. Status maps now retain the original
ordered arena and immutable publication control; native failure reasons share
their original text grant. Caller failure spelling crosses the admitted text
home once. Completion of an immutable status report constructs its replacement
through the same native authority, preserves existing verified statuses, and
returns the original first refusal without publishing a partial report.
Canonical-byte export now returns retained admitted bytes/control through the
same encoder as the streaming digest; the export latches its first typed failure
and never publishes partial bytes. Native executor stream arrays, pending lookup
metadata, status maps, search bridge arrays and unweighted-stratum publication
also retain admission. The actual native fusion entry retains the original
authority even when all producers fail before yielding a stream; caller-created
streams cross the same constructor after one admitted registration boundary.
The original reservation fixture now covers failure spelling lifetime, native
sharing without reauthoring, final-owner release and first-copy refusal with
the original cause. Remaining native metadata and full five-contract acceptance
still require completion. No compiler or runtime claim applies to this batch.
Native query and exclusion preparation now use the original admitted parser/cache
through the actual retrieval callers. Prepared handles retain native variable
spellings and slot arrays; both levels of exclusion lookup arrays retain their
original authority. The shared fallible array collector transports semantic
diagnostics and physical refusals without replacing their cause. On-demand cursor
opening now preserves EvaluationFailure through the actual native call and source
binding path; only the legacy resident entry projects its diagnostic. Scope
binding and retained row-owner creation preserve EvalError. Production stratum
keys cross the admitted IRI copy boundary before their native metadata publication.
New original-reservation tests exercise real query/parameter preparation, shared
plan cloning, owner lifetime after engine/account scope drop, final refund and
first refusal identity. These new tests and source remain uncompiled and unrun.
Scoped materialized execution now reaches the existing admitted evaluator body
under the same original authority, including its visitor before scratch release.
Its row/cell/constraint publication no longer converts physical failures into
strings. QueryWorkspace can borrow an already-owned capability; any legacy child
charge in that borrowed scope retains its original native allocation and resize.
Unserved-term derivation now uses one admitted scratch/output body for both
resident and native callers. Native search publication retains its original array
and immutable control; original semantic fixture assertions remain unchanged.
Planned-resolution publication now admits each original key, nested weight array,
ordered arena and immutable control without re-deriving the waist's values.
Candidate depth storage uses the original ordered arena, with shared immutable
ownership; native publication admits caller keys/arena/control once. Union's depth
iteration now borrows canonical ordering instead of allocating sort scratch.
Candidate contract publication admits keys/domains/arena/control before birth.
Already-native domain, depth and weight owners are shared without reallocation.
Manual native/portable composition fixtures and existing sharing-set assertions
were adapted without weakening their expected semantics.
Candidate loss arrays now retain native publication. Producer bindings/decisions
are immutable caller-plan records retained from compilation through shared owners,
so executing and consuming a candidate stage no longer deep-clones their nested
strings/term-index arrays. The union moves those original owners without another
copy; assertions still require exact records and per-term loss.
Unscoped materialized retrieval now reaches the original governed prepared visitor
body instead of transferring a resident SparqlResult. Scoped and unscoped reads
use one admitted variable/cell/row collector. The governed visitor retains its
original identity frame through complete/exhausted receipt consumption; its
exhaustion box is admitted through the original frame before allocation. Registry
identity, sole-witness checking, original ranking/indexing, probe removal and
pre-depth materialized row counts remain on the production path. Bounded native
execution no longer substitutes a resident memo scratch for the evaluator's
original admitted scratch, even when the supplied dataset itself is resident.
Original-account fixture source now consumes a real VALUES result after prepared
execution, engine and account scope drop, then requires final-cell refund.
Rustfmt parsing of the coherent owner/preparation/materialization source unit
completed successfully. This proves syntax only; the batch remains uncompiled
and all new acceptance tests unrun. Remaining union/native publication surfaces
and the full five-contract qualification still require completion.
Candidate union now retains native ordered subject/producers/rank storage and
native rank arrays. Original lexical term factories admit caller-authored keys
before installation. Publication shares original grants; canonical export uses
the same canonical/evidence encoders with admitted bytes and diagnostic text.
Whole-stage refusal carries partial observed evidence by value, without allocating
an error box after admission stops. Existing duplicate, receipt, moving-attestation
and frozen-byte controls retain their assertions against the native containers.
Formatting passed. Consolidated compiler1 terminated101 on two missing method
documentation comments in the shared lexical home; both are corrected. Runtime
and full five-contract acceptance remain unrun for the new source batch.
Compiler collections2-6 exposed the shared membership authority accessor,
obsolete resident production wrappers still used by tests, receipt Debug,
two original-account fixture API errors, typed tuple constructor support, and
19 downstream retrieval API/type sites. Corrections keep resident regression
helpers test-only, admit counted lookup hash storage through the existing
AdmittedMap, preserve exact row multiplicity, and retain native array views.
The view reservation boundary now has typed native egress beside its legacy
diagnostic projection, both using the same original reservation body.
Scoped and unscoped exclusion lookups now call the original witnessed native
body under the lookup's authority. Visitor failures preserve EvalError instead
of formatting physical failures into producer-status strings. Canonical exports
check their latched first refusal before any nested allocation.
Compiler7 is live in native-publication-compiler-7.log; no compilation/runtime
or full-contract PASS is claimed for the current batch.
Consolidated workspace/all-target compiler16 is terminal0. This includes all
shipping crates, host bindings, examples, benches and test targets. The final
collected fixture migrations preserve original independent expected bytes,
statuses, ranks, multiplicity and cross-schedule answers; output comparisons
borrow native carriers or explicitly snapshot caller-side test oracles.
Python egress borrows native membership/metadata/arrays until converting actual
language values, without intermediate Rust deep copies.
Strict workspace/all-target Clippy1 is live in native-publication-strict-1.log.
This compiler PASS does not establish runtime, ownership/refusal, performance,
portable or full five-contract acceptance. No commit/PR/merge is claimed.
Strict1/2 stopped on the nested renderer's argument/style sites. The native
renderer now accepts its three delimiters together, preserving the unchanged
resident entry and exact spelling; statement arms are explicit.
One non-qualifying workspace/all-target diagnostic collection reached every
downstream target and reported12 distinct lint sites/families. All are corrected
as one batch. Shared borrowed iteration forwards the original payload iterator;
receipt identity frames are explicitly retained through consume; native on-demand
faults now carry a disjoint original EvalError rather than an optional error plus
unused String. Native indexing/retained-row failures stay typed through that
same original fault home. No error box is allocated after refusal. The concrete
depth-check error retains its existing identifiers by value under a reasoned
local lint expectation, matching the union's original first-refusal requirement.
Unchanged strict workspace/all-target Clippy3 is running in
native-publication-strict-3.log. Diagnostic completion is not qualification.
Full original five-contract scope remains required;
no PR or completion claim is authorized by this progress record.

Strict3 terminated PASS for the complete workspace/all-target batch. The next
review found result-control refusal returning before settling already-open reads.
Corrected both that path and later setup-arena refusal: every invocation settles;
the failure moves its original execution/read arena, metadata and missing-entry
settlements into the returned owner without allocating replacement evidence.
The focused regression preserves the first physical cause, checks both reads'
settlement slots and verifies the exact original buffer pointer survives.
Runtime1 was an invalid unsupported cargo test flag invocation; no test ran.
Runtime2 stopped compiling at a mutable AdmittedVec iteration site in this new
batch; corrected through the original slice iterator. Strict4 collected only
test-module placement, corrected by moving the module after production items.
Normal strict5 is terminal PASS, exit0, on all workspace/all-target consumers.
Original native semantic/ownership/shape matrix is now live session8973
in native-publication-runtime-3.log; poll that actual execution handle to determine
live or terminal state.
No runtime, full-contract, full-gate or publication acceptance is implied yet.

Runtime3 is terminal101 during code generation: depth_free_membership and
text_exclusion_lookup hit the compiler's recursive Unpin auto-trait depth through
SharedWorkspace<NativeSparqlEngine> and its complete prepared-cache graph. The
original lexical Shared pointer now explicitly implements Unpin independent of
its payload: moving this pointer never moves the allocated payload. Send/Sync,
drop checking, ownership, allocation, admission and counters are unchanged; no
lint suppression or consumer recursion-limit increase was added. A PhantomPinned
payload control checks owner Unpin and unchanged payload address across moving
the owner into a vector. Runtime4 caught unnecessary qualification in that test,
corrected without waiver. Strict6 followed by runtime5 are now live sequentially
in original execution94924 under the existing4-job16GiB/no-swap lane. Logs are
native-publication-strict-6.log and native-publication-runtime-5.log. No runtime
or final qualification is implied until the actual phases terminate successfully.
