# Issue #516: Loss ledger as queryable RDF 1.2: content-identified reasons, and merging accepted reasons into the answer

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

Render the loss ledger as **RDF 1.2**, queryable by the same evaluator, so a caller can ask:

> "Why did we discard these 100 items? Oh, X and Y. X is valid; give me all the Y entries and merge them with the answer."

Today purrdf's ledger is stable JSON with string kind codes (`rdf-core/src/loss.rs`, surfaced by `cli/src/ledger.rs`).

## Design

Katamari ADR-0045 45.20 and `docs/design/PURRDF-SEAM.md` §3.5 at `e65fe50`. Patrick: "abuse RDF 1.2 maximally".

- **A row's evidence is a reifier** of the solution's triple term. Its standpoint, information state, completeness, reasons and frame are annotations on it.
- **A reason is a reifier of what it concerns**, as a triple term (the constraint, triple or vault that could not be decided), with its cause annotated. Today's loss kinds become causes.
- **Reasons are content-identified.** The IRI is a digest of the canonical description, so 2,279 rows missing the same value share one reason node, and "why these 100?" is a `GROUP BY ?reason` returning a handful of rows with counts.
- **Frames are nodes.** A meet of standpoints or scenarios is identified by the digest of its sorted components.
- **`merge(answer, accepted reasons)`** folds the undecided rows carrying those reasons into the established stream, in order. Each admitted row keeps its reason plus an acceptance annotation, and the acceptance is part of the merged answer's identity: a visible, explicit override.
- **Undecided rows keep their ranks and scores**, so a merge into a ranked or fused answer is an ordered merge, not a re-query.
- **No named graphs.** The ledger is the answer's own graph; reasons are reifiers, not graph names.
- **Lazy.** The interned evidence table (see the per-row evidence issue) is the ledger's dictionary, rendered only when read.

The vocabulary is drafted under a placeholder `kat:` prefix, with GMEOW candidates in Katamari's `docs/design/GMEOW-UPSTREAM.md` rows 12-13.

## Acceptance criteria

- A query with an undecidable filter returns its undecided partition's reasons as RDF 1.2 that the evaluator can query.
- Identical reasons are one node.
- A merge of accepted reasons yields the rows in answer order, each annotated with its acceptance.
- Output is byte-identical across runs and targets.


## Comments (1)

### paudley — 2026-10-09T21:04:51Z

## Addendum: projecting the ledger before it reaches a reader

A reason names, as a triple term, exactly what could not be decided. That can be content a reader may not see, or a vault's existence. So a presenter may need to drop or coarsen ledger annotations before returning an answer.

Because the ledger is RDF 1.2, that projection is an ordinary `CONSTRUCT` over the ledger graph. We ask only that the rendering keep it easy:
- the **fact of incompleteness** stays a separate statement from the **reasons**, so a projection can keep "incomplete, reasons withheld" while dropping every reason node;
- a reason's cause stays a separate statement from its triple term, so a projection can keep the cause and drop what it concerns.

The rule on Katamari's side (`docs/design/PURRDF-SEAM.md` §3.5): a presenter may withhold *why*, never *that* an answer is incomplete within the reader's scope.


