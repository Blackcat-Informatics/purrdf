# Native row control and layout review

Bounded source verdict: PASS. This is not performance or complete R2 acceptance.

Root independently reviewed the native-control extraction in workspace.rs and
the Resident/Owned(RowOwner) factoring in solution.rs, including every changed
attachment, dereference, evidence, clone and unique-extraction match. The native
factory retains the same checked SharedWorkspacePayload layout and charges it
before Shared::try_new. Payload fields still destroy the value before its grant.
Clone and unique mutation delegate to the unchanged Shared implementation;
public SharedWorkspace keeps caller Arc support and delegates native construction
to the sole admitted factory. No allocation, refcount or unsafe home was added.

Both new tracked controls are admitted before the original row cells move.
The resident/spilled grant and shared original cell owner remain in the same
payload variants. The outer two-way enum changes representation only: both owned
kinds still follow the prior shared-clone and fallible resident-extraction paths.
There is no raw bounded extraction or replacement query/evidence owner.

The first pointer-only correction did not restore row stride: its runtime test
observed 48 bytes. That failed result remains in recovery-native-control-focused.log.
The corrected focused log records the unit_sequence layout assertion passing
at 40 bytes, plus the original-query/rank/extraction control. The separately
executed owned_row_regressions control passes in recovery-native-layout-grant.log:
tracked clone retains the original admission, final drop releases it, and a
budget with no control room returns the original typed refusal. The earlier
mistaken zero-match filter is not counted as execution evidence.

Reviewed the added solution_row_alloc probe: it opens the measurement only after
constructing original cells and evidence, requires two publication allocations,
requires clone/intermediate destruction to allocate/release nothing, then measures
exact control and original spill bytes released by the last owner at both inline
and spilled widths. Root's receipt readback confirms all three solution-row
allocation tests pass in recovery-native-layout-focused.log, alongside the
join-allocation control and all 58 ingress/prepared/retrieval cases. The actual
grant control runs one test in recovery-native-layout-grant.log.

The complete affected collection, original fixed-work and 36-lane throughput
comparison, physical scaling/controls, portable executions and mandatory gates
remain required. No inference about unchanged numeric-control timing follows
from this representation review.

Root also reviewed the subsequent append_capacity correction: explicit checked
addition returns the same WorkspaceBoundOverflow on overflow, and successful
addition passes unchanged capacity/required arguments to the existing growth
home with unchanged error mapping. It removes eager error construction from
successful appends without changing admission or growth policy. Source PASS;
emitted-code and performance effects require the candidate's actual receipts.
The added overflow case uses the real reservation entry with one existing byte
and usize::MAX additional bytes. It checks the exact error, preserved original
byte, absent grant, unchanged live bytes and no latched admission failure before
continuing the existing owned-refusal test. Root readback confirms its 58-case
family passes, plus 20 ingress, 17 prepared, four allocation controls and all
19 native physical controls in recovery-row-append-focused.log. Strict package
checks subsequently pass. The raw recovery-row-append-diagnostic.log contains
five alternating pairs with 1,000 queries apiece and exact enabled/running
agreement. Median baseline 32,191,577.729 and current 30,833,837.549 retired
instructions per query confirm the reported 4.217688% reduction. The retained
row-push disassembly shows the direct successful-capacity branch. This proves
the bounded instruction correction, not the original throughput acceptance.
