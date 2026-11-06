GTS composition and relational projection failures previously reduced native causes to text. This change retains owned native causes, bounded offending-term evidence, original blob digests and writer/configuration failures through structured inspection and Python callers. Successful wire paths retain their existing behavior.

Closes #1107

Validation: 82 native GTS unit cases and 19 ingestion cases passed in the normal test profile. Strict release Clippy passed for the RDF library, ingestion target, affected benches and Python library. The Python runtime smoke covered writer/signing configuration, scoped blank/native fields, receipts and both relational wrappers. Independent substantive review found no actionable findings. Normal signed commit hooks passed. Synchronization with current main completed without conflicts and with normal signed hooks. Hosted CI has not run for this branch.
