fix(python): keep Cargo color markup out of wheel licenses

The hosted wheel's License-Expression contained ANSI styling around Cargo duplicate markers because CARGO_TERM_COLOR=always affected the parsed cargo tree output. The metadata parser correctly rejected that wheel before upload. The backend now explicitly requests --color never for that machine-readable invocation; its existing self-test models the styled marker regression.

Refs #375. Reviewed head 48510206097df57bd2d5dee4a21ca3d35fba38dc; base 2250a18ff174bc9f66a149d5946d1555b9ea1243. One changed path: bindings/python/purrdf_build_backend.py. Library code, bindings runtime, grant selections, dependency selection, versions and release-notes checker are unchanged.

Validation: normal staged-snapshot commit hook, signed commit, actual Cargo 1.99 backend self-test, Ruff and diff checks. Requalification of a retained copy of the actual failed hosted wheel under CARGO_TERM_COLOR=always passes both metadata parsing and the locked publisher check, with native shared-library payload byte-identical. Independent source review GO confirms the narrow change. No complete runtime CI rerun is claimed; the library retains its earlier full qualification.

Standing .goals satisfied. Deficiency ledger is empty. Added-line deferral scan has no matches. No base changes or conflicts. Failed and cancelled release runs remain preserved, with no functional 3.0.0 publication before this repair.
