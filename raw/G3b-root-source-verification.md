# Root source-freeze checks

Document and manifest checksums pass the producer's manifest; whitespace passes,
and the sole tracked changed path is docs/design/purrdf-simd.md. Root's normal
git diff --binary patch matches the producer's retained patch byte-for-byte,
SHA-256 dd41c2fb51a065f10b622976eca465350218b1b10ae53fc5ac2c7acd2d73f78f.
Remote readback confirms unchanged base b6f7c9b0 and PR head 8364b29c.

An earlier fail-fast root comparison used git diff --binary --full-index and
returned1 when compared to the producer's default-format patch. The sole mismatch
is the index line's full versus abbreviated blob IDs, as the complete zero-context
diff in G3b-root-patch-format-difference.txt proves. Source bytes are identical.
The default-format repeat and comparison exit0. Both patch representations remain
retained; the full-index representation has SHA-256
a7ecbddb34dae677e4c37cebe58e6af889952203c5570c49a1d82ab7e1a390d3.
No source or history mutation occurred in these read-only checks. This formatting
comparison failure is not relabeled as a passed command or a source failure.
