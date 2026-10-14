Current source `20ede055f`: [CI attempt 2](https://github.com/Blackcat-Informatics/purrdf/actions/runs/38428369796/attempts/2) finished with 41 passing jobs, 4 skipped and one failure.

The cancelled native integration 5 shard passed its standard retry, and the test aggregate now passes. The workflow, compiler, cache, profile and 30-minute timeout are unchanged. The original cancellation receipt remains retained.

The sole failed job remains the optimized WASM package: 446/449 tests pass, with synchronous/LATERAL/NOT EXISTS depths 135/236/134 below the unchanged 181/281/180 assertions. The actual evaluator repair remains in progress under #531. Workspace, conformance, all seven assembly configurations and architecture controls pass on this head. No complete hosted pass, merge or closure is claimed.
