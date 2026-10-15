# Capacity constructor closure review

The implementing session reviewed the three changed constructor-reachable sources:

- query/view.rs adds two subject-keyed trait overrides. They forward original
  GlobalTermIds through the existing logical streams and do not mint local IDs.
- session.rs changes physical cache ownership, hash allocation admission and
  recency keys. CacheKey contains an authenticated snapshot and a u64 block slot.
  The new derive implements ordinary value equality/hash, not term construction.
- session/cache.rs contains indexed lookup and recency links. Its only macro is
  the standard debug_assert on preadmitted physical table capacity. It does not
  introduce an ID constructor, macro expansion capable of minting IDs, count
  narrowing or a capacity-error panic conversion.

The actual native index reports 151 constructor-reachable sources and new closure
identity source:c8ca74272d7f928f. Exactly 40 old proofs became stale. All 40 still
match one unique original sink, and every individually registered authority hash
is unchanged. The refresh updates only their constructors field from the old
source:26d9019e0f8ab807. Reasons, classes, sites, authorities, checker implementation
and acceptance are untouched. Test-only cache_control.rs is not in the shipping
constructor closure. No new proof row or exemption is introduced.

This is a source review by the implementing session, not independent approval.
The final helper gate must validate the complete refreshed ledger.

The first refresh script stopped before writing: it assumed the native index's
capacity_sources array was a map. The corrected script reads its file/identity
records, verifies every authority and unique sink, then changes exactly the 40
closure fields. That scripting error is not a passed gate or a source defect.
