<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# PurRDF Brand

## Name

The project name is **PurRDF** — a cat pun on *purr* + *RDF*. The shared `R`
does double duty: `Pur` + `RDF`, pronounced "purr-D-F". Always write it as
`PurRDF` in prose and titles; never `PurrDF`, `PURRDF`, or `Purrdf`.

All machine identifiers stay lowercase `purrdf`: the crates.io crate family
(`purrdf`, `purrdf-core`, `purrdf-gts`, …), the PyPI package, the npm/ESM
package (`@blackcatinformatics/purrdf`), and the C library (`libpurrdf`).

## Positioning

PurRDF should read as fast, rigorous, dependable data infrastructure — the
RDF 1.2 carrier layer that other systems (including the GMEOW stack) build
on — rather than as an application or a framework. It is deliberately
optionality-free: one behavior, every consumer, every language.

## Tagline

The RDF 1.2 toolkit with a purr: primitives, codecs, SPARQL, SHACL, and
graph transport.

## Repository description

PurRDF — a fast, dependency-light Rust toolkit for RDF 1.2: interned
primitives, Turtle/TriG/N-Triples/N-Quads/RDF/XML/JSON-LD codecs, RDFC-1.0
canonicalization, SPARQL 1.1/1.2, SHACL Core validation, the GTS
append-only graph transport, and Python/WebAssembly/C bindings.

## Family system

PurRDF shares the black-cat silhouette of the "g + cat-sound" family (see
`gmeow-ontology/docs/BRAND.md`). Keep the family recognizable by reusing the
shared `cat-head-core` SVG group verbatim and swapping only the **service
object** held by the cat. This repository's service object is
`service-triple` — an **RDF triple**: a subject node bitten by the cat,
bound to an object node by a directed blue predicate arrow, with a literal
frame below bound by red and green predicate arrows and a yellow datatype
dot. It replaces `gmeow-ontology`'s `service-graph` and `gmeow-gts`'s
`service-chain`, and deliberately reuses their visual vocabulary: the round
paper nodes of the ontology graph and the paper frame of the GTS log.

The four accent colours (red `#ea4335`, blue `#4285f4`, yellow `#fbbc05`,
green `#34a853`) are isolated to the `service-triple` group for theming.

## Colour tokens

- cat / ink: `#111214`
- paper (nodes, frames): `#fffdf5`
- feature (eyes, whiskers): `#ffffff`
- accents: red `#ea4335`, blue `#4285f4`, yellow `#fbbc05`, green `#34a853`

## Logo assets

- `docs/purrdf-logo.svg` — the canonical PurRDF logo (cat + RDF triple),
  including a soft white glow so the black silhouette remains legible on
  dark backgrounds.
- `docs/social-preview.svg` — the editable GitHub sharing-card source
  (1280×640).
- `docs/social-preview.png` — the rendered 1280×640 GitHub social preview.

Use the SVG for README, icon, and card placements. The README references
this asset by relative path so branch previews render the branch's current
logo.

Rebuild the PNG after editing the SVG:

```bash
rsvg-convert -w 1280 -h 640 docs/social-preview.svg -o docs/social-preview.png
```

GitHub repository social previews are uploaded in **repository Settings →
Social preview** (there is no API for this). Upload
`docs/social-preview.png` there.
