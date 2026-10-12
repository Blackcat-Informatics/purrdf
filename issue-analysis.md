<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# #536 issue analysis

The live issue body and its one comment require two complete fixes on main-derived source: Shapes must compile at the declared MSRV without raising/suppressing the gate, and the original directed numeric endpoint replay must run under `PURRDF_REQUIRE_SIMD_PATHS=numeric:portable,numeric:neon` on actual Arm64. The comment adds the original `purrdf-xsd` library forced-path unit and integration shard. PR #535's hosted failures establish the defects, not any fix; its packaging/tooling patch was rejected and is outside this delivery.

At main `dd79ef77e`, `propagate_sets_with_limit` supplies `&[BTreeSet<usize>]` to `purrdf_core::graph::scc_component_index<E>`, whose `for<'e> &'e E: IntoIterator<Item=&'e usize>` bound is the inference site named by MSRV E0275. Naming `E=BTreeSet<usize>` at that call preserves the same graph, SCC implementation, resource limit, and result.

`FloatProductBackend` already implements `Backend` with portable and NEON paths, and the two original XSD tests already request the `numeric` family. The shared `dispatch::parse` rejects an unknown family before checking a family's path. Main's sorted `FAMILIES` omits `numeric`; adding it is the production contract correction. Preserve parser refusal for unknown paths, actual backend availability, the original product/endpoint vectors and digests, and the forced-path assertions.

The required outcome is a passing affected native check, unchanged full local gate, hosted MSRV 1.98 workspace check with compiler-version evidence, hosted Arm64 replay with both required paths, and the issue-comment's affected library/integration shards, followed by normal PR/hosted/integration. This host's provided nightly and x86 checks cannot substitute for MSRV or Arm execution. No source transfer or check has occurred in this checkout at this review.
