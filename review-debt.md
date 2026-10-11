# PR 532 review-debt assessment

VERDICT: PASS for the user-authorized revised shipping disposition. Original
technical qualification remains FAIL/incomplete; hosted execution/review is
not complete and is not represented as passing.

Reviewed PR [532](https://github.com/Blackcat-Informatics/purrdf/pull/532),
published head `5e1f3658a`, using the supplied current `pr-checks.txt`,
`pr-comments.md`, `pr-review-submissions.json`, `pr-inline-comments.json` and
`pr-review-threads.json`. Applied the Stage 3 distinct review-debt requirement.
No source suite, source edit, forge posting or merge was performed by this audit.

| Review surface | Captured observation | Disposition |
|---|---|---|
| General PR comments | One CodeRabbit processing notification, no substantive finding | Informational notice; no code change or reply needed, no completed review claimed |
| Review submissions | Empty array | No submitted approval or changes-requested review; not an approval claim |
| Inline review comments | Empty array | No captured inline finding to adjudicate |
| GraphQL review threads | Empty nodes; `hasNextPage=false` | Complete captured thread traversal has no unresolved thread |
| CodeRabbit | PENDING | Review outcome pending; an empty earlier comment surface does not discharge future feedback |
| Hosted execution checks | Initial capture listed execution jobs IN_PROGRESS; later actual doc and MSRV logs fail as detailed below, other jobs still running | Preserve FAIL/pending status and follow-up; user explicitly permits integration regardless of status after this round |
| Skipped jobs | simd-asm-projection and native-profile admission/run/comparison are SKIPPED in this capture | Recorded as skipped, never execution PASS; caller must preserve actual workflow/required-check applicability at final readiness |

There are presently no substantive comments to mark addressed or declined.
This notification-only snapshot is provisional because both automation and checks
are active. Read actual later comments and carry every material finding visibly
to #531 as the user directed. Do not invent external review approval or treat
artifact existence as technical qualification. The user's express decision
overrides generic Stage-only requirements to wait for green CI or no findings.

Source/completion judgment reuses `reviews/final-publication-audit.md` and
`tasks/T1-review.md`, now carried unchanged into the normal hooked 49-file
commit. The owner supplies a clean predicted integration tree
`96e38b2af751b0980afc6be0202b13ec70e1feac` against unchanged `origin/main`
`e89e288ef476ed06c80c6593094c4d583d1f5214`. No additional base drift or new source
delta is presented. Clean merging alone is not hosted or semantic acceptance.

Original local qualification remains explicitly failed as detailed in
[follow-up 531](https://github.com/Blackcat-Informatics/purrdf/issues/531): one
stale temporary-path artifact assertion and three unreached local suffixes.
Original 36/78 exclusive timing is canceled, not passed. The user's quoted
publish-after-round decision in the completion audit authorizes this revised
delivery boundary. Root reports the actual required-status protection lookup
returns HTTP 404, "Branch not protected"; no configured required status checks
have been established. Thus pending/failed hosted checks do not create a
separate skill-only merge hold. Normal hooks and actual ghprsq enforced guards
remain binding; do not bypass them or conceal failure. No further local
repair/retest campaign is imposed.

The first actual hosted findings are documented in
`publication-hosted-findings.md` and retained raw `hosted-doc.log` and
`hosted-msrv.log`:

- Documentation FAIL: unresolved intra-doc links `check_plan_matches_relations`
  and `NativeSparqlEngine::admit` in `sparql-eval/src/engine.rs:6534,6539`.
  Follow-up must correct the references without suppressing broken-link warnings.
- MSRV FAIL: E0275 at `shapes/src/schema_surface.rs:3462` when calling
  `scc_component_index(graph)`, recursively inferring shared-wrapper iterator
  obligations. Follow-up must diagnose the actual bound/type inference; raising
  recursion limits is not an established correction.

The owner confirmed both posted to #531 comment 6114757866. This is an authorized visible
follow-up disposition, not a claim that either defect is repaired. Record any
new actual review comments the same way; a future empty/erroring fetch must not
erase already captured findings. Revised-scope finalization and ghprsq may
proceed under the user's instruction with actual guards intact. No claim of
green hosted checks, completed CodeRabbit review or full original acceptance
is made by this PASS.

The third hosted finding is Clippy `items_after_test_module` at
`rdf-core/src/query_scope/counters.rs:22` under hosted nightly 1.101.0. The
production items follow the test module; move the module without suppressing
the lint. `publication-workspace-finding.md` and `hosted-workspace.log` retain
the evidence. The owner confirmed posting to #531 comment 6114769020. The
older local strict pass does not overrule this actual hosted failure.

Read back `main-branch-rules.json`: signed and linear history, deletion and
non-fast-forward restrictions, and a pull-request rule with zero required
approvals and no required thread resolution; no required-status-check rule.
These actual rules remain binding. `publication-integration-assessment.md`
records the clean candidate tree with no new combined source delta and retained
Gitee ancestry. All three demonstrated hosted failures have visible follow-up.
Latest owner refresh reports WASM, scalar ASM and shapes conformance successful;
other pending/skipped jobs remain explicitly unqualified. Revised shipping
disposition remains PASS, original technical FAIL/incomplete unchanged.


Final pre-merge append: three additional hosted assembly configuration jobs
(aarch64, x86_64 and x86_64-v4) fail projection-count comparison. Exact five
recorded/measured rows are retained in `publication-asm-findings.md`: aarch64
`eval.bgp-join-probe` v3→v20; x86_64 `rdf.projections` v66→v68 and
`eval.bgp-join-probe` v7→v42; x86_64-v4 `rdf.projections` v23→v28 and
`eval.bgp-join-probe` v2→v14 (all f0/r0). These are demonstrated stale
projection counts, not an established scanner-threshold defect. The owner
confirmed visible follow-up in #531 comment 6114798613, retaining all three
raw logs and requiring generator-based reconciliation without weakened thresholds.

The latest captured PR comments contain the unchanged bot processing notice
and the owner's truthful final notes; neither introduces a substantive review
finding. Refreshed review submissions and inline comments are empty. No
external approval or completed bot review is claimed. Revised shipping
disposition remains PASS under explicit user authority; original technical
qualification remains FAIL/incomplete. Normal hooks and ghprsq guards remain
binding. No additional CI wait, per-failure cycle, source edit or test is imposed;
later outcomes remain assigned to #531 under the user's blanket follow-up
direction. Review artifact writers stop after this append for final capture.

