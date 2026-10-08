# Native glossary delivery progress

PR #502 is open at pushed b1833aeed. Actual settled local `make check` and
`make check-i18n` completed exit0, independently reviewed against all four issue
requirements and nine plan criteria. The real book rendered33 pages and25
SPARQL fences, with seven poisoned render cases refused; the catalogue has2,956
translated entries, zero fuzzy/untranslated entries and two obsolete entries.
The native gate exercised69 rows,43 rejection rules,24 keep-English tokens,
1,716 production controls and4,181 scan units. These are actual local results;
current hosted CI remains pending.

Final-head review identified two bounded corrections: K rows must reject regex
anchors rather than compile an unusable literal keep token, and restoration
must preserve an existing panic while still hard-failing ordinary failed writes.
Both source corrections and meaningful controls are prepared and independently
reviewed for design, but their affected execution, normal commit and push have
not yet run. The former full/render PASS remains attributable to b1833aeed;
it does not certify these uncommitted corrections. No acceptance is deferred.

After focused validation, normal hooks/push and current hosted/review acceptance,
integration will use ghprsq with selected Stage evidence, followed by cleanup.
