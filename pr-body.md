Loss-ledger checks returned formatted strings at the built-in API boundary, preventing callers from inspecting the original refusal. Completeness checks now return owned, ordered missing and unexpected code lists. Built-in and explicit-profile soundness checks return the same native error, with an explicit conversion from borrowed operands to owned storage.

Projection, schema-import and LinkML callers retain those native causes through structured inspection and the standard error chain. Entry order, pair-before-code precedence, duplicate-insensitive completeness, empty-ledger behavior and human diagnostics are preserved. Shipped examples use owned native errors, and the migration guide documents the changed signatures.

The capacity constructor-source identity was refreshed after reviewing the Core loss-only delta; capacity sinks, owner authorities and proof classifications are unchanged.

Validation: focused native Core/profile/projection cases, five selected Shapes unit cases and thirteen real loss-ledger integration cases passed. The final constructor-only correction uses the shared lexical constructor macro; affected production crates, selected tests and five example families pass release Clippy with warnings denied. Formatting, authored prose and normal signed commit hooks passed. The final full helper census passed; hosted CI is pending.

Closes #1117.
