# Issue #1628: Full-text chapter: a Chinese worked search showing tokens, positions and ranked rows

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github
Labels: documentation

## Body

## Problem

The only worked search in the full-text chapter is English (`"quick brown"`,
`docs/book/src/sparql/full-text.md:104`), and the Rust wiring selects the grapheme fallback with
`Analyzer::empty_lexicon()` (`:82`). Chinese segmentation is exercised only in tests
(`crates/text/tests/segmentation.rs:203`), and no example prints tokens or positions. A reader
cannot see how the fallback and a supplied lexicon differ.

## Acceptance

- [ ] A Rust example indexes Chinese text including `南京市长江大桥` and runs a search under the
      grapheme fallback and under a small caller-supplied lexicon.
- [ ] It prints tokens, positions and ranked rows, and asserts the expected output.
- [ ] The book chapter and its Simplified Chinese edition show the same worked search.
- [ ] The example states that the index is a Rust-host facility (`full-text.md:118`).
- [ ] The lexicon in the example is original and its terms are listed.

## Out of scope

Evaluation corpora, ranking-quality measurement and segmentation fixes (open issue on Chinese
analysis and retrieval qualification).


## Comments (0)

