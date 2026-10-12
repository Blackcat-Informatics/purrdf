# Referenced artifacts — issue #536

Everything the issue names, checked against disk. Facts only: whether a
file is a viable port source, or may be relicensed, is decided in the plan.
Paths resolve against the main checkout, then against a sibling checkout
only when that repo is named (first path component, or in the prose just
before the reference). "not here; exists in sibling(s)" is NOT resolved.

## Paths

| Reference | Exists | Resolved in | Kind | Lines | Licence header |
|---|---|---|---|---|---|
| `crates/shapes/src/schema_surface.rs:3462` | yes | repo | file | 6801 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `crates/hash/src/dispatch.rs:285` | yes | repo | file | 467 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `crates/shapes/src/schema_surface.rs` | yes | repo | file | 6801 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |
| `crates/hash/src/dispatch.rs` | yes | repo | file | 467 | SPDX: MIT OR Apache-2.0 OR MulanPSL-2.0 |

### History: `crates/shapes/src/schema_surface.rs:3462`

- `0088bc920 Make datatype ranges and OWL schema coverage exact`
- `c170fce26 fix(shapes)!: every OWL property takes a node or a checked literal (OWL 2 Full floor)`
- `00f20cc81 fix(shapes): the complement of a restriction every individual meets admits no instance`
- `0e07eb3da fix(shapes): an empty qualifier bounds nothing, and an empty range states its cell exactly`
- `fe6916280 fix(shapes)!: read object-property data ranges by OWL 2 Full, and project owl:Nothing as false`
- `5526b23a2 fix(shapes): widen class fillers too, report widened and self-restricted cells as approximations`
- `2c78b616e fix(shapes): cross-kind values are a property-level fact, and cross-kind hasValue and hasSelf are read`
- `97617c43e fix(shapes): an object property restricted to a data range takes its literals`
- `c90227340 fix(shapes): judge string facets exactly, read cross-kind ranges, empty unions`
- `6aa0b4b91 fix(shapes): read a datatype property over a class by OWL 2 Full`

### History: `crates/hash/src/dispatch.rs:285`

- `b8b661d2a refactor: one body per job across the foundation crates`
- `f9757ee7a refactor!: one Backend trait across hash, deflate and CSV kernels; PURRDF_REQUIRE_SIMD_PATHS (#366)`

### History: `crates/shapes/src/schema_surface.rs`

- `0088bc920 Make datatype ranges and OWL schema coverage exact`
- `c170fce26 fix(shapes)!: every OWL property takes a node or a checked literal (OWL 2 Full floor)`
- `00f20cc81 fix(shapes): the complement of a restriction every individual meets admits no instance`
- `0e07eb3da fix(shapes): an empty qualifier bounds nothing, and an empty range states its cell exactly`
- `fe6916280 fix(shapes)!: read object-property data ranges by OWL 2 Full, and project owl:Nothing as false`
- `5526b23a2 fix(shapes): widen class fillers too, report widened and self-restricted cells as approximations`
- `2c78b616e fix(shapes): cross-kind values are a property-level fact, and cross-kind hasValue and hasSelf are read`
- `97617c43e fix(shapes): an object property restricted to a data range takes its literals`
- `c90227340 fix(shapes): judge string facets exactly, read cross-kind ranges, empty unions`
- `6aa0b4b91 fix(shapes): read a datatype property over a class by OWL 2 Full`

### History: `crates/hash/src/dispatch.rs`

- `b8b661d2a refactor: one body per job across the foundation crates`
- `f9757ee7a refactor!: one Backend trait across hash, deflate and CSV kernels; PURRDF_REQUIRE_SIMD_PATHS (#366)`

## Identifiers (files containing each at HEAD)

- `msrv`: 8
- `E0275`: 1
- `scc_component_index`: 5
- `aarch64`: 64
- `directed_endpoint_product_replay`: 2
- `numeric`: 664
- `PURRDF_REQUIRE_SIMD_PATHS`: 16

## Workspace packages named

- `purrdf-shapes`
- `purrdf-xsd`

## URLs

None.
