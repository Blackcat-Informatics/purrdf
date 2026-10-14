# #470 current contract readback (pre-integration)

The live issue body (open on October 13) requires removal of the bounded public
integer/decimal API, exact terminating division with otherwise 18-place
half-even default, exact temporal seconds, CLI/Rust/Python/C/WASM propagation,
governor profile vectors, Breaking guidance and English/zh-Hans book. Its later
comment assigns #443's kernel performance work separately. The current #470
branch is clean and pushed at `50ae94847`.

Source inspection found no missing production signature or host-policy caller:
`XsdValue::{Integer,Decimal}` hold `exact::{Integer,Decimal}`;
`DivisionPolicy::xsd_default` is `ExactOrScale { scale: 18,
rounding: HalfEven }`; `Duration::seconds()` returns exact `Decimal`; and
the CLI, Python `division=`, C dataset policy and WASM `divisionPolicy`
route through that policy. The current CLI/Python/C/sync+async WASM fixtures
exercise a >18-place terminating quotient, repeating default, true 3/8
half-even tie, explicit exact, bare scale, 21-place `SECONDS`, invalid-policy
refusal and adjacent valid behavior. The C fixture restores the default after
its policy checks. Governor profile version is 18 and its vector manifest is
v18. CHANGELOG Breaking, book English and translated zh-Hans msgstr cover the
changed behavior. These are source observations, not a new host test run.

The retained branch receipts cover XSD, CLI, installed Python/C, selected
optimized WASM numeric tests, governor replay, native/actual-wasm numeric
determinism, metadata/i18n and regenerated conformance matrix. The corrected
helper registration at `50ae94847` passes its unchanged gate, but a whole
`make check` has **not** passed on that head. The former full optimized Node
check failed three original #531 depth floors on the old shared evaluator;
no current #470 package pass follows from the separate #531 source work.

Remaining qualification: integrate the qualified #531 shared repair into #470;
rebuild the actual linked CLI, installed Python and C, and optimized WASM host
artifacts because they embed the changed evaluator, then run the directly
affected numeric host cells and complete optimized package check. Run the
required full repository gate on that integration head; obtain final
current-source review, hosted checks and PR review; merge only through ghprsq
and verify #470 closure. The retained numeric/XSD, profile and documentation
receipts need no repetition where their source is unchanged. No #443 kernel
threshold or SIMD acceptance is part of #470. The drafts in
`tasks/T4-pr-body-draft.md` and `tasks/T4-squash-notes-draft.md` must be
refreshed with actual final-head results before publication.
