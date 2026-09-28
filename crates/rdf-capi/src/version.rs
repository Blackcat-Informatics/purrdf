// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! ABI version reporting and dataset capability introspection.

use purrdf_core::RdfStoreCapabilities;

use crate::handles::PurrdfDataset;
use crate::status::{PurrdfAbiVersion, PurrdfCapabilities, PurrdfStatus};

/// ABI major version. `0` signals the surface is still **beta** — the freeze
/// discipline (append-only status enum, drift-gated header) is in place, but the
/// version stays pre-1.0 until a real C consumer + the rdflib shim exercise it.
///
/// # The bump rule this triple obeys
///
/// The project's pre-1.0 policy — `docs/book/src/project/releases.md`, the
/// "Pre-1.0 semver policy" section — is: while the version is `0.x`, a **minor**
/// bump (`0.x` → `0.(x+1)`) may include breaking changes, and a **patch** bump
/// (`0.x.y` → `0.x.(y+1)`) is bugfix-only and API-compatible. So while MAJOR is
/// `0`, **an incompatible C-ABI change rides the MINOR component**; MAJOR does
/// not move, because moving it would declare the 1.0 stability this surface has
/// explicitly not earned yet (the paragraph above).
///
/// A change is incompatible — and therefore MUST bump MINOR — when a host built
/// against the previous header would mis-execute against the new library:
/// removing an exported function, renaming one, retyping or reordering its
/// parameters, inserting a parameter anywhere but the end, changing its return
/// type, or renumbering a status discriminant. `tests/abi_signatures.rs` pins
/// the complete exported prototype list to this triple, so such a change cannot
/// reach a release without an author deliberately touching these constants.
pub const PURRDF_ABI_MAJOR: u32 = 0;
/// ABI minor version. It TRACKS THE EXPORTED SIGNATURES: every change to the
/// parameter list, the return contract, or the documented behaviour of an exported
/// symbol bumps it, including a purely additive out-param — additive in source is
/// still a recompile for every C consumer, and a library whose minor is below the
/// header's cannot honour the call the header describes. It is the number a consumer
/// linking against an unknown build reads back from `purrdf_abi_version` to decide
/// whether the header it compiled against and the library it loaded agree, so it must
/// never stand still across a signature change.
///
/// `0.6.0` → `0.7.0` carries four incompatible signature changes, deliberately
/// bundled into one unreleased bump rather than split across four:
///
/// 1. `purrdf_shacl_validate_to_sarif` and 2. `purrdf_shacl_entail_to_ntriples` each
///    gained a `shapes_base_iri` parameter **in the middle** of their existing
///    parameter list, between `shapes_ttl` and `data_nt`. For a host compiled against
///    `0.6.0` that is a silent, unguardable break: it passes `data_nt` into the
///    `shapes_base_iri` slot and its `PurrdfBuffer **` out-pointer into `data_nt`,
///    which the boundary then reads as a NUL-terminated C string. The parameter is
///    deliberately positional rather than appended — it belongs immediately beside the
///    document it qualifies, and one declared break beats a permanently confusing
///    argument order — so the version, not the signature, absorbs the incompatibility.
/// 3. `purrdf_serialize_jsonld_configured` gained `base_iri` after `media_type`, the
///    slot it occupies on `purrdf_serialize`.
/// 4. `purrdf_serialize` gained `out_directional_literals_dropped` and
///    `out_named_graph_rows_dropped` **before** `out_error`, so a `0.6.0` host passes
///    its `PurrdfError **` into a `size_t *` slot.
///
/// Those four were bundled into one bump rather than split across four, so a consumer
/// recompiled once for all of them; splitting would have broken the same consumer four
/// times for one reason.
///
/// # `0.7.0` → `0.8.0`: seventeen added symbols, eight changed ones and appended statuses
///
/// The prepared-shapes-product surface exports eight new entry points —
/// `purrdf_shapes_product_encode`, `_open`, `_admit`, `_admit_expecting`, `_rebuild`,
/// `_rebuild_expecting`, `_certify` and `_error_dimension` — and APPENDS
/// `PurrdfStatus::ShapesProductError = 11`. The SHACL change path exports a ninth,
/// `purrdf_shacl_validate_changes_to_sarif`, with its own `PurrdfShaclChangeScopeKind`
/// discriminant. The shapes-graph tools export three more —
/// `purrdf_shacl_apply_rules`, `purrdf_shacl_eval_node_expr` and
/// `purrdf_shacl_lint_shapes`.
///
/// The ninth to twelfth ride this SAME unreleased bump rather than a later one, exactly as the
/// `0.6.0` → `0.7.0` breaks were bundled: `0.8.0` has shipped in nothing, so there is
/// no library answering it that exports a different surface, and splitting would make a
/// consumer recompile twice for one reason. A symbol added AFTER `0.8.0` ships is a
/// different question, and the paragraph below is the answer to it.
///
/// Every one of those twelve is additive: no discriminant was renumbered, and a host
/// built against `0.7.0` calls each symbol it called before with the same arguments —
/// except `purrdf_shacl_validate_to_sarif` and the three conclusion-directed
/// `purrdf_entail_*` services, whose incompatible changes are described below.
///
/// It bumps anyway, and the reason is the sentence at the top of this comment rather
/// than a judgement about additivity. `0.7.0` SHIPPED — it is the ABI of the released
/// `2.0.0`, `2.0.1` and `2.0.2` libraries, which export seventeen fewer symbols than this
/// one does. Leaving the triple still would mean two different shippable libraries
/// answering `purrdf_abi_version` identically while exporting different surfaces, so a
/// host that compiled against this header and loaded the older library would be told
/// they agree and would then fail at symbol resolution. The minor exists precisely to
/// make that question answerable, and a number that cannot distinguish two shipped
/// libraries is not answering it. Additive changes are cheap for the CONSUMER, not free
/// for the VERSION.
///
/// The same unshipped bump also changes the signatures of eight symbols `0.7.0` shipped
/// (`purrdf_shacl_validate_to_sarif`, `purrdf_shacl_entail_to_ntriples`, the four
/// `purrdf_entail_*` services and `purrdf_query_entailment_governed`); every one is an
/// INCOMPATIBLE change, described in the paragraphs below. The first four:
/// `purrdf_shacl_validate_to_sarif` gained `conformance_disallows` /
/// `conformance_disallows_count` — the SHACL 1.2 conformance-disallow set — between
/// `data_nt` and `out_buffer`; and `purrdf_entail_certain_answers`,
/// `purrdf_entail_graph_entails` and `purrdf_entail_verify_entailment` each gained
/// `premise_iris` / `premise_iri_count` — the IRIs the premise document was read from, so
/// an `owl:imports` of the premise's own IRI resolves in place — between `import_count`
/// and `out_answer`. A host built against `0.7.0` must recompile; the bump they ride is
/// the one that already says so, rather than a second export for the same job.
///
/// The same unshipped bump carries the shapes graph's `owl:imports` table, too. Seven
/// shapes-graph entry points — `purrdf_shacl_validate_to_sarif`,
/// `purrdf_shacl_validate_changes_to_sarif`, `purrdf_shacl_entail_to_ntriples`,
/// `purrdf_shacl_apply_rules`, `purrdf_shacl_eval_node_expr`, `purrdf_shacl_lint_shapes`
/// and `purrdf_shapes_product_encode` — each gained `import_iris` / `import_documents` /
/// `import_count` before their out-parameters (incompatible: a `0.7.0` host passes its
/// out-pointer into `import_iris`); `PurrdfStatus::ShapesImportError = 12` is APPENDED;
/// and three accessors are added, `purrdf_shapes_import_error_kind`,
/// `purrdf_shapes_import_error_iri_count` and `purrdf_shapes_import_error_iri`. Every host
/// now refuses a shapes graph whose `owl:imports` closure is not in hand with the same
/// typed refusal, where the C surface used to validate the importing document alone.
///
/// `purrdf_shacl_eval_node_expr`, one of the symbols this bump adds, names its node
/// expression by one of three selectors — `expr`, `expr_at` with `expr_via` /
/// `expr_via_count`, or `expr_turtle` — each nullable, exactly one given.
///
/// The same unshipped bump carries the caller's evaluation limits. `purrdf_shacl_apply_rules`
/// gained `max_stored_facts` / `max_join_steps` between `max_generated_terms` and
/// `import_iris`, and `purrdf_entail_materialize_to_nquads` gained the same two between
/// `program` and `out_nquads` — each a nullable `const uint64_t *`, NULL for the target's
/// default. Both are incompatible (a `0.7.0` host passes its import table or its
/// out-pointer into the new slots), and both ride this bump for the reason the others do.
/// `purrdf_shacl_entail_to_ntriples` gained all four of `purrdf_shacl_apply_rules`' limits —
/// `max_term_generating_rounds`, `max_generated_terms`, `max_stored_facts` and
/// `max_join_steps`, each a nullable `const uint64_t *` — between `import_count` and
/// `out_buffer`, so an entailment run is bounded exactly as a rules run is. Incompatible
/// for the same reason, riding the same bump. Both rules entry points also gained a nullable
/// `PurrdfBuffer **out_diagnostics` immediately before `out_error` — the shapes graph's
/// mandatory diagnostics (one `diagnostic RULE SHAPE` line per empty `sh:in` / `sh:xone`
/// list), which every run reports — incompatible for the same reason (a `0.7.0` host passes
/// its `PurrdfError **` into the new slot), riding the same bump.
///
/// The same unshipped bump adds `purrdf_shacl_check_rules` — the check-only SPARQL 1.2 RL
/// entry point every host exposes, with its own `PurrdfSrlCheckLevel` discriminant — for
/// the reason the other added symbols ride it.
///
/// The same unshipped bump carries the shapes-graph IRI. `purrdf_shacl_validate_to_sarif`,
/// `purrdf_shacl_validate_changes_to_sarif`, `purrdf_shacl_lint_shapes` and
/// `purrdf_shapes_product_encode` each gained a nullable `shapes_graph_iri` immediately
/// after `shapes_base_iri` — the IRI SHACL-SPARQL's `$shapesGraph` is pre-bound to, which
/// `purrdf validate --shapes-graph` names and no C host could until now. Incompatible (a
/// `0.7.0` host passes its data or import table into the new slot); it rides this bump
/// for the reason the others do, and sits beside the base it resolves against for the
/// reason `shapes_base_iri` sits beside the document it qualifies. The two rules entry
/// points gained the same nullable `shapes_graph_iri` in the same place —
/// `purrdf_shacl_apply_rules` after `shapes_base_iri` and before `srl`, and
/// `purrdf_shacl_entail_to_ntriples` after `shapes_base_iri` and before `data_nt` — so a
/// `sh:SPARQLRule`'s `$shapesGraph` is pre-bound as `purrdf rules --shapes-graph` binds it.
///
/// The same unshipped bump carries SHACL 1.2 Core §6.3's `subClassOfInShapesGraph`:
/// `purrdf_shacl_validate_to_sarif` gained `bool subclass_of_in_shapes_graph` between
/// `import_count` and `out_buffer` — `false` is the specification's default. Incompatible
/// (a `0.7.0` host passes its out-pointer into the new slot); it rides this bump for the
/// reason the others do.
///
/// The same unshipped bump closes an entailment premise's `owl:imports` on every entailment
/// service. `purrdf_entail_materialize_to_nquads`, `purrdf_entail_consistency` and
/// `purrdf_query_entailment_governed` gained `import_iris` / `import_documents` /
/// `import_count` and `premise_iris` / `premise_iri_count`, and refuse an unresolved import
/// or an unused table entry as the other entailment services do. The three
/// conclusion-directed services and `purrdf_query_entailment_governed` gained
/// `max_stored_facts` / `max_join_steps`, each a nullable `const uint64_t *`, after the
/// premise IRIs; `purrdf_entail_consistency` keeps its `step_cap` / `work_cap`, which bound
/// its tableau. All incompatible (a `0.7.0` host passes an out-pointer into a new slot),
/// riding this bump. Two symbols this bump adds changed before shipping:
/// `purrdf_shacl_validate_changes_to_sarif` gained `conformance_disallows` /
/// `conformance_disallows_count` after `removed_nt` and `bool subclass_of_in_shapes_graph`
/// after `import_count`, matching `purrdf_shacl_validate_to_sarif`; and
/// `purrdf_shacl_eval_node_expr` gained a nullable `PurrdfBuffer **out_diagnostics`
/// before `out_error`, as the rules entry points did.
///
/// One of them is worth a second look regardless: appending a status is sound, but
/// RENUMBERING one is invisible to `tests/abi_signatures.rs`, which compares prototypes
/// and never sees an enumerator's value move. The discriminants are therefore pinned
/// separately, by `the_status_enum_is_append_only` in `tests/abi.rs`.
pub const PURRDF_ABI_MINOR: u32 = 8;
/// ABI patch version. Reset to `0` by the MINOR bump documented above.
pub const PURRDF_ABI_PATCH: u32 = 0;

/// Write the libpurrdf ABI version to `*out`.
///
/// # Safety
/// `out` must be null-checked-writable for one `PurrdfAbiVersion`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_abi_version(out: *mut PurrdfAbiVersion) -> i32 {
    unsafe {
        ffi_guard!(PurrdfStatus::Panic as i32, {
            if out.is_null() {
                return PurrdfStatus::NullPointer as i32;
            }
            *out = PurrdfAbiVersion {
                major: PURRDF_ABI_MAJOR,
                minor: PURRDF_ABI_MINOR,
                patch: PURRDF_ABI_PATCH,
            };
            PurrdfStatus::Ok as i32
        })
    }
}

/// Convert kernel capabilities to the `#[repr(C)]` flag struct.
fn capabilities_to_c(caps: RdfStoreCapabilities) -> PurrdfCapabilities {
    PurrdfCapabilities {
        named_graphs: u8::from(caps.named_graphs),
        quoted_triples: u8::from(caps.quoted_triples),
        reifiers: u8::from(caps.reifiers),
        annotations: u8::from(caps.annotations),
        source_locations: u8::from(caps.source_locations),
        loss_records: u8::from(caps.loss_records),
        lookaside: u8::from(caps.lookaside),
    }
}

/// Write the dataset's capability flags to `*out`.
///
/// # Safety
/// `dataset` must be a live handle; `out` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_capabilities(
    dataset: *const PurrdfDataset,
    out: *mut PurrdfCapabilities,
) -> i32 {
    unsafe {
        ffi_guard!(PurrdfStatus::Panic as i32, {
            if dataset.is_null() || out.is_null() {
                return PurrdfStatus::NullPointer as i32;
            }
            *out = capabilities_to_c(PurrdfDataset::arc(dataset).capabilities());
            PurrdfStatus::Ok as i32
        })
    }
}
