# Evidence EXISTS and fuel-trace movement

Read-only source/golden/receipt diagnosis. No execution, regeneration, source
edit or process change. `recovery-exists-owner-focused.log` reports **15** moved
cases; the failure formatter prints only the first ten. The remaining five and
their full current ledgers/sweeps are not present in this receipt.

## What changed

This is an execution-strategy change, not merely repaired ledger attribution.
`query_eval_env` (`sparql-conformance/src/run.rs:745`) installs harness relations
for every query. `EvalCtx::may_have_evidence` therefore makes preparation preserve
evidence even when a particular query never calls a registered relation.

`PreparedExists::build_with_evidence_admitted` (`expr.rs:14435`) copies the
original body instead of applying ENF. `exists_prepared` disables its positive
seeded fast path when evidence is requested, and its definition path at
`expr.rs:2526` uses the original normalized body rather than the synthetic
first-witness Slice. The probe path also coalesces witness evidence instead of
using only bucket presence. The recent structural source-map fix makes a copied
OPTIONAL map correctly; it did not introduce these evidence-mode decisions.

ENF proves emptiness equivalence, not preservation of every witness's evidence.
Keeping the body and all witnesses is justified when evidence is actually needed.
That does not independently justify blanket new fuel behavior for a query whose
only connection to evidence is an unused registry entry.

## First ten reported cases

| Case | Old -> current fuel | Source explanation |
| --- | --- | --- |
| existsGraphVariableCorrelated | 34 -> 31 | Definition execution omits the first-witness wrapper. Two invocations (one successful, one empty) omit two wrapper entries and one wrapper committed row. |
| existsMinusDomain | 39 -> 36 | Same omitted first-witness wrapper mechanism across the correlated definitions; no new numeric or row-owner charge accounts for the decrease. |
| existsNestedCorrelation | 45 -> 39 | Both levels use evidence-preserving definitions and omit their synthetic wrappers. |
| existsOptionalPadding | 16 -> 19 | ENF previously erased LeftJoin to its unit left operand. Retaining it adds LeftJoin entry/output and the empty right BGP entry: three units. |
| existsSubselectLimit | 22 -> 43 | ENF previously erased the length-preserving Slice and the resulting normalized pattern used the probe strategy (the golden explicitly records exists-probe-answered=1). The preserved Slice/Project instead take correlated definition execution. Exact allocation of all 21 new units requires the current per-node ledger; it is not captured in the failure summary. |
| notExistsOptionalPadding | 12 -> 15 | Same retained LeftJoin and empty right BGP as the positive OPTIONAL case; output answer remains empty. |
| bbMemberFalse | 11 -> 10 | Empty NOT EXISTS right operand loses a synthetic Slice entry. The golden RHS PropertyFunction has algebra-node-entry=2 but only one invocation; this is not evidence of lost producer-work metering. |
| modeRestrictedSubsumption | 11 -> 10 | Same empty NOT EXISTS wrapper removal, with the original mode-restricted producer still invoked. |
| exists04, nested positive | 37 -> 33 | Two successful nested definitions each omit wrapper entry and wrapper output. |
| exists05, nested negative | 32 -> 29 | Successful inner and empty outer definitions omit the corresponding two-plus-one wrapper charges. |

These explanations follow the original fixture/golden and changed control flow.
Only the displayed totals and unchanged displayed answer/resource fields are
runtime observations. No fresh complete trace or per-node counter capture was
performed by this reviewer.

## Acceptance disposition

Do not classify all movements as a fuel-accounting bug, and do not accept them
merely by regenerating. The decreases have an explicit removed computation;
the increases have an explicit retained computation and changed strategy. No
source evidence here establishes a missing charge or wrong answer. Evidence
queries must preserve the whole witness-information law even if that costs more.

The ordinary-path applicability remains a real issue: the approved plan preserves
default producer/query behavior and separately requires the original throughput
controls. These fixtures are plain graph queries except the two explicit property
calls, yet an unused registry causes their strategy change. Establish whether
evidence is reachable for the particular prepared body (including nested sites,
sources and shape providers) before retaining unnecessary evidence work, or
explicitly justify the conservative path against those requirements. Do not simply
turn evidence off globally: real producer evidence and rowless search declarations
must remain visible. This review is not authorization to weaken acceptance or
change the harness environment.

Before any intentional golden update, inspect all 15 complete current blocks:
answer, per-node charges and every fuel-sweep barrier/certificate, not just their
metered totals. The original comparison checks all of these. The formatter stops
at the first differing line and first ten cases, so this receipt cannot establish
their full equivalence. Required throughput and full-gate acceptance remain open.
