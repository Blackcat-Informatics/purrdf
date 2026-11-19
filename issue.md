# Issue #1339: Canonical labelling is factorial in like blank siblings; converge on one labelling with the other GTS engine

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github
Labels: enhancement, performance

## Body

## Problem

The blank-node labelling of the canonical profile follows RDFC-1.0: Hash N-Degree Quads tries every order of a group of related blank nodes that share a hash, at every level. The work is factorial in the number of like siblings, and ordinary data reaches it. A dataset with 186,485 blank nodes was reported in which one tree of 246 blank nodes (a node with up to ten blank children that share a first-degree hash and differ only in their subtrees, ten to twelve levels deep) takes nearly all the work: a debug build needed 306.8 million charged units for the dataset, and seconds without that one component. A few more like siblings put such data out of reach. The only protection today is the caller's budget and a typed refusal.

## Goal

One canonical labelling shared with the other GTS engine, before the next minor release, chosen as the best available design and not as the smallest change to RDFC-1.0:

- deterministic and isomorphism-invariant, defined over RDF 1.2 (triple terms, reifiers, annotations) for both presentations;
- the best known work bounds: linear or near-linear on trees, lists, paths and the other shapes ordinary data holds, and the strongest practical method for the symmetric shapes that are hard for every known algorithm;
- builds for `wasm32-unknown-unknown`;
- one specification both engines implement, with shared vectors.

Agreement with RDFC-1.0 is not a requirement here. The profile is already documented as its own form, and RDFC-1.0's labelling is the limitation being removed. Where a design keeps RDFC-1.0's labels for some nodes, that is a property to weigh, not a goal.

How the engine behaves on an input it cannot finish (a budget, a stop signal, a typed refusal) and how its memory is accounted are existing mechanisms of this implementation; the labelling's definition must not depend on them.

## First steps

- Reproduce the reported shape as a synthetic fixture and record instructions and charged units against fan-out and depth.
- Read the two engines' drafts against each other and against the published work on canonical labelling.
- Settle the labelling and its profile version with the maintainers before any implementation.

## Acceptance

To be written with the chosen design. It will include: the reported shape and its synthetic family within a stated polynomial bound; the W3C canonicalization suite; the differential set; shared vectors byte-equal across both engines; and a statement of which existing digests move.


## Comments (0)

