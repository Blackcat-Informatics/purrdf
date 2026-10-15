The optional dictionary-sharing proposal in review
https://github.com/Blackcat-Informatics/purrdf/pull/549#pullrequestreview-5527789356
is declined for this PR.

The captured Base owns an independent, ID-preserving dictionary.
`RelationStore::term_bytes` charges both physical arenas; the existing native
and portable base/retraction control explicitly checks their combined footprint,
original IDs and refusal before sink interning. No single-copy or constant-cost
Base snapshot claim is made.

Issue #492 requires compact 12/8/12-byte RDF carrier rows, graph-free rule
positions and partition keys, default-query conformance and equal rule models.
It does not promise identical term-arena allocation or admission cost between
the representations. Replacing this explicit reasoning-store ownership is an
optional representation change. The implemented ownership and actual accounting
remain part of the tested contract; the separate recursive negative-Base delta
regression is being corrected and is not discharged by this disposition.
