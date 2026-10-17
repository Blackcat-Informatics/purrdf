# Select the XML 1.1 text domain consistently

Hosted SHACL 1.2 conformance on merged PR 576 reports two extra datatype violations in the frozen singleLine case: form feed and vertical tab. XSD 1.1 sections 1.3 and 3.3.1 explicitly permit selecting XML 1.1 Char. Select that single domain for the native text datatypes rather than weakening conformance or discarding pre-allocation validation.

## Task 1: Repair the common character authority

Add the XML 1.1 Char terminal in purrdf-lex, keeping XML 1.0 parser behavior unchanged. Switch the one shared native String/AnyURI validator to it. Preserve NUL, U+FFFE and U+FFFF refusal before allocation; preserve all XML 1.1 controls verbatim except the existing four XML whitespace collapse characters for URI. Bind finite-length alphabet arithmetic to the same ranges and census the exact selected alphabet in its regression. Remove obsolete XML 1.0-only text assumptions and tests.

## Task 2: Verify real consumers and integrate

Run lexical terminal checks, complete XSD suite, admitted allocation controls, actual WASM portable tests, strict affected clippy and the actual SHACL 1.2 conformance suite. Capture original hosted failure and actual local outcomes. Use normal hooks/push, attributable CodeRabbit review where available, ghprsq and cleanup only after archive verification. No host/resource probes, corpus factories, compatibility path or new dependency. Independent subagent dispatch is unavailable; do not invent one. Broader hosted CI may remain pending under the explicit user merge instruction, but this observed conformance failure must be demonstrated repaired before merging.
