# Issue #577: fix(xsd): select XML 1.1 characters consistently for native text datatypes

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

Hosted PR 576 conformance exposed a character-domain mismatch: the frozen SHACL 1.2 singleLine case expects form feed and vertical tab to remain string values, while the new shared validator selected XML 1.0. XSD 1.1 sections 1.3 and 3.3.1 explicitly permit XML 1.1 selection. Select XML 1.1 Char once for native String/AnyURI, add its first-party lexical terminal without changing the XML 1.0 parser terminal, and update finite-length domain cardinalities from that same character set. Preserve NUL/U+FFFE/U+FFFF refusal before allocation, preserve XML 1.1 controls, and pass native/WASM text contracts, exact allocation tests, finite-space boundaries and the actual SHACL 1.2 conformance suite. No conformance suppression, alternate validator or dependency is needed. Failure evidence: https://github.com/Blackcat-Informatics/purrdf/actions/runs/39030094987/job/117388654055 . Normative: https://www.w3.org/TR/xmlschema11-2/#string and https://www.w3.org/TR/xml11/#charsets .

## Comments (0)

