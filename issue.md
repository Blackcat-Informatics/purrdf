# Issue #1393: Canonical labelling lowers nested triple terms to term nodes

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Problem

The canonical labelling admits a triple term inside a triple term and flattens it in place into each line that uses it. A chain of n statements, each naming the triple term of the one before, gives lines whose size is quadratic in n although the input is linear. The format's second generation defines a lowering that replaces each triple term that is a component of another triple term by a term node with one record line, so every cost is linear in the number of distinct nested terms. A 5.0 that labels nested input without the lowering would label it one way and a later release another, so identifiers over nested triple terms would move twice.

## Required behaviour

- The labelling admission lowers nested triple terms by the format's rule: one term node per distinct nested term, one record line per node, the source identifier and source order of the format text, the size, weight and refusal rules of the format text, and lowering from term ids so that no expanded term is formed.
- A dataset with no nested triple term, and a dataset whose triple terms are not nested, lower to themselves line for line and keep every label, position, identity and digest.
- Refusals stay typed, including a collision of two distinct nested terms.

## Acceptance

- The vectors the format owner publishes for the lowering reproduce byte for byte: line lists, sources, digests, positions, `guaranteed` and walk totals. Differences are reported against the passage of the text.
- The three-case statement is shown by the existing labelling vectors reproducing byte for byte after the change: no triple term, triple terms that are not nested, and a triple term inside a triple term.
- A chain of nested triple terms at two or more sizes shows linear retained memory, with requested and retained bytes and allocation counts, and no time measurements.


## Comments (0)

