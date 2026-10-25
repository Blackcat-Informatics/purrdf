<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Compiled glob language

`purrdf-glob-v1` matches an entire UTF-8 surface, case-sensitively and without
normalization. `?` consumes one Unicode scalar; `*` consumes zero or more.
Brackets contain scalar members and inclusive scalar ranges. Leading `!` or `^`
negates a class. A first or final hyphen is literal. Backslash quotes exactly the
next scalar, inside or outside a class. An empty/unclosed class, reversed range,
trailing escape, named POSIX class or unescaped brace outside a class is a typed
syntax refusal at an original pattern byte offset. Quote a closing bracket to include it.
Other punctuation is literal. This language does not claim POSIX conformance.

Text mode treats slash as ordinary text and collapses consecutive stars.
Path mode uses only `/` as separator; a backslash never becomes a separator.
Literal separators are retained, including leading, trailing and repeated ones.
Ordinary stars, question marks and classes never consume a slash. A star can match
an empty component; a question mark or class always consumes one scalar.
No special dotfile rule exists.

In path mode, exactly two unescaped stars occupying an entire component are recursive.
`**/` consumes zero or more complete components, each ending with a slash;
terminal `**` consumes any remaining suffix, including empty. Thus `a/**/b`
matches `a/b` and `a//b`, `a/**` matches `a/` but not `a`, and `**/` matches
empty text. `a/` does not match `a`; `a/b` does not match `a//b`.
Every other run of unescaped stars has ordinary star semantics, so `a**b` is
equivalent to `a*b`, and `***` cannot cross a path separator. Quoted slashes are
literal separators too, including after a recursive component.

Compilation validates/counts before admitting exactly sized token/range arrays.
Each class is sorted and merged in place, then tested through the shared scalar
range-table home. Compilation takes at most `O(P log P)` time for `P` pattern
bytes, with linear owned storage and no alternative expansion.
The plan owns its initially empty admission account; copied input text is not
retained. Failed compilation destroys buffers before refunding admission.
Successful plans retain admission until destruction. No sharing or deep clone is
implied by this uniquely owned plan.

Execution uses constant workspace and allocates nothing. Separate checkpoints
retain a component star and a directory globstar: a later component failure can
retry the next directory even after the inner star has matched. Execution can
rescan suffixes; this is not a claim of linear running time. One work unit is
charged before each token/end dispatch, each class-membership predicate attempted,
and each scalar consumed while retrying either checkpoint. A refused
unit reports the input byte offset, consumed work and original allowance; no
partial match decision is returned. Even an empty pattern/input needs its end
dispatch. `u64::MAX` permits every representable work count, and the next unit
still refuses. Repeated decisions have identical work under identical inputs.

Literal matching reads its lexical form verbatim. IRI matching borrows an already
parsed IRI reference and selects lexical spelling, scheme, authority, path, query
or fragment explicitly. Percent escapes are never decoded. An absent component
is a completed negative decision, charged one selection predicate. It is never
coerced to a present empty component. Path mode applied to
an IRI path recognizes only literal slashes, never `%2F` or `%2f`.

## Ordered pattern sets

A compiled set uses the same language and one mode for every source pattern.
It retains source order and duplicate occurrences. Matching returns every matching
source index in order, rather than a chosen winner or a deduplicated pattern.
An empty set returns no indices and consumes no work.

Compilation validates every pattern before allocating flat token, range and
program-boundary arrays under one initially empty admission account. A pattern
failure retains its original index and compiler cause; combined storage failure
retains its native cause without inventing a responsible pattern. Arrays die
before their account is refunded. No source text or separately owned single-pattern
plans are retained. Storage grows linearly with total pattern size and count.

The caller supplies an index workspace with at least one slot per source pattern.
Insufficient workspace refuses before execution. Matching allocates nothing and
uses constant internal workspace beyond that caller-owned output. Patterns execute
in source order through the same matcher, with one shared work meter: successful
work is the sum of individual decisions. An absent IRI component costs one negative
selection per pattern. Exhaustion retains the responsible pattern index, original
input offset, aggregate consumed work and original aggregate allowance. No report
is returned on refusal; workspace contents then remain scratch, not a partial answer.
The immutable set can be reused after any execution refusal.

Programs occupy flat storage and execute individually. No transition sharing,
asymptotic speedup or linear-time matching guarantee is claimed.

## RDF-authored plans

`rdf::PreparedPlan` compiles a portable source description under the exact schema
version `purrdf-glob-rdf-v1`. It retains the original frozen dataset and uses the
ordered compiler above. No executable bytecode is read or written. The caller
supplies all thirteen vocabulary roles through `rdf::Vocabulary`; every role must
occur exactly once, have an absolute IRI, and differ from all other roles and
`rdf:type`, `rdf:first`, `rdf:rest` and `rdf:nil`. Unknown roles are refused.

The plan root has exactly one value for each field below, in the default graph.
No other outgoing field or named-graph field is accepted on a plan or pattern.

| Predicate role | Required value |
| --- | --- |
| `rdf:type` | The caller's `description_type` IRI |
| `version` | String `purrdf-glob-rdf-v1` |
| `language` | String `purrdf-glob-v1` |
| `mode` | String `text` or `path` |
| `surface` | String `literal`, `iri`, `iri-scheme`, `iri-authority`, `iri-path`, `iri-query` or `iri-fragment` |
| `patterns` | A finite, well-formed RDF collection of pattern resources |

Each pattern resource is an IRI or blank node, has exactly one `rdf:type` value
equal to the caller's `pattern_type` IRI, and one `source` string containing its
glob. Configuration strings have datatype `xsd:string` and no language or direction.
Repeated pattern resources remain distinct occurrences through their list cells.
The empty collection is valid. Missing or multiple values, malformed collections,
incompatible versions and malformed globs return typed refusals with original
source context.

Execution consumes the prepared plan and an original input dataset/term ID.
Literal selection reads the original lexical form; IRI selection borrows original
component slices without normalization or percent decoding. The complete result
owns its input and plan snapshots, ordered matching indices and aggregate work.
Its `into_plan` method recovers the compiled plan for another successful execution.
Refusal retains both snapshots, original IDs, selected surface and native cause.
Borrowed structured inspection constructs neither diagnostic strings nor term trees.
Literal datatype and triple-child inspection reports original local IDs, resolved
through the retained snapshot without recursive traversal.

Export consumes a completed result and takes a caller-supplied absolute evaluation
IRI and output admission. That identity must differ from plan, pattern and prior
evaluation resources in the source. Harmless existing IRI occurrences are allowed.
The staged output contains the source description, with coherent blank identities,
and these evaluation fields under the caller's vocabulary:

| Predicate | Value |
| --- | --- |
| `rdf:type` | `evaluation_type` IRI |
| `plan` | Imported plan identity |
| `input` | Original input term, including datatype, language and direction |
| `matched` | One RDF 1.2 triple term `<<(cell rdf:first pattern)>>` per matching occurrence |
| `work` | Aggregate work as `xsd:integer` |

The two matched entries for a repeated pattern quote different original membership
cells. Native term import, collection walking, triple construction and freeze use
the shared RDF kernel homes. Generated storage and its shared control are admitted
before publication; an export refusal retains its output account through destruction
of its native diagnostic. No completed output is returned on storage refusal.

The facade and CLI apply existing Turtle 1.2 and N-Triples 1.2 codecs outside this
crate. Codec ingress remains caller-owned allocation; it is not retroactively
claimed as an admitted parser. `glob-rdf PLAN INPUT` requires `--root`,
`--evaluation`, `--input-subject`, `--input-predicate`, all `--vocabulary ROLE=IRI`
mappings, `--from`, `--to` and `--max-work`. Its input is the unique default-graph
object selected by the supplied subject and predicate IRIs. Both documents use
the explicit input syntax. The complete result is serialized before stdout is
touched; a parse, schema, work, storage or serialization refusal emits no RDF.

Brace alternatives, filesystem traversal, Rust import resolution and compiled
identities are outside these capabilities. They are not silently approximated here.
