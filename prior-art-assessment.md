# Independent prior-art and reuse assessment

Read the gathered issue, prior-art.md and references.md; no forge refetch. The
brief found no earlier attempt for this issue. Related graph-lifetime/shared-view
work explains the existing implementation but does not provide a caller-selected
freeze API. Keep its behavior rather than importing another implementation.

Actual source homes:

- MutableDataset::freeze owns one compaction/import and surviving-location remap.
- snapshot_view_with_limits and DeltaAdmission own the finite five-coordinate
  retention checks; append_delta owns RDF12 delta classification.
- ViewLimits::check owns the existing view-retention-limit diagnostic and exact
  dimension messages. Defaults remain64 sources,16,777,216 terms,67,108,864 rows,
  1GiB RDF payload and512MiB auxiliary retention.
- shared_views.rs complete_source() and surface() already cover all three roles,
  scoped blanks, nested triple terms, directional language literal and declared
  empty graph. Reuse those fixture/oracle homes for new public API controls.
- typed_snapshot_limits_price_role_masks_and_refusal_adds_no_successful_work
  already proves auxiliary-boundary snapshot refusal and stable successful work.
- freeze_carries_base_quad_locations, freeze_empty_mutation_equals_base_quads,
  graph_existence_modes and mutable_declared_graph_drain provide applicable
  location, role and graph-lifetime neighbours.

The additive delegating API fits these homes without a second loop, shared
mutable policy field, new generic resource layer, semantic Cargo feature or
tooling change. Existing snapshots retain their explicit policy. Standalone
large-corpus native acceptance is still needed for the issue's reported route;
unit/read-only evidence is not a substitute. No source edits or builds performed.
