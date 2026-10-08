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

The full local `make check` and actual `make check-i18n` both passed at the pushed
source. Seven poisoned render cases refused; all six actual book gates passed
across 33 pages and 25 parsed SPARQL fences, with zero fuzzy/untranslated catalogue
entries and zero fresh-template drift. Independent completion review found all
issue requirements and accepted plan criteria met. Earlier failed attempts and
their corrections remain in the qualification record.

The current main integration candidate is clean. Its only base advancement is
the additive BLAKE3 subtree API, covered by matching focused hash qualification;
existing hash paths are unchanged. Hosted CI and final review remain separate
acceptance steps before protected integration.

Closes #260.
