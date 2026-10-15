Bound-subject queries over a segmented stack inherited an empty annotation
iterator and a full reifier-table scan. They now use the existing subject-keyed
logical streams, preserving historical classification, removals and graph slots.

Block-cache hits now use source-qualified hash keys and recency links. Cache
growth, replacement overlap, pinned blocks and escaped owners retain the shared
physical admission account. The reviewed constructor-source registrations are
refreshed without changing the checker or adding exemptions.

Validation:

- Full persisted 2,000/200,000-claim LIMIT/COUNT controls pass at three block
  layouts under a 256 MiB source-admission ceiling. Existing and absent bound subjects match independent
  counts and resident execution, touch at most 64 terms and read under 1 MiB.
- Keyed historical parity passes across 128 generated publications on native
  and actual WASM. The four-annotation control covers declaration removal,
  orphan annotations and ordinary-row reclassification.
- A supplemental four-annotation fixture matches the reported 600,263 terms and
  190,832,640-byte image. Present keys return five rows, absent keys zero, with
  at most eight touched terms and 345,600 provider bytes under the same 256 MiB
  keyed budget. Its million-row full COUNT completes with an explicit 512 MiB
  budget; at 256 MiB it returns typed residency refusal. The primary committed
  256 MiB regression remains unchanged. Both supplemental outcomes are retained.
- Eight native cache/ownership/failure controls pass, including 262,144 hits
  each at 64 and 4,096 entries, LRU eviction, pins, reloads and final-owner release.
- Five fixed-work instruction repetitions of that same complete cache control
  improve from 3,499,736,115 to 1,072,996,224 setup-subtracted instructions
  (69.341% fewer). These are optimized native test-profile counts; fixture
  preparation remains included. No timing acceptance is used.
- Warning-denied core/evaluator all-target Clippy, helper hygiene and normal
  commit hooks pass. The single full local gate passes, including native tests,
  doctests and release WASM, on the critical repair before carrier integration.
- The synchronized tree also passes affected cache/history/query controls,
  strict compilation, helper hygiene and actual WASM execution. The reviewed
  conflict resolution preserves the new carrier authorities and changes only
  combined constructor-closure identities. The canonical tree matches that
  tested combination; the temporary validation checkout is removed.
- Independent source review reports no actionable finding. Its private/test
  docstring-percentage warning has an explicit posted disposition. Full hosted
  CI for the synchronized publication is running; no merge is claimed yet.

Closes #553.
