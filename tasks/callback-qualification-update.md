The entered evaluator boundary now uses the existing resident abort guard.
An actual answer callback panic restores its installed cap, evidence frames and
node cursors; a returned callback error keeps its original cause. A subsequent
evaluation on the same context succeeds. Existing provider code is reused.

The fix is published as88dfc2b7c after the normal commit hook. Native callback1/1,
existing provider1/1, positive-scope2/2, native stack1/1, scalar WASM stack1/1 and
SIMD WASM stack1/1 pass. Warning-denied affected Clippy and the unchanged helper
census pass. The original stack assertions are retained.

The final local gate and hosted run
https://github.com/Blackcat-Informatics/purrdf/actions/runs/38523547585 are running.
Hosted library/Python/optimized package, all seven assembly configurations and
their generated projection remain pending. The fixed-work DISTINCT evidence
does not establish an instruction win; no timing acceptance is used.
