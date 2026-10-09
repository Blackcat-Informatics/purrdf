# Issue #513: text: UAX #29 sentence boundaries beside word boundaries, on the pinned Unicode tables

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

UAX #29 **sentence** boundaries (rules SB1-SB998) in `purrdf-text`, beside the word boundaries it already has (`text/src/unicode.rs`, `word_bounds` and `word_indices`), from the same Unicode 17.0.0 tables (`purrdf_lex::unicode::UNICODE_VERSION`).

## Why

Katamari is taking ownership of embedding (ADR-0046, Proposed). A long literal is embedded as several windows, each with its span, because whole-record vectors blurred multi-topic records in `lillith_memetics`' sprints.

The segmentation rule is part of the embedding space's **identity**, so it must be deterministic, language-independent and pinned:
- sentence boundaries by UAX #29 at a pinned Unicode version;
- sizes and overlap in code points;
- a sentence longer than a window cut at a code-point boundary.

memetics' lab splitter (ASCII `. ! ?` before a space, sized in bytes) was not fit to be identity, which is why it has to be the standard algorithm. The same segmentation serves any family that windows text.

## What it needs

- `sentence_bounds` and `sentence_indices` with the same contract as the word functions, over the workspace's pinned tables.
- Conformance against the Unicode `SentenceBreakTest.txt` for the pinned version.
- Byte-identical output across targets.

## Acceptance criteria

- Passes every `SentenceBreakTest.txt` case for Unicode 17.0.0.
- A version bump is visible: the version is part of what a caller records as identity.


## Comments (0)

