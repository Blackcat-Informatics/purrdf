# Draft; not ready to publish

Replace the decimal-limb arbitrary-precision implementation with the shared binary u64 BigInt home. Integer, Decimal, Rational and Geo spilled exact arithmetic reuse its inline storage, bounded scratch and schoolbook/Karatsuba multiplication; decimal parsing/rendering stay at the boundary. Preserve exact answers, DivisionPolicy/F&O failures and allocation-before-work admission.

Move the numeric governor to profile v14 with binary-limb work/allocation accounting and certified magnitude separation before expensive decimal conversion. The supported corpus remeasurement retained its existing frozen bytes and digest. Meaningful failures in overflow conversion, power/division result capacity and comparison pricing were retained and corrected without weakening their refusal assertions.

Final publication validation must be filled from actual settled owning receipts: current interval/numeric continuation, strict/hygiene and profile/corpus; real wasm/CLI; matched existing small/large arithmetic benchmark results including any regressions; independent evidence verdict; normal signed hooks and pushed head. Earlier311 unit/30 certified, affected float22, XSD-tail47 and Geo87 passes are scoped historical evidence, not a blanket current-source qualification. Fresh current-head native/Python/C/WASM CI remains pending until a PR exists. No performance or completion verdict is claimed by this draft.

Closes #477.
