# Issue #1394: Canonical labelling requests bytes quadratic in the length of a path

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Problem

On an input whose lines form a path, such as a chain of nested triple terms after the term-node lowering, the canonical labelling's requested bytes grow quadratically in the path length while its allocation count, retained bytes, peak bytes and walk units stay linear. Measured on the lowering branch with `purrdf_alloc_probe`: n = 1,000 requested 22.7 MB, n = 10,000 requested 1.31 GB, n = 100,000 requested 121 GB, against peak 3.37 MB, 34.0 MB and 351.6 MB. Main shows the same requested bytes as the walk branch on the 1,000-node path bench, so the cost is in the labelling engine and not in the lowering. Requested bytes that grow quadratically are work that grows quadratically: the engine reallocates or copies state per step whose size is proportional to the path.

## Required behaviour

- Labelling a path of n lines requests bytes linear in n, with labels, positions, digests and walk totals unchanged.

## Acceptance

- Requested bytes, allocation count and retained bytes at three or more path lengths, before and after, from `purrdf_alloc_probe`, showing linear requested bytes after the change; retired instructions on the existing labelling bench shapes not worse.
- Every labelling vector, worked example and differential test reproduces byte for byte.


## Comments (0)

