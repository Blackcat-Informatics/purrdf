# External caller chain input

Read-only consumer checkout: `/home/paudley/Active/gmeow-ontology`, donor commit
`38f568b04a5f6509e98ccb5e3b387623f1974b58`. These are caller input, not library
vocabulary or a claim that the entire consumer ontology is certified. The exact
ordered `logic:propertyChainAxiom` operands are supplied through the consumer's
OWL grounding as `owl:propertyChainAxiom`; no operand or recursive head is renamed.

Rows in `gmeow-chain-input.tsv` correspond to:

1. `slices/extensions/sensory/module.ttl:87`, `hasSensoryQuantity`.
2. `slices/core/events/module.ttl:377`, `eventLocation`.
3. `slices/core/events/module.ttl:1535`, `wasAssociatedWith`.
4. `slices/core/observations/module.ttl:369`, `hasReferenceFrame`.
5. `slices/core/organization/module.ttl:457`, `memberOf`.
6. `slices/core/places/module.ttl:2363`, `hasCoordinates`.
7. `slices/core/places/module.ttl:2395`, `hasGeometry`.
8. `slices/core/places/module.ttl:2698`, `locatedAt`.
9. `slices/core/temporal/module.ttl:1104`, `periodStart`.

The committed portable controls use example.org terms and the same ordered shapes.
The separate caller probe supplies the exact consumer IRIs to the public Reasoner,
checks the combined regular RBox and every positive consequence, then independently
adds each contradicted consequence and requires inconsistency. Runtime PASS is recorded
in `gmeow-nine-1.log`: the combined RBox is decided consistent, all nine exact
positive consequences are entailed, and each independently contradicted consequence
is decided inconsistent. The probe source is preserved in `gmeow-chain-probe.rs.txt`.
