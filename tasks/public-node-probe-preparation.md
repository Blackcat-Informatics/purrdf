# Bounded public Node XPath acceptance probe

NOT RUN by reviewer. This does not add a shipping source test or a new WASM framework. The Stage-only probe uses Node's standard assertions and the existing package-root `index.mjs`; it neither imports Rust internals nor bypasses generated JS Error construction. GL owns the sole admitted heavy lane and current source-bound package capture.

After the current candidate optimized package is built through the already supported active-SDK/private capture route, run:

```sh
node /home/paudley/Active/purrdf/.worktrees/406-native-xpath-regex/.stage/native-xpath-regex/tasks/xpath-public-node-probe.mjs /absolute/current-source-bound-package/index.mjs
```

Supply the public entry of the actually captured candidate package, with its matching generated pkg/ files. Do not assume the old ignored worktree pkg is current. Use the admitted installed Node supporting JSPI; missing async capability must be actionable failure, never skip or fallback. The existing public package `ready()` and real synchronous/asynchronous APIs are the only execution surface.

The prepared probe asserts 50 cases: unselected compatibility plus both exact dated names distinguish noncapturing grammar and successful backreferences on QueryEngine ASK and SHACL SARIF, each sync/async; successful native REPLACE outputs actual b rows on both lanes; exact64KiB source neighbor succeeds beside65,537-byte refusal with Error.code/message xpath-pattern-bytes, both native laws across query/SHACL sync/async; three misspelled names refuse naming both accepted laws on sync/async query. A refusal is never counted as support. Valid neighbor after each failure also proves that failed job/error leaves the instance usable. SARIF carries actual shaclConforms from real targeted data, not an empty report oracle. No timings or additional semantic breadth is asserted.

Expected process exit0, 50 PASS lines, final JSON passed50/failed0/skipped0. An actual exception/assertion or unavailable async environment exits nonzero; preserve it and diagnose the actual source/binding cause. Do not change expected observations merely to make the probe pass. If package APIs differ, compare against actual index/source signature first and record any correction to this prepared script transparently.

Prior old-head package runtime log has439 actual Node passes but no XPath-specific public Node cases. Native `xpath_regex/exports.rs` and native async tests explicitly run below the JS error boundary; they do not demonstrate generated public Error.code or options routing in the real bundle. This narrow probe fills that concrete acceptance gap without repeating the broad package suite or inventing a new generic WASM semantic gate.
