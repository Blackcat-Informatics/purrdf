# Independent issue analysis

The captured issue asks for caller-selected ViewLimits at MutableDataset's
compaction boundary. It does not request removing finite snapshot defaults or
changing graph-lifetime policy. Existing freeze() calls snapshot_view(), which
delegates to snapshot_view_with_limits(ViewLimits::default()). The default
512MiB auxiliary ceiling therefore applies even when the consumer uses larger
limits for its other snapshots. There is currently no freeze override.

An additive freeze_with_limits(&self, ViewLimits) with freeze() delegating using
the unchanged default meets the stated API option. It must pass those exact
limits into the existing snapshot admission and retain the single importer,
RDF-role classifier, surviving base-location remap, final builder validation and
successful-work accounting. A second compaction body or ignored limits would
fail the requirement. No host policy change or new unbounded global default is
required by this issue.

Acceptance must demonstrate all five coordinates (sources, terms, rows, payload,
auxiliary) at their actual public admission home: below-limit typed refusal,
exact-boundary success, unchanged owner and successful-work counters on refusal.
An admitted higher-limit freeze must preserve ordinary/reifier/annotation rows,
scoped/nested term identities, declared empty graphs, and base locations while
retained prior snapshots remain unchanged. Existing complete_source()/surface()
helpers provide the RDF12 oracle and must be reused rather than duplicated.

The small boundary controls cannot alone demonstrate the reported large-corpus
failure. The actual approximately950k-claim native compaction route must execute
with the consumer-selected higher limits before completion, with original
default refusal and successful larger-limit publication distinguished. No huge
synthetic allocation is needed merely to test each arithmetic boundary.

Review performed read-only against captured issue542 and actual main5144 source;
no build or source mutation. Implementation and runtime qualification remain open.
