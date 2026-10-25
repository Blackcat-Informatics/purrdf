RDFC's n-degree search held a native call frame for each newly visited related
blank. It now suspends invocations in the shared heap work list and resumes
each path with the child's hash and issuer. Both pruning points, related-hash
and permutation order, successful bytes, work charges and stop checkpoints
remain exact. The production recursive call is removed.

Closes #710. Parent #489 remains open for configurable/removable search budgets,
configuration/work evidence, dataset-state controls and heap UDF execution.

Native qualification: 60 core canonicalization cases, 12 frozen profile cases
and all six W3C harness cases, including poison. The new 256-node traversal
passes on a 128 KiB native stack. Strict checks cover both production libraries
and the two conformance consumers. No new dependency or profile change.
Full helper hygiene and normal commit hooks pass. The same 173 constructor
sources and original sinks remain bound; only 40 aggregate source pins change.

Hosted qualification exposed an overbroad Miri filter: the new semantic stack
case matched `small` and exhausted the job deadline. The four storage commands
now select `--lib small::tests::`, retaining all 18 SmallVec cases under the
existing default, tree-borrow, strict-provenance and 32-bit models. The BLAKE3
streaming probe and 30-minute hosted deadline are unchanged. The complete
corrected `make miri` gate passes locally.
