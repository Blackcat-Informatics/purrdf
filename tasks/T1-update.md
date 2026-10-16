The resident-construction repair is pushed in commit `66974d6b3`. Declared limits
remain limits; the empty builder no longer reserves them. The public constructor
and original typed refusal paths remain unchanged.

Two focused native controls pass: maximum accepted limits allocate nothing at
construction, tiny-input allocations match small limits, high-ID content seals
and reopens identically, and rejected term/row ingress preserves accepted input.
Affected strict Clippy and helper hygiene pass. Normal commit/push hooks pass.

Byte-aware block packing and its reader/certifier changes are in progress on the
same branch. Both requested behaviors will ship together; the issue remains open.
