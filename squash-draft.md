# Allow caller-selected limits for mutable compaction

Closes #542.

`MutableDataset::freeze_with_limits` admits the intermediate snapshot with the caller's `ViewLimits`, allowing the same limits for snapshot publication and compaction. `freeze` delegates with default limits. The existing compaction body remains the sole implementation; term remapping, RDF row tables, graph policy, source locations, sidecars and successful work accounting retain their production paths.

The complete-source regression covers all five inclusive admission boundaries, one-short refusal, unchanged successful counters on refusal, sufficient-limit retry, default parity and retained-snapshot immutability. An explicit native reproduction generated 950,000 claims and 3,800,000 rows with 577,600,360 auxiliary bytes: default 512 MiB compaction refused, while caller 1 GiB snapshot and compaction succeeded with complete table populations and sampled identities preserved. The small fixture compares every RDF value. The facade consumer parses and serializes resolved RDF through the new public API.

Completed focused qualification: 1,303 core unit tests, 60 shared-view tests, 11 graph-policy tests, six declared-graph drain tests; explicit large reproduction; affected warning-denied Clippy; executable method doctest; native umbrella consumer; core and umbrella WASM compilation. Normal commit hooks and push passed for 3d9571582.

Original mandatory full gate terminal0. Subsequent shared strict, rustdoc, Python/MXCSR fixture and documentation corrections are separately qualified through their affected checks and normal hooks/push; the current clean published head is20ede055f. Both Python scoreboards agree with the actual4,223 hosted cases, and the154-claim check passes. All seven current assembly configurations, architecture controls and aggregate conformance pass.

PR543 CI38428369796 attempt2 is terminal:41 jobs pass,4 skip, and optimized wasm-package fails446/449 solely on the unchanged synchronous/LATERAL/NOT EXISTS depth floors135/236/134 versus181/281/180. The native integration5 shard and dependent aggregate passed the unchanged standard retry; the original timeout/cancellation remains recorded. The separate evaluator production repair is an actual acceptance prerequisite. Current REST reviews, inline comments and threads contain no actionable debt; the bot's shared-correction scope warning has an evidence-based disposition. Final affected qualification, review and actual integration assessment remain open. This draft is not final squash notes or a readiness verdict.

Design choices: explicit per-call limits match the snapshot API without persistent owner policy or changed defaults. Larger retention admission does not bypass builder validity, storage capacity or allocation failures. No duplicate compaction body, dependencies, semantic features, compiler/cache customization or direct-main production edits were introduced.

Authoritative plan: .stage/rdf-core-mutabledataset-freeze-uses/plan.md. Selected evidence: .stage/rdf-core-mutabledataset-freeze-uses. Final integration uses ghprsq and requires verified issue closure, mirror readback and archived evidence before cleanup.

Defect-Class: resource-admission
