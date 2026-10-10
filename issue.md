# Issue #521: Cover law, generalized: identity cover for opaque bytes, a format-neutral emit side, and span references

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

Generalize the cover so **any content**, not only parsed formats, can be addressed by span under the one cover law (`rdf-core/src/cover.rs`, which is already format-neutral on the decode side).

1. **An identity cover for opaque bytes.** A lawful cover with one span over the whole content, for any blob of unknown or unparsed format. Every piece of content then has a cover, so annotations can point at byte ranges of it uniformly.
2. **A format-neutral emit side.** A cover builder that codecs write against: spans, declared continuations and the source digest. MIME, delimited records (CSV, logs, chat transcripts) and future formats then share one emitter as they already share one decoder.
3. **Span addressing as a reusable reference.** A way for *other* statements to refer to a span of covered content (offset and length, or a code-point range, under the cover's identity). Repairs, extracted regions, OCR boxes and embedding windows can then all say exactly which bytes they are about.

## Why

Katamari (ADR-0045 and ADR-0046, Proposed) keeps two things separate:
- **content**, which includes deterministic functions of content such as a cover;
- **judgments about content**, such as a repair, a sniffed type, an OCR result or an embedding window.

A judgment has to point at what it judges. Embedding windows are spans of a literal (ADR-0029 29.22 as amended), repairs are claims about byte ranges of a raw message, and loss-ledger reasons name what could not be decided. One span-addressing scheme over one cover law serves all of them, and is general to RDF tooling.

## Acceptance criteria

- The identity cover of any byte string reconstructs exactly.
- A delimited-record codec written against the emit side round-trips.
- A span reference resolves to exactly its bytes, and is refused when its cover's digest does not match.
- Determinism across targets.


## Comments (0)

