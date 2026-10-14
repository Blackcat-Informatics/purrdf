# Exact-numeric integration with current main

The qualified evaluator and complete host repairs now reside on main, so the
remaining numeric branch can integrate them without retaining the old failed
stack implementation. The production source merges without conflict.

The two conflicted documents are resolved by content. CHANGELOG retains exact
numeric/default/temporal guidance, the new projection migration and remembered
graph defaults. The Chinese catalog preserves both translations: 434 conflicts
were source references, with one header date and two additive translated sections.
Three duplicated graph entries had identical translations and were combined
with the standard `msguniq`. The existing mdbook extractor and `msgmerge` then
regenerated locations from the merged English book. `msgfmt --check --check-format`
passes; exact division and temporal translations remain present. No side or
translation was discarded wholesale and no tool was modified.

Affected evaluator/WASM/C/CLI/Python all-target compilation passed. Its first
invocation named a nonexistent Python package and executed no compilation; the
corrected command uses the actual manifest's `purrdf-python` package. That argument
failure is preserved separately and is not called a source failure or a pass.

The subsequent complete capacity/storage landing merges without conflict and
passes the same affected all-target compilation. Both integrations completed
their normal hooks and were published. The current combined package is running;
the earlier division-policy operation regression passed on the host integration.

Original exact-kernel, policy/profile and host receipts retain their source.
Current combined installed artifacts, affected numeric/temporal cells, mandatory
final qualification and publication remain pending. The distinct optimization
contract is not folded into this defaults/API delivery.
