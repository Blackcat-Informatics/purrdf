<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Single-pattern glob language

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

Pattern sets, brace alternatives, filesystem traversal, Rust import resolution,
RDF projection, plan serialization and compiled identities are outside this
single-pattern language. They are not silently approximated here.
