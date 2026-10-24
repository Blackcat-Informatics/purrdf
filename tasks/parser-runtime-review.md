# Parser/runtime guard assessment

Owner review only; no independent review claimed. The parser's single expansion
scope admits physical stack, inserts a typed in-flight key and removes it on all
Result paths. Shape and expression identities remain separate. Property parsing
gets its own domain and guards its entire constraints/path/nested-property body,
closing the old entry gap. Inline property shapes can legitimately hold both
their shape wrapper and property-body key. A recursive backedge is rejected by
the already-live corresponding domain. Both unconstrained shape stand-ins and
the nested-property edge skip are removed.

Shared references are not a global visited-set failure: completed keys release
before another branch reaches the node. Path parsing uses the existing stack
home as well. Runtime expression and validation guards retain existing logical
ceilings and propagate physical failure. No new runtime package, feature, tool,
cache policy, duplicate helper, callback fallback or serializer change is added.

Actual affected native controls and strict/helper results are in validation.md.
The English policy and paired Chinese text distinguish cyclic shape expansion
from custom function calls, and describe failure rather than a partial verdict.
The sheval and current W3C Core sources support the explicit choice, without
claiming least/greatest fixed-point results. Chinese wording was owner-checked
against the English text; no native-reader or independent approval is claimed.

Preparation still has recursive stage-0 lowering ahead of the runtime guard.
That known gap is not waived or exported to a new issue; it remains required
work in this branch. The current unit can be committed/pushed for preservation,
but does not qualify the complete issue or authorize a partial-completion PR.
