# Issue #570: Submit W3C SHACL implementation reports (EARL) against the current test suites

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github
Labels: enhancement, conformance

## Body

## Problem

The Data Shapes Working Group lists implementations on its reports pages. Each
page is built from EARL files that implementers submit to
`shacl12-test-suite/reports/` (SHACL 1.2) or `data-shapes-test-suite/reports/`
(SHACL 2017). PurRDF has no report on either page.

PurRDF's own SHACL 1.2 scoreboard is also graded against an outdated suite.
`vectors/shacl12/` is pinned at w3c/data-shapes `5069ed35` (2026-09-24), which is
134 commits behind `gh-pages` (`4cd46986`, 2026-10-16), with 360 changed files
under `shacl12-test-suite/tests/`. The W3C harnesses grade but emit no EARL. Six
approved 1.2 entries are graded by the XSD 1.1 canonical decimal spelling
instead of their approved expectation, so an honest report would show them as
failures.

## Proposed solution

1. Re-vendor `vectors/shacl12/` to the current `gh-pages` commit with
   `scripts/vendor-shacl12.py`, and update the frozen-corpus manifest. Check
   whether `vectors/shacl/` matches the current `data-shapes-test-suite/tests/`,
   and re-vendor it if it doesn't.
2. Make every approved entry pass against its approved expectation. Fix engine
   defects. Where an approved test appears to contradict the specification
   (including the six decimal-spelling cases), raise it with the Working Group
   with a specification rationale. Report it as failed until the WG settles it;
   don't amend the expectation or ledger the test away.
3. Emit EARL from the Rust W3C harnesses (`w3c12_conformance`,
   `w3c_conformance`) through `purrdf_conformance_kit::outcome::to_earl`:
   - one assertion per approved entry, with `earl:automatic` mode;
   - `passed` or `failed` graded against the approved expectation only;
   - `inapplicable` for a conformance class PurRDF does not claim;
   - test IRIs that the WG reports page keys correctly;
   - a test subject with `doap:name` "PurRDF <version>", `doap:homepage`, and a
     `doap:description` naming the PurRDF commit and the suite commit;
   - Patrick Audley as the asserter.
   Output must be byte-deterministic, with no timestamps. Proposed
   (`sht:proposed`) entries are excluded.
4. Generate both reports from a tagged PurRDF release, check them in the WG's
   actual reports page, and prepare the upstream submission PR.

## Acceptance criteria

- [ ] `vectors/shacl12/` (and `vectors/shacl/` if it has drifted) is vendored
      from a recorded current `gh-pages` commit; `make check` and
      `make conformance` pass.
- [ ] Every approved 1.2 and 2017 entry passes against its approved
      expectation, or is reported as failed with a WG issue that gives its
      specification rationale.
- [ ] The harnesses emit deterministic EARL for both suites, and harness tests
      fail if the EARL and scoreboard disagree, in counts or per entry.
- [ ] Both EARL files load in the WG reports pages with no page errors, no
      "no data" rows for approved entries, and counts equal to the scoreboard.
- [ ] Both reports are generated from a tagged release and submitted upstream,
      with the release and suite commit stated.

## References

- [SHACL 1.2 implementation reports](https://w3c.github.io/data-shapes/shacl12-test-suite/reports.html)
- [SHACL test suite: submitting implementation reports](https://w3c.github.io/data-shapes/data-shapes-test-suite/) (section "Submitting Implementation Reports")
- [EARL 1.0 Schema](https://www.w3.org/TR/EARL10-Schema/)
- Related: #384 (community corpus and cross-engine EARL)


## Comments (0)

