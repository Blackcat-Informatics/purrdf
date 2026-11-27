# Issue #1450: An anonymous GTS blank node projects under a label a document blank node can spell

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Defect

A GTS blank-node term with no `v` (an anonymous blank node) projects under a label that a document blank node can also spell, so two distinct nodes merge into one.

- `GtsFoldView` renders an anonymous blank node as `_:b{tid}`. A document blank node labelled `b{tid}` in the same graph renders as the same token.
- The dataset paths generate labels of the form `gts_bnode_*` for anonymous blank nodes. A document blank node with the same spelling denotes the same node: a probe holding four distinct subjects projects three quads instead of four.

The projection of a term that states no triple is now un-nameable by any document (an injective `(label, scope)` encoding and a reserved scope). Anonymous blank nodes need the same guarantee.

## Acceptance

- On every path (`GtsFoldView`, `dataset_from_gts_graph`, `flattened_dataset_from_gts_graph`, `import_gts_graph`, `import_gts_events` and its dictionary and blob forms, `export_gts_rdf_nquads_with_dictionary`), an anonymous blank node is a node no document blank node can denote, and every path projects the same graph up to blank-node renaming.
- A test holds a document blank node spelled like each generated form (`b{tid}`, `gts_bnode_*`) beside the anonymous node and asserts both survive as distinct nodes on every path; the neighbouring case without the clash is unchanged.
- MIGRATION-5.0 and the CHANGELOG state any change to emitted labels.

## Evidence

Found by the independent review of the change that made the unbound triple-term node un-nameable; its probe output is kept with that review.


## Comments (1)

### paudley — 2026-11-21T21:18:53Z

Decision: this ships after 5.0. The fix builds on the reserved blank-node scope constants and the injective label encoding that the unbound triple-term change introduces; it changes emitted anonymous blank-node label text, so its CHANGELOG entry states that.


