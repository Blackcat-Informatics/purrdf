<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Mnemonic BLAKE3 identities

A full word identity carries the complete original BLAKE3 digest in 23 familiar
words. It can be decoded without a dataset or lookup service. The digest remains
the content identity; the word spelling provides a persistent presentation and
a transcription check.

## Usage

Use explicit input/output formats. A hex input is exactly 64 lowercase digits;
raw input is exactly 32 bytes. Text files may have one terminal LF as framing.

```sh
purrdf word-id encode --from hex --input digest.txt --output words.txt
purrdf word-id decode --to hex --input words.txt
purrdf word-id decode --to raw --input words.txt --output digest.bin
purrdf word-id inspect --from words --input words.txt
purrdf word-id batch --from hex-lines --to word-lines --input digests.txt
purrdf word-id batch --from word-lines --to hex-lines --input words.txt
```

Input/output default to `-` for standard streams. Inspection shows the exact
digest, canonical spelling and payload/check separation. Single text conversion
validates before opening an output file. Batch conversion reuses bounded
buffers, preserves successful-record order, and reports each refused input's
original record and byte position on stderr. Any refused record makes the final
exit status one, including when a downstream reader subsequently closes stdout.
An all-valid batch follows the ordinary graceful stdout-closure convention.

Full canonical text starts with `b3w1: ` and has one ASCII space between words,
with no trailing space. Native decoding strictly checks spelling, version,
separators, padding and the check word. It performs no automatic correction.

Rust callers use `purrdf_hash::word_id::Reference::Full` over the existing
`hex::Digest32`. `encoded_len` and `encode` support caller-owned output;
`to_text` allocates its one string fallibly. `decode` returns the original full
digest or a typed prefix selector, and `decode_full` requires the complete form.
Ordered batch APIs preserve original per-record refusals and reuse caller buffers.

## Theory

The frozen alphabet has 4,096 lowercase ASCII words, sorted lexically. Each
index carries 12 bits. Append eight zero bits to the big-endian 256-bit digest,
then read 22 consecutive indices. Decoding checks those padding bits and
recovers the original digest exactly. Alphabetic indexing makes canonical full
word text preserve digest byte ordering under raw ASCII comparison.

The final word is a systematic 12-bit check over a fixed finite field. At a
fixed form and word count it detects every single-word substitution and every
swap of unequal words, including a swap with the check word. There is no error
correction. Multiple errors can cancel; content verification and exact digest
comparison remain authoritative.

The alphabet derives from EFF's curated long wordlist. Its reproducible greedy
selection considers spelling confusion and measured token cost on pinned
`cl100k_base` and `o200k_base` vocabularies. Complete serialized references were
measured, including headers and fresh checks. See the
[provenance](../crates/hash/data/word-id/PROVENANCE.md) for attribution, licensing,
selection decisions and tokenizer equivalence, and the
[performance report](design/purrdf-word-identities-performance.md) for measured costs.

## Implementation and scoped references

The single runtime implementation is `purrdf_hash::word_id`, in the existing
zero-dependency hash owner. It uses direct bit extraction, fixed stack indices,
an immutable contiguous word blob and generated perfect-hash metadata. Every
lookup validates complete word bytes. Rust tools generate and analyze the
dictionary offline; runtime conversion has no tokenizer or model dependency.

Checked `b3p1: ` spellings contain one through 21 leading payload words and a
fresh prefix check. The lexical codec returns a validated digest-bit selector.
It supplies no dataset scope, uniqueness or minimum-length claim. Full forms
remain identical across datasets.

The remaining scoped owner belongs above this lexical codec: it must retain an
immutable scope/snapshot and complete digest index, calculate exact uniform and
per-entry prefix minimums, and resolve a checked selector uniquely against that
same snapshot. Unknown, ambiguous, wrong-scope and unavailable-source results
must remain explicit. Growth must preserve old snapshot bindings. Dataset
planning/resolution commands and visual rendering of resolved words, prefixes
and integer aliases belong to that scoped integration; the current word
conversion commands operate on complete identities.

The [canonical specification](design/purrdf-word-identities.md) gives the exact
packing, field polynomial, seeds, count-first recurrence, error/publication law
and native interfaces. Those version-one lexical laws are frozen.
