# Unchanged numerical control readback

The adjacent 36-lane comparison remains rejected. In particular, integer total
comparison reports a significant increase. No numerical regression is waived.

Read-only linked-artifact inspection after the window repair compares the
baseline and candidate query_eval binaries in the preserved default-baseline
and default-current release caches. The named XSD numeric_total_cmp function
contains 317 instructions in each binary. Every instruction and operand is
equivalent after replacing RIP-relative relocation displacements, local branch
addresses with their original function offsets, and direct call addresses with
their unchanged named targets. The integer/integer path has the same original
128-bit comparisons, branch sequence, stack setup and return. This observation
does not prove cache or timing equality.

The small value_total_cmp dispatcher is also instruction-equivalent. Its
baseline start is 0x771a10, candidate start 0x850570: the candidate dispatcher
crosses a 64-byte boundary. Placement alone has not established causality, and
no alignment or XSD arithmetic source change has been made for the benchmark.

Contemporaneous adjacent receipts show unequal sibling occupancy, including
the ORDER LIMIT baseline and candidate pair. A later read-only 2.5-second
/proc/stat sample during strict compilation shows every logical CPU at 99–100%
busy. It supplies no observed quieter physical core. Re-observe CPU and sibling
occupancy during the exclusive measurement window, preserving sibling work and
processes. Only actual settled-source measurements can qualify default speed.
