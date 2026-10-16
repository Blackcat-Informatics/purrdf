# Paged statistics implementation review

Owner review under the zero-subagent instruction; this is not independent approval
or whole-issue completion.

The original sealed statistics now support the two native identity widths through
one private codec/merge home. External providers retain the public ViewTermId
contract. Native rows and term IDs are unchanged. All retained buffers and merge
scratch use the existing Memory admission; failure scopes refund partial work.
Rows add only over the paged constructor's disjoint streams. Keys and domain IDs
translate into the global dictionary before sorting; duplicate domain identities
collapse there. Selected frequencies obey the combined resident selection cap,
while class frequencies remain complete. The property control includes reversed
local interning, overlapping domains, graphs, three streams and over-cap objects.

Page headers own admitted copies of metadata, not materialized RDF rows or their
grants. Translation identities and an acyclic shared bundle bind the merged
metadata to the exact page set. Unchanged warm parts share the bundle after an
O(page-count) identity check. Remapping invalidates it; subsets and compaction
merge their actual surviving headers. Public PagePart fields remain unchanged.
Operational reads borrow metadata without query admission or materialization.
The existing page digest now includes canonical statistical words, including
joint frequencies; changed joints with identical old marginals are detected.

SPARQL uses provider row counts even when physical work is supplied independently.
The work cost itself is retained. EXPLAIN reports the actual metadata provider.
The adapted receipt comparison still checks every row/domain/accuracy input and
every execution/consumption line; only provider provenance and the explicitly
asserted physical descriptor differ between resident and paged storage.

Executed evidence: five metadata controls including exact translated union,
five page-summary controls, the original allocation-refusal/refund control,
rare-class native/paged execution, reifier-only plan changes and small claim-join
controls pass. The original million-claim receipt retains its scope: the native
rows and statistics it consumed are unchanged. Earlier strict affected compilation
and helper checks pass. The shared allocator observes zero allocations, requested
bytes and retained bytes for both native and paged domain reads. The measured-host
control preserves physical work while consuming independent row counts. All three
affected fallible EXPLAIN controls and the SERVICE receipt control pass, as does
the public reifier plan/source control. Final strict core/evaluator libraries and
tests pass with warnings denied. The final helper census passes; all forty
existing capacity proof contracts remain unchanged, with source identities and
moved sink locations refreshed from the native census. No checker was changed.

Resolved findings: partition fixture identity collision; combined frequency-cap
parity; missing statistical cardinality when host physical cost was supplied;
receipt comparisons assuming identical metadata provenance. Failed logs remain.

Required remaining work is segmented/effective-snapshot statistics, the actual
native rule consumer, structural component estimates, final profile/generated
reconciliation and whole-issue qualification. There is no partial PR or closure.
