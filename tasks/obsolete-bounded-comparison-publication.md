The obsolete bounded decimal comparison exemption is removed in50ae94847, committed through normal hooks and pushed to the existing delivery branch. It named a comparison that now exists only under cfg(test), while production ordering uses exact::Decimal; the checker correctly rejected the stale registration.

Only that eight-line exemption was removed. The enforced bigint detector, all other live variants and checker remain intact. The existing helper gate passes81 enforced jobs,21 live variants,118 distinct groups and128 hash domains. Independent scoped review and the user's clarification that there is no tooling freeze are recorded in the delivery Stage.

This resolves the demonstrated registration failure. Exact/default host acceptance and final full qualification remain open, so no issue completion is claimed.
