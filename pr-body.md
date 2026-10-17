The shared native text validator selected XML 1.0 characters, causing two extra datatype violations in the frozen SHACL 1.2 singleLine case. Select the XSD-permitted XML 1.1 character domain explicitly for String and AnyURI. Keep XML 1.0 parsing unchanged, preserve pre-allocation invalid-character refusal, and derive finite-length alphabet counts from the same terminal ranges as admission.

The lexical terminal suite (27 tests), complete XSD suite, strict all-target clippy, three actual WebAssembly text tests, and full SHACL 1.2 conformance (16 tests) pass. The previously failing singleLine case passes with its expectations and the conformance ledger unchanged.

Closes #577.
