<!-- SPDX-License-Identifier: CC-BY-4.0 -->
# Paired default benchmark assessment

VERDICT: NOT MET. No whole-group completion or full-gate readiness is inferred.

Frozen shipping source current-build-3 terminated0 under the original bench
profile and RUSTFLAGS=-D warnings. Baseline is original main eb1fd88b0 at the
preserved detached source. Identical original query_eval binaries ran all36
lanes sequentially on CPU8, --bench --quick, using a fresh shared
default-paired-estimates-3 store. Baseline saved paired-main; current used strict
--baseline paired-main. Both executions terminated0,36 measured,0 routine failures.
Receipts: default-paired-baseline-3 and default-paired-current-3 logs/results.

Five statistically reported regressions remain: OPTIONAL-heavy11.18%, UNION4
10.28%, ORDER/LIMIT7.95%, DISTINCT department53.24%, int-vs-double window total
2.60%. These are explicit first-party harness observations, not waived failures.
Many other lanes improve; several direct unchanged-XSD numeric controls improve
about39%, so artifact/environment effects need assessment rather than attributing
every change to evidence. Original default-throughput-within-noise acceptance
is not established by the zero benchmark routine failure count.

Source review identifies enlarged original retained-row/key transport as a
concrete candidate: RetainedRow always contains RowStorage plus RowEvidence
owner/id/membership/rank, including ordinary queries. Ordinary DISTINCT avoids
ordinal/coalescing but still keeps the wide row as its key. The implementation
owner will inspect and batch justified compact ordinary keys/transport and
OPTIONAL/UNION/order-path changes, preserving original shared admission owners,
tracked evidence, all original allocation controls and actual public callers.
No unsupported blanket noise dismissal, reduced panel or weaker threshold.

Independent compact-carrier review finding: current RowEvidence::is_default
checks id and membership/completeness flags, deliberately not original_rank or
query owner. Reusing that semantic predicate as an empty-storage predicate
would silently discard original candidate ranks for a default-supported row
and could drop its explicit owner. The compact default attachment must elide
only metadata-empty carriers (including rankNone and ownerNone), preserving the
existing public default evidence law. Original rank must survive actual clone,
operator transport and acceptance without producer re-query or recomputation.
This source-backed finding was delivered before implementation qualification.
