# Issue #584: Requalify capacity source proofs after canonicalization cancellation

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

PurRDF PR583 workspace CI run 39047566280, job 117439266716, fails at helper-census with 80 diagnostics: capacity proof records report stale/duplicate/incomplete source proof and their corresponding logical-capacity-panic sites remain undischarged. The sink sites each match once, and existing records pin constructor identity source:3d088bde0e0c4767. This was not covered by the focused runtime/Clippy checks; no whole-workspace pass was claimed.

The cancellable canonicalization change modifies core's frozen-owner source domain. Determine the exact changed constructor/authority closure from the existing census output and review every relevant mutation/constructor boundary. Requalify still-valid capacity proofs only against the actual current source; repair any newly reachable logical-capacity panic by propagating the typed error. Never suppress the rule, raise a ceiling, replace the reviewed closure with an unpinned proof, or blindly bless sink/authority changes.

Acceptance: retain evidence of old/current closure differences and unchanged or repaired sinks/authorities, run helper-census's source-proof self-tests and complete current workspace helper check, strict affected Clippy and normal hooks; publish and integrate via ghprsq. Preserve root REWORK_LIST.md and unrelated work. This is immediate remediation of the hosted finding, supporting the ongoing GMEOW native-ownership work.


## Comments (0)

