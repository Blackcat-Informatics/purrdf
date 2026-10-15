The repair is published in PR #554 from `f4fe33779`, with source implementation
`f8cbba207` and reviewed constructor-registration refresh `2f550d04c`.

The original bound-subject reproduction returns 1 instead of 2 on main. The
corrected full 2,000/200,000-claim control passes across three layouts, including
independent and resident counts for existing/absent keys, at most 64 touched terms
and less than 1 MiB of keyed provider reads. Full LIMIT/COUNT parity is retained.
Native and actual WASM historical controls pass; the explicit four-annotation
case preserves every annotation through declaration removal and reclassification.

Eight native cache/owner/failure controls pass, including 262,144 hits each at
64/4,096 entries, LRU/pins/reload/final-owner release and tight buffer admission.
Five fixed-work instruction repetitions of the same complete cache control use
69.341% fewer setup-subtracted native instructions. No timing acceptance is used.
Strict affected-package Clippy, actual helper hygiene and normal hooks pass.

The final local make check is running. Hosted checks and material independent
review remain pending; no merge or issue closure is claimed yet.

The Stage PR-creation command returned a JSON-decoding error after the forge had
created the PR. The normal branch resolver found the existing PR #554, so no
duplicate request or tooling change was made.
