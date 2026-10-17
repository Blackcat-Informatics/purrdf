The imported-closure replay correction is published in PR #593, commit c86272091.
The original CLI producer/checker regression passes unchanged. The mismatch was
native term coordinates changing during canonical export/reparse; checking now
builds a fresh context from the consumer's own resolved snapshot through the one
shared verifier. Current-support checks, canonical export and proof bytes remain
unchanged.

Passing focused qualification: two native public harness cases (sixteen frozen
vectors plus three imported-source shapes and negative neighbours), the actual
CLI case, five native current-support controls, both exact scalar-WASM cases,
affected strict Clippy, standard helper hygiene and normal commit/push hooks.
No full local gate, external corpus, resource probe or performance timing was run.

Owner assessments are not independent approval. The principal uncertainty is
wider combinations beyond the focused source shapes; hosted CI and actual PR
feedback remain pending. No follow-up replaces an issue requirement.

The PR creation command reported unparseable output after publication; branch PR
lookup returned #593, so no duplicate creation was attempted.
