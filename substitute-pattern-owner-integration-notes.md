# Original-owner prebinding pattern conversion

`substitute-pattern-owner-draft.patch` changes the existing native spelling body,
not its grammar or pushability policy. The resident wrapper delegates to the same
fallible body. Root authored Stage postimages only; shipping source is writer-owned.

The new `term_pattern_from_ground_with_memory` borrows the enclosing original
Memory. It admits each OpenTriple vector replacement and each actual TriplePattern
box before fallible allocation. Shared lexical leaves retain their intrinsic
immutable owners. Successful output leaves only its box layouts in the enclosing
query grant, after destroying traversal scratch. Seed-only blank refusal destroys
completed subjects before releasing their layouts and returns semantic None;
physical refusal returns StorageError and must become the original typed refusal.
On any physical failure, enclosing payloads must die before the frame/grant.

Required companion: TriplePattern Child destruction must use the allocation-free
common dismantle_tree loop, which the grounding helper owns in tree.rs. Existing
DropWork destruction can otherwise allocate outside this grant. Full query rewrite
must use this new native body with its original query frame and preserve that
grant with the returned AdmittedQuery; resident calls alone do not satisfy bounded
production acceptance. No estimated AST census is a replacement for that grant.

Three proposed unit cases cover 600 generated recursive-reference comparisons,
completed-subject blank cleanup, physical refusal versus semantic refusal, and
zero-new-allocation shared-leaf success. The existing 100000-depth small-stack
fixture now reaches the same body through the resident wrapper. These cases are
unrun; rustfmt parsing and ordinary git apply --check passed. This proposal does
not claim full physical/public qualification or completed query rewriting.
