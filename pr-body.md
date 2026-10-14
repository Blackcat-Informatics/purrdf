Caught WASM faults now expose a stable poison code, and synchronous/asynchronous
queries report matching stack refusal endpoints and messages. HTTP consumers
receive 400 for typed stack refusal while actual execution faults remain 500.
Nested `SELECT *` shares ordered projection snapshots, and GTS canonical identity
traversal reuses structural keys instead of repeatedly expanding deep terms.
The deep asynchronous regression retains its answer, region and recovery checks
with fewer timer yields.

The current optimized package passes all 458 Node tests, TypeScript checks,
packed-tarball installation/import and high-u64 identity ABI checks. This includes
the original nesting floors, both-lane refusal/recovery, poison gates, idle
SERVICE/LOAD deadlines, deep returned terms and Cloudflare error classification.
Native parser/projection/GTS scaling and frozen-vector evidence is retained;
the integrated evaluator/WASM compilation and nested-star runtime also pass.
The earlier full local gate and the subsequent affected checks are separately
recorded in the selected Stage validation index. Hosted PR checks and final
review are pending.

`GraphPattern::ProjectStar` is a Rust source API addition. The migration guide
documents exhaustive-match adaptations and requires the 4.0 major release.

Closes #365.
