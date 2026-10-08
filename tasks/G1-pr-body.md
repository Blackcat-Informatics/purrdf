Chinese glossary checks previously used a separate Python implementation, and
link destinations could supply an English anchor or retained identifier that
readers never see. The shared native gate now checks visible text, preserves
whole-document fence state and original source locations, and runs the same
pattern, table, catalogue and token rules from Make, CI and staged hook snapshots.

The gate reuses the existing bounded native regular-expression engine. It retains
the supported glossary/PO contracts and narrow Research Object typography rule,
tests every current token row, and fails on malformed inputs or unsupported
pattern constructs. The obsolete Python glossary implementation is removed;
independent catalogue rendering remains part of i18n qualification.

Validation already demonstrated: native production scan of 69 glossary rows,
1,716 controls and 4,181 units; 41 applicable real-paragraph poisons; actual
external-root, content-selected Markdown and opposite staged/worktree poison
controls; strict clippy and affected hygiene. Independent review caught and
verified corrections for multiline URL masking and exact Markdown whitespace.

The full local `make check` and actual `make check-i18n` both passed at b1833aeed,
with unaffected-scope evidence reused for the bounded correction below. Seven poisoned render cases refused; all six actual book gates passed
across 33 pages and 25 parsed SPARQL fences, with zero fuzzy/untranslated catalogue
entries and zero fresh-template drift. Independent completion review found all
issue requirements and accepted plan criteria met. Earlier failed attempts and
their corrections remain in the qualification record.

Current head cdc43aaa6 additionally rejects malformed regex anchors on literal K
rows and preserves the original panic during failed restoration. Seventeen affected
Rust tests, strict clippy, production controls and caller/hygiene gates pass;
independent review confirms all four issue requirements and nine plan criteria.
The real review finding is addressed and its thread resolved.

Main now includes the merged rules engine. Its clean integration candidate and
semantic interaction are being assessed separately; older additive-hash evidence
is not claimed to cover that new delta. Current-head hosted checks remain pending
and must pass before protected integration.

Closes #260.

