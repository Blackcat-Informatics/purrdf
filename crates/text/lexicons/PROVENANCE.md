<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Baseline lexicon data

The five source files are data from ICU 78.3, commit
`21d1eb0f306e1141c10931e914dfc038c06121da`, under
`icu4c/source/data/brkitr/dictionaries/`. No ICU implementation code is used.
The Rust dictionary engine and artifact generator are independent first-party
implementations, licensed `MIT OR Apache-2.0 OR MulanPSL-2.0`. That expression
does not relicense these source data or their derived artifacts.

| File | Raw entries | SHA-256 of original bytes |
| --- | ---: | --- |
| `cjdict.txt` | 315,964 | `e73fd72048981d0cc13e9dc436a7eaba07ffb6eff58c8a59dc75c1df746663a0` |
| `thaidict.txt` | 26,383 | `3166abde40c0f44ab91c28f5ce96d7d1472cb7882e1c0bda0a72f8f69dba4274` |
| `laodict.txt` | 30,550 | `3c876934a3fa81031d2333525eafaca6a7c9f842e3b98f18c38880420afb5d36` |
| `khmerdict.txt` | 81,028 | `87bee2d17cd5148aa36957eb05409eefc124de8ad519b81b789298ef3e60b5d9` |
| `burmesedict.txt` | 41,120 | `61d8abc3d9102b2f9bf0c9f44db0d7ab89b18172d8cd26832e4c83174bd8673b` |

The source files retain their complete original bytes and headers, including
UTF-8 BOMs where present. Only leading BOM, blank lines and comment lines are
excluded when reading entries. Every remaining entry participates in artifact
generation. CJK costs are preserved; the other four sources receive cost one.

## Terms applying to each artifact

The current ICU contribution layer uses Unicode License V3, reproduced in
`notices/Unicode-3.0.txt`. The following additional notices are copied verbatim
from the respective source headers into the named files under `notices/`.

- **Chinese/Japanese (`cjdict.txt`):** Google BSD terms; two libtabe BSD notices
  (TaBE/Pai-Hsiang Hsiao and Academia Sinica), the Chih-Hao Tsai attribution, and
  IPADIC/NAIST terms including ICOT's distribution conditions and complete
  no-warranty text. Keep all of these notices with the derived artifact.
  ICU's source explicitly records removal of CC-CEDICT-only words to avoid its
  incompatible share-alike terms; no CC-CEDICT data is separately imported here.
- **Thai (`thaidict.txt`):** Unicode terms and the IBM/Apple attributions in the
  original header.
- **Lao (`laodict.txt`):** Unicode terms plus the Brian Eugene Wilson/Robert
  Martin Campbell redistribution conditions and disclaimer. The header names
  the upstream Lao dictionary revision and acknowledgments.
- **Khmer (`khmerdict.txt`):** Unicode terms and the IBM attribution in its header.
- **Myanmar/Burmese (`burmesedict.txt`):** Unicode terms plus the LeRoy Benjamin
  Sharon redistribution conditions, disclaimer, and Myanmar Karen Word Lists
  non-endorsement clause. The upstream source and ICU's modifications are
  recorded in the original header.

These grants permit redistribution and modification while retaining their
notices and disclaimers; their additional attribution and non-endorsement
conditions remain in force. ICOT's stated applicable-law distribution condition
is retained, not paraphrased away. They do not require relicensing the independent
Rust implementation. Distribution alongside any offered implementation-license
choice must still include these separate data terms; do not label the data
itself as triple licensed, Unicode-only, or public domain. No warranty or
endorsement by any upstream contributor is asserted.

The complete controlling upstream inventory is available at
[the pinned ICU license](https://github.com/unicode-org/icu/blob/21d1eb0f306e1141c10931e914dfc038c06121da/LICENSE).
Only the applicable dictionary notices are distributed here; unrelated ICU
implementation component licenses are not evidence that those components were
incorporated.

## Deterministic transformation and identities

`cargo run -p purrdf-text --example gen_lexicons --locked` produces the five
separate `.cbor` artifacts, collision reports and `artifacts/manifest.json`.
Use `-- --check` to verify their bytes. The manifest includes raw BLAKE3 hashes,
source URLs, counts, semantic identities and physical artifact identities.
`src/segment/baseline.rs` contains only generated identity strings, no words.

Generation applies the declared compatibility-caseless normalization and removes
word-internal controls from matching keys. It preserves all source entries via
their canonical key; collisions select the minimum source cost and list every
colliding spelling/cost in the corresponding report. Ordinary caller dictionaries
instead reject conflicting canonical costs. Khmer/Myanmar join controls and
Japanese iteration marks are legitimate source data, not discarded entries.
Burmese punctuation-category words `၏` and `၍` are also retained.

Artifacts contain one UTF-8 arena, monotonically increasing key offsets and costs
in Unicode UTF-8 lexical order. The canonical CBOR layout is
`[law, unicode_version_bytes, arena_text, offsets, costs, semantic_identity]`.
The loader verifies expected physical BLAKE3 identity, shortest CBOR heads, exact
shape, bounds, canonical keys, order, costs and the semantic identity. Neither
runtime representation is serialized: radix and minimal acyclic lookup derive
from identical artifact bytes. Runtime never reads a path or fetches a lexicon;
callers supply the artifact bytes and expected identity explicitly.

Baseline vocabulary and its costs are useful deterministic inputs, not claims of
complete linguistic coverage, calibrated probabilities, or optimal segmentation
for every consumer. Chinese/Japanese frequencies were trained by upstream on its
corpora, as its notice states. Consumer-specific quality is measured separately.

## Separate distribution

`python3 scripts/package-text-lexicons.py` first runs the native generator in
check mode, then writes `target/dist/purrdf-text-lexicons-VERSION.tar.gz`.
`--check` verifies an existing archive against the same complete inventory.
The reproducible archive contains the five physical artifacts, manifest,
collision reports, original source data, all six applicable notice files, this
provenance document and an internal SHA-256 manifest. The release checksum
manifest additionally binds the complete archive alongside the C distribution.
The runtime library embeds only baseline identities, never these dictionaries.

Recipients must retain the notices and disclaimers when redistributing the
source data or derived artifacts, including when repackaging only one dictionary.
The manifest identifies that dictionary's specific notice and the common Unicode
notice. Load selected files as bytes at the host boundary; the library neither
opens archive paths nor obtains missing artifacts automatically. The standard
profile requires all five identities. Explicit empty and caller-dictionary
profiles remain separate, fingerprinted choices.
