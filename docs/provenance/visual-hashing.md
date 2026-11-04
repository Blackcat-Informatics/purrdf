<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0 -->

# Native visual fingerprints

The rendering law and frozen alphabet derive from
[visual-hashing 1.0.0](https://github.com/Blackcat-Informatics/visual-hashing/tree/519c5bbdb5e0ef4c6c70aa0b5cf42a8877f6e776),
revision `519c5bbdb5e0ef4c6c70aa0b5cf42a8877f6e776`. Original copyright is
Blackcat Informatics® Inc., 2026. Its offer is
`MIT OR Apache-2.0 OR MulanPSL-2.0`. The repository's three license texts are
byte-identical to that revision's texts; recipient inventories retain them.
The nine files under `vectors/emojihash` and `vectors/randomart` are the
unchanged upstream corpus, offered under those same terms.

The inherited sources are `src/emojihash.rs` and `src/randomart.rs`.
The alphabet and corresponding spoken labels remain exact. The implementations
now use allocation-free stack kernels, admitted counting and rendering passes,
fallible exact output reservation, and an original admission owner retained
through output destruction. Upstream's `src/blake3.rs` is not incorporated.
The one `purrdf_hash::blake3` output reader supplies the XOF.

The scheme identifier is `visual-hashing-1.0.0`. Emoji consume successive
six-bit groups, most significant first, from BLAKE3 XOF output over the exact
input bytes. Emoji and spoken labels consume the same indices. The requested
symbol count is positive and explicit; an empty or unrepresentable count is
refused. Randomart walks the original bytes in two-bit groups, least significant
first, across the frozen 17 by 9 grid and ramp. Original label padding and
borders are preserved, including the different empty-label header. The start
marker precedes the end marker if both occupy the same cell.

For content identities, `purrdf_core::visual_fingerprint::present` supplies
exactly the original `Digest32` bytes. The shared hexadecimal reader accepts
either case before presentation; canonical digest serialization stays lowercase.
An alias must first resolve through its exact native scope. Future reversible
word adapters must likewise decode the complete digest first.

The native `scoped_content_aliases` example creates two scopes in which the
same content receives different ordinals. It calls `AliasScope::resolve` with
each complete `ScopedAlias`, passes the returned `Digest32` to `present`, and
checks identical emoji, spoken labels and randomart. Its printed rows retain
the full digest, scope, short ordinal and visual rendering as distinct values:

```sh
cargo run --release --locked -p purrdf-core --example scoped_content_aliases
```

The CLI presents a full digest without requiring an alias map:

```sh
purrdf fingerprint --digest 000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f --symbols 11 --max-bytes 4096 --max-work 100000
```

It prints the full digest, scheme, requested symbol count, emoji, corresponding
spoken labels and randomart as separate values. `--max-bytes` covers the native
rendering's live UTF-8 storage; `--max-work` covers decoding, counting, scanning
and native output copies. Resource refusal publishes no partial presentation.
GTS transport keys use the same renderer over their original public-key bytes,
preserving their existing fingerprints, rather than treating a key as a content ID.

A short fingerprint is a human comparison aid with possible collisions. It is
not reversible, an exact equality test, or cryptographic verification. Compare
the complete digest and verify content using its authoritative identity.
Visual similarity implies no content similarity.
