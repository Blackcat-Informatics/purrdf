# PR 504 feedback: root source check

2026-10-08. Four review findings remain open. This is source adjudication, not a
completed fix or execution qualification.

* `property_fn_eval.rs`: iterative `Apply` uses the Join resume for a mandatory
  application. The recursive reference always refuses a contextual application,
  while preserving optional/conjunction column summaries. The empty-left Join
  shortcut can therefore admit a read that omits application semantics. Fix the
  production resume and extend meaningful differential coverage to Apply.
* `property_fn_plan.rs`: iterative `collect_bound` enters the mandatory RHS with
  the unchanged context. The recursive reference maps each policy input only
  when its driver is certainly bound by the LHS or enclosing context. The fix
  must preserve that mapping and optional-left-only certainty, and exercise a
  real bound-mode planning consumer with an unbound control.
* `service_endpoints.rs`: Apply merges RHS occurrences directly; LeftJoin makes
  RHS occurrences indirect. Optional Apply requires policy-sensitive summary
  handling, including a parent lookup consumer, not only reference agreement.
* `quad_store.rs`: projection currently removes and inserts in surface table
  order. A later excluded default reifier can remove an earlier named ordinary
  projection of the same value. Removal before insertion repairs that order
  hazard, but `MutableDataset::insert_rows` restores all suppressed physical
  occurrences of an equal base value. The implementation and regressions must
  determine whether restoration introduces observable duplicate query rows.
  Do not claim a two-pass change is sufficient from the one erasure witness.

The single implementation writer owns the coherent fix and focused native
checks. An independent reviewer owns feedback adjudication and review debt.
The ordinary static unit query mode must retain its direct snapshot path.
Reassess analytical cost evidence only for changed paths; retain attributable
unchanged evidence. No merge is authorized by green CI alone while these
findings remain unresolved. Full portfolio scope remains active.

## Working-delta follow-up

Root read the new empty-base/fresh-delta selection helper and binding delegation.
The compatibility fast path is removed in the inspected working source; native
unit QueryMode still returns its snapshot directly. All compatibility graphs
must expose one value per RDF triple even when no named graph exists. Required
regressions therefore compare exact result bags for default-only physical-role
overlaps with absent, empty and populated unrelated named graphs. Counts and
uniqueness alone cannot prove the selected terms are correct.

The new public helper accepts an optional TermValue graph selector. Unlike the
Python extraction boundary, its current working body does not validate literal,
quoted-triple or relative-IRI selectors. A supplied invalid selector must fail
through the existing graph-name ingress home; a valid missing graph selects an
empty default without inventing a named slot. Writer owns this new-boundary
repair and focused controls. No runtime acceptance is claimed here.

Old-head PR CI is now terminal with 48 successful and two declared skipped
contexts. It does not qualify the working source changes. The separate 308
measurement campaign has six of twelve uploaded cold/warm pairs; full current
cohort validation and measured reduction remain pending.
