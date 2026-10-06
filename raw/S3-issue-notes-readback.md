rdf: retain chronological paged generations with pinned mutable snapshots

Independent paged bases and sealed delta generations now compose with a mutable
value head. Reads pin the entire source descriptor vector and resident batch,
apply chronological removals and reinsertion by RDF value, and share indexed
physical paging, admission, exact page/byte receipts and sticky operational errors.
Cross-source ordinary, reifier and annotation classification follows native
RDF 1.2 semantics, including original association and graph scope.

Canonical folding sorts typed effective rows and declaration-only graphs under
an explicit positive page bound and reconstructs native pages. Public RDF and
umbrella facades expose the APIs; guarded/prepared evaluator consumers and a
runnable example demonstrate answers and actual eager PackBuilder carrier parity.

Files: core paged/stack.rs and query/provider/global/mutable integration;
core/evaluator regression suites and example/support; RDF and umbrella facade;
backend contract G11; private operation counters registered in the existing wasm
interleaving ledger; workflow assembly diagnostics/projection and shared-home
description. No dependencies or semantic features were added.

Storage/log policy, depth scheduling and atomic publication are issue-specified
consumer responsibilities. Snapshot metadata costs linear retained-dictionary work;
repeated publication can have cumulative quadratic cost. No throughput claim is
made. A plain compacted base preserves current graph membership; hidden populated
declaration-lifetime policy requires separate consumer data.

Normal signed/hooked base synchronization had no conflicts. The affected evaluator
was qualified afterward. Current head d48be7c960adfeaba44d642630cde4a66d7e86e7,
tree5d150fa4076cc75e293b517f79dd080b6ea807ed equals the clean integration candidate
against ancestor base b6f7c9b0f6b84ffe496719e39f2d2ba52d5ed3ac.

G1 routes ordinary and checked validated literal ingress through one embedded-blank
registration home; cold List/Map/recursive-triple parity failed before repair and
passes afterward. G2 bounds native graph postings before summary work and uses
the existing failure latch at ID-row filters. Sparse work stays fixed as unrelated
pages grow; plain stacks inspect zero reifier summaries. At two fixed sources and
pages, descriptor calls stay constant as rows grow and cached rereads add none.
Full materialization, point/metadata and final descriptor checks remain; actual
ordinary/prepared/fold consumers refuse both delayed drift kinds.

Local qualification: final G2 paging units18, core paging/graph85 plus strengthened
buffered-side witness1, evaluator33, root facade1, runnable consumer, four affected
packages all-target clippy and release wasm libraries, final affected core clippy,
helpers81jobs1833files80domains and thread-local53. Earlier unchanged mutable42,
G1 global24 and effective-profile evidence are reused with recorded applicability.
Formatting, whitespace and exact source manifests passed. Task1/Task2/G1/G2
independent reviews and normal signed commit hooks passed.
Wasm compilation is proven locally; whole-workspace local suites and local wasm
runtime remain unrun. Final hosted workspace and wasm runtime checks pass.

CR1 is addressed at discussion_r4200408253 and its thread is resolved. The bot's
documentation percentage was declined with a posted repository-contract rationale;
new public APIs and error semantics are documented and warnings-denied checks pass.
G3a adds actual failed-shard diagnostic uploads and an explicit hosted full-matrix
projection through the existing make gate; static checks and independent review
pass. Actual hosted v3/v4 parent bodies contain the expected scalar gathers and
count zero vector work under unchanged laws; no selector defect was found. G3b
applies the exact complete generator-produced document, qualified on all110sites
and seven configurations with compiler stability/artifact/replay binding. Normal
final CI37537944878's seven independent shards and aggregate also pass, every
report matches the actual source/manifest/compiler and projected cells. Its actual
synthetic merge a582b06e has that same5d150fa4 candidate and expected base/head
parents. CI37537944878 and Docs37537944833 completed SUCCESS at the exact final
head. All40 normal CI jobs pass; the full PR check surface has48 successes and
two expected skips (main-only deployment and manual-only projection). CodeQL
and CodeRabbit succeed. Hosted wasm package/execution and conformance pass.

Final independent completion reviews/S2-completion-final.md PASS binds head d48,
base b6 and tested/predicted candidate tree5d. Its full requirement reconstruction
and actual public-entry-point evidence discharge HF1/F1, PF1, PF2/CR1 and G3.
Stage3 reuses this qualifying audit because source, requirements, base, candidate
and functional inputs remain unchanged, with no additional integration delta.
Distinct review-debt.md PASS dispositions complete feedback, reviews/inlines and
the resolved thread. Bot workflow-separation and percentage warnings are declined
with actual gate-ownership/public-documentation reasons at PR6026605515; its
stale full-matrix statement is corrected by actual completed qualification.

Standing .baseline/.goals are satisfied through first-party shared homes, explicit
typed failures, RDF1.2 value identity, bounded indexed paging and wasm portability.
No source/process references, new dependencies or semantic features were added.
Added-source scans have zero hits; full issue/PR prose hits are explicit negations
of cuts/deferrals, with individual false-positive evidence in
raw/S3-scan-dispositions.md. No requirement is omitted and .deficiencies has no
entries. Source is clean apart from selected untracked/unignored Stage evidence,
which ghprsq archives separately. Final archive/result/closure/cleanup are verified
after publication rather than claimed from these pre-merge qualifications.

Authoritative plan: .stage/paged-tier-an-lsm-style-stack-of-sealed/plan.md,
SHA-256 ce1721dae29798cf2657336171b9981a0a1d86eec1ef0682392fc7379081ea29.
Selected evidence directory: .stage/paged-tier-an-lsm-style-stack-of-sealed.
Defect-Class: none
Closes #457.
