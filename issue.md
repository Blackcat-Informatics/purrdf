# Issue #482: text: rank inputs past QUERY_TERMS_MAX, DOCUMENTS_MAX and FIELD_LENGTH_MAX exactly instead of refusing them

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

`purrdf-text`'s BM25F profile refuses caller data at chosen numbers: a query with more than 1,024 distinct analyzed terms, a corpus of more than 2^40 documents, a merged field longer than 2^24 tokens, a term frequency above 2^24, and more than 16 ranking fields. Each bound exists so that every intermediate fits `i128` and the score fits 56 bits. We ask that the ranking law keep its exact, deterministic arithmetic **without** refusing inputs past these numbers: the bounds become a fast path, not a ceiling.

## Current behaviour (`main` at `6273b6173`)

- `crates/text/src/ranking.rs:26-41`: `MAX_FIELDS = 16`, `QUERY_TERMS_MAX = 1024`, `DOCUMENTS_MAX = 1 << 40`, `FIELD_LENGTH_MAX = 1 << 24`, `TERM_FREQUENCY_MAX = 1 << 24`, `FIELD_WEIGHT_MAX`, `SCORE_MAX = 65_536 * 10^12`, `SCORE_BITS` derived (56).
- Refusal sites: `score.rs:516-520` refuses a needle with more than 1,024 distinct terms (the message hard-codes "1024"); `ranking.rs:363` refuses a corpus over `DOCUMENTS_MAX`; `ranking.rs:397` refuses a query over `QUERY_TERMS_MAX`; `ranking.rs:571` refuses a field length or term frequency over 2^24; `index.rs:826` and `index.rs:1660` refuse a merged field length over 2^24 at index build; `ranking.rs:226` refuses a score outside `[0, SCORE_MAX]`.
- `crates/text/RANKING.md:107-123` gives the reasoning: `ln(2N+2) < 29` for `N <= 2^40`, so the raw score is below `1024 * 29 * 2.2 * 10^12`, and all intermediates stay below `i128::MAX`. It also says the remaining 72 bits of a `u128` are left for a host's tie key, and that purrdf exports no packed comparator.
- All of these numbers are folded into the profile identity (`ranking.rs:237-260`), which is correct and should stay so for whatever replaces them.

## Why it matters

Katamari adopted this ranking law for its full-text family, and the same constants appear in its own ADR-0030 for the same reason (packing score and tie key in one `u128`). Its scale review classified them as defects, not precedents:

- **Queries.** Katamari's query floor is `query(any string)`. A pasted message, a document used as a "find things like this" needle, or an AI composing a long recall query can exceed 1,024 distinct terms. Today that is refused rather than ranked.
- **Fields.** One email body or extracted attachment text can exceed 2^24 tokens (a mailing-list digest, a log, a book). Today the index build refuses the document, so a term in it has no full-text provider at all, which Katamari's retrieval contract treats as a defect (every term always has at least one search provider, and full text is always one of them).
- **Corpus.** 2^40 documents is far away for one mail vault (15 million messages), but Katamari's memory vaults are meant to last decades, and a constant that refuses on growth is the shape of failure we are trying to remove everywhere.
- **Principle.** Katamari's highest-assurance profile permits no chosen limit on what a caller stores or queries, only physical ones. A width that is too narrow is fixed by widening the representation, not by refusing the input.

The packing that motivates `SCORE_BITS` is the host's choice, and Katamari is moving away from it: a variable-length, order-preserving byte key (length-prefixed complemented score, then tie key) keeps a single-comparison order without any bound. So purrdf does not need to protect a 72-bit tie key on our behalf.

## Proposed direction (a sketch)

1. Keep the current `i128` arithmetic as the fast path for inputs inside today's bounds, with byte-identical results.
2. Outside those bounds, compute the same law with the workspace's exact arbitrary-precision numbers (the exact numeric tower of #423, and #477's single big-integer type), applying exactly the same truncation at each operation, so the score is the same mathematical value whichever path computed it.
3. Replace the fixed `SCORE_MAX` with a bound derived from the actual query and corpus (`terms * ln(2N+2) * 2.2`), reported with the result, so a host that wants a fixed-width key can check whether a given answer fits it rather than having inputs refused.
4. Keep the profile identity exact: the identity names the law and the rounding, and no longer depends on a ceiling. If a profile version bump is needed, say so in the changelog.
5. `MAX_FIELDS`: if 16 is a representation choice (a fixed-size array), widen it; if it is a work bound, make it the profile's declared property.

## Acceptance criteria

- A needle with 5,000 distinct terms ranks, and the ranking equals the independent reference (`tests/reference/bm25f.py`, which already uses unbounded integers).
- A document with one field of 2^25 tokens is indexed and scored; the reference agrees.
- A corpus declaring 2^41 documents prepares and scores; the reference agrees.
- For every input inside the current bounds, scores are byte-identical to today's (a golden comparison over the existing conformance corpus).
- No refusal in `purrdf-text` remains whose only cause is one of the constants above. Typed errors remain for genuinely invalid input (negative weights, inconsistent totals).

## Related

#298 (declared these bounds inside the profile identity), #412, #423, #477.


## Comments (0)

