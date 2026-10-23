# Review evidence projection

The first ghprsq atomic push was rejected by the existing Stage attribution
guard in current-inline-comments.json. Remote inspection confirmed no main,
PR or audit-ref publication: PR 687 remained open, main stayed df1a0e515 and
the remote PR audit namespace was empty. The failed command is not delivery.

The rejected bytes were the upstream review service's HTML attribution header,
not the substantive finding or the reviewer's identity. Every captured copy
of that same header was removed from the selected evidence. This covers the
REST review/submission/comment projections, GraphQL thread projections, merge
brief comments, and earlier captures of those same review bodies. Literal and
JSON-escaped HTML spellings receive the same deletion. No encoding or compression
is used to conceal the prohibited content.

Preserved fields include original comment/review/thread IDs, URLs, authors,
paths, commit references, positions, diff hunks, state, complete finding and
suggestion prose, prompt text, public replies, and the exact current-head review
coverage marker. Only the unrelated attribution header is omitted. The original
REST/GraphQL endpoints and stable IDs remain in the projections, including inline
comment 4357622144 and thread PRRT_kwDOTKq-Ms6vh5AT. No material review content,
unresolved finding or acceptance limit is removed. Source and qualification
results are unchanged.

These are explicit provenance projections, not byte-exact raw API captures.
The existing attribution guard is rerun over every selected evidence file before
retrying the same ghprsq merge. It remains enabled during the actual push.

Preflight completed with exit 0 for every selected evidence file using the
unchanged installed attribution guard's file checker. JSON projections remain
parseable, the original inline comment ID/path/body and thread pagination remain,
and the exact reviewed-head coverage marker still names 06f735f24. No source,
test selection, compiler, review verdict or acceptance result changed.
