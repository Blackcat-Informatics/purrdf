# Enforce the native string domain

Issue 575: both native string parse doors currently copy arbitrary Rust strings, although the modeled XSD string domain and range alphabet are XML Char sequences. Reject non-XML scalars before allocation, preserving every valid lexical byte and all valid whitespace. AnyURI must consume the same borrowed XML validator before its existing collapse formatter; no second validator or legacy unchecked parse branch remains.

## Task 1: Repair both production parse doors

Add the borrowed XML string reader in the existing simple-value home, using purrdf_lex::terminals::is_xml_char. Route ordinary String parsing and try_parse_with_memory through it, retaining XsdError datatype/lexical context and Forg0001 for admitted refusal. Keep low-level supplied-buffer UTF-8 copying separate: it is not a datatype admission API. Inspect production XsdValue::String constructors for bypasses.

## Task 2: Verify the selected contract

Extend the existing portable text-primitive contract with invalid String/AnyURI character cases and a verbatim whitespace String case. Extend the allocation-accounting invalid-input matrix so rejected String lexicals allocate nothing. Run the XSD crate suite, warnings-as-errors all-target clippy, actual native/WASM portable contract and affected entailment tests. Tests construct only explicit synthetic inputs; no GMEOW corpus production, runtime hardware identity or machine probes are authorized.

## Task 3: Publish and merge

Use normal hooks, push, actual CodeRabbit review where available and ghprsq only. Preserve failed/unrun evidence and keep full hosted CI distinct; existing user authority permits merging with subsequent CI findings tracked. This corrects the native admission invariant without adding dependencies, semantic features, alternate readers or compatibility aliases. No unrelated worktree or dirty work may be removed.
