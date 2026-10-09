# Settled additive qualification selection

Execution pending Task2 completion and normal commit. This selection follows actual Makefile and shared WASM runner source, not target names alone.

The issue requires cargo semver-checks, make check and make wasm. The accepted additive comparison is pre-change main aab23cbf2 for changed public core/evaluator APIs, before the separately authorized breaking default adaptation. Run the real installed cargo-semver-checks command against that baseline; no major-release override may conceal an accidental break.

The actual make check recipe ends with make rdf-core-hygiene and make wasm. A successful complete make check with CI=1 therefore includes the real required make wasm invocation and its all-release-crate library build. Retain the nested command output and exit, verify installed wasm32 target, and do not repeat that same build solely to spell the target separately. Missing wasm target must fail under CI=1, never become local SKIP success. The one full workflow execution remains reserved for settled source.

This build does not prove runtime identity. Use the owning portable Rust production-update target on testkit's harness=false runner, if admitted by Task2, natively and on wasm32 through scripts/wasm-test-runner.sh. The existing runner requires the pinned wasm-bindgen CLI, observes module completion/status and rejects a silent return. Preserve real resolver/engine execution, all three physical record roles, nested/CDT identity and governed refusal neighbors rather than a compile-only claim. Existing numerical wasm targets demonstrate the supported invocation pattern; no new runner, semantic feature, dependency, JavaScript fixture growth or duplicated oracle is needed.

Public optimized package tests are a separate host-interface claim. No binding interface is changed by this additive native repair; select existing affected host controls when required by source assessment and hosted gates. make check does not itself invoke wasm-pkg-test: that target belongs to explicit package/release lanes. Never relabel native tests or a wasm library build as package runtime acceptance.

Use the current admitted managed nightly SDK, bounded jobs8/libtest8, private /opt/purrdf-401-qualification target/build/tmp and outer user scope64GiB/Swap0. Required hooks run normally. Record actual commands, terminals, consumers and any failure in validation.md; reuse unaffected Task1 evidence by explicit source assessment. Full portfolio and all remaining deliveries remain active.
