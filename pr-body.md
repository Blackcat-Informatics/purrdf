CDT parsing discarded the original UCHAR and language-tag decoder failures, and
generic consumers could not inspect every lexical, comparison or fallible-reader
field. Retain those native causes inline and expose all four error families
through borrowed structured inspection. Standard `source()` follows the same
original lexical/storage/terminal failures; `UcharError` implements the standard
traits at its existing lexical home.

Public regressions cover input disposal, exact offsets and bounds, canonical
diagnostic keys, native downcasts, all comparison classifications, storage refusal
and accepted neighbours. Reusing consumer buffers measures zero allocations for
original diagnostic strings and the complete nested cause chain. Existing CDT
messages, grammar, deterministic output and admission bounds stay unchanged.

Validation: complete native CDT suite (166 unit/integration cases and 55 doctests),
strict workspace/all-target Clippy with warnings denied, and final CDT all-target
Clippy all pass. Normal commit/push hooks pass. Current-head hosted checks and
independent review must complete before landing. No semantic WASM selection,
dependency change or upstream work is included.

Closes #696. This completes the CDT leaf under #558; that parent's remaining
shipping inventory and host requirements remain open.
