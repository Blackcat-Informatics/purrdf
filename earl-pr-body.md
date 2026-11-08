Refresh the SHACL 1.2 suite and vocabularies to W3C revision
`40231dfb97c5b606d22ae733947c0d14b4f45c07`, retaining the independently supported
SPARQL Rules snapshot. Add the explicit November editor's-draft profile for
eager list arguments and run-once execution per distinct focus node. Earlier
dated laws and existing v4 public contracts remain supported.

Native and CLI node-expression grading share typed failure handling. Correct
declaring-focus validation, recursive empty-shape behavior, conservative change
footprints and resident-body reuse, with product and host compatibility checks.
Update corpus receipts, measured claims and the documented source-commit EARL
workflow. This change supports the refreshed reports for release 4.0.1; the
release preparation remains tracked in #1235.

Validation:

- Native SHACL 1.2: 800 listed cases passed, plus one unlisted case; the root
  suite contributes 591 cases and independently pinned SRL contributes 209.
- Clean committed-source EARL generation passed. The unmodified current W3C
  browser page reports all 591 root cases passing, with no missing, unknown or
  failed results. The approved SHACL 2017 inventory matches all 120 assertions.
- Prepared-product equivalence: 468 cases passed; vocabulary comparison: 457
  reports; differential comparison: 476 base cases and 861 mutations.
- Affected native and CLI validation, expression and SRL cases passed, as did
  strict library, binary and selected-test Clippy. Normal hooks and signing
  passed. Independent substantive review's required corrections are resolved.

Hosted checks and forge review are pending. Final release reports will be
regenerated from the tagged 4.0.1 source; existing 4.0.0 assets remain intact.
