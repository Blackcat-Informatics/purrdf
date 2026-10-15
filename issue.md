# Issue #556: Segmented stack reads an ordinary quad as an annotation when its subject is a reifier in another layer

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

In a `SegmentedStack`, an ordinary quad whose subject is the reifier of a reifier row held in a
*different* layer is returned as an annotation of that reifier: `quads()` and
`quads_for_pattern` do not yield it, `annotation_quads()` does, and the view records no read
error. The same two rows sealed into *one* image read back as written (one ordinary quad, one
reifier, no annotation). So how rows are split across layers changes the answer.

## Reproduction (purrdf `0d075201f`)

Two images built with `SegmentedBuilder` (build limits sized to the rows; 4096-byte blocks,
32 records), stacked oldest first with `SegmentedStack::publish`, queried through
`SegmentedStackSnapshot::query`:

- image A: `push_reifier(<x>, <<( <ts> <tp> <to> )>>, None)`
- images B1, B2: `push_quad(<x> <later> <a>)`, `push_quad(<x> <later> <b>)`, one each

| stack | `quads()` | `quads_for_pattern(<x>, <later>, _)` | `reifier_quads()` | `annotation_quads()` |
|---|---:|---:|---:|---:|
| [A, B1, B2] | 0 | 0 | -- | 2 |
| [B1, B2, A] | 0 | -- | 1 | -- |
| [B1, B2] | 2 | -- | -- | -- |

and in one image, `<x>`'s reifier row with `push_quad(<x> <p> <a>)`: `quads()` 1,
`reifier_quads()` 1, `annotation_quads()` 0.

Over A and one image `push_quad(<x> <p> <a>)`, `quads()` is 0; with A's reifier subject
changed to `<r>`, it is 1. `read_error()` after the walks over [A, B1, B2] is `None`.

Katamari's reproductions:
`crates/katamari-query-purrdf/src/layers.rs` `reifier_subjects::an_ordinary_row_on_another_layers_reifier_is_read_as_an_annotation`
and, end to end, `crates/katamari-query-purrdf/tests/query.rs`
`a_claim_about_a_claim_reads_back_as_written_from_its_layers` (both pinned `should_panic`
until this is fixed).

## Impact

A claim about a claim asserts a triple whose subject is the earlier claim's reifier. Persisted
as layers, that triple disappears from the ordinary stream and appears as an annotation, so a
persisted graph's answers differ from the same data in one dataset. Katamari cannot persist
its graph as segmented layers for a vault holding such claims until the stack classifies rows
the same way whatever the layering.

## Expected

A stack answers as the union of its layers' rows (less removals) would in one image: an
ordinary quad stays ordinary regardless of which layer holds a reifier row for its subject.


## Comments (0)

