<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Canonical labelling of blank nodes

This document records the design of the canonical labelling of blank nodes,
`nested-refinement-labelling` version `1`, and the research behind it: the problem
and its measurements, the requirements and their sources, the published work
consulted, each choice with the alternatives weighed, the status of each claim,
the open questions and the consequences for consumers.

**Status: provisional.** The definition is provisional until two independent
implementations agree on shared vectors. This repository holds one definition
implemented twice (a reference engine and an optimised engine). Both are held to
each other, but they share a reader of the specification and are not independent
in the sense that matters. The document is updated as the design settles.

The normative statement is the frozen specification, revision 3.12, SHA-256
`088cb74fba0f5f733224157ab7c3a6e06f4a3f3447edf4c01bb15cf15c58809a` (the identity
recorded in [`MIGRATION-5.0.md`](../MIGRATION-5.0.md)). The specification text is
not part of this repository. What this repository carries is the implementation
(`crates/rdf-core/src/canonical_labelling/`), the worked examples that revision
states (`crates/rdf-core/src/canonical_labelling/vectors/`), and the lowering and spelling rules of the migration guide.
This document describes the design and its evidence; it does not restate the
specification.

## 1. The problem

### 1.1 What RDFC-1.0 does with tied blank nodes

RDFC-1.0 [1] labels blank nodes in two stages. *Hash First Degree Quads* hashes,
for each blank node, the quads that mention it with that node and every other
blank node replaced by two markers. A node whose hash is unique is named at once,
in hash order. *Hash N-Degree Quads* handles the nodes whose first-degree hashes
tie. For one node it groups the related blank nodes by the hash of their relation,
tries every permutation of each group, recurses into the members not yet named and
keeps the lexicographically least path.

A group of `g` related nodes with equal relation hashes therefore costs at least
`g!` permutations at one level, and each permutation recurses into the members'
own neighbourhoods, so the factors multiply down a tree. Nothing in the algorithm
separates like siblings by what lies beneath them: it permutes them. Colour
refinement (section 4) separates such siblings in a few rounds when their
subtrees differ. The Recommendation states that the algorithm "can behave poorly"
on some inputs, requires implementations to defend against denial-of-service attacks and notes
that calls to *Hash N-Degree Quads* may be limited [1, section 4.4.3]. Section 7.1
of the Recommendation (non-normative) discusses poison datasets. It names no
structural remedy.

The 4.x engine of this repository ran RDFC-1.0 and extended it to the RDF 1.2
constructs RDFC-1.0 does not address, by spelling reifier bindings and annotations
as quads that carry reserved IRIs (`docs/RDF12-CANON-PROFILE.md`). It defended
itself with a caller's budget and a typed refusal.

### 1.2 The measured case

A tree of 246 blank nodes inside a dataset of 186,485 blank nodes took nearly all
of the 4.x engine's search work: 306.8 million charged units of the 4.x charging
law (one unit per *Hash N-Degree Quads* frame entered and one per permutation
attempted), measured on a debug build. The dataset is private and is not
reproduced. A synthetic tree of the same shape has 246 blank nodes and 491
statements: two hubs, each with ten children that differ only in the length of the
chain below them. Its cost needs the hub to be not unique at first degree. Only
unit counts are recorded; no time measurement is part of this document or of the
acceptance of the work.

The shape is ordinary. It needs a blank node with several blank children that look
alike at first degree and differ further down, which a record with repeated
sub-records, a list of alike blank items or a repeated annotation pattern
produces. Hogan's survey of blank nodes in 43.6 million web documents found
blank-node structure mostly trivial with hard cases rare but present: the largest
document held 254,288 blank nodes, one held a 16-clique of indistinguishable blank
nodes, and with automorphism pruning disabled 971 graphs did not finish within the
author's limit [3]. No published measurement covers RDF 1.2 structure (reifiers,
annotations, nested triple terms); none was found.

### 1.3 Counts of the new engine

`cargo bench -p purrdf-core --bench canonical_labelling`
(`crates/rdf-core/benches/canonical_labelling.rs`) labels named shapes in a
separate process and reports counts only: no clock is read and nothing asserts a
threshold. The counts below come from one run at the head this document was
written against, with the default scale (1,000 for the first three shapes).
`explored units` is the optimised engine's work units over the calls it explored,
summed over all four parts (start, refine, fix, entries); the limit counts every
part except start. `reference units` is the same figure for the reference engine
over the same input. The
allocation columns come from the counting allocator over admission and search.

| Shape | Nodes | Lines | Guaranteed | Branching calls | Explored units | Reference units | Allocations | Requested bytes | Retained bytes |
|---|---:|---:|---|---:|---:|---:|---:|---:|---:|
| path | 1,000 | 999 | yes | 0 | 19,977 | 19,977 | 48,192 | 11,644,303 | 419,678 |
| triangles | 3,000 | 3,000 | no | 1,000 | 96,994 | 131,994 | 323,281 | 20,826,948 | 1,858,804 |
| hub with leaves | 1,001 | 1,000 | yes | 0 | 18,999 | 18,999 | 66,295 | 6,760,930 | 476,213 |
| complement of cycles 4, 6 | 10 | 70 | no | 4 | 6,974 | 68,202 | 17,403 | 1,317,215 | 27,850 |
| complement of cycles 3, 5 | 8 | 40 | no | 4 | 2,726 | 26,100 | 7,421 | 569,530 | 16,800 |
| quadratic residue 13 | 13 | 78 | no | 2 | 6,632 | 76,843 | 10,706 | 772,214 | 30,403 |
| cub8 | 8 | 24 | no | 3 | 1,282 | 5,932 | 3,112 | 224,580 | 10,934 |
| `Z4 x Z4` | 16 | 96 | no | 4 | 18,739 | 594,112 | 38,203 | 2,766,276 | 36,680 |

`cargo bench -p purrdf-core --bench canonical_labelling -- --counts` reports
retired user-mode instructions, allocations and requested bytes of `label` in fresh
processes. The figures below are the median of five samples:

| Shape | Instructions | Allocations | Requested bytes |
|---|---:|---:|---:|
| path of 1,000 nodes | 61,416,594 | 48,192 | 11,644,303 |
| quadratic residue 13 | 5,084,033 | 10,706 | 772,214 |
| 3-cube | 1,533,933 | 3,572 | 269,744 |
| `Z4 x Z4` | 17,081,718 | 38,203 | 2,766,276 |
| claims: four messages of 20 reifiers, 6 to 8 annotations each, guaranteed | 14,645,767 | 32,487 | 2,380,158 |

Observations: a path and a hub with leaves, the shapes with a long chain or many
alike leaves, cost units close to linear in the lines and are labelled without a
search. `triangles` (1,000 identical directed triangles) branches once per
triangle and still costs about 32 units per node. The symmetric graphs show what
the walk's skips and stops save over the reference engine's tree, by a factor of
about 4.6 on `cub8` and about 32 on `Z4 x Z4`. These counts do not yet include the
4.x engine on the same shapes (section 6).

## 2. Requirements and their sources

| Id | Requirement | Source |
|---|---|---|
| Q1 | RDF 1.2 only. The input includes triple terms, reifier bindings, annotations and blank graph names; no lowering drops them. | `AGENTS.md`, RDF 1.2 only |
| Q2 | The core builds for `wasm32-unknown-unknown` and needs no threads. | `AGENTS.md`, everything is wasm-able |
| Q3 | No compromise inherited from earlier work. Agreement with RDFC-1.0 is not a requirement. | `AGENTS.md`, GREENFIELD and SOTA |
| Q4 | Byte determinism across implementations and languages: no dependence on hash-table order, word size or recursion depth. Every comparison is on byte strings or unsigned integers of stated width. Every recursion has an explicit-stack equivalent. | `AGENTS.md`, byte determinism; the container format of GTS |
| Q5 | Working memory linear in the input. | A consumer that canonicalises about 16.6 million quads in 16 GiB |
| Q6 | Meaning is carried by typed structure, never by the spelling of a string. No step of the definition obtains meaning by parsing a string it produced. | `AGENTS.md`, make illegal states unrepresentable |
| Q7 | One hash, with no variants and typed domain separation. | Consumers that already identify content by BLAKE3 |
| Q8 | A stable identity per blank node that a component keeps alone or inside a larger dataset, and a deterministic map from source identifiers to positions. | Consumers that mint identity from canonical labels |
| Q9 | A named, engine-independent admission predicate and a typed refusal that names the component and its size. | Consumers that gate on "no component needed a search" |
| Q10 | A declared empty named graph, including one with a blank name, is canonical input. | The dataset model: a named graph is a member with or without statements |
| Q11 | The work of a search is bounded by an engine limit and a refusal, never by a partial result. | `AGENTS.md`, hard failures |

Two things are not requirements. Byte agreement with RDFC-1.0 on easy data was
considered for three conveniences (a stock tool could check easy data, the W3C
suite would give byte-level coverage, fewer labels would change) and rejected: it
fixes a second serialisation and a mandatory hash inside the labelling. The W3C
suite remains the gate of the RDFC-1.0 entry point. The budget, stop signal and
memory accounting of the 4.x engine are mechanisms of an engine, not properties of
a canonical form: the definition has no caller budget and an engine states its own
limit.

## 3. The design

### 3.1 Input, lines and terms

The input is a set of typed lines (`lines.rs`) of five kinds: `quad`,
`reifier-binding`, `annotation` (with a count), `empty-graph` and `record`. A term
is an IRI, a literal, a blank node or a triple term; absence is always an explicit
option and never an empty string. Admission interns every constant leaf by its
encoding and every blank node by its source identifier, and stores each line as a
token sequence in walk order. Every encoding the definition orders is a function of
those tokens and of what each blank leaf is replaced by, so nothing is spelled as
text and nothing is parsed back. Lowering a dataset view to lines is outside the
definition; `view.rs` and the migration guide state PurRDF's own rule. A composite
literal that names a blank node is lowered to a composite node with one `record`
line, so the blank nodes it names take part in the labelling. A triple term that is
a component of another triple term is lowered to a term node (section 3.6), so every
triple term the definition receives from a view has depth one.

The labelling encodes a language tag as the bytes its input gives, so the fold
belongs to the input. A dataset view supplies the tag folded to lower case; a
format that compares tags as bytes does not fold. The canonical RDF digest of a
dataset and a digest over terms as written are two identities, equal exactly
when every tag is already lower case.

### 3.2 Components and refinement

The incidence graph has the blank nodes on one side and the lines that hold a blank
node on the other. Lines are masked: every free blank leaf is replaced by one
marker, and a fixed blank leaf by its position. Each connected component is labelled
alone, from a fresh start of one cell of nodes and cells of lines by masked
encoding. Colour refinement runs as a queue of `(side, colour)` pairs over two
ordered partitions (`reference.rs`, `Engine::refine` and `Engine::split`):

* a colour is the offset at which its cell starts;
* a splitter is one cell of either side acting on the other; the key of an element
  is the sorted list of its incidence labels into the splitter, a label being the
  ascending occurrence numbers of the node in the line;
* fragments take offsets in ascending key order, the empty key first;
* when the split cell is already queued, every fragment is queued; otherwise all
  fragments but the largest, ties to the smallest colour;
* one refinement pops at most `2V` pairs, where `V` is the number of nodes plus
  lines of the component.

Colours are integers, never hashes, and no hash enters the search.

### 3.3 Calls, certificates and positions

A call receives an ordered partition and applies one of three rules:

1. *Discrete.* The node partition is discrete: every node is fixed.
2. *Nest.* The partition has a singleton cell but is not discrete: the singletons
   are fixed with their positions and the call nests on each free component that
   remains. A nested call inherits the parent's partition restricted to its
   component.
3. *Branch.* No cell is a singleton: the call individualises each member of the
   cell of colour zero in turn, refines, and recurses.

The certificate of a call is `(s, E, C)`: the number of nodes it fixes, the sorted
entries of the lines it completes (each entry holds a fully fixed masked line and
the largest position among its blank occurrences), and the certificates of its
nested calls in order. A branching call takes the smallest certificate among its
children. `certificate.rs` encodes certificates and provides `rebuild`, which
reconstructs the line set from a certificate alone; the tests check that every
certificate rebuilds to itself. The component identity is the BLAKE3
key-derivation digest of the certificate encoding under the context
`nested-refinement-labelling 1 component`. Top components are ordered by identity,
positions follow from the structure of the calls and the order of components, and
the canonical line sequence replaces each blank leaf by `fixed(position)` and
lists encodings in ascending bytewise order. Its digest is BLAKE3 in
key-derivation mode under `nested-refinement-labelling 1 lines`, over the line
count and the concatenated encodings.

### 3.4 The result record

The result is typed throughout: the placement of every node (position and stable
identity `(component digest, local position, copy index)`), the canonical lines and
their digest, and a structural record. The record is a function of the input alone
and equal across engines: per component the node, line and label-element counts,
the cells after the first refinement, whether the incidence graph is acyclic, the
calls of the result path by rule, the target-cell sizes, the attaining children,
the work units by part and the factorised order of the automorphism group.
`guaranteed` is true exactly when every branching call of the result path has a
target cell that is one twin class. A completed search-free run proves the
predicate and is the labelling. Figures that depend on the engine or on source
identifiers (`explored`, `twin_units`, `comparison_units`, `stopped_on_s`,
`stopped_on_e`) are kept apart from the record.

Refusals are a closed list: `input` (ill-formed lines), `graph` (an `empty-graph`
line names a graph another line carries), `width` (a quantity exceeds 64 bits) and
`limit` (the engine's limit tripped on an input that is not guaranteed). Both
engines use the limit `L = 2^34` limited units and trip only past `max(L, A)`,
where `A` is the allowance of the whole input: the sum of every component's
allowance plus the charge of the split into top components (`reference.rs`,
`prepare`). The allowance of one component is `2S + O + 2·O·bits(V) + M +
V·bits(V) + 4W` over its start charges `S`, label elements `O`, incidences `M`,
and start-weighted node count `W` (`static_record`). The allowance is what the
search-free run needs, so no limit refuses a guaranteed input.

Both engines decide `limit` by the walk of section 8.3 of the specification. The
walk is one sequential computation, a function of the input as given, source
identifiers included, and of `L`:

* The search-free run comes first, on every component, and none of its units
  enters the walk.
* The walk charges the split of the input, then each top component in ascending
  order of its smallest source identifier: its start charge, the refinement of its
  fresh start, then its calls.
* At a branching call it visits the target cell in source order. It skips a member
  related to an earlier one by a twin class or by a checked map from equal
  children or from below, and stops a child whose `s`, or whose entries at equal
  `s`, exceed those of the best. The twin tests of the call follow the fixing of
  its first child and precede that child's nested calls.
* The running total saturates at `2^64 - 1`. The walk ends with `limit` at the
  first charge that carries it above `max(L, A)`; the refusal names the component
  of that charge, its node count and the total.

The walk's home is `walk.rs`: the running total and its trip, the totals by kind,
the twin classes of a component, the relation of a call, the witnesses of equal
children with their cap and the check of a map. For a component the search-free
run does not label, the reference engine explores every child of every branching
call for its result and charges only what the walk charges; the optimised engine
explores what the walk visits and nothing else. Both make every charge of the
walk, so they refuse the same inputs with the same fields. No label, digest,
position, identity, record or map depends on the walk or on `L`.
`label_with_walk` labels under a given limit and returns, besides the result, the
walk's totals by kind (`WalkTotals`), the figures the specification's row `walk`
prints. The default canonical form records the running total of the walk as its
`SearchEvent::LabellingUnit` count: the walk's total when the labelling completes,
and on a refusal with `limit` its value after the charge that passed the limit,
the refusal's `units`. `Labelled::explored` is a separate engine figure that
holds no twin units.

### 3.5 The optimised engine

The reference engine is the definition written for clarity and explores every
child of every branching call. The optimised engine computes the same result on
dense arrays with an explicit stack of calls (`optimised.rs`), and its search is
the walk of section 3.4: it skips a member of the target cell that a twin class or
a checked map from equal children or from below relates to an earlier one, stops a
child on `s` and then on `E`, and completes every other child. The chosen child is
the first completed child that attains the best certificate, and the attaining
count is the number of members whose class holds a completed child equal to the
best. On a component the search-free run labels, the walk runs the calls of that
run, one child at each branching call with the same twin tests, so the optimised
engine makes the charges the run recorded at once, and walks the step charge by
charge only when the walk ends inside it. Pruning is an optimisation of a
definition that has none, and the tests compare the two engines, their results and
the totals of their walks, on a corpus under three relabellings, on the
enumeration spaces and on every row of the walk.

### 3.6 Nested triple terms and term nodes

The definition admits a triple term inside a triple term and writes it whole into
every line that holds it, so a chain of `n` statements whose objects each name the
triple term of the one before gives lines whose bytes grow as `n` squared. The
lowering of a dataset view (`view.rs`, `nested.rs`) follows the second-generation
GTS format, section "Nested terms, lowering and term nodes":

* A NESTED TERM is a triple term that stands as a component of another triple term
  of a line or of a record's term list. Two nested terms equal as values are one.
  Each distinct nested term becomes one TERM NODE, a blank node of the lines that is
  no blank node of the view, and one `record` line `record(P, [node, N(s), N(p),
  N(o)])`, `P` the eleven bytes `E([5, [[1], [1], [1], [1]]])` and `N(x)` the term
  node of `x` when `x` is a triple term and `x` otherwise. A triple term that stands
  in a slot of a line or directly in a record's list keeps that place, with its own
  components lowered.
* The lowering reads triple terms by the identifiers of their components and
  interns each nested term once, components first, over a heap work list: no
  triple term holding another is ever expanded. What is linear in the distinct
  terms is the representation: the tokens and records the lowered set retains, one
  record of bounded size for each nested term. The bytes of the leaves are not
  bounded that way: the source `K` of each leaf component is copied and hashed once
  for each distinct nested term it stands in, so a chain whose terms share one long
  interned predicate processes that predicate's bytes once per term, and the leaf
  bytes processed per occurrence enter the work bound. A record's own term values
  are walked in the same way, every occurrence of them.
* The source identifier of a term node is `E([d(x)])` with
  `d(x) = H(gts2 nested term, E([K(s), K(p), K(o)]))`, BLAKE3 in key-derivation
  mode. `K` of a blank node is `E([0, h(b)])`, `h(b)` the digest under the same
  context of its source identifier `E([ordinal, label])`; of a triple term
  `E([d])`; of an IRI or a literal the byte string of `E([1, text])` or
  `E([2, lexical, form, x])`, the texts whole as the view holds them. A composite
  node stands for its literal and takes the literal's `K`. The digests only order
  nodes: they enter no line, payload or digest.
* SOURCE ORDER. The source form is decided per input: the statements of the view
  that the scope selects together with the caller's records, which is what the
  lowering reads. When that input holds a nested term, every blank node of it takes
  the source identifier `E([ordinal, label])`, the scope ordinal as an unsigned
  integer and the label as a byte string, the bytes the format gives a blank node of
  a fold. Term nodes (first byte `0x81`) then precede every blank node of the
  input (`0x82`), and the order of twins and of copies of a top component, the only things
  source identifiers decide, is the format's. When the input holds no nested term,
  its blank nodes keep the source identifier of four big-endian bytes of the scope
  ordinal and the label, which orders labels bytewise; the format's form would
  order a shorter label first and so move the positions of twins in inputs the
  lowering does not change. A nested term only in a graph the scope leaves out does
  not change the form, and a nested term only in a record changes it even when the
  view holds none. Both forms are injective, so either names each blank node once.
* THE DATASET PATH AND A GTS READER. The canonical map of the dataset path is not
  the positions a GTS reader reports or stores. Source identifiers decide only the
  order of twin blank nodes and of copies of equal top components, and the reader's
  source identifiers come from its own folds; for those nodes the two maps can differ
  by an automorphism of the lowered set. When the dataset path is given the same
  lines and records as the reader's labelled set, the canonical line sequence, its
  bytes and its digest are the same. A GTS fold adds a `record` line for each
  binding, suppression entry, blob entry and pair of VALUES (one for each typed
  value); the dataset path adds none unless the caller passes them as records, so without them its sequence and digest differ. The
  dataset folds the case of a language tag to lower case, and the format keeps the
  text as written, so a literal whose tag has capitals gives different line bytes
  unless the view presents the tag as written. A canonical file written from the
  dataset path's positions holds the canonical bytes of its own set, and the order
  of delivery a reader reports for it comes from that reader's own run, not from
  the positions it was written from.
* RDF 1.2 nests a triple term only in the object of another, and a dataset refuses
  any other nesting when it is frozen. The lowering applies the same rule to every
  view and every record: a triple term that is the subject or the predicate of a
  triple term is refused (`input`, a slot holding a variant it does not admit)
  before anything of it is lowered. Each triple term then holds at most one nested
  component, so the spelling of a line is one chain, never longer than the
  input's own written form; a shape whose spelling doubles with each record, such
  as `t(k + 1) = <<( t(k) p t(k) )>>`, reaches the labelling through no entry.
* SIZE AND WEIGHT. The size of an input is its lines before the nested-term records
  plus its nested candidates: for each distinct triple term reached (and each triple
  term of a record's values, per occurrence), its components that are triple terms.
  The weight is the length of the encoding of every line and every nested-term
  record with each blank leaf written `fixed` (nine bytes), computed from the tokens
  of the lowered lines and the lengths of the constants. `LabellingCapacity` holds a
  read's capacity in lines, its capacity in bytes and the total weight admitted;
  `label_view_with_capacity` judges the size, then the weight against what the read
  has left, then the collision of two nested terms' digests (`LoweringRefusal`),
  then the definition's own refusals, the first that applies being the one returned.
  One definition refusal is judged before the size: a triple term in the subject or
  the predicate of a triple term is refused as an input with the slot it occupies
  while the record is read, because no lowered set exists to measure for that shape.
  A weight once admitted stays in the total; after a refusal by weight every later
  input of the read is refused by weight. The view is read once: size and weight
  are counted while the view is lowered, from term identifiers, and are judged
  before any digest is formed, any nested-term record is built or the engine runs.
* THE THREE CASES. An input with no triple term and an input whose triple terms
  all have depth one lower to the lines they always gave, with the same source
  identifiers, positions, identities and digests. An input with a nested term is
  labelled as its lowered set, whose canonical line sequence, digest and positions
  differ from those of the unlowered lines.
* POLLS. The lowering polls the caller's stop signal at each step of the nested
  walks. After the walk, an input with a nested term polls at each blank node of
  the text-form pass, once before the digests, once for each nested term in the
  digest pass, once for each nested term in the term-node source pass and once for
  each nested term as its record is built, so a stop set during a deep nested term
  is observed there. When a capacity is given,
  the signal is read once before the size is judged and, for an input whose size
  is admitted, once more before its weight is admitted, so an observed stop is
  the outcome and not a capacity refusal of the input it interrupted. Without a
  capacity, an input with no nested term polls none of the nested-term passes
  above, and an input with a nested term polls each of them. A document with a
  nested term also has one more line for each nested term than the lines it
  spells, and the document loops poll once per line of the sequence in each of
  their two passes.
* COST OF A CHAIN. The lowered set of a chain is linear. The walk units, the
  allocations and the retained and peak bytes of the labelling are linear, and the
  bytes it requests from the allocator and its instructions are quadratic. For a
  chain of `n` statements, `t(1) = <<( a a a )>>` and `t(k + 1) = <<( a a t(k)
  )>>`, each with the quad `a a t(k)`, measured on one AMD Ryzen AI MAX+ 395 host
  with rustc 1.102.0-nightly (5df8c02af 2026-11-16), each size alone under a 16
  GiB memory scope. The allocation, byte and walk columns of the rows of 1,000 and
  10,000 are those of the `chain` report lines of the bench profile, and equal the
  test profile's (`opt-level = 3` with debug assertions and overflow checks),
  where the row of 100,000 was measured with a current-thread window. The
  instruction column is the bench profile's (fat LTO, no debug assertions), from
  the counting row:

  | `n` | lowered lines | lowering: requested bytes / allocations / retained bytes | `label_view`: requested bytes / allocations / retained bytes / peak working bytes | walk units | `label_view` instructions |
  |---|---|---|---|---|---|
  | 1,000 | 1,999 | 2,003,229 / 23,268 / 324,117 | 22,689,938 / 111,404 / 1,513,433 / 3,369,981 | 76,916 | 1.18e8 |
  | 10,000 | 19,999 | 19,979,565 / 231,798 / 3,536,397 | 1,306,523,706 / 1,111,994 / 15,426,713 / 34,021,253 | 769,916 | 6.58e9 |
  | 100,000 | 199,999 | 182,359,453 / 2,316,825 / 33,791,421 | 121,106,177,690 / 11,117,073 / 152,691,737 / 351,609,557 | 7,699,916 | 6.06e11 |

  The lowering, the lowered lines, the walk units, the allocations and the
  retained and peak bytes of `label_view` grow tenfold with `n`. The bytes
  `label_view` requests from the allocator and the instructions it executes grow
  about a hundredfold: they are quadratic in `n`. That cost lies in the labelling
  engine over the lowered set, not in the lowering. The rows of 1,000 and 10,000
  are regenerated by `cargo bench -p purrdf-core --bench canonical_labelling`,
  whose report has one `chain` line for each size, and their instruction counts by
  `PURRDF_LABELLING_COUNT_SAMPLES=1 PURRDF_LABELLING_COUNT_NODES=10000 cargo bench
  -p purrdf-core --bench canonical_labelling -- --counts chain` (1,000 is the
  default size). The same counting command at
  `PURRDF_LABELLING_COUNT_NODES=100000` regenerates only the instructions, the
  `label_view` allocations (11,117,073) and requested bytes (121,106,177,690) of
  the 100,000 row. It also prints a whole-process peak of 356,691,181 bytes, which
  includes building the chain, so the table's peak and retained bytes, the
  lowering columns, the lowered lines and the walk units of that row are from an
  earlier test-profile run with a current-thread window and are not regenerated by
  any checked-in command. The counting row labels the prepared chain, so its
  instructions exclude building it, and each is one sample.
* The canonical document spells a term node as the triple term it stands for, read
  from the components of its record, and writes no nested-term record. Term nodes
  take positions, so the labels of a view's blank nodes in a document need not be
  consecutive.

## 4. Published work consulted

**RDFC-1.0** [1] is the baseline whose cost motivates the work and the RDF
Dataset Canonicalization the 4.x engine implements. Its algorithm uses neither an
automorphism group nor a refinement fixpoint.

**Individualisation and refinement.** McKay and Piperno [2] describe the method of
nauty and Traces: refine a colouring to an equitable partition, pick a cell,
individualise each member in turn, refine again and recurse to a discrete colouring
(a leaf). A refinement function, a target-cell selector and a node invariant fix the
tree, and the canonical form is the best leaf under the invariant; pruning by
automorphisms and by invariants never removes every optimal leaf. This is the shape
the definition needs: a tree fixed by the text, with traversal and pruning left to
the engine. Junttila and Kaski [10, 11] describe bliss, whose component recursion
corresponds to the nesting of rule 2 here, though the definition nests inside the
certificate rather than as a search heuristic.

**Colour refinement.** Berkholz, Bonsma and Grohe [8] give tight bounds for
canonical colour refinement, in the neighbourhood of `(n + m) log n` work. Paige and
Tarjan [9] describe the partition-refinement algorithms whose rule of queueing all
but the largest fragment gives that bound; the queue rule of section 3.2 follows it.
Neuen and Schweitzer [6] show families on which every individualisation-refinement
algorithm takes exponential time, whatever the selector. Cai, Fürer and Immerman
[7] show that refinement of any fixed dimension fails to distinguish some pairs of
graphs. Arvind, Köbler, Rattan and Verbitsky [5] study the
graph classes on which colour refinement decides isomorphism; forests are among
them.

**The cost of the leaf definition.** Babai and Luks [4] show that finding the
lexicographic leader of an orbit of strings is hard in general, which is why the
form is the best leaf of a pinned tree and not the least adjacency matrix over all
relabellings. Miyazaki [12] analyses the worst case of McKay's canonical labelling
algorithm.

**RDF.** Hogan [3] defines iso-canonical labelling of blank nodes by hashed
colour refinement followed by a nauty-style search, with automorphism pruning, and
measures it on web data; his earlier work [13] concerns skolemising blank nodes
while preserving isomorphism. Hogan's test framework found a defect in the pruning
of the conference version, which is the direct reason this design treats pruning as
a proved optimisation of a pruning-free definition tested differentially.

**Hash.** BLAKE3 [14] is a single algorithm with no variants and a key-derivation
mode that takes a context string. The definition uses it for the two shared values.

Hogan's figures in section 1.2 and the pruning defect in his conference version
were checked against the author's copy of [3], and the statements about RDFC-1.0
against the text of the Recommendation [1]. The statements about the other works
rest on their titles and abstracts and on the implementation itself; the papers'
bodies were not re-read for this document. No source of nauty,
Traces, bliss, saucy, dejavu or blabel is copied into this repository.

## 5. Choices and alternatives

| Choice | Alternatives weighed | Reason |
|---|---|---|
| The form is the minimum certificate over the leaves of a tree pinned by the refinement, the selector and the certificate. | The minimum over all labellings consistent with the refined partition; the lexicographically least adjacency form. | The leaves of an individualisation-refinement tree are a subset of those labellings, so the first object is not what a practical engine finds [2]. The second is hard to compute [4]. |
| Colours are integers; no hash inside the labelling. | Hashed colours, as RDFC-1.0 and Hogan [3] use. | A hashed colour can coincide with an existing one and merge two cells, so refinement bounds and termination depend on the hash. Integers add no collisions and compare cheaper. |
| Only what changes the result is normative: refinement, selector, certificate and order. Search order and pruning are free if proved. | A normative case split (tree canonisation on forests, leaf minimum elsewhere). | One definition keeps an engine that misses a shortcut correct and merely slower. |
| One splitter-queue refinement over the node-line incidence graph. | Synchronous rounds; splitting by node cells only. | Rounds are quadratic on chains. Splitting by node cells alone is weaker than splitting by line signatures. The queue rule follows [9]. |
| Three-way rule per call: discrete, nest, branch. | Nesting at every child. | Nesting at every child regresses when refinement leaves no singleton. |
| Nested calls inherit the parent's restricted partition. | A fresh start in every nested call. | A fresh start re-refines the whole remaining component at each level; the restriction is stable and corresponds between isomorphic siblings. |
| Top-level calls start fresh, per connected component. | One refinement over the whole dataset. | A component's cell order could depend on unrelated components through the largest-fragment rule, and a component's lines would depend on the rest of the dataset. A fresh start also permits per-component caching. |
| The target cell is the cell of colour zero. | Smallest, largest, maximally non-uniformly joined cell [2, 10]. | The result-path lemmas, the structural record, the group order and the map depend on it. No selector removes the exponential worst case [6]. |
| The certificate is `(s, E, C)`. | `(E, C)` without the node count. | Two non-isomorphic components of four and five nodes with equal `E` and `C` exist, so `s` is required; `rebuild` is the completeness check. |
| No node invariant compared before the certificate. | A trace or cell-size invariant as in Traces [2]. | It would break "the nested minimum is the global minimum" and would be normative: adding it later changes bytes and needs a new version. |
| No stronger refinement than colour refinement. | Two-dimensional Weisfeiler-Leman refinement. | It cannot split vertex-transitive or distance-regular graphs at the root, every fixed strength has exponential families [7, 6], and it would more than double what two engines must reproduce. `guaranteed` plus the typed `limit` refusal state the hard residue honestly. |
| The forest case gets no special rule. | A normative tree canonisation for acyclic components. | Colour refinement on a plain incidence forest never branches, so the general rule already costs one path. The result is proved for the coloured, edge-labelled case rather than cited from [5], which covers uncoloured graphs. |
| One hash, BLAKE3 in key-derivation mode, 32 bytes, one context per value. | SHA-256; a selectable hash. | Consumers already identify content by BLAKE3 and it is native to the workspace; a selectable hash would make identities incomparable. Component order depends on identity bytes, so the hash changes positions as well as digests. |
| Identity is the typed triple `(component digest, local position, copy index)`; the document keeps short decimal labels that nothing parses. | A prefixed string identity; one global position counter. | A global counter renames every later node when an unrelated component is added. Meaning from parsing a string is excluded by Q6. |
| Ties among equal digests are broken by the map rule: the child with the smallest source identifier. | A certificate comparison between top components; a `collision` refusal. | Comparing certificates on digest equality would run on every pair of copies and keep certificates for many small components. The collision resistance of the hash is the stated assumption of the definition; a collision is not detected. |
| `empty-graph(term)` is supplied once for each named graph no statement line carries. | A declaration line for every graph; a sentinel quad. | RDF has no "declared and non-empty" state. A sentinel is a reserved code word that Q6 excludes. |
| Reifier bindings and annotations are typed lines; the reserved IRIs of the 4.x overlay are gone. | Keeping the overlay. | The overlay spells typed structure as quads with reserved IRIs and reads it back, which Q6 excludes. |

## 6. Settled, open and owed

**Settled in the implementation** (checked by the tests named):

* The definition of sections 3.2 to 3.4 as implemented by the reference engine, with
  every worked example of the frozen revision reproduced byte for byte
  (`every_worked_example_reproduces_byte_for_byte`).
* The optimised engine's agreement with the reference engine on the corpus under
  three relabellings (`the_optimised_engine_agrees_on_the_corpus_under_three_relabellings`).
* Certificate completeness for the cases tried: every certificate rebuilds to
  itself (`every_certificate_rebuilds_to_itself`), and relabelling or reordering the
  input changes only the map (`relabelling_and_line_order_change_only_the_map`).
* BLAKE3 for both shared values, the identifier `(nested-refinement-labelling, 1)`,
  the closed refusal list and the line kinds.
* The walk of the specification in both engines: equal totals by kind on the
  thirteen inputs of the specification's row `walk` and on the shared comparison
  inputs, and equal outcomes and fields on its rows `limit-rows` and
  `limit-relabel` (`walk_tests.rs`).
* The lowering of nested triple terms (section 3.6): the lowered sets of the
  format's worked example and its lowering cases, labelled through a view, agree
  with the same sets written by hand and labelled by the definition alone, in lines,
  sources, positions, digests, structural record and walk totals; refusals by size,
  weight and digest collision beside their neighbouring admitted inputs; linear
  allocation over chains of nested terms (`nested_tests.rs`,
  `tests/canonical_labelling_lowering_alloc.rs`).

**Open:**

* *Independent agreement.* No second implementation written by a different reader
  has produced the same canonical line sequences and digests on shared vectors. The
  definition is provisional until it has. The vectors in `crates/rdf-core/src/canonical_labelling/vectors/` are the worked
  examples; a larger shared set that includes non-trivial automorphism groups is
  needed.
* *Verification without search.* No certificate is known that lets a reader check
  that a canonical claim is the minimum on a non-discrete residue without repeating
  the search. If every canonical claim had a short checkable certificate, two
  checked claims with different forms would certify non-isomorphism, which would put
  graph non-isomorphism in NP; that is not known. What a reader can always check is
  that the document is the dataset relabelled; canonicity under a stated
  `(name, version)` is checkable by recomputation, at a cost no method bounds in
  general. A file must not claim "canonical, verified in bounded time".
* *Hard families.* Colour refinement splits no regular graph, so strongly regular
  graphs, rook graphs, hypercubes and Paley graphs stay one cell at the root. The
  stops of section 3.5 cut the work on `Z4 x Z4` and `cub8` by large factors, but no
  bound is claimed for these families in general, and exponential lower bounds are
  known for individualisation-refinement algorithms [6] and for McKay's algorithm
  [12], and no refinement of fixed dimension distinguishes every pair of
  non-isomorphic graphs [7]. The refusal states the limit.
* *Refinement strength and node invariant.* Both are excluded from version `1`. A
  stronger refinement or an invariant would be a new version, never an option.
* *Dataset-state identity.* `DATASET-STATE-DIGEST.md` defines a separate identity
  with its own blank labelling. Whether it adopts this definition is undecided.

**Evidence owed:**

1. The 4.x engine and the labelling on the same shapes, in instruction counts and
   allocation counts (section 1.3 has the labelling only): the 246-node synthetic
   tree and a parameterised family of its shape against fan-out and depth.
2. The two acceptance workloads, first run only: millions of small datasets, each a
   star of blank spans around one blank reifier, run cold one at a time; and one
   carrier of about 16.6 million quads in 16 GiB.
3. Calibration of the limited work unit against instruction counts.
4. The hard families on the optimised engine in counts: complete bipartite graphs up
   to 300 by 300, the hypercube of dimension 8, rook graphs, Paley graphs, random
   cubic graphs and coloured Cai-Fürer-Immerman graphs.
5. The share of residual components of real datasets that are isomorphic, which
   decides what nesting buys.
6. Agreement of the native, WebAssembly, Python and C hosts on the shared vectors.

## 7. Consequences for consumers

[`MIGRATION-5.0.md`](../MIGRATION-5.0.md) is the migration contract; this section
states why each consequence follows from the design.

* **Every blank-node label and every canonical digest of a dataset with a blank
  node, a reifier, an annotation or a declared empty graph changes.** Positions
  come from the order of components and the structure of calls, not from RDFC-1.0's
  hash order, and the digest is BLAKE3 over the typed line sequence, not SHA-256
  over N-Quads text. The digest also moves for datasets with no blank node.
* **An input with a triple term nested inside a triple term is labelled as its
  lowered set** (section 3.6): the input is the statements the scope selects
  together with the caller's records, its canonical line sequence, digest and
  positions are those of the set with one term node and one record per nested term,
  and the positions of its blank nodes need not be consecutive. An input whose
  triple terms all have depth one, and one with no triple term, are labelled as
  before.
* **The dataset path's canonical map is not a GTS reader's.** For twin blank
  nodes and for copies of equal top components, the positions `label_view` gives
  and those a GTS reader reports or stores can differ by an automorphism. The
  canonical bytes and the digest are equal only when the dataset path is given
  the same lines and records as the reader's labelled set, including the records
  of bindings, suppression and blob entries and pairs of VALUES (one for each
  typed value), and language tags as the format writes them (the dataset folds their case). A
  reader's order of delivery comes from the reader's own run.
* **Labels are output only.** A consumer reads the typed placement, not the decimal
  label. Position order differs from the byte order of labels (`10` precedes `2`),
  so a document read back by its labels is not promised the identity map.
* **A component has the same stable identity alone or inside a larger dataset**,
  because top components start fresh. Its position is offset by the components
  before it, and the copy index among equal components can shift when an equal copy
  is added.
* **Identity of automorphic nodes is a property of the input document**, since the
  map rule picks the smallest source identifier. The claim of canonicity is about
  the canonical line sequence.
* **A finite search bound refuses a hard input with the `limit` outcome** and
  publishes no bytes. An input the labelling finishes without a search succeeds
  under every bound, including `0`. The fields of a refusal are not a property of
  the dataset: they are those of the walk, functions of the input as given, source
  identifiers included, and of the limit, the same from every engine.
* **RDFC-1.0 remains a separate entry point** and the W3C suite gates it. The 4.x
  algorithm stays callable to recompute stored 4.x digests.

## 8. References

Bibliographic records were checked against the Crossref DOI registry, and the W3C
and BLAKE3 entries against the publisher documents themselves. The checks cover
authors, title, venue, volume, pages and year. Where the registry gives an online
year that precedes the year of the printed volume, the printed year is used.

1. Longley, D., Kellogg, G., Yamamoto, D. (eds.). *RDF Dataset Canonicalization*.
   W3C Recommendation, 21 May 2024.
   <https://www.w3.org/TR/2024/REC-rdf-canon-20240521/>.
2. McKay, B. D., Piperno, A. Practical graph isomorphism, II. *Journal of Symbolic
   Computation* 60, 94-112, 2014. DOI 10.1016/j.jsc.2013.09.003.
3. Hogan, A. Canonical forms for isomorphic and equivalent RDF graphs: algorithms
   for leaning and labelling blank nodes. *ACM Transactions on the Web* 11(4),
   article 22, 2017. DOI 10.1145/3068333.
4. Babai, L., Luks, E. M. Canonical labeling of graphs. *Proc. 15th ACM Symposium on
   Theory of Computing (STOC '83)*, 171-183, 1983. DOI 10.1145/800061.808746.
5. Arvind, V., Köbler, J., Rattan, G., Verbitsky, O. Graph isomorphism, color
   refinement, and compactness. *computational complexity* 26(3), 627-685, 2017.
   DOI 10.1007/s00037-016-0147-6.
6. Neuen, D., Schweitzer, P. An exponential lower bound for
   individualization-refinement algorithms for graph isomorphism. *Proc. 50th ACM
   Symposium on Theory of Computing (STOC '18)*, 138-150, 2018. DOI
   10.1145/3188745.3188900.
7. Cai, J.-Y., Fürer, M., Immerman, N. An optimal lower bound on the number of
   variables for graph identification. *Combinatorica* 12(4), 389-410, 1992. DOI
   10.1007/BF01305232.
8. Berkholz, C., Bonsma, P., Grohe, M. Tight lower and upper bounds for the
   complexity of canonical colour refinement. *Theory of Computing Systems* 60(4),
   581-614, 2017. DOI 10.1007/s00224-016-9686-0.
9. Paige, R., Tarjan, R. E. Three partition refinement algorithms. *SIAM Journal on
   Computing* 16(6), 973-989, 1987. DOI 10.1137/0216062.
10. Junttila, T., Kaski, P. Engineering an efficient canonical labeling tool for
    large and sparse graphs. *Proc. 9th Workshop on Algorithm Engineering and
    Experiments (ALENEX 2007)*, 135-149. DOI 10.1137/1.9781611972870.13.
11. Junttila, T., Kaski, P. Conflict propagation and component recursion for
    canonical labeling. *Theory and Practice of Algorithms in (Computer) Systems
    (TAPAS 2011)*, LNCS 6595, 151-162. DOI 10.1007/978-3-642-19754-3_16.
12. Miyazaki, T. The complexity of McKay's canonical labeling algorithm. In *Groups
    and Computation II*, DIMACS Series in Discrete Mathematics and Theoretical
    Computer Science 28, 239-256, 1997. DOI 10.1090/dimacs/028/14.
13. Hogan, A. Skolemising blank nodes while preserving isomorphism. *Proc. 24th
    International Conference on World Wide Web (WWW 2015)*, 430-440. DOI
    10.1145/2736277.2741653.
14. O'Connor, J., Aumasson, J.-P., Neves, S., Wilcox-O'Hearn, Z. *BLAKE3: one
    function, fast everywhere*. Specification.
    <https://github.com/BLAKE3-team/BLAKE3-specs/blob/master/blake3.pdf>.
