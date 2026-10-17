# Native URI cast consumer review

Owner source assessment; not independent review or an execution pass. The
published CodeRabbit assessment covers f90, before this correction.

The original full local run and hosted integration-5 reproduce the same
URI-to-hexBinary admission failure. XsdDatatype::from_iri now recognizes AnyUri,
so the old fallback StringOnly row no longer applies. The generic native
nonnumeric row admitted binary/duration targets; it also admitted native and
unmodelled nonnumeric inputs to the newly available AnyUri target.

The corrected existing gate handles URI targets before generic native/list rows:
only the native URI or existing string row is admitted. URI sources then admit
only identity, with string targets already admitted by the shared string door.
There is no added classifier, parser or compatibility body. Derived string
admission remains through is_string_row.

URI-to-string now reads the native governed value, so XML whitespace collapse
is retained and malformed values cannot become valid strings through copying.
String-to-URI leaves collapse at the native URI parser, avoiding the caller's
second normalization allocation. Both routes preserve existing storage and
terminal-error propagation. No datatype value-space or range implementation is
changed by this repair.

Native operator controls retain all old negative cases, add both binary
directions, calendar/duration/numeric/boolean/list-to-URI refusals, and positive
normalized string/self/string-input and derived-string controls. The actual
governed portable target checks both directions for six primitive pairs plus
normalized positive conversions. Its original numeric transcript is unchanged.

The stale range/entailment prose is corrected without changing entailment
runtime logic. No exact-output fixture references the removed unsupported-URI
example. Required executed native/WASM/strict checks, normal hooks, refreshed
hosted feedback and final integration remain in validation.md. The review
cannot clear those pending requirements by source inspection alone.
