<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Word alphabet provenance

The verbatim `eff_large_wordlist.txt` was published by the Electronic Frontier
Foundation on July 18, 2016. Joseph Bonneau's official methodology calls it
EFF's own list and explains its selection using Ghent recognition/concreteness
data, spelling/homophone exclusions and prefix exclusion:

- <https://www.eff.org/files/2016/07/18/eff_large_wordlist.txt>
- <https://www.eff.org/dice>
- <https://www.eff.org/deeplinks/2016/07/new-wordlists-random-passphrases>
- <https://www.eff.org/copyright>

The source has no embedded license statement. On November 4, 2026, EFF's
copyright policy explicitly licensed EFF-original website material under
Creative Commons Attribution 4.0 International unless otherwise noted. The
methodology identifies this list as EFF's own original selection. This is the
grant and provenance basis used here; it is not a separate file-specific grant.
The copyright page's image link still pointed to Attribution 3.0 US, while its
explicit prose and license link named Attribution 4.0 International. Preserve
the original attribution and this distinction. `EFF-NOTICE.txt` and the complete
Attribution 4.0 license accompany recipient inventories.

The original list has 7,776 rows. Four spellings contain punctuation and are
excluded: `drop-down`, `felt-tip`, `t-shirt`, `yo-yo`. The remaining 7,772 words
are lowercase ASCII letters of length three through nine, lexically ordered
and prefix-free. They are the candidate universe; no recognition score is
invented for individual words.

## Reproducible greedy selection

Use byte Levenshtein distance: insertion, deletion and substitution each cost
one; a transposition is two edits. Construct the undirected candidate graphs
for distance exactly one and exactly two. For each word, measure token count
of its leading-space spelling in both pinned vocabularies below.

Repeatedly remove the live word with the lexicographically largest tuple:

```text
(live distance-one degree, live distance-two degree,
 maximum of the two token counts, sum of the two token counts,
 byte length, ASCII spelling)
```

Recompute affected live degrees after each removal, until exactly 4,096 remain;
then assign indices by ascending ASCII spelling. This is a deterministic greedy
objective, not an optimality claim or a psychological guarantee. EFF's original
recognizability and homophone curation supply the human-language foundation.
The selected set has zero distance-one pairs and 153 distance-two pairs, from
2,363 and 26,134 respectively in the filtered candidate universe. The complete
removal trace, exclusions, surviving costs and degrees are published under
`crates/hash-conformance/data/word-id/`.

## Pinned offline tokenizers

Use the `cl100k_base` and `o200k_base` merge ranks and exact pretokenizer
definitions published in OpenAI `tiktoken` 0.12.0, under its MIT license:

<https://github.com/openai/tiktoken/blob/0.12.0/tiktoken_ext/openai_public.py>

The canonical asset URLs and SHA-256 identities in that pinned source are:

| Vocabulary | SHA-256 |
|---|---|
| `https://openaipublic.blob.core.windows.net/encodings/cl100k_base.tiktoken` | `223921b76ee99bde995b7ff738513eef100fb51d18c93597a113bcffe865b2a7` |
| `https://openaipublic.blob.core.windows.net/encodings/o200k_base.tiktoken` | `446a9538cb6c348e3516120d7c08b09f57c36495e2acfffe59a5bf8b0cfb1a2d` |

Analysis used byte-identical cached copies from `tiktoken-rs` 0.12.0,
repository commit `32de8dc0526d67f2c266c4e5e7c6a8ec5a0ce3d7`, and independently
checked the official SHA-256 values. Its original OpenAI MIT grant credits
OpenAI and Shantanu Jain, 2022; the Rust package's MIT grant credits Roger
Zurawicki, 2023. These vocabularies are offline tool inputs and are not linked or
redistributed in runtime conversion. The manifest also gives their BLAKE3
identities, which the analyzer checks before use. Missing or changed inputs fail.

The analyzer is restricted to the canonical ASCII language. In both exact
published regexes, `b3w1:` or `b3p1:` splits into the five pieces `b`, `3`, `w`
or `p`, `1`, `:`. Each subsequent piece is one space plus lowercase letters.
The word separators therefore cannot merge across words. No general Unicode
pretokenizer is claimed. Rank-BPE repeatedly merges the earliest lowest-ranked
adjacent pair, with whole-piece rank hits returned directly. Native XSD base64
decodes the pinned rank-record tokens.

Independent expected token IDs come from OpenAI's public hello/world test and
the published OpenAI Cookbook multi-piece `antidisestablishmentarianism`
example, plus direct complete-token records for header and leading-space
pieces. The cookbook snapshot used has BLAKE3
`f71d11da6f517e0cea5e60d7d5fb186996661f074f727168d1978ba5f968d103`:

<https://github.com/openai/openai-cookbook/blob/main/examples/How_to_count_tokens_with_tiktoken.ipynb>

The independent multi-piece IDs are `[519, 85342, 34500, 479, 8997, 2191]` for
`cl100k_base`, and `[493, 129901, 376, 160388, 21203, 2367]` for `o200k_base`.
This single lowercase-letter piece exercises ranked merging. Published sample
data is read, never executed as Python.

For each payload count one through 22, measurement serializes 4,096 native
references from `BLAKE3(u64_le(index))`, indices zero through 4,095. Counts below
22 use the abbreviated form and its fresh check; count 22 uses full padding and
the full check. Every complete rendered byte string is analyzed, including the
header and check. Consequently only the 16 legal final full-payload indices
occur. Published minima/maxima describe this corpus, not universal token bounds.
