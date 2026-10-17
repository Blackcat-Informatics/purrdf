# Requirement candidates — issue #594

Lines from the issue body and comments that matched a lexical rule, copied
verbatim with the rule that fired. This is a CANDIDATE list: rules both
over-match (a "should" in passing) and under-match (a requirement stated
without a keyword). The issue-analyzer keeps or drops each row, adds what
the rules missed, and accounts for every silent comment. Code fences are
skipped.

| ID | Source | Line | Rules | Text |
|---|---|---|---|---|
| B1 | body | 1 | modal | The prepared native reasoner currently has to be consumed to obtain its proof checking context. A caller retaining one prepared source for repeated decisions cannot borrow that same immutable context for checking; it must give up the reasoner or prepare another one. This obstructs the shared native store boundary required by downstream logic execution. |
| B2 | body | 15 | modal | Prepared verification must reuse the immutable consumer context identity instead of immediately canonicalizing the whole source a second time. Retain the explicit supplied-source verification boundary, with both forms sharing source/question/trust-base binding and the same replay implementation. The actual validation caller must use prepared verification. Tests must compare complete replay reports across both storage and input choices and reject a different prepared source, wrong question, unprepared query context and stopped caller. |
| C2.1 | comment 2 (paudley, 2026-10-17T21:35:58Z) | 19 | modal | Source inspection after the first runtime checks found that validation canonicalizes the complete source in Reasoner::with_proofs and then immediately repeats it in ServiceProof::verify. Add verify_prepared against the sealed immutable consumer context. Factor source/question/trust-base binding into one helper and retain the existing supplied-source check as an explicit validation boundary using the same replay implementation. The production validation caller will use the prepared path. Owning/borrowed and source/prepared paths must return identical reports, with wrong source context, wrong question, tampered proof and stopping receipt still rejected. This eliminates one duplicate complete-source identity computation per actual validation check; no elapsed-time speedup is claimed without measurement. This strengthens the same prepared-store scope and leaves the larger ownership plan open. |

## Comments that matched no rule

- comment 1 (paudley, 2026-10-17T21:23:01Z)
