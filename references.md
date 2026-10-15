# Referenced artifacts — issue #556

Everything the issue names, checked against disk. Facts only: whether a
file is a viable port source, or may be relicensed, is decided in the plan.
Paths resolve against the main checkout, then against a sibling checkout
only when that repo is named (first path component, or in the prose just
before the reference). "not here; exists in sibling(s)" is NOT resolved.

## Paths

| Reference | Exists | Resolved in | Kind | Lines | Licence header |
|---|---|---|---|---|---|
| `crates/katamari-query-purrdf/src/layers.rs` | NO | not here; exists in sibling(s): katamari | — | — | — |
| `crates/katamari-query-purrdf/tests/query.rs` | NO | not here; exists in sibling(s): katamari | — | — | — |

## Identifiers (files containing each at HEAD)

- `SegmentedStack`: 10
- `quads_for_pattern`: 125
- `SegmentedBuilder`: 21
- `SegmentedStack::publish`: 0
- `SegmentedStackSnapshot::query`: 0
- `None`: 2009
- `reifier_subjects::an_ordinary_row_on_another_layers_reifier_is_read_as_an_annotation`: 0
- `a_claim_about_a_claim_reads_back_as_written_from_its_layers`: 0
- `should_panic`: 31

## Workspace packages named

- `purrdf`

## URLs

None.
