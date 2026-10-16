Fixed in `225e078da`. `reserve_buffer` now evicts only for mandatory payload/cache
and request-ledger admission. Optional routing growth uses spare capacity and
returns its own `Residency` refusal to the existing hint-skip path. Other errors
retain their original propagation and latch.

The existing pressure case keeps its original two-slot cache and adds 128 slots.
On the previous source, optional bookkeeping grows from 94,624 to 102,944 bytes
by reclaiming payload space despite only 4096 spare bytes. The correction passes
with unchanged bookkeeping, exact value readback, no source fault and healthy
progress after releasing the held grant. All 33 native storage/stack cases,
21 segmented units, original eight consumer lanes under 256 MiB, both actual
scalar/SIMD pressure executions, strict checks and fresh helper hygiene pass.
The same test and runner are used; no new lookup path or timing acceptance is
introduced. Current-head hosted qualification remains before integration.
