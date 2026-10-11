# Issue #518: Optimizer: a ShapeProvider extension for exact physical operators (ordered window, bucketed series, host shapes)

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

A `ShapeProvider` trait the optimizer consults after logical rewriting. Given a subtree, a provider may offer a **physical operator that computes exactly the same rows**. purrdf ships the general shapes:
- dictionary match (#512);
- ordered window (ordinal arithmetic over (sequence, position) as one ordered scan);
- bucketed series (grouping by a typed truncation of time as one ordered pass).

A host (Katamari) registers store-specific ones, such as a merge in segment order.

## Why

Katamari ADR-0045 45.16 and 45.19, and `docs/design/PURRDF-SEAM.md` §2.3 at `e65fe50`. purrdf keeps the only planner (ADR-0045 45.19), so physical knowledge enters through an extension point, not a second planner.

## Contract

- A shape never changes an answer: rows equal literal evaluation, byte for byte, tested by running both.
- A shape that cannot be exact is not a shape; it is an approximate function, labelled as such.
- Evidence ids (the per-row evidence issue) pass through a shape unchanged.

## Acceptance criteria

- For each shipped shape, property tests of shaped against literal evaluation.
- A host-registered shape is consulted and can be refused.
- Determinism holds.


## Comments (0)

