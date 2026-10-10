# Prepared execution marginal allocation diagnosis

Full check12 failed two mutable performance pins: five warm memo calls each
measured21 instead of26, and five bare admitted-plan calls each measured50
instead of68. The fixture explicitly defines these as the current marginal
cost, not ceilings or immutable semantic receipts. All answer and reuse checks
remain unchanged.

The actual main executable recorded by merged PR525 was used read-only:
`/opt/purrdf-text-ranking-qualification/build/debug/build/purrdf-sparql-eval/b9c28de31d5e7f81/out/prepared_execution-b9c28de31d5e7f81`.
Its original memo test measures five23 calls; its bare-plan test measures five48
calls. The current full12 executable independently reproduces five21 and five50.
These are different ownership representations. The old intermediate26/68 totals
are not assigned invented per-component counts.

The matched captures are `prepared-baseline-memo-real.*`,
`prepared-current-memo.*`, `prepared-baseline-plan.*`, and
`prepared-current-plan.*`. Their demangled stacks include preparation and two
warmups, so aggregate heaptrack totals are not substituted for the five measured
windows. The initial baseline memo capture selected a nonexistent renamed test
and ran zero cases; it is not evidence.

The cheaper schema producers are visible in actual source and stacks:
`SharedSchema` retains the existing immutable `Shared<SchemaData>`;
`VarSchema::union_admitted` returns the left schema when no variable is added;
`BgpProjection::new_admitted` retains its working schema when no hidden column is
removed. The native memo trace retains inner `SchemaBuilder::finish` publications
and their variable buffers, with no additional outer schema-control publication.
VM argument ownership is sparse, so resident arguments allocate no array of empty
lease slots. Original native payload grants and checked shallow clones remain.
The bare-plan call also performs native re-admission and owns its controls and
buffers; its total is not inferred from the resident main total.

The constants and accompanying source explanation now state the measured21/50
marginal costs. The original exact five-call assertions, warmup boundaries,
memo verification switch, answer checks, repeated-run cleanup and all physical
ownership assertions are unchanged. Consolidated affected qualification is in
`full12-complete-family-affected-1.log`; until terminal success it is NOT MET.
