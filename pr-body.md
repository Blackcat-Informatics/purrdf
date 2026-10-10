Closes #472.

Packed Turtle now emits nested blank properties and collections through the native
heap work list. It retains every statement, cycles/shared/quoted references and
statement metadata while keeping the structural-key guard at forty. Indentation
stays byte-identical through forty actual levels, then saturates at 160 spaces so
deep output remains linear in size.

The shared public corpus executes actual 100,000- and 1,000,000-node inputs through
packed rendering and the native Turtle/default and TriG/named routes. Six frozen
byte-length/BLAKE3 receipts match on native and optimized wasm; metadata reparses
and native byte reserialization pass. Exact original guard neighbors, reversed
interning/rows including named TriG, cycles/shared/quoted blanks, a 100k collection
and multi-object indentation are covered. Object-key equality and ordering also
have a direct consistency control.

Validation: affected core/RDF all-target strict checks pass; final native and
optimized portable public corpora each pass 7/7; the direct key control passes;
original-renderer neighbor capture passes. The first-party benchmark measured all
three depths with no failures (medians 4.645µs, 236.3ms, 2.928s). The required full
`make check` passed: static/hygiene/generated checks, native workspace tests and
doctests, the standalone preserve-order consumer and optimized wasm release build.
The public Turtle corpus passed7/7 again within that full gate.

Confidence is lowest in extrapolating host benchmark medians under concurrent
load; they are report-only observations. The output change callers should know
about is indentation saturation beyond actual level forty. No dependency,
semantic feature, namespace default or enlarged stack is added.
