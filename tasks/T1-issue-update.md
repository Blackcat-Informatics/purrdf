Task 1 is committed and pushed as dd9109995f056e61aea8361a80ad9977a57ec2bf.
The signed commit passed the normal repository hooks, and remote branch readback
matches that commit.

The public core stack composes independently sealed generations with ordered
value removals and a mutable resident batch. Its retained snapshots share one
page cache, budget and sticky failure gate across all sources. Canonical folding
rebuilds bounded pages and preserves actual eager typed page-byte identity.
G11 documents dictionary costs, RDF 1.2 classification and graph lifetime.

An independent task review passed the exact committed tree after repairing
chronological orphan classification, direct metadata drift gating, explicit
graph declarations across seals, and subject-indexed side probes.

Executed validation: 82 focused paging/graph integration tests, 42 mutable and
23 global-dictionary unit tests; core all-target clippy with warnings denied;
helper hygiene, formatting and whitespace checks. No dependencies, features or
ledger exemptions were added.

Task 2 now covers the public evaluator/prepared-query consumers, runnable Rust
example and wasm qualification. Those checks, hosted CI, PR review and final
integration have not yet run. The core task result does not establish whole-issue
completion.
