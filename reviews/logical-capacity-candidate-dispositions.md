# First census candidate dispositions

The unchanged helper gate failed. This is a source-review inventory, not a gate
pass or an exemption list. No production files were edited to obtain these
dispositions. Line numbers refer to the retained first gate receipt.

## Stored local/global identity reconstruction

These calls reconstruct existing identities; they do not admit a new table slot.
The original owner must keep the table below its niche ceiling. Current source
does so at the interner/LocalCapacity mint, and the original reduced-counter
tests remain relevant. The current local syntax rule does not yet prove these
cross-method ownership facts. No message string or whole-function exemption is
proposed.

| Source under crates/rdf-core/src | First-gate line(s) | Actual boundary evidence |
|---|---:|---|
| ir/builder.rs lookup_hashed, lookup_owned_blank | 565, 591 | Hash indexes contain existing inserted term ordinals; the find closure also indexes the live term vector. |
| ir/builder.rs transaction_try_with_memory | 817 | Iterates the appended suffix of the same admitted interner before truncation; removes corresponding content IDs. |
| ir/builder.rs append_builder_with_memory | 2194 | Enumerates the validated source builder's existing term_count; narrowing already propagates StorageError. |
| ir/composite.rs term_ids | 261 | Native source count or selected.ids count; selected IDs were checked during project construction. Delta is a separate typed iterator. |
| ir/composite.rs project | 845 | Immediately preceding u32::try_from(ids.len()).is_err returns diagnostic; enumerate yields indices strictly below that checked length. |
| ir/dataset.rs find_term_hashed | 1502, 1505 | Frozen term-index entries come from the admitted interner. |
| ir/mutable.rs visit_blank_identities | 606 | Iterates the frozen base's term_count. |
| ir/mutable/delta_view.rs new, term_ids | 114, 203 | Enumerates frozen delta/base term tables. |
| ir/event_sink.rs emit | 95 | Enumerates self's frozen term table before emitting rows. |
| ir/global.rs find | 432 | Global dictionary hash index holds admitted global ordinals. |
| ir/global.rs value_index | 888 | Enumerates the resident term Vec, whose addressable length is below the u64 niche ceiling. |
| ir/ingest.rs drive | 586, 601 | Both loops enumerate the same frozen dataset; first finds blank scopes, second emits terms. |
| ir/paged/provider.rs logical_rdf_byte_len | 458 | Enumerates an RdfDataset parameter. |
| ir/paged/query.rs validate_materialization | 677 | Uses the already-built PageTranslation term_count, not an arbitrary provider count. |
| ir/paged/stack.rs snapshot_with_head | 392 | Enumerates each retained GlobalDictionary's resident term Vec. |
| ir/paged/translation.rs try_build, remap | 98, 179 | try_build copies a frozen page's term table; remap keeps that existing local table length and changes only global IDs. |
| ir/validate.rs validate_triple_terms_with_memory | 453, 501 | term_count is supplied from the builder being validated; the same admitted terms are read in both passes. |
| turtle_render.rs find_iri | 254 | Enumerates a borrowed RdfDataset. Existing term_id_by_iri already owns equivalent lookup; no migration applied. |

## Offset and encoding representations

| Source under crates/rdf-core/src | Line(s) | Actual evidence and remaining distinction |
|---|---:|---|
| ir/segmented/build.rs global | 104 | first_term_index + limits.terms is checked before publishing SegmentedBuildLimits; the builder admits the actual term count beneath that limit. |
| ir/segmented/build.rs local | 121 | checked subtraction, then index < stored builder.term_count; u32 narrowing already returns AddressExhausted. |
| ir/segmented/build.rs map_id | 510 | Called by the encoder with the same checked first-index interval and original local TermId. Cross-method interval proof is not yet represented in the rule. |
| ir/segmented/build.rs encode_dataset | 535 | Frozen dataset term iteration; narrowing already propagates AddressExhausted. |
| ir/segmented/format.rs decode_quad | 460 | graph is decoded u64; this branch proves graph != 0. Therefore graph - 1 is at most u64::MAX - 1 and valid as GlobalTermId. |
| ir/embedding/metadata.rs encode_contracts, encode_external_bindings_canonical | 383, 1076 | Local fixed constants SPACE_ID_INDEX_RECORD_LENGTH=40 and EXTERNAL_LOOKUP_RECORD_LENGTH=80, both u64. No input-dependent length. |
| ir/embedding/writer.rs encode_matrices_section | 954, 962, 971, 979, 987 | Local fixed u64 record lengths 160, 152, 40, 72, 72. No input-dependent length. |
| ir/embedding/wire.rs header | 321 | FileLayout::plan calls validate_descriptors; that rejects count > PURREMB_MAX_SECTION_COUNT=65535. Private entries preserve cardinality. |

## Physical and value-shape checks

| Source under crates/rdf-core/src | Line(s) | Disposition |
|---|---:|---|
| ir/dataset.rs permutation | 1162 | Resident infallible wrapper around try_permutation; native rows already have the Quads bound, equal to native permutation ceiling. The remaining failure is physical allocation. Reduced test capacity still belongs to the fallible producer path. No blanket exemption for reserve/try methods. |
| ir/embedding/writer.rs write_matrix | 641 | row_bytes.len()==row_capacity verifies the encoded row matches validated stored_dimension. It is an exact byte-shape invariant, not a representability ceiling. |
| ir/embedding/writer.rs prepare_matrix | 716 | matrix_bytes.len()==capacity verifies serialization agrees with previously checked_matrix_byte_length. Again a value-shape equality, not a logical ID cap. |

## Input-dependent candidates requiring further work

| Source under crates/rdf-core/src | Line(s) | Finding |
|---|---:|---|
| term_fixture.rs build_page | 148 | Public shipping helper accepts an arbitrary triples slice and expects push_quad. Documentation identifies test support, but lib.rs exports it without cfg(test). No structural fixed bound on the public input. |
| term_fixture.rs graph_slots | 211, 214 | Exactly two authored quads, with a fixed small term set; unlike build_page/corpus, this has a fixed cardinality proof. |
| term_fixture.rs graph_membership_corpus | 244 | Public graphs:usize drives row growth; no authored upper bound is checked. Typed Result propagation is warranted if this shipping helper remains under the rule. intern_value also expects the actual fallible intern_value_with_memory; the first name-based scan did not identify that wrapper, so the four initial hits are not a complete proof for this module. |
| xsd_regex/xpath/compile.rs flat_sets | 980 | u32 offset conversion follows grow/CompileSlots admission, but Limits stores u64 bounds. A u32 representability proof has not been established; cannot bless the expect from its diagnostic text. |
| xsd_regex/xpath/pike.rs analyze | 400 | node index conversion follows ProgramNodes/CompileSlots admission. The observed public bounds are u64; a concrete u32 ceiling or typed refusal still needs verification. |

This accounts for all 46 first-run candidates. Fixed constants and exact byte
shape assertions can be handled by narrow expression facts. Resident ID and
interval invariants require source-backed boundary evidence; a generic exemption
for from_index or a new unchecked constructor would conceal real mint failures.
No production expansion has been made pending review of this table.
