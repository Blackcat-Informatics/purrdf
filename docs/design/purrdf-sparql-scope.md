<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Binding identity and scope in SPARQL algebra

Binding preservation needs three kinds of evidence: the category and owner of a
binding, the relationships a transformation must preserve, and the observable
multiset its algebra produces. An identity graph establishes the first two only
when its declarations and correspondence come from an authoritative input. It
does not establish unrestricted query equivalence.

## Semantic categories

| Category | Identity and lifetime | Observable behavior |
| --- | --- | --- |
| User variable | Its legal name in the applicable query/subquery scope | A projected binding, or an expression input, under the query's visibility rules |
| Pattern blank | An existential owned by one source basic graph pattern | Matches a graph term; its spelling never identifies a particular dataset blank |
| Generated path witness | One fresh non-distinguished binding per translated sequence junction | Connects the translated operands, including generated UNION arms; disappears from visible columns without merging rows |
| Template blank | A template label interpreted under its allocation rule | Shared within one instantiated template; fresh across solution rows and applicable requests |
| Dataset blank | A concrete RDF identity with its dataset scope | Substitution or pre-binding preserves that identity rather than converting it to an existential |

The normative rules are [BGP blank-label scope](https://www.w3.org/TR/sparql11-query/#BGPsparql),
[property-path translation](https://www.w3.org/TR/sparql11-query/#sparqlTranslatePathPatterns),
and [template blank allocation](https://www.w3.org/TR/sparql11-query/#templatesWithBNodes).
The RDF carrier is RDF 1.2; historical SPARQL 1.1 clauses supply the applicable
query-algebra laws. Quoted-term cases have an explicit RDF 1.2 profile.

`Variable` keeps its `Arc<str>` layout. `Variable::hidden` uses an unspellable
identity; `hidden_blank` additionally records the source BGP blank label.
Internal per-leaf blank slots and pre-bound expression stand-ins have distinct
namespaces. A test for an arbitrary leading NUL would incorrectly combine those
categories. The hidden-witness predicate is the existing `Variable::is_hidden`.

## Invariants and minimized hazards

For `?s (ex:p|ex:q)/ex:r ?o`, the intermediate identity belongs to both
alternative operands and the following edge:

```text
Join(Union(BGP(s, p, h), BGP(s, q, h)), BGP(h, r, o))
```

Replacing one occurrence of `h` with a different existential loses the
connection. Ordinary hidden-observer admission can accept that altered algebra:
all remaining variables occupy legal match roles. Its intended predecessor is
needed to establish the lost relationship.
The independently specified data makes the difference measurable: the connected
translation produces three `o` rows; a lost or prematurely projected junction
produces six `o` rows and three `other` rows.

| Hazard | Minimized example | Legal neighboring control | Required evidence |
| --- | --- | --- | --- |
| Accidental capture | Rename a generated witness to a caller's `?x` | Bijective renaming into fresh names | Category declarations and an injective correspondence |
| Source scope escape | Write the same source blank label in separate source UNION BGPs | Compiler-created UNION arms with independent local blanks, or one explicitly shared generated witness | Source BGP ownership; raw spelling alone is insufficient |
| Lost connection | Rename only the final-edge occurrence of the sequence witness | Rename every occurrence consistently | Original binding partition and source-site correspondence |
| Merged fresh bindings | Give two independent sequence junctions one identity | Share the single junction across alternatives where translation requires it | Distinct authoritative origins and injectivity |
| Premature projection | Remove a junction before the next edge joins it | Remove it at final visible result egress, retaining each row | Binding liveness and projection boundary contract |
| Explicit observation | Project, group, calculate with or emit a hidden witness | Use it in match terms or internal match VALUES | Typed role admission |
| Carrier alias capture | Reuse a caller's legal name as an internal alias | Fresh legal aliases after a complete caller-name census | Alias map, reserved ordinary names and visible projection |
| Lost bag multiplicity | Replace `ex:p\|ex:p` with one arm | Reassociate joins or distribute a join over all UNION arms | Source branch multiset and the transformation's bag law |
| Template identity reuse | Allocate the same output blank for two solution rows | Repeated label within one instantiated template | Runtime allocation and result graph controls |
| Dataset/category confusion | Replace a pre-bound concrete blank with pattern blank syntax | Keep the scoped dataset term in the binding carrier | Concrete term identity and native/prepared execution controls |

Source blank reuse and generated witness sharing have different authorities.
Rejecting all repeated blank spellings in raw UNION arms would create a false
positive: compiler-produced local existential slots may legitimately use equal
opaque spellings. Conversely, accepting a syntactically valid tree cannot prove
that a translator preserved a source binding it failed to record.

## Three candidate designs

**Existing identities plus borrowed admission.** This retains compatible public
algebra and checks hidden identities only in their observable roles. It supports
shared generated witnesses without asking raw labels to cross source-scope
boundaries. It cannot recover missing source intent, prove a bag-preserving
rewrite, or establish runtime template freshness.

**Explicit binder/scope declarations.** A typed side record distinguishes origin,
owner, category and allowed uses. Uses are checked against declarations; a
projected-away binding cannot remain live beyond its declared boundary. This
makes category and declaration/use inconsistencies decidable. A fully declared
but unintended replacement graph can still pass: the graph needs a predecessor
contract to establish which relationship was lost. Replacing `Variable` itself
would also change the public AST layout and downstream construction APIs; a
borrowed or prepared side record avoids that compatibility cost.

**Transformation certificates.** Freeze input obligations before changing the
algebra. A certificate carries stable source sites and an explicit correspondence
to output bindings, category/owner declarations, visible schema, liveness and
source branch multiplicities. Check partitions under a bijective rename rather
than comparing variable spelling or complete tree hashes. Distribution copies
syntax while preserving the source-site obligation in each relevant arm;
reassociation changes operator shape while preserving connections. The checker
establishes only its declared relation. General expression equivalence, arbitrary
property-function behavior and runtime allocation require different evidence.

The recommended design combines the existing compact identities, one typed
borrowed role checker, provenance-bearing side records where a transformation
needs them, and semantic bag controls. A wholesale AST replacement does not by
itself supply the missing predecessor relationship. The experimental graph types
belong to dev-only evidence; their finite contract does not justify replacing the
compatible public AST.

## Authoritative homes and boundary contracts

`purrdf_sparql_algebra::scope` owns hidden observer invariants and typed diagnostics.
Existing validation entry points delegate to this home. It uses the exhaustive
borrowed `NodeRef` traversal, so quoted terms, nested expressions and deep trees
share one child enumeration. Deterministic node locations identify algebra sites;
they are not fabricated source byte offsets. Ordinary raw name and term admission
remains a separate existing contract.
The typed `ScopeError` exposes the binding, hazard, observer role, location and
`action()` repair guidance; the existing parse-error adapters retain their API.

The same module owns the existing positive-spine blank-ownership walk. Evaluation
and text serialization both use it: positive joined fragments share their source
blank identity, while raw UNION-arm blanks stay local. Serialization gives
independent owners distinct legal blank labels. If required braces split a shared
owner into separate textual BGPs, it carries that identity as a fresh unprojected
variable instead. Ordinary parsed blank syntax retains its bytes when no such
repair is needed.

| Boundary | Check and authority | Failure evidence |
| --- | --- | --- |
| Parser | Source BGP ownership, fresh path origins and legal source roles before translation | Source label/position and conflicting owner; query syntax cannot spell a hidden identity |
| Public raw/compiler algebra | Typed hidden observer admission plus structural admission; explicit source declarations when supplied | Hazard, observer role, identity and deterministic algebra site |
| Split/merge and distribution | Preserve binding partitions under source-site correspondence, owner/category and source-arm multiset | Missing site, changed partition, ownership mismatch or missing branch |
| Substitution and alpha-renaming | Injective permitted rename; preserve concrete dataset terms, ordinary bindings and quoted slots | Capture/category mismatch or non-bijective correspondence |
| Projection and repeated normalization | Preserve live connectors until the declared egress boundary; normalized identity obligations remain stable | Escaping use or removed live connector |
| Serialization and reparse | Fresh alias bijection after complete caller census, explicit visible schema, bag-equivalent carrier | Alias collision, changed ordinary name, visible-schema difference or semantic bag mismatch |
| Result egress | Remove non-distinguished columns one row at a time; apply visible identity to DISTINCT and aggregate observations | Hidden visible column or incorrect observable bag |
| Template instantiation | Keep template-local sharing and per-row/request allocation identities | Result graph with merged independently allocated nodes |

Production provides typed role admission and hygienic carrier admission. The
dev-only prototype evaluates predecessor certificates. The table specifies the
authority each boundary requires.

The implementation specification is bounded by these responsibilities:

1. Keep variable storage and the parser's canonical hidden identity constructors.
   Record source ownership/origin only at boundaries that possess it.
2. Use the algebra-owned typed checker for role admission; preserve existing
   public validation adapters and their accepted language.
3. Build declaration/use side records once during preparation where reuse needs
   them. Their memory belongs to the measured prepared artifact, not to every
   `Variable` value.
4. Require each transformation to declare its correspondence and permitted bag
   law. Freeze predecessor obligations before applying the transformation;
   constructing obligations solely from the result certifies the same defect.
5. Check liveness and alias mappings at projection/carrier boundaries. Retain
   semantic controls for cardinality and concrete identity; a structural success
   never replaces them.
6. Retain minimized failing mutations and neighboring legal controls. Run
   deterministic property generation with shrinking, plus independent template
   freshness and native/prepared/carrier result oracles.

Compiler-algebra diagnostics remain distinguishable from normative syntax
failures and from resource-admission limits. The experimental certificate proves
its stated finite obligations; it does not provide a universal semantic proof.

## Measured candidate bounds

The executable corpus has 14 legal neighboring controls and 19 intentionally
altered inputs. Each detector reads the supplied altered algebra or declarations;
case names do not select its answer. All three admit all 14 legal controls.
The observed detection counts are:

| Candidate | Detected hazards | Undetected hazards | False positives among these controls |
| --- | ---: | ---: | ---: |
| Existing hidden-role admission | 1 | 18 | 0 |
| Explicit binder declarations | 10 | 9 | 0 |
| Frozen predecessor contract | 19 | 0 | 0 |

These counts describe this finite corpus, not a population detection rate.
The certificate prototype checks positive SELECT BGP/Join/UNION binding
incidences with flat subject/object terms and named source predicates, constant
GRAPH names, outer projection and inner projection that retains ordinary
bindings. Small VALUES records and flat CONSTRUCT template category records
extend that model; template graphs must be absent or constant. Source predicates
and region markers carry trusted research provenance. It retains duplicate
branches and the predecessor's binding partition, so a completely declared
unintended split or merge still fails.

The experimental proof has a closed admission boundary. A form whose binding
incidences it cannot represent must produce a typed refusal and a
`not_established_from_input` report status. Such a refusal is not credited as
detection of an intentionally altered supported input. Production admission and
the independent semantic controls cover the full quoted-term and subquery
behavior separately.

The report separates 13 unavailable inputs and six unavailable transformations
from the 33 supported comparison cases. Typed refusals cover recursive pattern or
ground triple terms, variable query/template GRAPH names, variable predicates
without source provenance, inner projection removing ordinary bindings,
ASK/DESCRIBE observer forms, contradictory reused source sites and independent
raw owners whose copied sites cannot name distinct owners. The last boundary is
observable: two separate GRAPH owners over two edges yield four rows, while
merging them into one positive raw-blank owner yields two. A contract cannot be
frozen from that ambiguous predecessor. UPDATE is outside this Query-based
prototype and is covered by the production allocation experiments.

No candidate establishes arbitrary ground-term/filter changes, VALUES
constant/UNDEF semantics, general OPTIONAL/MINUS equivalence or runtime allocator
freshness. Execution controls independently establish their named expected bags
and identity invariants. This finite corpus does not establish general query
equivalence.

The actual Rust layouts are:

| Type | Native x86_64 bytes | wasm32 bytes |
| --- | ---: | ---: |
| `Variable` | 16 | 8 |
| `Binder` | 12 | 12 |
| `Identity` | 40 | 24 |
| `Atom` | 64 | 48 |
| `Contract` | 72 | 36 |
| `Evidence` | 312 | 156 |

The side record adds memory without changing public `Variable`. All 13
comparative tests, including the seeded properties and 100,000-deep inputs, run
both natively and on wasm32 through the first-party Node harness. The layout
table is measured on each target; allocation costs and timings below are native
measurements.
Allocation windows count requested traffic, live retention and peak working
bytes separately, including allocated map nodes and excluding system allocator
bookkeeping or rounding. For the representative input,
encoded-variable declaration preparation retains 596 bytes and contract
preparation retains 2,376 bytes, with a 4,916-byte working peak. The existing
role check allocates nothing on that input. At 100,000 nested graph nodes,
the AST's conservative `Query::retained_size_bytes()` accounting charges
18,000,685 bytes; it charges shared strings per occurrence and excludes allocator
metadata. The allocation window measures 1,022 bytes retained by the contract, and
contract preparation peaks at 3,148,064 working bytes. Deep checks remain
iterative; machine-stack safety does not imply constant heap scratch.

Reuse avoids rebuilding the predecessor contract, but still extracts and checks
the supplied output algebra. Its cost belongs at a transformation/admission
boundary rather than being charged once for every solution row. Encoded-variable
declaration measurements cover the current variable record census; they do not
include obtaining authoritative source ownership that the input did not retain.

## Timing method and measured costs

The native run used an AMD Ryzen AI MAX+ 395 on x86_64 Linux, with 16 physical
cores and 32 logical CPUs. The compiler was `rustc 1.100.0-nightly`
(`4b6d04e70`, 2026-09-13). `scope_checks` uses the workspace bench profile:
optimization level 3, fat LTO, one codegen unit and symbols retained without
DWARF. Each of the 30 benchmarks used three seconds of warm-up, five seconds
of measurement and 100 samples. The first-party harness reports median and MAD
with a seeded 10,000-resample bootstrap and 95% interval. Preparation uses
`iter_with_large_drop`, which excludes destruction of the returned artifact
from its timed interval. Reuse includes extracting and checking the output.

Median microseconds per operation:

| Input | Current role check | Explicit check | Variable declaration preparation | Contract preparation | Reuse check |
| --- | ---: | ---: | ---: | ---: | ---: |
| Representative | 0.253 | 4.150 | 0.682 | 3.548 | 4.477 |
| 256 branches | 8.288 | 125.937 | 13.174 | 112.907 | 156.210 |
| 1,024-triple BGP | 18.599 | 536.501 | 44.315 | 494.946 | 494.447 |
| 64 nested GRAPH nodes | 0.585 | 2.580 | 0.667 | 4.952 | 5.142 |
| 512 nested GRAPH nodes | 3.370 | 16.277 | 2.960 | 17.464 | 17.197 |
| 100,000 nested GRAPH nodes | 940.842 | 5,346.298 | 759.970 | 4,107.285 | 3,763.334 |

These measurements establish the cost of this prototype on the recorded host.
There is no timing acceptance threshold or cross-hardware speed claim. Reuse
saves retention/preparation work but still pays for the output's incidence
extraction and comparison; it is not uniformly faster than preparation.
At depth 100,000, the role-check median interval is 0.869–1.053 ms and the reuse
interval is 3.738–3.809 ms. The complete samples, intervals, outliers, compiler,
hardware and source hashes are retained in
[scope-benchmarks.json](evidence/scope-benchmarks.json).
[scope-investigation.json](evidence/scope-investigation.json) records all supported
and unavailable outcomes, layouts and native allocation windows. The allocation
probe runs in the optimized development profile with debug assertions and
overflow checks enabled; timings use the bench profile described above.

## Carrier and allocation boundaries

Concrete dataset blanks can enter native algebra through ground bindings, with
their `(label, scope)` identity intact. SPARQL text has no ground blank constant:
`_:label` in a graph pattern is an existential, and VALUES does not admit that
syntax. A checked carrier must refuse a concrete blank binding it cannot express,
including a blank nested in a ground triple term. Native injection remains a
separate supported boundary.

A carrier with no visible source variables uses a synthetic unit column to carry
each solution row. The SERVICE adapter restores the empty source schema by
removing precisely that transport column, preserving the row count. Direct
execution of the carrier still exposes its declared unit column; comparing it
to a zero-column source bag requires that explicit restoration.

Template label counters separate allocations within an evaluation. Publication
into an existing destination also requires a namespace disjoint from its retained
default-scope blank identities. The template home supplies one deterministic
namespace selection law, used by CONSTRUCT append and UPDATE. UPDATE inventories
the native mutable destination through a borrowed visitor, including suppressed
base terms and blanks nested in delta triple terms and composite literals;
it does not freeze the dataset to perform that check. A blank-free operation
whose registered expressions cannot have stateful effects avoids this census.
The namespace remains under a caller's requested prefix for INSERT templates;
the variable-free DATA path keeps its existing prefix contract.

Runtime allocation has a separate obligation from BGP ownership. Under
[SPARQL 1.1 §17.4.2.9](https://www.w3.org/TR/sparql11-query/#func-bnode),
`BNODE()` must not reuse a dataset blank. The evaluator's shared fallible mint
seam skips occupied default-scope identities through reverse lookup and a lazy
index of concrete inputs, including blanks inside triple terms and composite
literals. Stateful raw, prepared and UPDATE patterns register ground inputs
before either sibling evaluates. User-function bodies that can allocate inherit
the caller's reservations. Reserved label storage is charged to the scratch
governor; a reverse-lookup failure remains a source-read error.
Input registration stops at the first scratch-budget trip. Each independently
evaluated arena tracks its own charged growth against the shared request budget,
including reservation copies in user functions. Inclusive budget controls cover
the exact retained-label charges. A template trip produces a typed exhausted
query outcome with an empty certified graph; UPDATE publishes no staged mutation.

These ownership and allocation checkpoints are specified by governor profile 10.
Separately charged aggregate buffers and child arenas cannot mask a later arena
allocation: the twelve-input SUM fixture retains 879 bytes of input values plus
its separate 74-byte result, costing 953 scratch bytes. The first-party profile
corpus pins the measured costs and inclusive boundaries under its new version
and content identity; consumers must remeasure scratch ceilings sized for the
preceding profile. Official conformance and GTS corpus bytes are unchanged.

BNODE, template and list-cell allocations use that seam. CONSTRUCT builds once
instead of replaying the template after a counter collision. A destination with
an existing `_:c1` therefore receives the fresh `_:c2`, while no-collision labels
retain their counter spelling. This collision case deliberately changes the
emitted label bytes while keeping the carried dataset identity distinct.

SERVICE ingestion treats `(label, scope)` as response-local identity. One mapping
per response preserves repeated identities across rows, bare bindings, triple
terms and composite literals, and gives separate responses identities distinct
from local data, computed inputs and prior allocations. It remaps only admitted
schema cells; surplus cells in a malformed response cannot allocate discarded
identities. Controls assert exact partitions and bag multiplicity, including
100,000-level triple terms on a 128 KiB stack. These execution obligations are
not proofs obtainable from the scope prototype's AST alone.

The existing end-to-end CONSTRUCT benchmark measures this execution seam over
30,000 source rows, with warm plan/order caches and no governors attached.
Two consecutive native runs on the recorded host used ten samples per case:

| Template | Previous allocator median (ms) | Shared allocator median (ms) |
| --- | ---: | ---: |
| Carries dataset blanks, allocates none | 10.100 | 9.108 |
| One fresh blank per row | 15.863 | 12.742 |
| One fresh blank shared by two output triples per row | 29.090 | 19.676 |

The blank-free control also moved, so these observations do not isolate a
kernel's causal speedup. The co-reference run has two high severe outliers;
its median interval is 19.14–22.67 ms. Full samples, intervals, source hashes,
compiler and workload parameters are in
[scope-runtime-benchmarks.json](evidence/scope-runtime-benchmarks.json).
These are native execution measurements, separate from the scope-prototype
timings and its measured wasm layouts.

Opaque raw pattern labels use the allocation-free
`purrdf_lex::terminals::is_valid_blank_node_label`, also re-exported at the
existing core codec API. Out-of-alphabet labels receive injective fresh aliases;
distinct identities and repeated uses survive without inventing a second
character-class implementation.

## Reproducible evidence

The comparative candidates have one dev-only implementation in
`crates/sparql-algebra/tests/support/scope_candidates.rs`. The integration suite,
report example and benchmark consume that implementation. Machine-readable
outcomes separate detected mutations, accepted legal controls and properties not
established by the available input.

```bash
cargo test --locked -p purrdf-sparql-algebra --test scope_invariants --test scope_candidates --test scoped_carriers
cargo run --locked -p purrdf-sparql-algebra --example scope_investigation
cargo bench --locked -p purrdf-sparql-algebra --bench scope_checks
cargo bench --locked -p purrdf-sparql-eval --bench query_eval -- construct_blank
cargo test --locked -p purrdf-sparql-eval --test scope_interactions
cargo test --locked -p purrdf-rdf --test scope_portable
CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER="$PWD/scripts/wasm-test-runner.sh" \
  cargo test --locked --target wasm32-unknown-unknown -p purrdf-sparql-algebra --test scope_candidates
make check
make wasm
make conformance
bash scripts/check-generated.sh
```

The comparative suites assert the experiment's finite expectations. Complete
repository qualification and official conformance are separate gates.

Portable RDF/query/expected-result fixtures are distinct from the frozen official
corpora. Their manifest declares stable semantic identifiers and profiles, and
their rationales derive exact observable bags independently of an engine's
answer. Generic SPARQL MINUS, VALUES and subquery legality must not be credited
as SHACL profile admission. Internal property functions, hidden identity
admission, witness lifetimes and planner budgets are implementation evidence,
separate from normative W3C conformance.
