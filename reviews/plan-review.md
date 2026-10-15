# Owner plan and implementation boundary review

The full issue's public read contract is represented by real persisted inputs,
all three-layer permutations, no-declaration and combined-image controls, both
graph scopes and keyed values. The existing classifier is the right shared home;
its former visibility argument was always true at all three production callers,
which already gate visibility. Replacing that unused argument with the actual
input-inference policy preserves the mutable operation while keeping sealed
ordinary rows explicitly typed. Removing the optional result removes the now
redundant visible-record expect at the mutable caller.

The paged counterpart shares the exact defect, so both adapters change together.
Explicit annotation demotion and chronological first-occurrence behavior remain
covered. No new classification implementation, runtime dependency, host adapter,
toolchain override, scope cut or timing requirement is introduced. Existing
foreign consumer tests cannot be claimed as executed from this repository.

Owner review only, not independent approval. Production/public-query and final
hosted qualification must finish before completion. Zero subagents are used.
