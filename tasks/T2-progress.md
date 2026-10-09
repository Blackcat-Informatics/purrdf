Task 2 implements the original Rust profiling controller over the existing native
Make/shard/C/downstream commands. Cold admission checks both target and build
directories; warm admission requires an unchanged successful prior arm. Comparison
refuses failed/missing/mismatched source, tool, configuration, concurrency, cache
declaration, retained evidence or target coverage.

Cargo remains freshness authority on every invocation. Current timing reports
are captured per actual child, preserving inherited reports and preventing nested
C builds from being attributed twice. The narrow counterfactual permits only
the already-merged nested dev-to-test profile correction. Results distinguish
aggregate work from hosted critical path and declared from observed cache state.

Focused checks passed: 13 controller/shared-helper tests, the real five-test C
smoke with all16 phases, all-target clippy, shard self-test/live inventory,
formatting and an actual executable Cargo delegation/timing seam. Independent
Task2 review PASS. No cold/warm campaign or speedup is claimed; hosted wiring,
measured reduction and full qualification remain Tasks3–5.

Normal verification hooks passed; `eb875cb8c` committed and pushed.
