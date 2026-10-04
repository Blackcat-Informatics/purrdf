<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# HTML character-reference data

The resolver in `../../src/html.rs` and generator in
`../../examples/gen_html_entities.rs` are independently written first-party
code under `MIT OR Apache-2.0 OR MulanPSL-2.0`. Their input is the prose
specification and the separately identified data below. No external decoder or
generator implementation was copied, adapted, translated, imported, or vendored.

## Named machine data

`entities.json` is the exact 145,897-byte output data file from
[WHATWG html-build](https://github.com/whatwg/html-build/blob/283a3531a61106d07d9a7d9fb3e6f3b9bfd33d70/entities/out/entities.json)
at commit `283a3531a61106d07d9a7d9fb3e6f3b9bfd33d70`:

```text
SHA-256 d741d877ac77c4194c4ad526b5b4a19aef8dfe411ab840a466891cdbb9f362e6
```

The upstream entities README identifies this file as checked-in generated
output, sourced from W3C's character entity data and HTML's legacy spellings.
It contains 2,231 spellings, of which 106 omit the semicolon. Every value has
one or two Unicode scalars. On 2026-10-04 its bytes were also verified equal
to `https://html.spec.whatwg.org/entities.json`.

The governing specification is the [HTML Standard snapshot](https://html.spec.whatwg.org/commit-snapshots/a5e15011a00ddefd648c29e4d27734f3e7ff821f/),
commit `a5e15011a00ddefd648c29e4d27734f3e7ff821f` (2026-10-03),
character-reference tokenization states and named character references.

## Numeric recovery data

`numeric-recovery.json` records the 32 outcomes for referenced values
U+0080 through U+009F, in ascending order. It was transcribed as data from
the snapshot's numeric character reference end-state table. The five values
not listed for replacement remain themselves. No executable source was used.

```text
SHA-256 724eb90205cb9cd426c92bba87ca2147b22a3eab4a11dadce6e88056b39b922f
```

## License and attribution

Copyright © WHATWG (Apple, Google, Mozilla, Microsoft).

`LICENSE-WHATWG` is the complete, unmodified upstream license at the same
html-build commit, including its CC-BY-4.0 and BSD-3-Clause texts:

```text
SHA-256 85dc6f5ccb57a6fe8c33d158f9fc8fc7ee5655a5d3db2cdd131c6a3d0f48a864
```

That notice and WHATWG IPR Policy §7.1.1 expressly place portions incorporated
into source code under BSD-3-Clause. These two data inputs are incorporated
into the generated source table `../../src/html/entities.rs`, whose SPDX
identifier is BSD-3-Clause. The data and generated table remain separately
identifiable; they are not relicensed under the first-party three-license
choice. The notice must accompany source and binary recipient distributions.
This uses the explicit source-incorporation grant, not an assumed waiver of
attribution or a claim that the data has no copyright.

## Reproduction and identity

```text
cargo run -p purrdf-lex --example gen_html_entities --locked > /tmp/html-entities.rs
rustfmt --edition 2024 /tmp/html-entities.rs
```

Compare that output byte for byte with `../../src/html/entities.rs`. The
generator parses the pinned JSON through PurRDF's native JSON reader, validates
the exact census, scalar values, legacy counterparts, unique names and compact
offset bounds, then sorts names and writes a flat name arena and fixed-size
entries. It preserves every spelling and value without filtering. The
generated table contains separate BLAKE3 identities of both original data
files; the resolver exports them for analyzer profile identity. No network,
filesystem access, third-party crate, or host-specific layout is used at run
time.
