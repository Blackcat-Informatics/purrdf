# Requirements ledger: issue #570: Submit W3C SHACL implementation reports (EARL) against the current test suites

Evidence: issue.md, requirements-candidates.md, requirements.json (machine copy of the same six candidates, cross-checked), brief.json (coverage: issue #570 + 0 comment(s) via github; link crawl: 4 issue(s) within 2 hop(s) of `#N` references; forge search on ['Submit', 'SHACL', 'implementation']: 9 other item(s); GhpRsq-Defect-Class trailers: 5 distinct class(es) in the last 200 commits; prior attempts: 0 commit(s) citing #570 (second parents walked), 0 other branch(es) named for it; PRs citing #570: 0; requirement candidates: 6 line(s); 0 comment(s) matched no rule; referenced artifacts: 7 path(s) (3 on disk), 6 identifier(s), 0 workspace package(s), 3 URL(s))

Line convention: "body L<n>" is the line number within the issue body, matching requirements-candidates.md (body L1 = issue.md L8, so body L<n> = issue.md L<n+7>).

## Requirements
| R-id | Kind | Source | Quote (verbatim) | From candidate |
|------|------|--------|------------------|----------------|
| R1 | list-item | body L18 (Proposed solution step 1) | "Re-vendor `vectors/shacl12/` to the current `gh-pages` commit with" | prose |
| R2 | list-item | body L19 (step 1, line 26) | "and update the frozen-corpus manifest. Check" | prose |
| R3 | list-item | body L20 (step 1, line 27) | "whether `vectors/shacl/` matches the current `data-shapes-test-suite/tests/`," | prose |
| R4 | list-item | body L21 (step 1) | "and re-vendor it if it doesn't." | prose |
| R5 | list-item | body L22 (step 2) | "Make every approved entry pass against its approved expectation." | prose |
| R6 | list-item | body L22-L23 (step 2, line-wrapped: "Fix engine" / "defects.") | "Fix engine defects." | prose |
| R7 | list-item | body L23 (step 2) | "Where an approved test appears to contradict the specification" | prose |
| R8 | list-item | body L24 (step 2) | "(including the six decimal-spelling cases), raise it with the Working Group" | prose |
| R9 | list-item | body L25 (step 2) | "with a specification rationale. Report it as failed until the WG settles it;" | prose |
| R10 | list-item | body L26 (step 2) | "don't amend the expectation or ledger the test away." | prose |
| R11 | list-item | body L27 (step 3) | "Emit EARL from the Rust W3C harnesses (`w3c12_conformance`," | prose |
| R12 | list-item | body L28 (step 3) | "`w3c_conformance`) through `purrdf_conformance_kit::outcome::to_earl`:" | prose |
| R13 | list-item | body L29 (step 3) | "one assertion per approved entry, with `earl:automatic` mode;" | prose |
| R14 | list-item | body L30 (step 3) | "`passed` or `failed` graded against the approved expectation only;" | prose |
| R15 | list-item | body L31 (step 3) | "`inapplicable` for a conformance class PurRDF does not claim;" | prose |
| R16 | list-item | body L32 (step 3) | "test IRIs that the WG reports page keys correctly;" | prose |
| R17 | list-item | body L33 (step 3) | "a test subject with `doap:name` "PurRDF <version>", `doap:homepage`, and a" | prose |
| R18 | list-item | body L34 (step 3) | "`doap:description` naming the PurRDF commit and the suite commit;" | prose |
| R19 | list-item | body L35 (step 3) | "Patrick Audley as the asserter." | prose |
| R20 | modal | body L36 (step 3) | "Output must be byte-deterministic, with no timestamps." | B1 |
| R21 | scope-out | body L37 (step 3, continues from L36) | "(`sht:proposed`) entries are excluded." | B1 |
| R22 | list-item | body L38 (step 4) | "Generate both reports from a tagged PurRDF release, check them in the WG's" | prose |
| R23 | list-item | body L39 (step 4) | "actual reports page, and prepare the upstream submission PR." | prose |
| R24 | checkbox | body L43 (Acceptance) | "- [ ] `vectors/shacl12/` (and `vectors/shacl/` if it has drifted) is vendored" | B2 |
| R25 | checkbox | body L44 (Acceptance, continues R24) | "from a recorded current `gh-pages` commit; `make check` and" | B2 |
| R26 | checkbox | body L45 (Acceptance, continues R24) | "`make conformance` pass." | B2 |
| R27 | checkbox | body L46 (Acceptance) | "- [ ] Every approved 1.2 and 2017 entry passes against its approved" | B3 |
| R28 | checkbox | body L47 (Acceptance, continues R27) | "expectation, or is reported as failed with a WG issue that gives its" | B3 |
| R29 | checkbox | body L48 (Acceptance, continues R27) | "specification rationale." | B3 |
| R30 | checkbox | body L49 (Acceptance) | "- [ ] The harnesses emit deterministic EARL for both suites, and harness tests" | B4 |
| R31 | checkbox | body L50 (Acceptance, continues R30) | "fail if the EARL and scoreboard disagree, in counts or per entry." | B4 |
| R32 | checkbox | body L51 (Acceptance) | "- [ ] Both EARL files load in the WG reports pages with no page errors, no" | B5 |
| R33 | checkbox | body L52 (Acceptance, continues R32) | ""no data" rows for approved entries, and counts equal to the scoreboard." | B5 |
| R34 | checkbox | body L53 (Acceptance) | "- [ ] Both reports are generated from a tagged release and submitted upstream," | B6 |
| R35 | checkbox | body L54 (Acceptance, continues R34) | "with the release and suite commit stated." | B6 |
| R36 | dependency | body L61 (References/Related) | "- Related: #384 (community corpus and cross-engine EARL)" | prose |

Kinds: checkbox, modal, list-item, scope-out, dependency, question, decision, prose.

Not ledgered (background, no behavior, artifact, constraint, or check asked for): body L1-L17 Problem paragraphs (state facts about the current pin, the gap, and the six decimal-spelling entries; their requirement content is carried by R1-R5 and NEEDS-JUDGMENT), body L56-L60 References, and the header (labels, state, repo). No sentence in the issue asks for a behavior that the rows above miss.

## Candidate dispositions
| Candidate | Disposition |
|-----------|-------------|
| B1 | kept → R20, R21 |
| B2 | kept → R24, R25, R26 |
| B3 | kept → R27, R28, R29 |
| B4 | kept → R30, R31 |
| B5 | kept → R32, R33 |
| B6 | kept → R34, R35 |

No candidates dropped (0). The six candidates in requirements.json match the six in requirements-candidates.md.

## Comment coverage
| Comment | Author | Rows / disposition |
|---------|--------|--------------------|
| (none) | - | Issue has 0 comments (`## Comments (0)`); nothing to cover. |

## Dependencies (from brief.json relations)
- #384 mentions [OPEN]
- #382 2-hop [CLOSED]
- #383 2-hop [state not recorded in brief.json]
- #385 2-hop [state not recorded in brief.json]
- Also listed in brief.json "related" with no recorded relation (not ledgered as dependencies): #490 [CLOSED], #387 [CLOSED], #323 [CLOSED], #375 [CLOSED], #130 [CLOSED], #339 [CLOSED], #330 [CLOSED]
- Body names #384 as "Related" (R36).

## NEEDS-JUDGMENT
1. R5 vs R9 (and R28): "Make every approved entry pass against its approved expectation." (body L22) / "Report it as failed until the WG settles it;" (body L25), and "expectation, or is reported as failed with a WG issue that gives its" (body L47): the issue does not say whether a failing approved entry must be made to pass or may be reported failed pending the WG; which governs is the question.
2. R6 vs R8: "Fix engine defects." (body L22-L23) / "(including the six decimal-spelling cases), raise it with the Working Group" (body L24), against "approved 1.2 entries are graded by the XSD 1.1 canonical decimal spelling" (body L12): whether the six decimal-spelling cases are engine defects to fix or specification contradictions to raise with the WG. Also unstated: which defects "Fix engine defects" covers.
3. R23 vs R34: "actual reports page, and prepare the upstream submission PR." (body L39) / "- [ ] Both reports are generated from a tagged release and submitted upstream," (body L53): whether the acceptance criterion requires an actual upstream submission or only a prepared PR.
4. R22: "check them in the WG's" (body L38): the meaning of "check them in" (verify on the page, or commit to the WG repository) is not stated.
5. R15: "`inapplicable` for a conformance class PurRDF does not claim;" (body L31): the issue does not list which conformance classes PurRDF claims.
6. R16: "test IRIs that the WG reports page keys correctly;" (body L32): the criterion for "correctly" is not stated.
7. R17 / R22: "PurRDF <version>" (body L33) and "a tagged PurRDF release" (body L38): no version or tag is named.
8. R18 vs R25: "`doap:description` naming the PurRDF commit and the suite commit;" (body L34) / "from a recorded current `gh-pages` commit; `make check` and" (body L44), against "(`4cd46986`, 2026-10-16)" (body L10): which suite commit the description names (the pinned 5069ed35 or the current gh-pages commit), and whether "current" refers to 4cd46986 or to a later commit.
9. R31 / R33: "counts equal to the scoreboard." (body L52) and "fail if the EARL and scoreboard disagree" (body L50): the only scoreboard named is "PurRDF's own SHACL 1.2 scoreboard" (body L8); the issue names no 2017 scoreboard, although R32 requires both EARL files.

## Self-check
Quotes checked: 36 R-row quotes (plus 6 NEEDS-JUDGMENT quotes). Misses: 0. One R-row quote (R6, "Fix engine" / "defects.") is line-wrapped and was matched on a whitespace-joined grep; the other 35 match issue.md per line. Comments covered: 0/0.
