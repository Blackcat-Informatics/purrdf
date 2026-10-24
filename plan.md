# Complete validation-request propagation

Implement the actual native callback boundary under the unchanged issue contract.
The public reproduction shows direct/native calls honoring the severity policy
while registered built-in and custom calls discard it. Original failing runtime
evidence is preserved in original-reproduce.log.

The chosen path retains a shared request in each Shapes value and dataset binding.
Canonical defaults have no per-request heap allocation. Expression guards and
standalone lowerings inherit that request; report-profile selection uses it when
there is no report capture. Production targets, rule conditions and native rule
heads bind the same request instead of retaining mask-only/default alternatives.

The existing bound function registry transports application invocation context to
ExprFnCall. SHACL binds its full request, actual graph name and shape-class
supplement there. Each call constructs a view over call.focus_graph, preserving
the callee's original declaration/model closure. Graph-name and severity payloads
are shared. Immutable bound-function data is retained once, with allocation-free
registry cloning. The root registry owns its request; that request owns a nested
registry sharing the exact bindings with a weak back-reference to the request.
Each invocation retains that request while its body runs. Nested queries install
the same existing function scope without a retaining cycle or a new global, and
moving the root registry out of its factory Arc cannot invalidate the request.
Raw engine bindings have no SHACL invocation context and retain default behavior.

Qualification must include the reproducer and inverse/default controls, interned
and foreign focuses, input disposal and independent requests, changing graphs and
nested callback queries, prepared/product-restored and complete reports, actual
graph overrides, rule heads/conditions and fresh-round graph behavior, selected
profile/regex controls, existing original-owner and allocation/report goldens,
and a meaningful request-payload allocation comparison. Test both registry moves
and zero-allocation clones after original input disposal. General semantics stay
native. Compile all affected workspace/binding callers and strict lint; run normal
hooks, hosted checks and independent current-head review before ghprsq landing.

Implementation and review remediation are published in PR #700. The current
head a010ace7b includes reviewed capacity-source proof refresh and downward
governed expression allocation pins after runtime head 6b5abee8b.
Final-head hosted checks/review and landing remain incomplete. No recursive
fixed-point behavior is claimed. Parent recursion/corpus/host requirements remain
open. Preserve the parked request-layout checkout and all original donor work.
No upstream dependency or external repository implementation is authorized.
