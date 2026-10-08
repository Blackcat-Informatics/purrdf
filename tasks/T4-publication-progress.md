# Task 4: local qualification complete; PR #503 open

The deterministic native Rust LUBM replacement is implemented and normally
committed/pushed. Four actual Make campaigns completed all fourteen queries,
including cold/warm default and custom-schema/nondefault generation. Independent
native graph/result-set checks, cache/parity checks, real command faults,
admission/tamper controls and 106 focused tests passed.

The settled full local `make check` exited 0. The independent final completion
audit passed against the original acceptance criteria. No required local gate
was skipped. The managed Cargo compiler ran in Stage's build slice, whose
effective limits were 48 GiB memory and 8 GiB swap; the outer Make scope's
64 GiB/no-swap settings did not apply to that compiler. This is correctness
qualification, with no performance claim.

PR #503 is open: https://github.com/Blackcat-Informatics/purrdf/pull/503
The `stagectl pr-create` wrapper returned a JSON parsing error after GitHub
successfully created the PR. Branch lookup confirmed the existing PR; creation
was not repeated. Hosted CI is running. Mandatory hosted jobs, complete review
adjudication and final integration through `ghprsq` remain pending.

Least confidence: the comparative interpretation of this native workload against
Java UBA. It intentionally supplies a distinct deterministic native profile,
not an identical Java random sequence or graph. Comparisons must use the same
profile, bytes, query bindings and regime.

What should be known: the query lane uses an explicit finite 100,000,000-step
join budget, and verifies full rather than partial acceptance. External ontology
and query bodies are not included in the selected Stage archive; their pinned
identities and transformation/replay dependencies are recorded. Failed earlier
runs are retained as failures. Other-repository submissions, including w3c-test,
are excluded; submissions already in flight remain untouched.
