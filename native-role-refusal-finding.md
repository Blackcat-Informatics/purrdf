An additional producer-retention case surfaced while consuming recorded GMEOW
inputs with native DL proof recording. The earlier broader authored-default
selection refused with `RoleHierarchyError::InvalidProperty { node: 6745 }`;
the public error identified only that private interner ordinal. This is not a
result for the corrected assembled default world, whose operation is still live.

The concrete call path at af8936e6f is `CeExtractor::role_of` in
`crates/entail/src/owl_dl/parser.rs`: it calls `RoleReader::role(r)` and converts
the error directly with `map_err(Into::into)`. `RoleReader::role` creates the
bare InvalidProperty variant. By comparison, the hierarchy construction path
attaches the existing source witness before returning its role refusal. A
restriction operand can therefore lose the source identity needed to interpret
the ordinal after the private parser owner has been destroyed.

This falls within this issue's producer-field retention work. The original
typed classification must remain Malformed; a repair must preserve physical
refusal priority and the original grant. Adding human presentation alone does
not discharge the structured-field contract. In particular, the existing
`RoleHierarchyError::with_source` allocates with a fresh Resident owner; that
must not become an escape from an admitted caller's grant. No speculative change
to that capture path has been made in the graph-projection PR.

The complete graph-projection and consumer integration remain tracked in
PurRDF 566/567 and GMEOW 1803/1837. Their native result and current benchmark
acceptance are not being replaced by this diagnostic observation.
