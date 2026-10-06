<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# `h3` v4.0 grid-family contract

This document specifies the `h3` v4.0 architecture and its qualification boundary.
The executable native grid is `CubeHilbertQ62V1`. Its keys are native cube keys;
they carry no `h3` equality or compatibility claim. `h3` is a distinct, explicit
family in this design, with no executable `h3` registration implied by the
document. This separation is part of the completed design contract.

## Family-qualified identity and reference

A persisted cell record carries `(family, profile, resolution, key)`. Native
cube and `h3` families cannot compare keys, reinterpret one another's integers,
share an index bucket or reuse cached covers. Their semantic grid identities
bind their mathematical assignment, orientation, encoding and declared
reference. Cover-classifier identities remain separate from point-key laws;
execution limits and implementation receipts remain separate from both.

`h3` uses an icosahedron, face-centered gnomonic projection, aperture-seven
refinement, 122 base cells, twelve pentagons at every resolution, and resolutions
zero through fifteen. Its documented sphere uses the WGS84 authalic radius.
These are spatial-index semantics; ellipsoidal distances still use the shared
metric engine and its declared geographic reference. [Official grid geometry](https://h3geo.org/docs/core-library/overview/)

An `h3`-compatible input profile declares longitude/latitude order and the
matching geographic reference. Provider coordinates and other datums require
an explicit operation chain. A consumer cannot relabel CGCS2000 as WGS84 or
replace ellipsoidal metrics with the `h3` sphere. Mathematical sphere geometry,
physical datum binding and conversion content have distinct identities.

## Independent geometry derivation

The geometric specification is realized from equations: construct a regular
icosahedron from permutations of `(0,±1,±golden_ratio)`, normalize its vertices,
derive face centers and tangent bases, then apply the declared global rigid
orientation. Use certified dot products for face selection and explicit exact
tie ownership. Project a normal onto its face tangent plane through the
face-centered gnomonic map; reverse that map for certified cell boundaries.

Orientation is a fixed mathematical parameter, not a copied face table. A
qualification dossier must bind sufficient published, independently licensed
geometric anchor data to certify its rigid pose and base-cell numbering.
Independently generated face adjacency, transforms, centers and digit maps must
reproduce those anchors and withheld observations. Insufficient anchor precision
or unresolved numbering ties refuse certification; a named orientation alone
does not establish bit-compatible output. No upstream implementation bodies or
coefficient arrays enter the engine or generator.

The aperture-seven integer lattice alternates Class II and Class III
orientation. The base grid is Class II; consecutive resolutions rotate by
about 19.1 degrees with alternating sign. IJK coordinates have a redundant
three-axis representation, and face transitions must preserve a canonical
normalization. Local IJ coordinates require an origin and are not a global
atlas. [Official coordinate systems](https://h3geo.org/docs/core-library/coordsystems/)

Derive the aperture matrices from integer triangular-lattice bases, certify
their determinant magnitude as seven, and derive their rotation from those
matrices. Do not substitute a rounded 19.1-degree rotation. Face-edge transitions
must be derived from common icosahedron vertices, including pentagon distortion
and deleted directions. Prepared immutable chart data can be shared; work,
precision and cancellation state stays in each worker context.

## Cell encoding and validation

For cell mode, the independent packing equation is

```text
key = (1 << 59) | (resolution << 52) | (base_cell << 45)
      | Σ digit_i << (3(15−i)),  i=1..15.
```

The highest bit and bits 58..56 are zero. Resolution occupies four bits and
base cell seven. Used digits are 0..6; unused digits are 7. Resolution is 0..15
and base cell 0..121. Pentagon paths exclude the deleted leading nonzero
direction 1 while the path remains on the pentagon branch. These conditions
are validated before hierarchy operations. [Official cell-mode format](https://h3geo.org/docs/library/index/cell/)

The family decoder also validates the mode before interpreting cell bits.
Directed edges and vertices are different index modes and use distinct typed
records. `h3`'s canonical display string is lowercase hexadecimal without
padding; the store representation is a family-qualified eight-byte big-endian
integer. Zero is invalid. Native fixed-width hex records therefore cannot be
mistaken for `h3` display strings. [Official index modes](https://h3geo.org/docs/core-library/h3Indexing/)

## Variable hierarchy and containment

The common hierarchy interface exposes validated parent/ancestor operations and
bounded children with an explicit count. It cannot assume four or seven
children. A hexagon has seven immediate children; a pentagon has its center
pentagon and five hexagons. For depth d, derive logical descendant counts as
`7^d` for a hexagon and `1+5(7^d−1)/6` for a pentagon. Check these counts and
memory before allocating. Roots and leaves have explicit typed boundary
behavior. The native fixed-four-child API retains its existing meaning.

`h3` parentage is exact logical containment, while its parent polygons only
approximately contain child polygons. Geometric cover classifiers must inspect
the complete target-resolution assignment footprint; replacing a parent by
logical children alone is insufficient proof of physical containment.
Refinement must use the same prepared geometric predicates as scalar queries.
[Official logical and geometric containment](https://h3geo.org/docs/highlights/indexing/)

Coalescing requires the complete valid sibling set, including the pentagon's
six-child case. A compressed logical subtree retains its true descendant count.
Mixed geometric covers count their final emitted cells and preserve their
classifier law. The cube's sentinel-range formula is family-specific. `h3` scans
use validated digit-prefix intervals at a declared stored resolution, removing
invalid and deleted paths; they cannot reuse the cube stride or infer counts
from the width of an integer interval. Parent/child and compaction behavior are
checked against the documented hierarchy operations. [Official hierarchy API](https://h3geo.org/docs/api/hierarchy/)

## Data-only compatibility qualification

Compatibility is established by frozen data with an exact v4.0 producer
identity, never by importing an implementation. Each acquired data artifact
retains its actual license, provenance and digest. Separate source-derived
geometry from independent observations, and keep qualification observations
outside the constant generator's inputs.

A complete receipt records the mathematical law, geographic binding, artifact
hashes, producer version, source/compiler/backend identities and these checks:

- All base cells and every pentagon branch; representative cells at all sixteen
  resolutions, both grid classes, and all face transitions.
- Frozen coordinate-to-key and key-to-boundary observations, including poles,
  longitude aliases, seams, exact ties and close neighbors.
- Independent bit packing, malformed modes/reserved fields/digits, canonical
  strings, parent/child cardinalities and complete sibling compaction.
- Conservatively covered physical searches, separating logical descendant
  membership from geometric containment and applying exact final refinement.
- Identical completed bytes across admitted portable and vector backends;
  typed precision/work/memory/output/cancellation refusals with no partial
  successful result.

Finite observations qualify agreement on their domains, not a proof of global
provider truth. Exhaustive encoding checks and independently certified geometry
establish separate claims. A receipt must identify each claim and its evidence;
native cube qualification cannot supply an `h3` compatibility receipt.
