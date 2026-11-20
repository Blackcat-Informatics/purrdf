# Issue #1369: docs/GTS-SPEC.md duplicates the specification that gmeow-gts governs

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## What is wanted

One specification of the GTS format. `AGENTS.md` here states that the GTS wire format is governed in `gmeow-gts`, but this repository carries its own copy of the specification, at an older version, under the same DOI.

## What the documents say now

- `docs/GTS-SPEC.md` here: document version 0.9-draft; "This version" points at a `purrdf-gts` repository; the text says the specification is maintained in that repository.
- `gmeow-gts/docs/GTS-SPEC.md`: document version 1.0-rc1, the same DOI.
- `docs/GTS-CONFORMANCE.md` and the implementer guide exist in both repositories in differing versions.

A reader of this repository cannot tell which text an implementation must follow, or whether `purrdf-gts` is meant to conform to the draft here or to the release candidate there.

## Acceptance

- This repository holds no second copy of the specification, the conformance statement or the implementer guide; each is replaced by a link to `gmeow-gts`.
- The README states which version of the specification `purrdf-gts` implements and where its conformance result against the `gmeow-gts` corpus is recorded.


## Comments (0)

