# SPARQL setup allocation diagnosis

The live performance pins are not semantic receipts: the original test explicitly
requires their exact closed forms to move when setup becomes cheaper. No pins
were changed during diagnosis. Admission277 and frozen governor traces remain
unchanged.

Matched heaptrack captures use the proven passing main executable and the current
managed executable, the same named test, one test thread and the existing original
corpus. The complete stacks are `sparql-final-{baseline,current}.demangled.stacks`.
They include preparation/warmup, so whole-process totals are not substituted for
the test's measured windows.

The additional two allocations in both SELECT surfaces come from the current
`CompiledBgp::project -> BgpProjection::new_admitted` path: it reconstructed a
column array and `Shared<SchemaData>` even when no blank column was removed. The
baseline projection kept its existing schema in that case. The corrected producer
retains the current immutable working schema's original owner; filtered layouts
still use the same admitted builder. Its mapping buffer and execution law remain
unchanged.

The expression surface's lower cost is also visible at actual production homes.
In the matched captures, allocations attributed directly to `eval_extend` fall
from154326 to123474: one per repeated scalar evaluation. `hash_join` falls by
61704, two per repeated scalar evaluation; the separately attributed index and
projection allocations are unchanged after their renamed native homes are paired.
There are30852 scalar calls, because the expression runs twice per focus node.
The underlying ownership change is concrete: `union_admitted` retains the
left schema when the right adds no variables, and `SharedSchema` transparently
retains the existing immutable `SchemaData` rather than cloning the variable array
and publishing another outer schema control. An unchanged schema clone no longer
allocates. These account for six fewer allocations per expression focus, without
changing its term values, joins or evaluation count.

VM argument metadata is sparse: original grants are retained only for actual
native owned values. Resident arguments no longer allocate an array of empty
lease slots. Arguments destroy values before leases, including refusal paths.
Native schema and argument lifetime/refusal checks remain independently exercised
by the consolidated evaluator and bounded public owner fixtures.

The latest projection correction must still be measured by the original N/2N,
governed N/2N/4N and fallback tests before any exact live pin is updated.
