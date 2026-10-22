Bind frozen implication warrants to their asserted premise

Frozen implication verification previously accepted an empty or weakened replacement premise because containment in the carried closure did not establish the original assertions. Retain the existing asserted-premise vector separately from the derived closure and compare sorted term values before replay. Equivalent content built with different dataset-local IDs or insertion order remains accepted.

The public property-chain regression reproduces the original failure, then verifies original/reordered acceptance and empty/weakened rejection after the repair. All 15 public cases pass natively and through the actual WebAssembly runner; 173 affected native library cases and strict all-target Clippy pass. Normal signed commit and push hooks passed. All 51 hosted checks pass with four configured skips. Independent review covers both files and reports no actionable code finding; its generic documentation metric is adjudicated against repository policy. Complete feedback and merge-input refreshes are unchanged, and the candidate matches the actual CI merge tree.

Closes #679. Parents #493 and #501 retain their complete remaining contracts.

Defect-Class: none
