The three full npm stack-depth assertions remain failing on the current retained
optimized numeric package. No stack budget, refusal floor, assertion or tooling
has been changed.

Using the package's existing `queryGovernedAsync` evidence API and the original
depth-control inputs, all18 queries at depths1,2,4,8,16,32 completed:

| Shape | Stack bytes at depth2 | Stack bytes at depth16 | Stack bytes at depth32 | Added bytes per level |
|---|---:|---:|---:|---:|
| LATERAL |34300|91420|156700|4080|
| Repeated NOT EXISTS |44956|144412|258076|7104|
| Walk NOT EXISTS |47772|147228|260892|7104|

The two EXISTS inputs have the same per-level slope from depth2, so their shared
recursive execution path needs diagnosis. These are live stack-byte observations,
not elapsed-time performance claims or evidence that the original floors pass.
The measured package precedes the subsequently committed SCC type annotation,
lifetime elisions and aggregate lint annotation; it has not been rebuilt for
those source-only corrections yet.

Source inspection identifies one bounded LATERAL candidate: `eval_application`
holds its post-left-child/right-side machinery in the frame while recursively
evaluating the left child. An out-of-line finisher, analogous to
`finish_left_join`, could separate that overlap while preserving ownership and
truncation. It has not been implemented or measured, and cannot establish a fix
for EXISTS. Individual optimized frames at `linked.filter` → `exists_prepared`
→ `eval_correlated` still need attribution before changing that path.

The complete package acceptance remains failed; this update closes no requirement.
