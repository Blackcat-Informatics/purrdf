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

The native scoped owner is `purrdf_core::content_prefix::PrefixPlan`. It retains
the real `content_alias::Presentation`: the caller-owned scope and exact mapping
revision, together with an admitted immutable index of complete digest bytes.
`from_aliases` captures the existing alias owner's current membership; source
and explicitly sorted-source constructors also accept the real presentation.
The caller must supply the complete membership belonging to that revision.
Original source, storage and work refusals remain typed. No dataset-size ceiling
is compiled into the owner.

Construction deduplicates equal digests, sorts them and compares adjacent pairs.
If two distinct neighbours share `b` leading bits, they need `1 + floor(b / 12)`
payload words. The largest such value is the exact uniform minimum. Each entry's
minimum uses its maximum overlap with either neighbour. Empty plans report zero;
singletons use one word. At 22 payload words the ordinary full form is emitted.
The summary reports distinct membership and the distribution of individual
lengths. Source rows and integer aliases remain separate from content deduplication.

Resolution validates the lexical check before consulting the index. A binary
lower-bound search and at most two candidate matches establish unknown, unique
or ambiguous membership; a check word never makes an ambiguous prefix unique.
Wrong scope and revision remain explicit. Growth requires a new plan and the
new presentation. An old immutable plan continues to resolve its original
references; supplying them to a grown plan refuses their stale revision.

## Scoped command usage

The tools reuse the existing `PURRAL01` alias manifest, including its original
assignment order and revision law. Choose a scope once for an append-only
assignment history; do not reuse it for another history. `SCOPE` below is that
caller's exact 64-digit lowercase hex scope. `REVISION` is the exact revision
retained when presenting the references, printed by `plan` on stderr. It must
be retained independently of a mutable filename.

```sh
purrdf word-id plan --scope SCOPE --input digests.txt --output snapshot.bin
purrdf word-id shorten --manifest snapshot.bin --scope SCOPE --revision REVISION \
  --from hex --length per-entry --input digest.txt --output short.txt
purrdf word-id expand --manifest snapshot.bin --scope SCOPE --revision REVISION \
  --to hex --input short.txt
purrdf word-id inspect-scoped --manifest snapshot.bin --scope SCOPE \
  --revision REVISION --input short.txt
purrdf word-id batch-scoped --manifest snapshot.bin --scope SCOPE \
  --revision REVISION --from hex-lines --to word-lines --length uniform \
  --input digests.txt --output prefixes.txt
purrdf word-id batch-scoped --manifest snapshot.bin --scope SCOPE \
  --revision REVISION --from word-lines --to hex-lines --length uniform \
  --input prefixes.txt
```

`plan` reads canonical hex lines once into admitted storage and makes one atomic
alias extension. It writes the binary manifest to the explicit output and reports
the scope, revision, uniform payload/checked lengths and per-entry histogram on
stderr. Duplicate digests reuse their original assignment. Native alias exports
can also supply the manifest directly; no alternate snapshot serialization exists.

Every scoped conversion restores that manifest, checks the expected presentation
and constructs one native plan. Buffered batches reuse the plan and their buffers
across all records. `--length uniform` and `--length per-entry` explicitly choose
the shortening law. `word-lines` in a scoped command accepts checked full forms
and checked prefixes; both require unique membership in the bound snapshot.
`inspect-scoped` shows the resolved full digest and the submitted selector's
canonical spelling, payload/check counts and check word. Full-form conversion
commands retain their original explicit formats and need no snapshot.

Malformed single input, stale revision or invalid manifest fails before the output
opens. Input/output alias checks include the snapshot itself and hardlinks;
canonicalization receipt destinations also protect it. Source I/O failures stop
processing. Batch refusals retain their original order and record/byte context,
including when a later downstream stdout reader closes.

Matching prefix text across datasets identifies candidates only: resolve within
each original snapshot, then compare complete digests. The native
`scoped_content_aliases` example decodes full words and checked prefixes, resolves
integer aliases and renders all three from the same exact 32 bytes. Emoji,
spoken labels and randomart are therefore identical for the same identity.
Visual comparison remains a human check, with exact digest equality authoritative.

The [canonical specification](design/purrdf-word-identities.md) gives the exact
packing, field polynomial, seeds, count-first recurrence, error/publication law
and native interfaces. Those version-one lexical laws are frozen.
