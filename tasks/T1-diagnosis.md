# Exact imported-source mismatch

The unchanged existing CLI case fails at line 1073, one selected test with thirty
filtered out. Its message matches the reported integration failure.

The temporary native comparison probe observes:

- Recorded frame: Individual(85); replayed frame: Individual(87).
- Both witnesses are the same concept 1, but over those different node references.
- In the consumer's reparsed source, id 85 denotes owl:imports; id 87 denotes Tom.

The failure is therefore a source-coordinate mismatch, not missing support that
could be repaired by waiving the inhabitant comparison. The CLI obtains canonical
N-Quads from ReasonerSession::premise_nquads and reparses it for checking; the DL
interner assigns ids in first-seen order. The source graph remains canonically
equal while this extra serialization changes its native checking coordinates.

The correction keeps the consumer's own resolved frozen dataset. A new
ReasonerSession::check_proof uses that snapshot and its already-verified imports
status to build a fresh checking context. The existing document API and snapshot
API delegate to one extracted checker; service/proof decoding remains before
source loading, preserving error priority. No search state or producer context
is shared. Canonical premise export remains a data export with the same bytes;
its incorrect proof-context claim is replaced by the actual snapshot entry.

The full invariant, support journal, proof encoding and native checker are
unchanged. The temporary probe has been removed: proof.rs has no source diff.
Rejected alternatives include skipping the comparison, silently remapping an
ordinal to a different source term, changing all default reasoner term ordering,
or starting a new proof-format/symbol-table migration for this source handoff.

Strict affected library/test and new public-target compilation pass. The unchanged
CLI regression, native public vectors and imported-source cases, five current-support
controls and both exact scalar-WASM cases pass; see validation.md for actual logs.
