# Issue #1386: A GTS file's content digest is computed over its terms as written, not over the folded dataset

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Problem

Two forms differ by design where a language tag is not already lower case.

- **The canonical form of an RDF dataset.** RDF 1.2 compares language tags without regard to case, so this repository folds them to ASCII lower case in the term interner (`interned_language`, through `purrdf_iri::langtag::identity_fold`). A dataset holds only the folded tag, and the labelling lowers it as given.
- **The GTS content digest of a file.** The GTS format compares a language text whole and bytewise and never folds. A file holding `"x"@EN-us` and `"x"@en-us` has two terms, two lines and one content digest over those two lines. The labelling text (revision 3.12, section 3.1) states that the fold belongs to the format that feeds the labelling, so it names no case.

Every route from a GTS file into a dataset folds (`import_gts_events`, `import_gts_graph`, `dataset_from_gts_graph`, `flattened_dataset_from_bytes`, all through `RdfDatasetBuilder::intern_literal_parts_admitted`). Two case-variant terms merge into one, and the written spelling is lost. The unfolded tag is not representable in `RdfDataset`, pack, columnar, segmented or embedding storage, and several of those refuse a non-lower-case tag outright. The raw `purrdf_gts` graph and `GtsFoldView` keep it, but neither is a `DatasetView`, so the labelling cannot be run over a file's own terms. Consequences:

- A GTS content digest computed in this repository from a file that holds a non-lower-case tag differs from the one the format's own engine computes.
- The refold certificate digest (`crates/rdf/src/gts_certify.rs`) hashes the folded dataset, so a file with case-variant tags verifies differently here than in a bytewise engine. This is a different digest from the content digest: it is a canonical RDF digest, for which the fold is correct.
- A round trip of such a file through a dataset loses a term.

## Required behaviour

- The labelling can be run over the terms of a GTS file exactly as the file holds them (a view over the raw graph), and the GTS content digest of a file is computed that way. The labelling text and the dataset interner are unchanged.
- The canonical form of an RDF dataset keeps the fold. It equals the GTS content digest exactly when every tag in the file is already lower case. The two are documented as different identities, in the labelling design document and in `docs/MIGRATION-5.0.md`.
- A dataset written to GTS writes the folded tag it holds. Reading a file whose tags are not lower case into a dataset reports the fold in the loss ledger, instead of merging terms silently.

## Acceptance

- The shared vector from the format owner (`terms-language-case-distinct`: one file holding `"x"@EN-us` and `"x"@en-us` as two terms, two lines, one content digest) reproduces byte for byte through the raw-graph view, with no fold.
- A neighbouring all-lower-case file gives the same digest through the raw-graph view and through the dataset path.
- The loss report names the merged term when a file with case-variant tags is read into a dataset.

Part of the 5.0 freeze (#1383): this decides which digest a consumer gets for a GTS file.


## Comments (0)

