# Settled full-gate qualification

Status: SETTLED FULL LOCAL REQUALIFICATION PASS after the failed initial attempt;
scoped host-golden remediation focused checks PASS. Hosted qualification and
merge remain pending. The initial attempt launched CARGO_BUILD_JOBS=8 make check
at source 9867eba224035eb86c128e3c4a8df97b2c986210.
Actual log: tasks/T6-logs/make-check.log. Root verified unified session 11058
completed with exit 2. The delegated monitor cannot access that parent-owned
session; it observed the failure in the preserved log and uses root's terminal
receipt for the process exit, never a log-tail inference of success.

The sequential gate completed fmt, workspace and downstream strict clippy,
check/build and hygiene/generated/preflight gates, then entered workspace tests.
The C ABI library reported 100 passing tests and one failure:
entail::tests::the_golden_vector_matches. The shared simple-regime vector's
contract hash was stale after the deliberate calculus-version change. The log
ends with Cargo test error and Makefile:172 Error 101; later test targets,
downstream runtime tests, core hygiene and full wasm release gate were not
reached. Compilation does not count as runtime qualification.

No ignored case observed before that failure is counted as passing coverage.
Deliberately running ignored first-party golden generators during remediation
does not establish test qualification. Their actual results, byte audits and
focused host checks belong in tasks/T6-host-golden-remediation.md.

Initial failed evidence remains intact. Root started the settled replacement
gate after the independently reviewed correction was committed as 87d6c3272.
Unified execution session 76482 completed with root-observed exit 0. Its log is
tasks/T6-logs/make-check-settled.log. The command uses CARGO_BUILD_JOBS=8,
BINARYEN_CORES=8 and private build/target/TMPDIR paths under
/opt/purrdf-479-t6-full, with the active nightly SDK bin directory first on PATH.
This supported direct-SDK route keeps the private wasm target directory intact;
the global Stage Cargo wrapper redirects that directory and was correctly
refused by the strict private-artifact validator during focused qualification.
Normal commit hooks remain enabled and used their ordinary configured tools.

The replacement passed the prior C ABI failure, all required workspace runtime
and documentation suites, hygiene/generated and downstream gates, and the full
wasm release library build. The final log line is the successful optimized wasm
release compilation, completed in 5m 07s. Ignored and filtered tests remain
excluded from passing runtime coverage. Separate actual Python and public Node
golden qualification is recorded in the remediation report.

Final independent completion review PASS, with no missing original479 criterion.
The clean integration candidate4c4298e28276eb45c2ae7855898837567693243e passed
59hash library tests, strict all-target hash clippy and wasm hash release build;
root session77193 actualexit0. Only additive BLAKE3 subtree APIs/docs differed;
independent interaction review found no invalidated rules evidence. Its temporary
detached checkout was restored and removed normally after qualification.

PR500 is OPEN and its sole plan published (comment6053263809). stagectl pr-create
reported unparseable output after successful external creation; a subsequent
authoritative branch lookup and PR metadata confirmed500, so creation was not
retried. Initial hosted checks and CodeRabbit are pending; no review threads or
review submissions were present at first complete capture. Stage2 assessment,
hosted qualification, final feedback review, ghprsq integration and cleanup remain
pending. Full local success does not establish those external prerequisites.
