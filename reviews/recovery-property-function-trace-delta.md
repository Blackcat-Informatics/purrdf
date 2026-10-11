# Review of the two remaining evaluator trace differences

Root read the complete old/current blocks in
`recovery-closed-empty-conformance.log:24–110` and the production branch at
`expr.rs:2522–2532`, together with the wrapper's source mapping at
`expr.rs:14470–14477`.

Both `bbMemberFalse` and `modeRestrictedSubsumption` retain the true answer,
digest `27229339670298e5`, scratch peak 153, and every provider invocation, row
and output charge. Their empty RHS executes directly for evidence, instead of
through the ordinary synthetic first-witness Slice. That Slice contributes one
algebra-node entry, mapped onto the original property-function node, and no row
charge because its source mapping has `counts_rows=false` and the RHS is empty.
The observed node-entry change from two to one therefore matches the actual
computation. Metered fuel 11 to 10 and the one-step earlier settled prefix and
completion follow that single removed execution; earlier cut points agree.

The existing generator may refresh these two trace cases. Review its diff to
ensure no unrelated expectations change. The temporary diagnostic reporter must
remain removed. The four original conformance targets' receipt shows the cost
planner, governor corpus and all 74 W3C manifests pass; this does not establish
performance or the full gate.

The changed fuel/cut-point behavior also requires the documented governor
profile version and prose update. R2 currently names version 14, while geo's
unmerged delivery independently names 15 for native kernel work. The second
delivery must reconcile both changes and advance the version on integration;
two independently changed schedules must not silently ship under a stale
behavioral identity.
