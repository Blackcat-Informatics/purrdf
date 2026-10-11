# Preserved EXISTS source-map repair

Bounded source verdict: PASS. No build, test, timing or shipping-source edit
performed by this reviewer; execution remains required.

`PreparedExists::build_with_evidence_admitted` already chooses between copying
the original pattern and applying emptiness normalization. The previous two
unconditional `ledger_source_map_admitted` calls applied normalization's spine
erasure laws even to a preserved structural copy. An erasable OPTIONAL therefore
paired its original leaf with the retained two-child OPTIONAL and reached the
correct structural-clone assertion.

The patch's local `source_map` closure uses the existing `map_clone_admitted`
for preserved patterns and the existing normalization mapper otherwise. Both
the prepared copy and its first-witness inner copy use that choice. All map
growth still follows the existing admitted insertion and work-list homes;
allocation refusal propagates through the original Result. The structural
assertion and normalization laws remain intact. The synthetic first-witness
wrapper retains the original root source and `counts_rows: false`.

The new regression exercises the failing preserved OPTIONAL through the real
preparation method. It checks both retained structures, their original root
identity and row-count flags, the synthetic non-counting wrapper, and complete
mapping population. Existing clone-map behavior supplies descendant traversal;
the original conformance reproductions remain necessary to verify production
callers. This review does not qualify the separate query-owner repair or the
remaining failed full gate.
