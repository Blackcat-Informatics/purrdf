The SPARQL planner now consumes snapshot-bound predicate/class frequencies,
positional distinct domains and RDF 1.2 reifier/annotation counts. Native,
compact, paged and persisted stores expose the same caller-implementable seam;
EXPLAIN records consulted counts, exactness and provider revision. Native
Datalog ordering consumes the same statistics.

Paged and physical segment domains merge after identity translation.
Chronological views retain conservative physical bounds across overlap/removals
and unknown distinct counts where exact union is unavailable. Original
allocation, header-only open and first-fault ownership remain intact.

All five issue demonstrations pass, including the actual million-claim/six-million-
annotation query with one million estimated and actual rows, native/paged rare-class
ordering, reifier-only and supplied-provider plan changes, and two physical
segments versus one combined image. All 49 hosted checks pass with four expected
optional skips. The current full gate includes all test shards, all seven assembly
configurations, conformance, architecture execution and installed host packages.
Both material review threads are resolved; the summary warnings are dispositioned
with concrete CI and source evidence.

Governor profile 20 identifies changed planning charges. Three trace charges
change with complete answers and peak cells unchanged; all 15 normative controls
and 712 traces pass. Persisted profile 2 adds authenticated statistics and refuses
old profile-1 images, which must be regenerated from their RDF inputs. Historical
storage observations retain their original source scope.

Final compiler-compatibility repairs preserve the same w/x/y stemmer predicate,
retain unreleased deprecation warnings and use standard local future pinning.
The scoped assembly writer preserves unmeasured columns; the complete seven-report
gate remains required. No timing-based acceptance is used.

Closes #484.
