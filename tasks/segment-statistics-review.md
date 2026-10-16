# Persisted statistical records and the actual reader

Owner review in progress, not independent approval or issue completion.

The producer translates sealed metadata into its declared full-width dictionary
interval. Data profile 2 adds fixed predicate/frequency/category records and a
raw compressed-domain section. Counts are read selectively; domain visits copy
at most one varint's bytes through authenticated blocks and the original reader
owner. Opening remains header-only, with no metadata cache, historical dictionary
or query-ID translation table added to the session. Native RDF row layouts and
the four original sealed metadata buffers retain their shapes.

The canonical record encoder also feeds native page-summary certification.
Complete segmented certification reconstructs actual RDF and compares every
statistical record, range, count, payload and block packing with the derived
metadata. The new tamper control recomputes the whole hash tree after falsifying
a row count; certification rejects the false semantic claim. The selective-count
control proves that count/frequency/category lookups read neither distinct-ID
payloads nor dictionary/RDF blocks. Exhausted request admission remains the
original EvidenceExhausted error, not a false zero.

Actual metadata parity passes all six controls, including full-width physical
IDs, three RDF streams, graph scopes and streamed domains across a larger
fixture. All eight certification/construction controls pass. Both groups of
three relevant primitive controls pass: varints and deltas. The shared decoder
now refuses excess high payload bits, invalid cursor overflow and cumulative
delta overflow with typed errors. Valid edge vectors, including u64::MAX and
repeated deltas, retain their original behavior.

The first seal-control filter ran zero cases and is not coverage; the corrected
certification_tests selection executes eight. Initial test compilation refused
a redundant qualification and a mutable const reference; both are corrected.
The first unused whole-domain decoder draft was replaced before wiring because
loading all domains would make ordinary count planning proportional to NDV.
Its failed dead-code check is retained as historical evidence, not a pass.

The existing exact-subject public query passes, but EXPLAIN found a real
integration regression: a coarse subject-bound marginal displaced the prior
exact range and changed order. The actual provider now tightens the shared
default count through its original subject range. Production compilation of
that correction passes; the unchanged EXPLAIN comparison now passes. Initial
strict findings are corrected without allows: large headers pass by reference,
byte error construction is lazy, constant chunks use the standard array view
and assignment punctuation is explicit. Current strict qualification passes.

The property now additionally compares two actual physical segments against a
combined segment, translating domain identities by RDF value; it passes.
The original thousand-source header-only open, shallow snapshot allocation,
actual query peak, range and fault-precedence controls pass. All five affected
page-summary certification controls pass. The native helper census passes with
the existing forty constructor proofs refreshed; no exemption was added.
The standard wasm32-unknown-unknown library check passes. This proves portable
compilation, not portable runtime execution. Portable execution and whole-issue
reconciliation remain required at final qualification.
Effective chronological snapshot statistics remain separate required work;
physical source statistics are not forwarded as exact layered-set statistics.
