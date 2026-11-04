<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Checked word identities

`purrdf_hash::word_id` renders the original `hex::Digest32` as recognizable
words. Full words reconstruct all 256 original bits without a dataset or lookup
service. The digest remains authoritative; this check is neither authentication
nor content verification.

## Frozen version one

The alphabet is exactly 4,096 distinct lowercase ASCII letter-only words, indexed
in ascending ASCII lexical order. Its newline-delimited source is
`crates/hash/data/word-id/alphabet.txt`. The manifest and provenance beside it
identify its immutable bytes, EFF source, selection rule and attribution.
Tokenizer upgrades never change existing identities. Any change to the words,
order, packing or check law requires a distinctly named scheme.

Full canonical syntax is `b3w1: ` followed by 22 payload words and one check
word, separated by single ASCII spaces. There is no leading/trailing whitespace.
The header is metadata, not another dictionary word. Decode rejects unknown
versions/words, mixed case, non-ASCII bytes and noncanonical separators. It never
corrects user input.

Interpret the 32 digest bytes as a big-endian integer; append eight zero bits to
the right and divide the resulting 264 bits into 22 consecutive 12-bit indices.
For each of the first ten three-byte groups:

```text
a = (byte0 << 4) | (byte1 >> 4)
b = ((byte1 & 15) << 8) | byte2
```

Apply the same extraction to the final two bytes with an implicit zero third
byte. Thus the final payload index has eight zero low bits, which decoding
checks before returning a digest. No hex intermediate, big integer or rehash
participates in conversion.

At fixed form and count, the systematic check detects every single dictionary
word substitution and every swap of two unequal words, including swaps with the
check word. Multiple errors can cancel; a uniformly random 12-bit syndrome
matches with probability one in 4,096. There is no correction.

The field modulus is `x^12 + x^6 + x^4 + x + 1`. Its `x` element has order
4,095. The exact recurrence is:

```text
mul_x(v) = ((v << 1) XOR (0x053 if bit 11 is set else 0)) AND 0xFFF
step(state, digit) = mul_x(state XOR digit)
```

Start full forms at `0xB31`, first process the payload count 22, then each
payload index in displayed order. The resulting state is the final check index;
do not feed the check back into the recurrence. Independent polynomial long
division verifies the finite field and its distinct positional weights in the
native conformance target.

Full canonical output preserves digest byte order under raw ASCII comparison.
Sorted dictionary indices preserve the first differing payload group, and the
trailing check cannot change that difference. This does not imply content
similarity. Locale collation, mixed schemes and mixed full/prefix forms require
decoded identity comparison.

## Checked lexical prefixes

`b3p1: ` uses one through 21 leading payload words plus a fresh check. Start at
`0xB32`, process its displayed payload count, then its displayed indices. Never
reuse the full-form check. At 22 payload words use the ordinary full form.

`Reference::from_digest` selects the form explicitly. Strict decoding returns
`Reference::Prefix` containing a `Prefix`: exactly 12 times the payload count
bits, with all undisplayed bits zeroed. `lower_bound`, `bit_len` and `matches`
support a scope owner's later index lookup. This carrier supplies no dataset
identity, uniqueness claim, shortest-prefix claim or implicit resolution.
`decode_full` refuses it rather than inventing the missing digest bits.

## Native ownership and publication

`Reference::encoded_len` includes the header, every separator and the check.
`Reference::encode` checks capacity before writing, leaves the unused output
tail unchanged, and reports the exact required length. `decode_full` validates
the complete form before publishing any digest byte. Errors retain byte offsets
and zero-based word positions where applicable.

Scalar conversion uses fixed stack indices and immutable generated data: no
maps, initialization locks, filesystem, RNG, model or tokenizer. `to_text`
reserves its one string fallibly and preserves the original allocator error.
`encode_batch` lends one caller buffer to an ordered callback; each result's
borrow ends before the next conversion. `decode_batch` yields ordered original
results/refusals without allocating a result collection. Native batch record
indices are zero-based.

## Conversion tools

The actual `purrdf word-id` command exposes explicit formats:

```text
purrdf word-id encode --from hex --input digest.txt --output words.txt
purrdf word-id encode --from raw --input digest.bin
purrdf word-id decode --to hex --input words.txt
purrdf word-id decode --to raw --input words.txt --output digest.bin
purrdf word-id inspect --from words --input words.txt
purrdf word-id batch --from hex-lines --to word-lines --input digests.txt
purrdf word-id batch --from word-lines --to hex-lines --input words.txt
```

Input/output default to `-` for standard streams. Single text records permit one
terminal LF as file framing; the codec itself permits no whitespace outside its
grammar. Raw input/output is exactly 32 bytes. No bare input is guessed to be an
alias, prefix or prose. Inspection reports the exact digest, canonical full
text, payload/check counts and check spelling.

Batch text uses LF framing; CR and other whitespace remain input data and are
refused. Buffers are reused, including while draining an overlong record.
Successful output records preserve input order. An invalid record emits no
output record, writes its one-based original record number and stream byte
position to stderr, and makes the final exit status one. Readers/writers stop
on original I/O failure. This successful-record stream is not an atomic file
transaction; single-record validation completes before opening an output file.

## Offline generation and measurements

The Rust `word_dictionary` example selects the frozen EFF subset and measures
actual serialized full/prefix references through this native codec. It accepts
the pinned EFF source, pinned `cl100k_base` ranks, pinned `o200k_base` ranks and
an output directory. Its exact selection rule and tokenizer equivalence boundary
are in the alphabet provenance. It never downloads inputs or invokes a model.

The Rust `word_tables` example takes the frozen alphabet, a Rust output path and
a frozen-vector output path. It generates the contiguous word blob, offsets and
reverse metadata, checks every candidate lookup and runs rustfmt. Runtime reverse
lookup retains the measured displaced perfect hash and validates complete word
bytes. The compact two-letter bucket alternative remains in host comparison
tooling only. Both candidates use identical word bytes and inputs.

`purrdf-hash-conformance` owns native behavioral vectors and the existing
`digests` benchmark's `word_ids` group. There is no duplicated general-semantic
WASM execution suite. See the accompanying performance report for measured
scope, allocation, table size and cache limitations.
