Native intake found a concrete prerequisite in the existing constraint engine:
the request disallowing only sh:Violation answers true for a warning-only inner
shape through native built-in/custom calls, but false through both registered
SPARQL call forms. Both callbacks create new default bindings. The runtime
reproducer covers a foreign focus and records all four observed answers.

The complete callback/request repair is being implemented as #699 on current
main. It retains the callee's original model, transports the full shared request
through the existing function-call boundary, and keeps each query's actual focus
graph. It includes default/inverse severity controls, ownership, request isolation,
applicable profile/subclass/graph and production-caller qualification.

The explicit LFP/GFP solver and all original corpus/host acceptance remain in
#698 and its parents #569/#690. Request-layout work is preserved; it is not a
standalone completed delivery. No upstream dependency or external repository
implementation is authorized or being performed.
