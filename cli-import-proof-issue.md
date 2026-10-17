The completed integration-2 job on merged PR589 exposes a real native proof replay failure:

https://github.com/Blackcat-Informatics/purrdf/actions/runs/39057788791/job/117469588750

`crates/cli/tests/entails_cli.rs::consistency_decides_the_imports_closure` fails at1073. Of31 CLI entailment cases,30 pass and this one fails. The CLI successfully decides the imported disjoint Cat/Dog source inconsistent and emits a negative proof. Checking that freshly produced proof against the same imported closure refuses:

`malformed OWL-DL proof term: current support does not establish the recorded inhabitant`

The refusal originates in `crates/entail/src/owl_dl/proof.rs`. Full raw and ANSI-decoded job logs are retained in native590 Stage. This is distinct from the shared v3/v4-to-v5 artifact drift: the actual WASM package and Python conformance proof-vector checks passed on589. Do not waive the current-support invariant or weaken the CLI replay assertion.

Acceptance: reproduce the exact CLI imported-closure producer/checker path locally on tiny inputs; repair the native current-support/journal or source-identity mismatch at its owning seam; preserve imported-source identity, local-clash authority, exact inhabitant/premise validation and tamper rejection. Add/retain focused native, public boundary and actual CLI regression controls, qualify affected portable/native paths and strict Clippy/hooks, inspect reviews and merge only via ghprsq. Full GMEOW corpus/optimized producer/full gate and resource probes remain outside authority. Native590 is a separate borrowed-equality prerequisite for GMEOW1843 and must not be described as resolving this defect.
