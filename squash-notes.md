fix: repair publishing preparation and retain Python diagnostics

The Cargo publisher precreated target/ for release notes before Cargo 1.99, preventing the CACHEDIR.TAG marker required by temporary-directory tests. Early summary validation now avoids writing that directory; the existing later writer remains unchanged. Actual Cargo 1.99 tiny tests reproduce the failure and prove the corrected startup sequence.

The Python native wheel/sdist and recipient audits passed, but the locked checker concealed an external exception and discarded the hosted wheel on failure. Check-only diagnostics expose approved exact exception-class constants without external data. Failed distributions are retained under the existing project artifact name; overwrite keeps the successful artifact roster unchanged. Strict validation and publish-mode credential sanitization remain intact. The underlying hosted-wheel error is not claimed resolved by this diagnostic change.

Reviewed head: 52df684184de67697361d5eacee3008bedd31c6a, base a63d3d573ebca7a0e7a78dd74ef03cfb856c7184. Refs #375.

Four paths: release-cargo.yaml, release-pypi.yaml, publish-python.py and test_python_publisher.py. Runtime/library source, versions, licensing rules and the release-notes checker are unchanged.

Validation: normal staged-snapshot hook and signed commit; actionlint; 18 locked publisher regressions; Ruff checks/format; exact retained distribution check; actual Cargo 1.99 paired startup test; diff checks. Independent four-path scope/security/artifact-roster review passes. Existing complete CI qualification of the unchanged runtime is retained; new hosted workflow checks are running and are not claimed passed. No additional runtime retesting is imposed before the maintainer-directed publishing retry.

Standing .goals remain satisfied. Deficiency ledger is empty. Full added-line deferral scan has no matches. No base changes or conflicts. All failed/cancelled attempts remain preserved; no functional 3.0.0 publication occurred before this repair.
