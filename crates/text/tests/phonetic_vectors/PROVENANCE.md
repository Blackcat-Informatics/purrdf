<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Phonetic rules and independent reference answers

`../../src/phonetic.rs` implements an independently designed contextual
pronunciation classifier from Lawrence Philips' [Double Metaphone description](https://jacobfilipp.com/DrDobbs/articles/CUJ/2000/0006/philips/philips.htm)
and the declarative Double Metaphone correspondence rules in table 4 of
[US20090043584A1](https://patents.google.com/patent/US20090043584A1/en).
It does not implement the distinct Metaphone 3 rule set in that publication.
The implementation uses no imported, adapted or translated implementation code.
Black-box results disambiguate pronunciation outcomes, including the significant
space in a terminal-J alternate. No rule is a corpus-specific word lookup.

The module separately declares its input law: pinned case folding, canonical
Latin accent removal, a finite widening table, internal apostrophe removal and
the analyzer's named control predicate. It refuses remaining unsupported input,
empty pronunciation, more than 2,048 expanded letters, code bounds outside
1–64 and distance bounds outside 0–64. Code length defaults and bucket behavior
are analyzer configuration, not hidden state in this module.

The first-party Rust code and first-party synthetic vectors are offered under
`MIT OR Apache-2.0 OR MulanPSL-2.0`. The Apache reference program is not
redistributed or linked. No Java, foreign runtime or third-party crate is
required to build or execute the tests.

## Frozen black-box corpus

`commons-1.18.0.tsv` is unchanged: 20,664 input/primary/alternate rows computed
by the published [Apache Commons Codec 1.18.0 artifact](https://repo.maven.apache.org/maven2/commons-codec/commons-codec/1.18.0/commons-codec-1.18.0.jar)
with `maxCodeLen=64`. These are algorithm outputs over synthetic strings and
individually selected words; no upstream implementation or externally collected
word database is present in the fixture. The test accounts for every row.
All 20,503 ASCII-letter inputs, including mixed-case words, are replayed at code
bounds 1, 2, 3, 4, 6, 8, 16 and 64. Raw spaces, punctuation, cedillas and tildes
in the remaining 161 rows belong to the reference's different input law;
they are not treated as expected answers for our canonicalized spelling.
Canonical spelling, domain refusal, widening, accent and control behavior have
separate explicit tests. Neither code trimming nor empty-code universal
matching is permitted.

| Artifact | SHA-256 |
| --- | --- |
| `commons-1.18.0.tsv` | `ad48c900e11a7bb4dc41cdab2e40a8291a85420f712d882e43db250279d3359b` |
| Published reference JAR | `ba005f304cef92a3dede24a38ad5ac9b8afccf0d8f75839d6c1338634cf7f6e4` |

To reproduce the frozen answers, send each first field unchanged to the JAR's
`doubleMetaphone(input, false)` and `doubleMetaphone(input, true)` operations
with bound 64. Empty fields and trailing spaces are significant. Do not
regenerate expected outputs with PurRDF.

## Additional contextual corpus

`context-oracle.tsv` holds 8,616 separately generated first-party probes: 4,096
seeded arbitrary ASCII words, single-letter insertions and substitutions around
25 example names/words, and repeated consonant patterns at lengths 63, 64, 65,
127, 128, 129, 1,024 and 2,048. Its answers were computed by the same pinned
unmodified binary oracle, never by the Rust implementation. SHA-256:
`3170a6cb3e09cc11b2ff45ec698b6386a4a15f6a150d1f84fa2c544daf155ee0`.
The test freezes BLAKE3 identities for both files, accounts for all 29,280 rows,
and verifies all 29,119 ASCII-letter inputs at each output bound.

## Exact scalar distance

The production kernel follows the edit graph recurrence and the global endpoint
lower bound `|i-j| + |(m-i)-(n-j)|`. It trims common scalar ends, decodes the
query once and reuses bounded row storage. The alternative prepared Myers
kernel represents horizontal/vertical differences with multiword bit vectors.
Unsigned addition and shifts explicitly carry across every 64-bit boundary.
Both kernels implement global unit-cost scalar Levenshtein distance, not
substring, byte, grapheme or transposition distance.

Algorithmic references are Ukkonen's [Algorithms for approximate string
matching](https://www.cs.helsinki.fi/u/ukkonen/InfCont85.PDF) and Myers'
[A fast bit-vector algorithm for approximate string matching based on dynamic
programming](https://doi.org/10.1145/316542.316550). No implementation code from
these algorithms' libraries is incorporated. An independent full-matrix oracle
checks exhaustive short mixed-scalar words and both kernels' block, threshold,
length, scratch reuse and oversized-grapheme boundaries. The benchmark records
query preparation separately from reused candidate comparisons; no unmeasured
performance superiority is implied by the choice of algorithm.
