# Issue #1382: Public row-set identities and fingerprints are BLAKE3 over sorted row digests

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

Part of #1345 (required behaviours 4 and 6). #1345 stays open until every unit lands.

## Problem

Identities and fingerprints exposed to callers are still derived from additive `Multiset256` sums: the public source content fingerprint and summary values, binding and index generation values, the snapshot identity of resident datasets and its compositions over composite, delta and paged views (which reaches hosts in execution receipts), and the text index source fingerprint.

## Required behaviour

- Every public identity or fingerprint derived from a set of rows is BLAKE3 over the sorted sequence of row digests with a stated domain, computed where every row is already visited. Every v4 signature stays; the values change in 5.0. No parallel API.
- Stored identities and receipts produced by 4.x stop matching; `docs/MIGRATION-5.0.md` and the CHANGELOG say so, with the affected receipts named.
- The additive accumulator is no longer the source of any public value called an identity, digest or fingerprint.
- Settle the unclear cases explicitly in the code and docs: page certification against a hostile provider, and class-membership edge sets taken from an ontology another party supplies.

## Acceptance

- For each changed identity, a test constructs two different row sets with equal additive sum and population and shows the identity now differs, beside a genuinely equal pair that still matches.
- Costs in retired instructions and allocations for the touched hot paths, before and after; no time measurements.
- Frozen goldens that move are updated with the reason stated.


## Comments (0)

