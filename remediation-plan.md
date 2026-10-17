# CodeQL test-report output remediation

PR #593's CodeQL check 117491526067 reports two cleartext-output alerts at the
new public regression's failure messages (lines 73/74). Both explicitly dump the
entire checked report. The annotated paths name certificate parsing and trust
counts. Regardless of the fixture's public inputs, logging the entire report is
unnecessary to assert these fixed status lines.

Remove only the two report-format arguments. Preserve both conditions, all
negative controls and the complete production implementation. Run affected
test-target strict compilation and the same two native/scalar-WASM harness cases;
then normal hooks and publication. Reuse unchanged production/CLI/native-support
evidence with its recorded scope. Refresh CodeQL, hosted CI and review debt before
ghprsq. No alert dismissal, suppression, waived assertion or scope cut is used.

This is owner remediation, not an independent approval. The prior completed
CodeRabbit source review remains attributable to c86272091; this delta changes
only two test failure messages and still needs passing hosted qualification.
