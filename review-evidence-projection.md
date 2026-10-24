# Imported review projection and archival

The first ghprsq atomic push was rejected by the existing attribution guard on
the helper-generated review-evidence.json payload. Remote verification confirmed
PR 693 remained open, mergeCommit was null, main remained e3eb29225 and no remote
stage archive existed. No publication succeeded. Guards, signing and hooks remain
unchanged; no alternate merge method is used.

The generated payload had not received the mechanical UI projection already
applied to the other comment captures. The same structural projection now covers
it: remove the review-stack authorization-link banner, suggestion/help/poem UI
blocks, and nontechnical HTML comments. Technical review text, warnings, source
IDs, account metadata, timestamps, stable source URLs, reviewed commit coverage
and risk labels remain complete. No substantive feedback is summarized away,
encoded or compressed. The projection is declared rather than presented as an
original HTTP byte capture. Source comments remain identifiable by their IDs.

The helper intentionally excludes files ending in .log. All nonempty settled
logs are frozen byte-for-byte under acceptance-evidence/ with the same stem and
.txt suffix. Each copy was compared directly with its source. The referenced
native, strict, hook and failed-attempt outputs therefore survive cleanup without
changing the helper's capture policy. Empty formatting logs contain no evidence;
their command outcomes are recorded in validation.md.

This is an evidence-only repair. Source, reviewed head, behavior qualification and
the clean candidate are unchanged. Retry still requires current checks/base and
the normal content/credential/signature guards; no failing gate is waived.
