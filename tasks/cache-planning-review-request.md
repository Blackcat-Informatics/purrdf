@coderabbitai review

Commit `39cae7092` completes cache admission at the pinned-block boundary. The preceding review cleared `2def24ab9`; an additional deterministic control then demonstrated that allocating a larger cache array before accounting for the pending block could still refuse a valid read.

Cache growth now uses the existing shared growth-capacity calculation to compare its net retained increase plus the pending block with the budget before allocating. It reclaims through the existing eviction home when necessary. Actual replacement-overlap accounting remains in Memory; the planner does not sum future block ownership with an earlier replacement peak.

The new control covers both 31 pinned blocks with one reclaimable header (retain 32 cache slots and succeed) and all 32 pinned with sufficient sequential headroom (grow to 64 slots and succeed). Seven internal session controls, 27 provider/stack controls, 22 public query controls, affected all-target warning-denied Clippy, and the actual 82-job/118-row helper gate pass. The only ledger updates are the 40 affected constructor identities; authorities, sinks, classes and reasons are unchanged.

Please review this final bounded delta, including both budget boundaries. Source will remain stable during review. Remaining complete local execution follows this recheck; current hosted checks are pending.
