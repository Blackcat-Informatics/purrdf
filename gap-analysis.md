# Delivery review

Implementer review, not independent judgment. The only shipping delta is the private frozen warrant field, moving the existing premise vector into it, sorting term values at construction/replay, and rejecting unequal assertions before frozen-instance checks. The existing single public verify path dispatches here for both standalone and composite frozen warrants; with_binding preserves the field. Named graphs remain outside the existing default-graph entailment contract.

Term-value ordering avoids dependence on local IDs; the actual native regression rebuilds the same assertions with reversed interning/quad order and passes. Original native reproduction fails because an empty premise is accepted. The fix also rejects a weakened subset while accepting the original. No new premise clone, hash implementation, matcher, dependency or feature is added. The original frozen closure and fresh-constant checks remain intact. Retaining the original vector adds retained warrant memory and sorting work; no performance improvement is claimed.

The first reordered-fixture compile omitted the TermFactory import, now corrected with a fresh native15-case pass. WebAssembly, strict lint, hooks, independent PR review and hosted gates remain pending. Parent493 source is preserved and is not included or qualified by this delivery.
