# Task 4 implementation and qualification status

This is an intermediate source and native-test record, not a Task 4 or #365
completion claim. The source is committed at `ae502bfc1` and synchronized with
main in `728d2d98d`; both commits are pushed. Optimized package acceptance
and final gates remain open.

`GraphPattern::Project` retains its explicit `Vec<Variable>` semantics. Parsed
simple `SELECT *` uses `GraphPattern::ProjectStar`, holding an ordered immutable
`ProjectionVars` snapshot. Append copies one binary-trie path; nested sub-SELECT
boundaries share earlier roots and their `OwnedText` variable spellings. The
shared `projected_names`/`ProjectionNames` view serves both forms. This adds a
public enum variant: downstream exhaustive matches must handle it. It does not
claim source compatibility for those matches.

The parser transfers an already-built `VarScope` through a one-child nested
sub-SELECT instead of reconstructing every enclosing prefix. Its deferred
`EXISTS` scope window uses the existing token stream and the matched opening
brace, so an `EXISTS` after an outer chain does not force inner prefix copies.
The parser keeps explicit projection for grouped and expression cases. RDFLib's
contextual-MINUS visibility uses a generation marker in the existing ordered
scope and publishes at each SELECT boundary. Trailing VALUES and both ordinary
and contextual-MINUS RDFLib fresh-variable routes use the persistent snapshot.
The existing serializer renders the star form as
`SELECT *` through the common projection view; it does not emit each prefix's
names. Algebra walkers, retained-size accounting, validation, substitution,
evaluator execution, SERVICE, property-function, reasoning and binding
consumers have been adapted to the two forms.

The initial cap-only candidate failed its unchanged accepted-neighbor work
control at 4.22× and 4.11×; see `T4-admission-diagnostic.md`. The first
persistent implementation failed the outer-`EXISTS` allocator control at
approximately 4.1×; the scoped-token correction passed at about 2× with
the unchanged 2k/4k/8k query; see `T4-outer-exists-diagnostic.md`. Both
failures remain retained evidence.

The optimized native algebra library completed 474 tests and the original
2k/4k/8k plain, inner-`EXISTS`, outer-`EXISTS`, carrier allocation and
8k parse/serialize/parse controls passed in the current algebra sweep. The
selected evaluator nested-star answer parity test passed. The full algebra
library and integration sweep passed. A subsequent same-thread counting
allocator control passed: the clone retained live bytes after dropping the
original query, and dropping the final clone returned the window's retained
bytes to zero. The final affected algebra rerun passed 474 library tests,
the nine chain controls, nine scoped-carrier tests and four serializer sweep
tests after the saved-empty projection correction. Subsequent public-route
corrections passed 2,000/4,000/8,000 accepted variable-work counts: ordinary
6,000/12,000/24,000, inner EXISTS 6,006/12,006/24,006, trailing VALUES
10,000/20,000/40,000 and RDFLib 6,000/12,000/24,000. Trailing VALUES
parse and carrier allocation, source-byte growth and roundtrip pass below 3x
at all doublings; `T4-public-routes-diagnostic.md` keeps the failed earlier
observations. The full algebra suite passed on the parser correction, and
the final carrier controls passed on the later serializer correction. Seven
affected packages passed all-target strict Clippy on the earlier source with
the identical current-main geo lint annotation applied temporarily; that
annotation is not part of this task commit and will arrive through normal
base synchronization.
Repeated contextual-MINUS subselect projections now pass the native parser,
variable-work and carrier growth controls. A later independent review found
shared-prefix and FILTER-wrapped MINUS carrier counterexamples. The shared
renderer now uses a single outer-snapshot position table to verify local
additions; 2k/4k/8k roundtrips, emitted bytes and carrier requested/peak bytes
pass below 3x for both shapes. These findings and measurements are retained in
`T4-public-routes-diagnostic.md`. The final affected evaluator run passed
1,611 library tests plus 3 nested-subselect, 4 source-header and 17 union
integration tests. Current-source helper census passed all 81 jobs and 118
distinct groups. The serializer current-BIND overlap passes an actual
2k/4k/8k saved-name visit census at 21,999 / 43,999 / 87,999. Current-source
seven-package all-target strict Clippy passed. The final focused algebra rerun
passed. The final-source optimized WASM module was built through standard
Cargo artifact registration, bindgen 0.2.125, Binaryen 130 and wasm-link,
without a private compiler target or PATH override. Its SHA-256 is
`ebf1f81d42eb2ad4bb6f504868629d8138fa7ecae4d0d27e5c79ce4895c39109`;
the final module has 415,593 SIMD opcode lines. Public `Dataset.query` returns
one row and complete ordered names at 10/20/40/80 nested-star levels. It
returns typed host-stack refusals at 160/320/640 and typed evaluation-stack
refusals at 2k/4k/8k; the 8k process reaches 194,824 KiB max RSS on a
349,775-byte input and does not trap. A healthy query succeeds after the 4k
refusal. The optimized package's complete Node suite has 450 passes and two
historical #531 stack-floor assertion failures (LATERAL admits only 248
levels versus the required 281-floor; FILTER NOT EXISTS admits only 142
versus the required 180-floor). Installed npm smoke and typecheck pass.
The full repository gate is running; #365 publication remains open.
No timing exclusivity is requested or used.
