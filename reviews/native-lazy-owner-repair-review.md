# Native lazy owner repair: bounded source review

Read-only review of the settled EvidenceOwner/EvalCtx/producer repair described
by its owner. No builds, tests, source edits or process changes. Prior trigger
analysis is `nested-negation-owner-diagnosis.md`. Compiler/runtime results belong
to the implementation owner and are not inferred here.

## Material remaining finding

**Open: aggregate fallback still forks the query dictionary.**
`evidence_exec.rs:1555–1578` retains a direct `QueryEvidence::new(workspace)`
inside `RowEvidence::native_aggregate`. Its production caller is
`modifier.rs:2117`; `group_evidence` starts from the bag's search evidence at
line 2135. An ownerless nonvirtual record is legal: public
`RowEvidence::default().with_unclosed_search(true)` is already exercised by the
existing test at `evidence_exec.rs:1927`. The shared adoption method preserves
such flags but has no owner to adopt, so it does not prevent this fallback.

COUNT over such an open search materializes a Lower bound. Separate groups can
therefore receive records from separate fallback dictionaries, recreating the
same combination failure downstream. Route this last fallback through the same
lazy original-query owner as EXISTS/negation, keeping workspace admission for
aggregate storage. The empty-aggregate check currently follows owner creation;
move that no-op return before fallback so an empty aggregate list does not
materialize a discarded dictionary. The implementation owner confirmed this
finding and will apply the bounded correction after the running focused job
ends. It remains open until the patch is inspected.

## Reviewed repair properties

- `EvidenceOwner` factors the existing shared `Mutex<Option<QueryEvidence>>`
  control from `EvidenceInput`, preserving its original native adoption/import
  behavior. No new dictionary algorithm or implicit import was introduced in
  `RowEvidence::combine`. `QueryEvidence::owns` still compares state identity and
  refuses genuinely different owners.
- `EvalCtx::evidence_owner` returns the explicit source's same control, or
  initializes a native-only control under its inline mutex. Ordinary contexts
  start with None. Virtual bindings and positive wholly virtual EXISTS results
  return before control creation. `native_evidence_for_child` initializes shared
  control before either worker or UDF child can create its first record, only
  when evidence is reachable and no source already shares it. Both context
  literal sites use that method; cloning an uninitialized dictionary no longer
  splits the native owner.
- Each new control goes through `SharedWorkspace::new_admitted`, whose native
  home computes/charges the allocation before `Shared::try_new`. Query dictionary
  creation remains lazy and uses the existing admitted home. Cloning controls
  shares them; control -> dictionary -> workspace has no new back-reference
  cycle. Retained rows own their dictionaries independently of context lifetime,
  and payload destruction precedes release of its original workspace grant.
- Property-function driving rows, emitted rows and final invocation search
  declarations bind through the same context owner before combining. Native
  shape emitted evidence and carried input search declarations do likewise.
  First producer ownership is adopted when possible; otherwise import uses the
  original reason/record home and preserves rank, completeness and membership
  flags. The already-owned path does not duplicate records.
- The per-pull `FilterContext` at `property_fn_eval.rs:1768–1800` is not a missed
  EXISTS owner fork: admitted call-read predicates prohibit EXISTS/custom/stateful
  evaluation and its fresh context installs no evidence source/registry. No
  speculative cursor lifecycle change is justified by that constructor.

## Verification applicability

The new `native_evidence_owner_is_lazy_and_shared_before_workers_create_records`
regression covers two workers and one UDF fork before first record, no-registry
and virtual-result laziness, explicit producer import preserving incompleteness,
and rejection of another query's records. Existing foreign-owner and admitted
storage/lifetime tests remain relevant; their mechanisms were reused, not
reimplemented. The regression does not by itself execute the W3C query path or
prove a performance/allocation bound for every default query.

After the aggregate correction, add focused coverage for two ownerless-open-search
aggregate results sharing the original owner and preserving Lower bounds, plus
the empty-aggregate no-op. Retain the complete W3C negation manifest with the
unchanged harness registry, native evidence ingress, ordinary allocation controls
and mandatory full gate. Throughput remains independently unqualified; this
repair changes code and context layout and must be represented by the final
performance artifact.

## Aggregate follow-up source disposition

The implementation owner subsequently repaired the open aggregate fallback.
`native_aggregate` now accepts the same fallible original-owner callback and
returns before resolving it for virtual records or an empty aggregate list.
`modifier::aggregate_evidence` obtains that owner from the actual EvalCtx;
the existing explicit-source route is unchanged. `native_evidence_for_child`
also retains an already initialized owner when no registry is installed, rather
than dropping it solely because `may_have_evidence` is false.

The new `native_groups_from_ownerless_search_share_the_original_query` regression
uses actual grouping/aggregation helpers on ownerless unclosed search evidence,
checks Lower COUNT bounds and combination across 0/1/2-member groups, and keeps
the resulting records usable after context/worker destruction. Source inspection
finds the reported fallback closed. Runtime/compiler acceptance is pending the
owner's focused collection; this does not mark the failing fuel trace or full
gate passed.
