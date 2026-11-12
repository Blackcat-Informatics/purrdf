<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# GB 18030 mapping sources

The streaming codec and Rust generator are independently written first-party
implementations. No ICU converter or generator code is incorporated. The
licensed mapping data below, edition changes and normative arithmetic supply
the mapping law. Generated data retains the original grants and notices.

## Original enumeration

`gbkuni30.txt` is byte-exact ICU-data mapping data, Git blob
`ff332150043bcb0e4a9beae2b54153230189f22f`, from
[`charset/source/gb18030/gbkuni30.txt`](https://github.com/unicode-org/icu-data/blob/main/charset/source/gb18030/gbkuni30.txt).
It has 240,424 bytes and 24,068 rows: 128 ASCII mappings and 23,940
two-byte mappings. Its SHA-256 is
`db5cd60503b038effcfcc48ebacd0edb47a898820d5cd24d406e027431578996`.

The associated `gb18030-source.html`, original Git blob
`5a23356ba6e9c89a4a0de9a2969f429a68370fcb`, describes the November 30,
2000 agency mapping release and its BMP complement enumeration. The generator
enumerates the 39,420 remaining BMP scalars once, excluding surrogates, before
any edition override. Supplementary scalars map arithmetically from
`90308130` at U+10000 through `E3329A35` at U+10FFFF. Rebuilding the
complement after changing two-byte assignments would change unrelated mappings.

`LICENSE-ICU-DATA` is the associated original license, Git blob
`7a7aef70e263aa5bd6cb88d3d1bd5d08e4119744`, SHA-256
`f72db45707e93c90c4bf400edfa5d38ce44fe692789e385548e627459c1bf6c9`.
It preserves Unicode-3.0 permission and historical IBM/ICU notices. Original
source attribution includes Copyright 2000–2007 International Business
Machines Corporation and others, and Unicode, Inc.

## Independent 2005 implementation reference

`gb18030-2005.ucm` is ICU release 71.1 mapping data, Git blob
`62b57a93fc24ccb400f75d6d7c02c5e2852382b5`, from
[`icu4c/source/data/mappings/gb18030.ucm`](https://github.com/unicode-org/icu/blob/release-71-1/icu4c/source/data/mappings/gb18030.ucm).
Its SHA-256 is
`4679b7c820a754eb0a0bf6873678ca67546a8a8b986ea6610da2b7a7d758ebb2`.
Its 30,862 explicit reversible mappings independently qualify the base table,
BMP enumeration and reciprocal 2005 change: `A8BC` maps U+1E3F and
`8135F437` maps U+E7C7. The file documents that change in its state-table
comment. Algorithmic ranges absent from the file are qualified separately.

`LICENSE-ICU-71` is the original associated `icu4c/LICENSE`, Git blob
`80b587723a67f7b979c3c0ad196748cf8e6f111e`, SHA-256
`af3e84c401f1a35e8d32d6eb1a33fe587c3981aa5cd206033d9527c1855b57a2`.
It retains the Unicode data/software permission and historical IBM/ICU grant.
This file is an implementer reference, not the official text of the standard.

## Current edition and amendment

[Unicode L2/22-274R](https://www.unicode.org/L2/L2022/22274r-disruptive-changes.pdf),
revised December 9, 2022, reproduces all 36 reciprocal GB 18030-2022
Annex D.1 mapping changes in Annex A. Its SHA-256 is
`a415991344a83fe117598a1c0ec7d2ca3a46a01facd2ea5ed46b0e7af05bae1f`.
The generator applies both directions of every pair. For example, `FE59`
becomes U+9FB4 while `82359037` becomes U+E81E. Glyph removals for six PUA
assignments and nine compatibility ideographs do not remove their Unicode
mappings and do not authorize normalization.

The official 54-page
[Amendment 1](https://std.samr.gov.cn/dcpspTools/gbPlan/download50?buzCode=2022004989)
has SHA-256
`001d45d8445b3aa2092113e0617bcfc237a0a911fc955dbd99735087b6eb7519`.
Item 3 preserves supplementary arithmetic. Item 7 explicitly maps
`8139A332`–`8139A335` to U+2FFC–U+2FFF and `8139BC33` to U+31EF;
these agree with the unchanged original enumeration. Added assigned Han and
stroke characters likewise retain the arithmetic mapping of their scalars.
Item 5 permits grouped 2005 compatibility for the 18 adjusted and 15 deleted
characters, excluding ordinary input methods. The codec exposes the complete
2005 mapping as a separately selected legacy profile; it never silently mixes
that permission into the current profile.

The official base-standard metadata page was inspected, but the full base
standard PDF was not obtained. Qualification therefore combines the original
licensed agency-data enumeration as described by ICU, an independently pinned
pre-2022 implementation mapping, the reproduced normative reciprocal changes,
and the official amendment. The PDFs are reference evidence, not redistributed
source material. This distinction is part of the provenance claim.

Current ICU/web tables are not adopted: some accept byte `80` as Euro,
map `A3A0` to U+3000, or keep old four-byte mappings as aliases. These are not
the strict reciprocal profiles implemented here.

## Reproduction

Run `cargo run --release -p purrdf-lex --example gen_gb18030` and format its
output with the managed `rustfmt --edition 2024`. The generator verifies both
mapping input Git object identities before generating the tables. Native
controls compare all independent reference rows, all Unicode scalars in both
profiles, split-independent source contributions and refusal boundaries.
