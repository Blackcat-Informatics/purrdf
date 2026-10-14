# Borrowed typed error causes

The unchanged census previously rejected six identical handwritten Error::source
bodies. One lexical error_source! declaration now owns that implementation;
the six original callers name their Storage variant through it. All other
variants continue to have no cause. Display, Debug, conversion, concrete types,
layouts and allocation behavior are unchanged. No checker, ledger, tooling,
build configuration or dependency changed.

The public lexical doctest ran through the provided Cargo under a4GiB/no-swap
scope, default directories, one build job: terminal0, one test passes. It verifies
typed downcasting of the original StorageError and None for a nonwrapper variant.
Normal formatting and diff checks pass.

Affected strict library compilation ran through provided Cargo under the same
4GiB scope: lex, columnar, conformance-kit, core, datalog, validate and xsd,
--locked --lib and -D warnings. Terminal0 at2m08; full receipt is
error-source-sharing-strict.log. This is actual propagation/compiler evidence,
not complete capacity or sparse-storage acceptance.

Independent source review passes in
../reviews/error-source-sharing-independent.md. The added lexical module guide
is a public link to the shared macro; it changes no behavior.

Unchanged make helpers-hygiene session98264 is terminal2. Its original self-tests
pass; the actual check no longer reports the six-source body group. Five other
previously failing groups remain: handle pinning, resident relation open,
segmented workspace reservation, nesting certificate and quoted-only capabilities.
The first bounded consolidation is therefore verified at its actual scope;
the broad census still fails. No normal commit/publication or whole S1 pass is
claimed. Full receipt is error-source-sharing-census.log.
