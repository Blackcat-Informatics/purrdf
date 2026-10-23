# Candidate integration assessment

## Final revised candidate

Fetched origin/main remains df1a0e515. Current head is 06f735f24; merge-tree
returned exit 0 and tree ae1a68a5a335611b3e1e4e53cd396e484c8cf88a. Relative to
the qualified head, the combined tree adds only the same base AGENTS link and
Rust guide described below. Production sources/build settings are otherwise the
current head's, including its actual RISC-V operational deadline correction.

The completed current workspace job checks out f01840095f35e65668265c2c4f9c2adc0528092f,
merging this head into df1a0e515. hosted-integration-commit.json gives those
exact parents and the identical candidate tree ae1a68a5a335611b3e1e4e53cd396e484c8cf88a.
All 50 checks passed with six configured skips. Thus required qualification
covers the actual combined tree. No additional local broad suite or synchronization
commit is required by the documentation-only base delta. Refresh for any changed
input before ghprsq and compare the actual landing once after publication.

## Earlier candidate snapshot

This assessed candidate is for 154d11d61. Current 06f735f24 adds only the hosted
measurement cell and RISC-V job deadline. Capture the revised candidate/hosted
merge identity and refreshed base after revised-head qualification, before any
merge claim. The snapshot below is not final current-head acceptance.

The actual fetched origin/main is df1a0e515. Its delta from the common ancestor
is documentation only: one AGENTS.md instruction linking docs/RUST_GUIDELINES.md,
and that 245-line generic guide. These documents have been read; repository
AGENTS.md and workspace manifest constraints take precedence over generic example
dependency/lint suggestions. No production API, layout, dependency, generator,
compiler setting or runtime selection changes in this base delta.

git merge-tree --write-tree --no-messages origin/main 154d11d61 returned exit 0
and candidate tree 5525b1758cb370ef7d2049aadb981ee28bba3db5. The candidate adds
only those two base documentation changes to the tested head. No synchronization
commit or fresh local broad runtime suite is warranted by this documentation
delta.

The current hosted workspace log actually checks out pull/687/merge at
f63f3a7e147c9cfb4c0d686d5dca1519250c01b7, described as merging 154d11d61 into
df1a0e515. The forge commit object (hosted-integration-commit.json) gives those
exact parents and tree 5525b1758cb370ef7d2049aadb981ee28bba3db5, matching the
candidate. Thus hosted qualification is running on the actual combined tree;
it is not inferred from two independently green inputs.

CI is not yet fully successful. Refresh completed required checks and feedback,
and reassess if either merge input changes before ghprsq. This assessment does
not authorize merging with an open failure or missing required check.
