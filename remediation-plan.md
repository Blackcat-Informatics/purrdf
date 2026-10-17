# Integrated cast-table repair

The required integration-5 job fails the existing anyURI-to-hexBinary refusal.
Its dependent test aggregator fails for that same cause; all other hosted jobs
pass or carry their expected skips. Preserve the failing run as evidence.

## Production correction

At the existing cast-source admission home, native AnyUri must retain its
primitive row: only string and identity targets are admitted. Remove its stale
unmodelled-row registration. URI-to-string conversion reads the parsed native
value, including XML whitespace collapse; string-to-URI delegates that mapping
to the native parser instead of allocating a redundant normalized copy.
Preserve source errors and workspace admission through the existing homes.

Extend the existing operator table with both binary refusals and positive
string/self/string-input cases. Add the same governed public controls to the
existing portable cast target. Once the executing Makefile invocation finishes,
register that case and the existing text-delta case in the normal wasm-test
selection and its documented inventory. Do not edit the executing launcher.

## Existing anyURI contract

Issue 571's native model and downstream replacement are already delivered by
the linked native work and GMEOW PRs 1838/1839. The current downstream manifest
selects native main 0994332d9; PR 1839 records 82 passing final-pin native
consumer/identity/witness/selector cases, native ownership with no compatibility
fallback, both refreshed attestations and actual native leaf WASM qualification.
Full unrelated GMEOW corpus/browser qualification is still separately pending.
The remaining discovered SPARQL consumer repair and false unsupported-anyURI
descriptions are being corrected here. Reuse actual unchanged model/consumer
evidence rather than creating another implementation branch. Do not claim
closure until the contract audit and corrected native/WASM executions pass.

## Qualification and publication

Let the original full gate finish; its nested standard release builds are live,
not assumed hung. Initial scheduling kept one owned Cargo stream. Once the gate
reached the prebuilt deep-request control (under 100 MiB resident, no active
compiler), that serialization was relaxed to allow one bounded focused build
alongside the read-only test execution. Check the original memory/disk admission,
retain eight build jobs/four test threads and a 16 GiB per-job limit, and let
standard Cargo coordinate shared access. Do not edit the executing launcher.
This is an owner scheduling correction, not a new user exception. Run the
affected operator table, the complete portable cast target natively and its new
case on actual WASM, strict affected compilation and required hygiene/registration
checks. Reuse unaffected full-gate results with explicit scope; never relabel the
failed original run a pass. Run missing final-gate components once. Commit and
push normally, refresh hosted feedback/CI, then merge only through ghprsq after
the real delivery gates. No new follow-up issue or partial closure is created.
