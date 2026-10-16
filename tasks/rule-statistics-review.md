# Native rule statistics consumer

Owner review, not independent approval. Whole issue remains incomplete.

Stratified::plan_with_statistics consumes the same core SnapshotStatistics seam
as SPARQL. Its caller supplies an atom-to-probe mapping in the provider's own
identity space. No RDF ID enters the executable or changes native join IDs.
Both ordinary and statistical planning lower through one implementation.

The existing binding-aware selection loop still ranks connectivity, known
positions, constants and repeated variables first. Available row counts break
the remaining ties before authored coordinates. Unknown counts retain the old
tie behavior. Certified cycles use the same atom priorities, retain whole-group
selection and preserve their internal leapfrog descent. Existing proof swaps
still restore authored source order. Statistics are order estimates, never row
bounds or resource certificates; this adds no suspended or partial-model claim.

The real native pipeline builds an executable with the sealed native RDF
provider, then evaluates it through seminaive::evaluate. It switches the order
from [0, 1, 2] to [1, 0, 2], keeps a connected five-row atom ahead of a disconnected
one-row atom, and retains all eight final facts and the one derivation with its
three authored premises. The control initially used a nonexistent len method;
the corrected row_count caller passes in rule-statistics-native-query-fixed.log.
All 31 affected plan-module controls pass, including cycles and authored swaps.
Strict Datalog library/test compilation passes with warnings denied. Native
helper census passes with no new duplicate, exemption or checker change.

This is an explicit immutable-snapshot planning path. The ordinary plan method
retains metadata-free behavior for programs without a statistics provider.
Resolving an atom into a probe is the caller's semantic boundary: a missing
mapping is unknown. Operational providers retain their original read-operation
completion obligation, as with the common statistics interface.

Segmented/effective-snapshot statistics, structural components and the final
whole-issue profile/portable/conformance acceptance remain required. No partial
PR, issue closure or independent approval is claimed.
