# Completion assessment of native XML 1.1 selection

This is an implementer assessment, not an independent subagent report; independent dispatch is unavailable. Reviewed the four-file patch, the issue, plan.md, the original hosted failure and all local test logs. CodeRabbit is independently processing the actual head; no completed review is claimed.

| Required behavior | Actual production-path evidence | Status |
|---|---|---|
| Shared XML 1.1 admission, distinct from XML 1.0 syntax parsing | New terminal enters the exhaustive terminal/range census; existing XML 1.0 terminal and character-reference decoder remain unchanged | 27 terminal tests pass |
| Both text primitives preserve admitted controls | Public parse tests call String/AnyURI with U+0001, U+000B, U+000C and U+001F | Native and actual WASM pass |
| Invalid scalars refuse before allocation | Existing public admitted parser accounting matrix covers NUL/U+FFFE String failures; ordinary text tests retain NUL/U+FFFE/U+FFFF refusal | Complete XSD suite passes |
| Range arithmetic uses the selected alphabet | Compile-time range cardinality plus independent full scalar census and finite-exhaustion/overflow cases | Complete XSD suite passes |
| Repair the observed SHACL regression without changing expectations | Actual w3c12_conformance binary runs the full approved suite and inventory on current code | All 16 tests pass, including singleLine |

Strict lex/XSD all-target clippy passes. Normal commit hooks and push passed for 66d5e3161. No new dependency, suppression, fallback or frozen-expected-report change was introduced. Every requirement in native issue 577 has a direct demonstration; full hosted workspace CI remains pending under explicit user merge authority. There is no deficiency-ledger entry or authorized source descope.

Integration candidate: fetched unchanged base f1ad5dfb634fc20622daa082a6bc2238d9a92789; clean merge tree 44cb53ea4b4d224d2fba73531b7d2f967b1ec051. There is no intervening base delta to invalidate the demonstrated behavior. Governing XSD profile choice is explicit in the public reader documentation and the terminal. The original 1.0-only reader is replaced, not retained as an alternative text-admission branch.
