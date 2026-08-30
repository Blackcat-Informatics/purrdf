// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf_entail_*`: SPARQL entailment-**regime** materialization, the rule
//! inventories, and the OWL 2 Direct-Semantics reasoning services — all over the
//! shared [`PurrdfBuffer`].
//!
//! # Two lanes, two certificates
//!
//! [`purrdf_entail_materialize_to_nquads`] is the **chase**: it renders a
//! `purrdf-reasoning-report 4` block whose completeness is `exact` /
//! `sound-incomplete <n>` — a difference of two rule tables.
//!
//! [`purrdf_entail_consistency`], [`purrdf_entail_classify`],
//! [`purrdf_entail_realize`], [`purrdf_entail_instances`],
//! [`purrdf_entail_entails`], [`purrdf_entail_profile`],
//! [`purrdf_entail_extract_module`], [`purrdf_entail_justify`] and
//! [`purrdf_entail_explain_conclusion`] are the **tableau**: they render a
//! `purrdf-dl-certificate 1` block (or that service's own certificate grammar)
//! whose completeness is `decided` / `decided-within-boundaries` /
//! `budget-exhausted`. The DL lane has no rule table to subtract, so reusing the
//! chase's notion would report "exact" for a search that ran out of budget — which
//! is why the two banners differ and a consumer cannot parse one as the other.
//!
//! # Not to be confused with [`crate::shacl`]
//!
//! The two modules sit beside each other and both say "entail":
//!
//! * [`purrdf_shacl_entail_to_ntriples`](crate::shacl::purrdf_shacl_entail_to_ntriples)
//!   is **SHACL-AF `sh:rule`** entailment. It needs a *shapes* graph and applies
//!   the rules that graph declares.
//! * [`purrdf_entail_materialize_to_nquads`] is **SPARQL entailment-regime**
//!   materialization over all seven regimes (`simple` / `rdf` / `rdfs` /
//!   `owl-rl` / `owl-direct` / `rif` / `d`). It takes no shapes at all: it
//!   closes a document under the regime's own rule table (or, for `rif`, the
//!   caller's).
//!
//! # One boundary, three hosts
//!
//! Nothing here reimplements the parse → close → serialize sequence. Every entry
//! point routes through [`purrdf_validate::regime`], the same string boundary the
//! WASM and PyO3 hosts call, so a byte difference between the three hosts is one
//! shared golden vector failing rather than three surfaces that quietly stopped
//! agreeing. This module adds the C-ABI byte framing and nothing else.
//!
//! # Ownership
//!
//! Exactly as
//! [`purrdf_shacl_entail_to_ntriples`](crate::shacl::purrdf_shacl_entail_to_ntriples):
//! each `PurrdfBuffer` the library hands out is released with
//! `purrdf_buffer_free`, and its bytes are read (borrowed, never freed by the
//! caller) with `purrdf_buffer_data`. `purrdf_entail_materialize_to_nquads`
//! hands out **two** independent buffers, so it needs **two** frees; on any error
//! it writes neither out-param and there is nothing to free.

use std::os::raw::c_char;

use purrdf_validate::regime::{
    ReasonerSession, ReasoningAnswer, certain_answers_to_string, check_dl_proof,
    classify_to_string, consistency_to_string, entails_to_string, explain_conclusion_to_string,
    extension_rules_string, extract_module_to_string, graph_entails_to_string,
    implemented_rules_string, instances_to_string, justify_to_string, materialize_to_nquads_string,
    profile_to_string, prove_to_string, realize_to_string, rules_string,
    verify_entailment_to_string,
};

use crate::buffer::PurrdfBuffer;
use crate::cstr_to_str;
use crate::error::PurrdfError;
use crate::status::PurrdfStatus;

/// Close `document` (N-Quads, which accepts N-Triples) under the regime spelled
/// `regime`, returning the canonical N-Quads closure and the rendered reasoning
/// report as two byte vectors. Native-testable, pointer-free core.
///
/// The parse → close → canonicalize → render sequence lives in
/// [`materialize_to_nquads_string`]; this only adds the C-ABI byte framing.
fn materialize_to_nquads_bytes(
    document: &str,
    regime: &str,
    program: &str,
) -> Result<(Vec<u8>, Vec<u8>), String> {
    let (nquads, report) = materialize_to_nquads_string(regime, document, program)?.into_parts();
    Ok((nquads.into_bytes(), report.into_bytes()))
}

/// Materialize an RDF document under a SPARQL entailment regime.
///
/// `document` is parsed as N-Quads, which accepts an N-Triples document
/// unchanged, so a document that names a graph keeps naming it. `regime` is one
/// of `simple`, `rdf`, `rdfs`, `owl-rl`, `owl-direct`, `rif`, `d` — the same
/// spellings the CLI, WASM and the Python surface accept — and ALL SEVEN
/// materialize; none is refused for being the regime it is.
///
/// `program` is the regime's own rule document. `rif` entails under the CALLER's
/// rules, so for that spelling `program` is a normative RIF-in-XML document (an
/// `Import` is refused: resolving one is I/O this boundary does not perform).
/// Every other regime's rule table is the specification's, so `program` must be
/// the empty string `""` — a non-empty one is an ERROR rather than a silently
/// discarded argument, because a caller who passed rules to `rdfs` believes they
/// ran.
///
/// On success `*out_nquads` receives the canonical (RDFC-1.0) N-Quads closure —
/// every input quad plus every triple the regime's implemented rules infer — and
/// `*out_report` receives the byte-stable rendered reasoning report, which names
/// which rules fired and how often, which specification rules did NOT fire, which
/// constructs were left at a boundary, the evaluation budget, and the calculus's
/// contract hash. **Free BOTH with `purrdf_buffer_free`.** The report is not
/// optional: all seventy-eight OWL 2 RL rules now run, so a caller that reported
/// "OWL-RL entailment" without saying which CONSTRUCTS were left at a boundary
/// would be making exactly the overclaim the report exists to prevent — a
/// complete rule table is not a complete closure.
///
/// On any error neither out-param is written, so there is nothing to free.
///
/// # Safety
/// `document`, `regime` and `program` must be non-null, NUL-terminated C strings;
/// `out_nquads` and `out_report` must be writable pointers; `out_error` must be
/// null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_entail_materialize_to_nquads(
    document: *const c_char,
    regime: *const c_char,
    program: *const c_char,
    out_nquads: *mut *mut PurrdfBuffer,
    out_report: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: the caller's contract above — `document`/`regime`/`program` are
    // NUL-terminated C strings (or null, checked here), and the three out-pointers are
    // writable (or null, checked here). The two `*out_* =` writes happen only after every
    // fallible step has succeeded, so a failing call leaves both untouched and
    // hands out no buffer the caller would have to free.
    unsafe {
        ffi_try!(out_error, {
            if document.is_null()
                || regime.is_null()
                || program.is_null()
                || out_nquads.is_null()
                || out_report.is_null()
            {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_entail_materialize_to_nquads",
                ));
            }
            let document = cstr_to_str(document)?;
            let regime = cstr_to_str(regime)?;
            let program = cstr_to_str(program)?;
            let (nquads, report) = materialize_to_nquads_bytes(document, regime, program)
                .map_err(|message| PurrdfError::new(PurrdfStatus::ParseError, message))?;
            *out_nquads = PurrdfBuffer::into_raw(nquads);
            *out_report = PurrdfBuffer::into_raw(report);
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// The rule table `regime` is *defined by*, one name per line. Native-testable,
/// pointer-free core.
fn rules_bytes(regime: &str) -> Result<Vec<u8>, String> {
    Ok(rules_string(regime)?.into_bytes())
}

/// Write the rule table the specification *defines* `regime` by — one canonical
/// rule name per newline-terminated line, in specification table order — to
/// `*out_buffer` (free with `purrdf_buffer_free`).
///
/// An empty buffer for a regime with no rule table of its own (`simple`, plus
/// `owl-direct`, which decides through the tableau, and `rif`, which entails under the
/// caller's rules — all three still MATERIALIZE). `owl-rl` yields all 78 rules of OWL 2
/// Profiles §4.3 Tables 4–9 whether or not this build fires them — that is the
/// point: diff it against `purrdf_entail_implemented_rules` to measure the gap.
///
/// # Safety
/// `regime` must be a non-null, NUL-terminated C string; `out_buffer` must be a
/// writable pointer; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_entail_rules(
    regime: *const c_char,
    out_buffer: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: as `purrdf_entail_materialize_to_nquads` — `regime` is a
    // NUL-terminated C string (or null, checked here), `out_buffer`/`out_error`
    // are writable (or null, checked here), and `*out_buffer` is written only
    // after the lookup has succeeded.
    unsafe {
        ffi_try!(out_error, {
            if regime.is_null() || out_buffer.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_entail_rules",
                ));
            }
            let regime = cstr_to_str(regime)?;
            let bytes = rules_bytes(regime)
                .map_err(|message| PurrdfError::new(PurrdfStatus::ParseError, message))?;
            *out_buffer = PurrdfBuffer::into_raw(bytes);
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// The subset of the rule table this build fires, one name per line.
/// Native-testable, pointer-free core.
fn implemented_rules_bytes(regime: &str) -> Result<Vec<u8>, String> {
    Ok(implemented_rules_string(regime)?.into_bytes())
}

/// Write the subset of `purrdf_entail_rules(regime)` this build's chase actually
/// fires today to `*out_buffer` (free with `purrdf_buffer_free`).
///
/// `purrdf_entail_rules(r)` minus `purrdf_entail_implemented_rules(r)` is the
/// regime's measurable gap — the same set the rendered report's `missing` lines
/// name.
///
/// # Safety
/// `regime` must be a non-null, NUL-terminated C string; `out_buffer` must be a
/// writable pointer; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_entail_implemented_rules(
    regime: *const c_char,
    out_buffer: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: identical contract to `purrdf_entail_rules` above.
    unsafe {
        ffi_try!(out_error, {
            if regime.is_null() || out_buffer.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_entail_implemented_rules",
                ));
            }
            let regime = cstr_to_str(regime)?;
            let bytes = implemented_rules_bytes(regime)
                .map_err(|message| PurrdfError::new(PurrdfStatus::ParseError, message))?;
            *out_buffer = PurrdfBuffer::into_raw(bytes);
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// The rules this build fires beyond the specification table, one name per line.
/// Native-testable, pointer-free core.
fn extensions_bytes(regime: &str) -> Result<Vec<u8>, String> {
    Ok(extension_rules_string(regime)?.into_bytes())
}

/// Write the rules this build fires BEYOND `regime`'s specification table to
/// `*out_buffer` (free with `purrdf_buffer_free`).
///
/// Disjoint from both `purrdf_entail_rules(regime)` and
/// `purrdf_entail_implemented_rules(regime)`: the normative table is a statement
/// about the specification and does not move because this build fires a sound rule
/// the table happens not to list. A rendered report names the same rules on its
/// `extension` line — this answers the question without materializing a dataset
/// first. Empty for a lane with nothing added to it.
///
/// # Safety
/// `regime` must be a non-null, NUL-terminated C string; `out_buffer` must be a
/// writable pointer; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_entail_extensions(
    regime: *const c_char,
    out_buffer: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: identical contract to `purrdf_entail_rules` above.
    unsafe {
        ffi_try!(out_error, {
            if regime.is_null() || out_buffer.is_null() {
                return Err(PurrdfError::new(
                    PurrdfStatus::NullPointer,
                    "null pointer argument to purrdf_entail_extensions",
                ));
            }
            let regime = cstr_to_str(regime)?;
            let bytes = extensions_bytes(regime)
                .map_err(|message| PurrdfError::new(PurrdfStatus::ParseError, message))?;
            *out_buffer = PurrdfBuffer::into_raw(bytes);
            Ok(PurrdfStatus::Ok)
        })
    }
}

// ── The Description-Logic reasoning services ────────────────────────────────
//
// Nine entry points, one shape: `(inputs…, out_answer, out_certificate,
// out_error)`. Each hands out **two** independent buffers, so each needs **two**
// `purrdf_buffer_free` calls; on any error neither out-param is written and there
// is nothing to free.
//
// The certificate is a separate buffer rather than a field of the answer for the
// same reason `purrdf_entail_materialize_to_nquads` hands out its report
// separately: a caller cannot read the answer without also being handed the
// evidence for how completely it was decided.
//
// They are written out one by one rather than generated by a macro because
// `purrdf.h` is a FROZEN ABI contract generated by cbindgen, which reads the
// source rather than the expansion — a macro-generated entry point would be a
// symbol in the library with no declaration in the header.

/// Write a boundary answer to its two out-params, or map its message to an error.
///
/// # Safety
/// `out_answer` and `out_certificate` must be non-null, writable pointers. Both
/// writes happen only after every fallible step has succeeded.
unsafe fn store_answer(
    produced: Result<ReasoningAnswer, String>,
    out_answer: *mut *mut PurrdfBuffer,
    out_certificate: *mut *mut PurrdfBuffer,
) -> Result<PurrdfStatus, PurrdfError> {
    let (answer, certificate) = produced
        .map_err(|message| PurrdfError::new(PurrdfStatus::ParseError, message))?
        .into_parts();
    // SAFETY: the caller's contract above — both pointers are non-null and
    // writable, and nothing fallible remains between here and the two writes.
    unsafe {
        *out_answer = PurrdfBuffer::into_raw(answer.into_bytes());
        *out_certificate = PurrdfBuffer::into_raw(certificate.into_bytes());
    }
    Ok(PurrdfStatus::Ok)
}

/// The error a null argument to `entry` is refused with, rather than dereferenced.
fn null_argument(entry: &str) -> PurrdfError {
    PurrdfError::new(
        PurrdfStatus::NullPointer,
        format!("null pointer argument to {entry}"),
    )
}

/// Decide whether an ontology has a model at all.
///
/// `document` is parsed as N-Quads (which accepts N-Triples). `step_cap` narrows
/// the per-decision tableau step cap; **0 means the knowledge base's own cap**, not
/// a cap of zero steps. The cap can only NARROW — a value above the knowledge
/// base's own ceiling has no effect — so this cannot be used to make a hard
/// instance answerable, only to make the `budget-exhausted` certificate reachable.
///
/// `work_cap` narrows the per-decision WORK cap on the same rules — **0 means the
/// knowledge base's own cap**, and it can only NARROW. It bounds what `step_cap`
/// structurally cannot: a round is a PASS over the completion graph rather than a
/// unit of cost, so an ontology can make each round enormously more expensive
/// without making the search take more rounds. A run that reaches it answers
/// `unknown` with `work` equal to `work-budget` in its certificate.
///
/// On success `*out_answer` receives `consistency true|false|unknown` and
/// `*out_certificate` the rendered `purrdf-dl-certificate 1` block, which says
/// whether the search was `decided`, `decided-within-boundaries` (some axiom of the
/// supplied ontology never became a DL clause — the certificate names each
/// construct) or `budget-exhausted`. **Free BOTH with `purrdf_buffer_free`.**
///
/// This is the only DL service that answers for an unsatisfiable ontology, because
/// it is the one that detects one; every other refuses rather than returning the
/// vacuous answer an ontology with no model gives.
///
/// # Safety
/// `document` must be a non-null, NUL-terminated C string; `out_answer` and
/// `out_certificate` must be writable pointers; `out_error` must be null or
/// writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_entail_consistency(
    document: *const c_char,
    step_cap: u32,
    work_cap: u32,
    out_answer: *mut *mut PurrdfBuffer,
    out_certificate: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: the caller's contract above; the out-params are written only by
    // `store_answer`, after the boundary call has succeeded.
    unsafe {
        ffi_try!(out_error, {
            if document.is_null() || out_answer.is_null() || out_certificate.is_null() {
                return Err(null_argument("purrdf_entail_consistency"));
            }
            let document = cstr_to_str(document)?;
            store_answer(
                consistency_to_string(document, step_cap, work_cap),
                out_answer,
                out_certificate,
            )
        })
    }
}

/// Classify: the entailed subsumption hierarchy over the ontology's named classes.
///
/// `*out_answer` receives `equivalent`, `subclass`, `direct` and `unsatisfiable`
/// lines (in that block order); `*out_certificate` the `purrdf-dl-certificate 1`
/// block. **Free BOTH with `purrdf_buffer_free`.** `step_cap` and `work_cap` behave
/// exactly as in [`purrdf_entail_consistency`].
///
/// Costs one tableau decision per ORDERED pair of named classes plus the
/// consistency check, which the certificate's `decisions` line reports so the cost
/// is a measurement rather than a surprise.
///
/// # Safety
/// As [`purrdf_entail_consistency`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_entail_classify(
    document: *const c_char,
    step_cap: u32,
    work_cap: u32,
    out_answer: *mut *mut PurrdfBuffer,
    out_certificate: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: identical contract to `purrdf_entail_consistency`.
    unsafe {
        ffi_try!(out_error, {
            if document.is_null() || out_answer.is_null() || out_certificate.is_null() {
                return Err(null_argument("purrdf_entail_classify"));
            }
            let document = cstr_to_str(document)?;
            store_answer(
                classify_to_string(document, step_cap, work_cap),
                out_answer,
                out_certificate,
            )
        })
    }
}

/// Realize: the entailed types of the ontology's named individuals, and the most
/// specific of them.
///
/// `*out_answer` receives `type` lines followed by `direct-type` lines;
/// `*out_certificate` the `purrdf-dl-certificate 1` block. **Free BOTH with
/// `purrdf_buffer_free`.** `step_cap` and `work_cap` behave exactly as in
/// [`purrdf_entail_consistency`].
///
/// # Safety
/// As [`purrdf_entail_consistency`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_entail_realize(
    document: *const c_char,
    step_cap: u32,
    work_cap: u32,
    out_answer: *mut *mut PurrdfBuffer,
    out_certificate: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: identical contract to `purrdf_entail_consistency`.
    unsafe {
        ffi_try!(out_error, {
            if document.is_null() || out_answer.is_null() || out_certificate.is_null() {
                return Err(null_argument("purrdf_entail_realize"));
            }
            let document = cstr_to_str(document)?;
            store_answer(
                realize_to_string(document, step_cap, work_cap),
                out_answer,
                out_certificate,
            )
        })
    }
}

/// Instance retrieval: the named individuals entailed to be instances of `class`.
///
/// `class` is ONE N-Triples term — `<iri>` or `_:label`, angle brackets included. A
/// class the ontology never mentions is not an error: nothing constrains it, so the
/// empty answer for it is a real answer.
///
/// `*out_answer` receives `instance <term>` lines; `*out_certificate` the
/// `purrdf-dl-certificate 1` block. **Free BOTH with `purrdf_buffer_free`.**
/// `step_cap` and `work_cap` behave exactly as in [`purrdf_entail_consistency`].
///
/// # Safety
/// `document` and `class` must be non-null, NUL-terminated C strings; `out_answer`
/// and `out_certificate` must be writable pointers; `out_error` must be null or
/// writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_entail_instances(
    document: *const c_char,
    class: *const c_char,
    step_cap: u32,
    work_cap: u32,
    out_answer: *mut *mut PurrdfBuffer,
    out_certificate: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: as `purrdf_entail_consistency`, with one more borrowed C string.
    unsafe {
        ffi_try!(out_error, {
            if document.is_null()
                || class.is_null()
                || out_answer.is_null()
                || out_certificate.is_null()
            {
                return Err(null_argument("purrdf_entail_instances"));
            }
            let document = cstr_to_str(document)?;
            let class = cstr_to_str(class)?;
            store_answer(
                instances_to_string(document, class, step_cap, work_cap),
                out_answer,
                out_certificate,
            )
        })
    }
}

/// Axiom entailment: does the ontology entail `axiom`?
///
/// `axiom` is ONE triple of the OWL 2 RDF mapping, in N-Triples syntax. Seven
/// reserved predicates select the seven named axiom kinds — `rdfs:subClassOf`,
/// `owl:equivalentClass`, `owl:disjointWith`, `rdf:type`, `owl:sameAs`,
/// `owl:differentFrom`, `rdfs:subPropertyOf` — and any other predicate is an
/// object-property assertion. No encoding is invented: this is the mapping the
/// reasoner's own reverse mapping reads.
///
/// `*out_answer` receives `entails true|false|unknown` and then the axiom as it was
/// READ (`axiom <kind>` plus one `term` line each), so a caller can see which axiom
/// its predicate selected. `*out_certificate` receives the
/// `purrdf-dl-certificate 1` block. **Free BOTH with `purrdf_buffer_free`.**
///
/// # Safety
/// `document` and `axiom` must be non-null, NUL-terminated C strings; `out_answer`
/// and `out_certificate` must be writable pointers; `out_error` must be null or
/// writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_entail_entails(
    document: *const c_char,
    axiom: *const c_char,
    step_cap: u32,
    work_cap: u32,
    out_answer: *mut *mut PurrdfBuffer,
    out_certificate: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: as `purrdf_entail_instances`.
    unsafe {
        ffi_try!(out_error, {
            if document.is_null()
                || axiom.is_null()
                || out_answer.is_null()
                || out_certificate.is_null()
            {
                return Err(null_argument("purrdf_entail_entails"));
            }
            let document = cstr_to_str(document)?;
            let axiom = cstr_to_str(axiom)?;
            store_answer(
                entails_to_string(document, axiom, step_cap, work_cap),
                out_answer,
                out_certificate,
            )
        })
    }
}

/// Certify the ontology against the OWL 2 profiles.
///
/// Purely syntactic — no tableau, no closure, no budget — so this is the one
/// reasoning service whose certificate is NOT a `purrdf-dl-certificate`: there is
/// no search whose completeness could be reported, and rendering a fabricated one
/// would be exactly the overclaim the certificates exist to prevent.
///
/// `*out_answer` receives `certified <profile>` lines, most restrictive first (EL,
/// QL, RL, DL, Full). `*out_certificate` receives the
/// `purrdf-owl-profile-certificate 1` block: every violation with its blocking
/// term, the node it was written on and the reason, a dense `certifies-<profile>`
/// line per profile, and the `one-directional true` gate — a certification PROVES
/// membership, a violation does NOT prove non-membership. **Free BOTH with
/// `purrdf_buffer_free`.**
///
/// # Safety
/// `document` must be a non-null, NUL-terminated C string; `out_answer` and
/// `out_certificate` must be writable pointers; `out_error` must be null or
/// writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_entail_profile(
    document: *const c_char,
    out_answer: *mut *mut PurrdfBuffer,
    out_certificate: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: as `purrdf_entail_consistency`, with no step cap to read.
    unsafe {
        ffi_try!(out_error, {
            if document.is_null() || out_answer.is_null() || out_certificate.is_null() {
                return Err(null_argument("purrdf_entail_profile"));
            }
            let document = cstr_to_str(document)?;
            store_answer(profile_to_string(document), out_answer, out_certificate)
        })
    }
}

/// Extract the locality module of the ontology for a seed signature.
///
/// `signature` is one N-Triples term per line (blank lines ignored). `method` is
/// `bot`, `top` or `star`; an unknown spelling is refused with the accepted set
/// named.
///
/// `*out_answer` receives the extracted module as canonical (RDFC-1.0) N-Quads —
/// the same serializer `purrdf_entail_materialize_to_nquads` uses.
/// `*out_certificate` receives the `purrdf-module-extraction 1` block: the method,
/// the axiom count, the signature the fixpoint CLOSED to, every triple kept
/// conservatively, and the `conservative` gate that says whether the module is the
/// minimal one or a sound superset. **Free BOTH with `purrdf_buffer_free`.**
///
/// # Safety
/// `document`, `signature` and `method` must be non-null, NUL-terminated C strings;
/// `out_answer` and `out_certificate` must be writable pointers; `out_error` must
/// be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_entail_extract_module(
    document: *const c_char,
    signature: *const c_char,
    method: *const c_char,
    out_answer: *mut *mut PurrdfBuffer,
    out_certificate: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: as `purrdf_entail_instances`, with two more borrowed C strings.
    unsafe {
        ffi_try!(out_error, {
            if document.is_null()
                || signature.is_null()
                || method.is_null()
                || out_answer.is_null()
                || out_certificate.is_null()
            {
                return Err(null_argument("purrdf_entail_extract_module"));
            }
            let document = cstr_to_str(document)?;
            let signature = cstr_to_str(signature)?;
            let method = cstr_to_str(method)?;
            store_answer(
                extract_module_to_string(document, signature, method),
                out_answer,
                out_certificate,
            )
        })
    }
}

/// Justify a Description-Logic axiom: a minimal subset of the ontology that still
/// entails it.
///
/// A tableau performs no derivation steps, so this is a JUSTIFICATION and
/// deliberately not called a proof; `purrdf_entail_explain_conclusion` is the chase
/// lane's genuinely derivational explanation, and the two are different kinds of
/// thing rather than two spellings of one.
///
/// `axiom` is read exactly as `purrdf_entail_entails` reads it.
///
/// `*out_answer` receives the justification's axioms as canonical (RDFC-1.0)
/// N-Quads — a justification introduces no term, so it is an ordinary RDF 1.2
/// dataset of axioms already present in the input. `*out_certificate` receives the
/// `purrdf-justification 1` block, whose `sufficient` and `minimal` lines are
/// **re-decided** over the justification alone and over each of its
/// one-axiom-smaller subsets, so they check the answer rather than restate it.
/// **Free BOTH with `purrdf_buffer_free`.**
///
/// An ontology that does not entail the axiom is an error, not an empty
/// justification: the empty set reads as "nothing is needed" and means the
/// opposite.
///
/// # Safety
/// `document` and `axiom` must be non-null, NUL-terminated C strings; `out_answer`
/// and `out_certificate` must be writable pointers; `out_error` must be null or
/// writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_entail_justify(
    document: *const c_char,
    axiom: *const c_char,
    out_answer: *mut *mut PurrdfBuffer,
    out_certificate: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: as `purrdf_entail_entails`, with no step cap to read.
    unsafe {
        ffi_try!(out_error, {
            if document.is_null()
                || axiom.is_null()
                || out_answer.is_null()
                || out_certificate.is_null()
            {
                return Err(null_argument("purrdf_entail_justify"));
            }
            let document = cstr_to_str(document)?;
            let axiom = cstr_to_str(axiom)?;
            store_answer(
                justify_to_string(document, axiom),
                out_answer,
                out_certificate,
            )
        })
    }
}

/// Explain one triple of a chase closure: which rules, from which premises.
///
/// `regime` is one of the same spellings every other entry point accepts.
/// `conclusion` is ONE N-Quads statement; its graph, if it names one, selects the
/// closure to explain.
///
/// `*out_answer` receives `asserted`, `steps` and one `rule` line per cited rule.
/// `*out_certificate` receives the `purrdf-chase-proof 1` block, whose `derived-*`
/// lines are what the CHECKER re-derived from the proof term and the clause
/// program — not what the proof claims — so a proof whose stated conclusion its own
/// premises do not license shows up as differing lines rather than a silent
/// `checked true`. **Free BOTH with `purrdf_buffer_free`.**
///
/// `rdf` and `rdfs` are refused by name: four of their rules conclude about a FRESH
/// blank node, an existential head has no Datalog semantics, and a "proof" of such
/// a step could only be believed.
///
/// # Safety
/// `document`, `regime` and `conclusion` must be non-null, NUL-terminated C
/// strings; `out_answer` and `out_certificate` must be writable pointers;
/// `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_entail_explain_conclusion(
    document: *const c_char,
    regime: *const c_char,
    conclusion: *const c_char,
    out_answer: *mut *mut PurrdfBuffer,
    out_certificate: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: as `purrdf_entail_extract_module`.
    unsafe {
        ffi_try!(out_error, {
            if document.is_null()
                || regime.is_null()
                || conclusion.is_null()
                || out_answer.is_null()
                || out_certificate.is_null()
            {
                return Err(null_argument("purrdf_entail_explain_conclusion"));
            }
            let document = cstr_to_str(document)?;
            let regime = cstr_to_str(regime)?;
            let conclusion = cstr_to_str(conclusion)?;
            store_answer(
                explain_conclusion_to_string(document, regime, conclusion),
                out_answer,
                out_certificate,
            )
        })
    }
}

// ── The conclusion-directed entailment services ─────────────────────────────────
//
// Written out one entry point at a time rather than generated from a macro: cbindgen
// parses the source and does NOT expand macros, so a macro-generated `#[no_mangle]`
// would compile, link and ship — and never appear in the committed header, which is the
// definition of a dark capability on this host.

/// Read the caller's `owl:imports` table out of two parallel C arrays.
///
/// Entry `i` declares that the ontology IRI `import_iris[i]` denotes the N-Quads document
/// `import_documents[i]`. Two arrays rather than an array of structs because a struct
/// crossing this ABI is a layout the caller has to reproduce; two `const char *const *`
/// and a count are what a C caller already knows how to build, and the ORDER is the
/// caller's — the boundary's table is a list rather than a map precisely so the same input
/// always produces the same run.
///
/// `count == 0` with two NULL arrays is the ordinary "imports nothing" case and is
/// ACCEPTED: there is nothing to dereference, so refusing it would make the common call
/// pass two dummy arrays. A NULL array with a non-zero count is a caller error and is
/// refused BEFORE any dereference, as is a NULL element inside a non-empty array.
///
/// # Safety
/// When `count` is non-zero, `import_iris` and `import_documents` must each address at
/// least `count` readable `*const c_char`, every one of which is null (refused here) or a
/// NUL-terminated C string that outlives the returned borrows.
unsafe fn import_pairs<'a>(
    import_iris: *const *const c_char,
    import_documents: *const *const c_char,
    count: usize,
    entry: &str,
) -> Result<Vec<(&'a str, &'a str)>, PurrdfError> {
    if count == 0 {
        return Ok(Vec::new());
    }
    if import_iris.is_null() || import_documents.is_null() {
        return Err(PurrdfError::new(
            PurrdfStatus::NullPointer,
            format!("null import array with a non-zero import count ({count}) passed to {entry}"),
        ));
    }
    let mut pairs = Vec::with_capacity(count);
    for index in 0..count {
        // SAFETY: the caller's contract above — both arrays are non-null (checked) and
        // hold at least `count` readable elements, so `index < count` is in bounds. Each
        // element is handed to `cstr_to_str`, which refuses a null pointer rather than
        // dereferencing it.
        let (iri, document) = unsafe {
            (
                cstr_to_str(*import_iris.add(index))?,
                cstr_to_str(*import_documents.add(index))?,
            )
        };
        pairs.push((iri, document));
    }
    Ok(pairs)
}

/// The certain answers of a basic graph pattern under an entailment regime.
///
/// A certain answer is a substitution the knowledge base ENTAILS the pattern under —
/// true in every model, not merely present in one closure — which is what SPARQL's
/// entailment regimes define the answers to a basic graph pattern to be.
///
/// `regime` is one of the same spellings every other entry point accepts, minus the two
/// this service is not total over: `owl-direct` is query-directed and `rif` entails under
/// a rule document, and each is defined by an input this signature does not carry, so
/// both are refused by name rather than served by a weaker lane.
///
/// `pattern` is N-Triples with `?name` in any position, the PREDICATE included. A blank
/// node in it is a NON-DISTINGUISHED variable — constrained by the match, not projected,
/// and not a column — which is what SPARQL says a query blank node is. A variable inside an
/// RDF 1.2 triple term is an ordinary variable: it binds, it is a column, and one NAME is
/// one VARIABLE wherever it was written, so a pattern using it above and below the
/// triple-term boundary is joined rather than split into two. A predicate variable
/// is projected like any other, and under `owl-rl` it also renders a `limit`: it ranges over
/// the whole predicate vocabulary, including the schema predicates and the constructs the
/// mechanisms beyond the rule table decide, and the closure holds neither.
///
/// `*out_answer` receives `mechanism`, one `var` line per projected variable, one `row`
/// line per certain answer, and a `limit` line per reason the row set may not be
/// EXHAUSTIVE. Every row is sound unconditionally; what needs a precondition is the claim
/// about a row that is NOT there, so no `limit` lines is the claim that the row set is
/// complete. `*out_certificate` receives the run's `purrdf-reasoning-report 4` block.
/// **Free BOTH with `purrdf_buffer_free`.**
///
/// A pattern with a projected variable is `mechanism strict-table`: the five mechanisms
/// beyond the rule table are not run for one, because a projected variable over what any of
/// them decides is a different question — and that one of them WOULD have been needed
/// arrives as a `limit` line naming the lane, never as an exhaustive empty answer. A
/// pattern with NO projected variable is a conclusion graph, is answered by the same fold
/// `purrdf_entail_graph_entails` runs, and names whichever of the seven reached it; such an
/// answer is the relation with no columns, so a `yes` is one bare `row` line and a `no` is
/// none.
///
/// `import_iris` and `import_documents` are the caller's `owl:imports` table, as two
/// parallel arrays of `import_count` C strings: entry `i` declares that the ontology IRI
/// `import_iris[i]` denotes the N-Quads document `import_documents[i]`. A premise carrying
/// an `owl:imports` states that its axioms are its own PLUS those of the documents it names,
/// so this is where those documents arrive — and the `owl:imports` triple stays exactly
/// where the caller wrote it. **PurRDF fetches nothing**: an ontology IRI the table does not
/// resolve is an error naming the document, never a network access and never a silently
/// empty import. `import_count == 0` with two NULL arrays is the ordinary "imports nothing"
/// case and is accepted; a NULL array with a non-zero count is a caller error and is
/// refused, never dereferenced. Resolution is transitive to a fixpoint.
///
/// # Safety
/// `regime`, `document` and `pattern` must be non-null, NUL-terminated C strings; when
/// `import_count` is non-zero, `import_iris` and `import_documents` must each address at
/// least `import_count` readable, non-null, NUL-terminated C strings; `out_answer` and
/// `out_certificate` must be writable pointers; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_entail_certain_answers(
    regime: *const c_char,
    document: *const c_char,
    pattern: *const c_char,
    import_iris: *const *const c_char,
    import_documents: *const *const c_char,
    import_count: usize,
    out_answer: *mut *mut PurrdfBuffer,
    out_certificate: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: as `purrdf_entail_explain_conclusion`, plus the two import arrays, which
    // `import_pairs` reads only after establishing that a non-zero count has non-null
    // arrays behind it.
    unsafe {
        ffi_try!(out_error, {
            if regime.is_null()
                || document.is_null()
                || pattern.is_null()
                || out_answer.is_null()
                || out_certificate.is_null()
            {
                return Err(null_argument("purrdf_entail_certain_answers"));
            }
            let regime = cstr_to_str(regime)?;
            let document = cstr_to_str(document)?;
            let pattern = cstr_to_str(pattern)?;
            let imports = import_pairs(
                import_iris,
                import_documents,
                import_count,
                "purrdf_entail_certain_answers",
            )?;
            store_answer(
                certain_answers_to_string(regime, document, pattern, &imports),
                out_answer,
                out_certificate,
            )
        })
    }
}

/// Does `premise` entail the conclusion GRAPH under `regime`'s rule table?
///
/// NOT `purrdf_entail_entails`, which asks the OWL 2 Direct-Semantics TABLEAU about one
/// AXIOM and renders a `purrdf-dl-certificate 1` block. This asks the regime's RULE TABLE
/// about a conclusion GRAPH and renders a `purrdf-reasoning-report 4` one. Different
/// question, different calculus, different certificate — and the two banners differ so
/// neither can be parsed as the other.
///
/// `*out_answer` opens `mechanism <name>`: WHICH of the seven mechanisms reached the
/// verdict. `strict-table` is the regime's own rule table, run once, with the conclusion
/// matched into (or proven absent from) its closure; the other five —  `refutation`,
/// `freeze`, `comprehension`, `reflexivity`, `data-range` — exist because that table DECIDES
/// no conclusion of that shape. `composite` is two or more of those folded over
/// one conclusion, which a conjunction can need and which is spelled that way rather than by
/// any one constituent's name. The name is the canonical spelling and never an enum ordinal,
/// so an eighth mechanism cannot renumber a reading of an old one.
///
/// Then THREE verdicts, never two: `entailment entailed` (with one `binding` line per
/// existential of the conclusion), `entailment not-entailed` (a PROOF — the procedure was
/// complete for this premise — with a `miss` line), or `entailment undecided` (what an
/// incomplete procedure is entitled to say instead, with an `undecided` line naming which
/// hypothesis of which theorem the input broke). Reading the third as the second would
/// turn a limitation of this library into a false statement about the caller's data.
/// **Free BOTH buffers with `purrdf_buffer_free`.**
///
/// `import_iris`, `import_documents` and `import_count` are
/// `purrdf_entail_certain_answers`'s, and apply to the PREMISE: the conclusion is a graph to
/// match rather than an ontology to close, so an `owl:imports` in it names nothing this
/// service resolves.
///
/// # Safety
/// `regime`, `premise` and `conclusion` must be non-null, NUL-terminated C strings; when
/// `import_count` is non-zero, `import_iris` and `import_documents` must each address at
/// least `import_count` readable, non-null, NUL-terminated C strings; `out_answer` and
/// `out_certificate` must be writable pointers; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_entail_graph_entails(
    regime: *const c_char,
    premise: *const c_char,
    conclusion: *const c_char,
    import_iris: *const *const c_char,
    import_documents: *const *const c_char,
    import_count: usize,
    out_answer: *mut *mut PurrdfBuffer,
    out_certificate: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: as `purrdf_entail_certain_answers`.
    unsafe {
        ffi_try!(out_error, {
            if regime.is_null()
                || premise.is_null()
                || conclusion.is_null()
                || out_answer.is_null()
                || out_certificate.is_null()
            {
                return Err(null_argument("purrdf_entail_graph_entails"));
            }
            let regime = cstr_to_str(regime)?;
            let premise = cstr_to_str(premise)?;
            let conclusion = cstr_to_str(conclusion)?;
            let imports = import_pairs(
                import_iris,
                import_documents,
                import_count,
                "purrdf_entail_graph_entails",
            )?;
            store_answer(
                graph_entails_to_string(regime, premise, conclusion, &imports),
                out_answer,
                out_certificate,
            )
        })
    }
}

/// `purrdf_entail_graph_entails` with the warrant RE-DECIDED, without running a reasoner.
///
/// The re-check re-derives nothing, deliberately: "the closure follows from the premise"
/// is the chase's claim and `purrdf_entail_explain_conclusion` is its checker, while "the
/// conclusion follows from the closure" is this one and is finite and purely
/// combinatorial — a graph homomorphism, or a set of lookups against a refutation's own
/// closure. Folding them would cost what the original call cost and give a caller no
/// independent check at all.
///
/// `*out_answer` is `purrdf_entail_graph_entails`'s, plus `warrant present|absent` and
/// `verified true|false|not-applicable`. `warrant absent` / `verified not-applicable` is
/// a `not-entailed` or an `undecided`: there is no evidence to re-decide, and a `false`
/// there would read as a failed check rather than as an absent one.
/// **Free BOTH buffers with `purrdf_buffer_free`.**
///
/// `import_iris`, `import_documents` and `import_count` are
/// `purrdf_entail_certain_answers`'s. The re-check runs against the premise AS WRITTEN
/// rather than against its imports closure: a warrant re-decidable from the caller's own
/// document is a stronger check than one only re-decidable against a graph the library
/// assembled.
///
/// # Safety
/// `regime`, `premise` and `conclusion` must be non-null, NUL-terminated C strings; when
/// `import_count` is non-zero, `import_iris` and `import_documents` must each address at
/// least `import_count` readable, non-null, NUL-terminated C strings; `out_answer` and
/// `out_certificate` must be writable pointers; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_entail_verify_entailment(
    regime: *const c_char,
    premise: *const c_char,
    conclusion: *const c_char,
    import_iris: *const *const c_char,
    import_documents: *const *const c_char,
    import_count: usize,
    out_answer: *mut *mut PurrdfBuffer,
    out_certificate: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: as `purrdf_entail_certain_answers`.
    unsafe {
        ffi_try!(out_error, {
            if regime.is_null()
                || premise.is_null()
                || conclusion.is_null()
                || out_answer.is_null()
                || out_certificate.is_null()
            {
                return Err(null_argument("purrdf_entail_verify_entailment"));
            }
            let regime = cstr_to_str(regime)?;
            let premise = cstr_to_str(premise)?;
            let conclusion = cstr_to_str(conclusion)?;
            let imports = import_pairs(
                import_iris,
                import_documents,
                import_count,
                "purrdf_entail_verify_entailment",
            )?;
            store_answer(
                verify_entailment_to_string(regime, premise, conclusion, &imports),
                out_answer,
                out_certificate,
            )
        })
    }
}

// ── Proofs: opt-in to produce, and a checker to consume ─────────────────────────

/// Write a proved answer to its three out-params, or map its message to an error.
///
/// # Safety
/// `out_answer`, `out_certificate` and `out_proof` must be non-null, writable pointers.
/// All three writes happen only after every fallible step has succeeded.
unsafe fn store_proved_answer(
    produced: Result<ReasoningAnswer, String>,
    out_answer: *mut *mut PurrdfBuffer,
    out_certificate: *mut *mut PurrdfBuffer,
    out_proof: *mut *mut PurrdfBuffer,
) -> Result<PurrdfStatus, PurrdfError> {
    let (answer, certificate, proof) = produced
        .map_err(|message| PurrdfError::new(PurrdfStatus::ParseError, message))?
        .into_proved_parts();
    // SAFETY: the caller's contract above — all three pointers are non-null and writable,
    // and nothing fallible remains between here and the three writes.
    unsafe {
        *out_answer = PurrdfBuffer::into_raw(answer.into_bytes());
        *out_certificate = PurrdfBuffer::into_raw(certificate.into_bytes());
        *out_proof = PurrdfBuffer::into_raw(proof.into_bytes());
    }
    Ok(PurrdfStatus::Ok)
}

/// Answer one Description-Logic service WITH the proof term of the run that answered.
///
/// **THE OPT-IN.** Every `purrdf_entail_*` entry point above is unchanged and records
/// nothing, so a caller who does not want a proof runs exactly the search they ran before
/// and pays exactly what they paid before. This one RECORDS — which costs the completion
/// graph of every tableau run it keeps — and hands back a document
/// [`purrdf_entail_check_proof`] can verify.
///
/// `service` is one of `consistency`, `class-satisfiability`, `classify`, `realize`,
/// `instances`, `entails`, `extract-module`; an unknown spelling is an error naming the
/// accepted set. `argument` is the question's own input in that service's grammar:
///
/// * `""` for `consistency`, `classify` and `realize` — a non-empty one is an ERROR rather
///   than a silently discarded argument;
/// * ONE N-Triples term for `class-satisfiability` and `instances`;
/// * ONE triple of the OWL 2 RDF mapping for `entails`;
/// * a `method <bot|top|star>` line then one term per line for `extract-module`.
///
/// `step_cap` and `work_cap` behave exactly as in [`purrdf_entail_consistency`].
///
/// On success `*out_answer` and `*out_certificate` receive exactly the bytes the same
/// question would produce WITHOUT a proof — recording is an observation the reasoner makes
/// of itself, never a lever it reads — and `*out_proof` receives the `purrdf-dl-proof 1`
/// document. **Free ALL THREE with `purrdf_buffer_free`.** On any error none of the three
/// out-params is written, so there is nothing to free.
///
/// # Safety
/// `document`, `service` and `argument` must be non-null, NUL-terminated C strings;
/// `out_answer`, `out_certificate` and `out_proof` must be writable pointers; `out_error`
/// must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_entail_prove(
    document: *const c_char,
    service: *const c_char,
    argument: *const c_char,
    step_cap: u32,
    work_cap: u32,
    out_answer: *mut *mut PurrdfBuffer,
    out_certificate: *mut *mut PurrdfBuffer,
    out_proof: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: the caller's contract above; the out-params are written only by
    // `store_proved_answer`, after the boundary call has succeeded.
    unsafe {
        ffi_try!(out_error, {
            if document.is_null()
                || service.is_null()
                || argument.is_null()
                || out_answer.is_null()
                || out_certificate.is_null()
                || out_proof.is_null()
            {
                return Err(null_argument("purrdf_entail_prove"));
            }
            let document = cstr_to_str(document)?;
            let service = cstr_to_str(service)?;
            let argument = cstr_to_str(argument)?;
            store_proved_answer(
                prove_to_string(document, service, argument, step_cap, work_cap),
                out_answer,
                out_certificate,
                out_proof,
            )
        })
    }
}

/// CHECK a proof against the CALLER's own ontology, question and answer.
///
/// **THE CHECKER**, and the shape [`purrdf_entail_verify_entailment`] set: a consumer holds
/// evidence and re-decides it here. Nothing in this call trusts the producer. The ontology
/// is parsed from `document`, the question is re-derived from `service` and `argument`, the
/// claims are read back out of `answer`'s own grammar, and the checking context comes from a
/// reverse mapping this call performs itself. The proof supplies the runs and nothing else,
/// so an `entails` proof for a different axiom, a proof for a different document, and a
/// genuine proof of some OTHER answer are each REFUSED.
///
/// `answer` and `certificate` may each be the empty string, and each empty one is a WEAKER
/// check that SAYS SO rather than one that quietly passed: with no answer the report reads
/// `answer not-checked`, and with no certificate a proof carrying a stopping receipt is
/// refused, because there is nothing for the receipt to be a receipt of.
///
/// On success `*out_report` receives the `purrdf-dl-proof-check 1` block — the digest and
/// input identity it checked, the runs it replayed, and the `attested`/`trusted`/`unattested`
/// counts with the producer-shared components the whole check rests on. **Free it with
/// `purrdf_buffer_free`.** There is no `verified` line: a verification that FAILED is an
/// error, so a rendered `true` would be a constant rather than a gate.
///
/// A `proof` document reading `availability not-recorded` is an ERROR naming that fact. An
/// answer nobody asked to record must never be presented as a verified one.
///
/// # Safety
/// `document`, `service`, `argument`, `answer`, `certificate` and `proof` must be non-null,
/// NUL-terminated C strings; `out_report` must be a writable pointer; `out_error` must be
/// null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_entail_check_proof(
    document: *const c_char,
    service: *const c_char,
    argument: *const c_char,
    answer: *const c_char,
    certificate: *const c_char,
    proof: *const c_char,
    out_report: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: the caller's contract above; `*out_report` is written only after the boundary
    // call has succeeded.
    unsafe {
        ffi_try!(out_error, {
            if document.is_null()
                || service.is_null()
                || argument.is_null()
                || answer.is_null()
                || certificate.is_null()
                || proof.is_null()
                || out_report.is_null()
            {
                return Err(null_argument("purrdf_entail_check_proof"));
            }
            let report = check_dl_proof(
                cstr_to_str(document)?,
                cstr_to_str(service)?,
                cstr_to_str(argument)?,
                cstr_to_str(answer)?,
                cstr_to_str(certificate)?,
                cstr_to_str(proof)?,
            )
            .map_err(|message| PurrdfError::new(PurrdfStatus::ParseError, message))?;
            *out_report = PurrdfBuffer::into_raw(report.into_bytes());
            Ok(PurrdfStatus::Ok)
        })
    }
}

// ── The session ─────────────────────────────────────────────────────────────────

/// A reasoning session over one ontology. Release with [`purrdf_reasoner_free`].
///
/// Every `purrdf_entail_*` function above takes the document as a C string and rebuilds
/// everything it needs, so asking three questions parses and reverse-maps the ontology
/// three times. This handle holds the parsed document instead: [`purrdf_reasoner_open`]
/// parses once, the first question needing a knowledge base reverse-maps once, and later
/// questions reuse both.
///
/// # Thread safety — UNLIKE [`PurrdfDataset`](crate::handles::PurrdfDataset)
///
/// This handle is `Send` but **NOT `Sync`**: answering a question MUTATES the shared
/// knowledge base, so two threads may not use one handle concurrently. A frozen dataset
/// may be read from many threads; a session may not. Move it between threads, or open
/// one per thread — the README's thread-safety table says the same.
pub struct PurrdfReasoner(ReasonerSession);

impl std::fmt::Debug for PurrdfReasoner {
    /// Delegates to the session, which prints the SHAPE of the problem rather than a
    /// dump of thousands of interned ids.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("PurrdfReasoner").field(&self.0).finish()
    }
}

/// Compile-time proof of the `Send`-but-not-`Sync` contract documented above.
///
/// Asserting `Send` matches [`PurrdfDataset`](crate::handles::PurrdfDataset)'s own
/// compile-time proof. There is deliberately NO `Sync` assertion: the services take
/// `&mut` and a future change that made this `Sync` would be a silent widening of a
/// published ABI guarantee, not a fix.
const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<PurrdfReasoner>();
};

/// Open a reasoning session over `document`.
///
/// `step_cap` narrows the per-decision tableau step cap for every question asked through
/// this session, and behaves exactly as in [`purrdf_entail_consistency`]: **0 means the
/// knowledge base's own cap**, not a cap of zero steps, and it can only NARROW.
/// `work_cap` narrows the per-decision WORK cap on the same rule — the cap on the
/// matcher, scan, closure and clone work done INSIDE a round, which a round cap cannot
/// see.
///
/// On success `*out_reasoner` receives a handle to free with [`purrdf_reasoner_free`].
///
/// Nothing is reverse-mapped here, so an ontology whose knowledge base cannot be built
/// still opens — and fails on the first question that needs one, with that question's own
/// error. That is deliberate: `profile`, `extract_module`, `justify` and
/// `explain_conclusion` never reason, and `profile` answers for any parseable document.
///
/// # Safety
/// `document` must be a non-null, NUL-terminated C string; `out_reasoner` must be a
/// writable pointer; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_reasoner_open(
    document: *const c_char,
    step_cap: u32,
    work_cap: u32,
    out_reasoner: *mut *mut PurrdfReasoner,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: the caller's contract above; `out_reasoner` is written only after the
    // boundary call has succeeded.
    unsafe {
        ffi_try!(out_error, {
            if document.is_null() || out_reasoner.is_null() {
                return Err(null_argument("purrdf_reasoner_open"));
            }
            let document = cstr_to_str(document)?;
            let session = ReasonerSession::open(document, step_cap, work_cap)
                .map_err(|message| PurrdfError::new(PurrdfStatus::ParseError, message))?;
            *out_reasoner = Box::into_raw(Box::new(PurrdfReasoner(session)));
            Ok(PurrdfStatus::Ok)
        })
    }
}

/// Release a reasoning session. No-op on null.
///
/// # Safety
/// `reasoner` must be null or a live session handle not already freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_reasoner_free(reasoner: *mut PurrdfReasoner) {
    unsafe {
        ffi_guard!((), {
            if !reasoner.is_null() {
                drop(Box::from_raw(reasoner));
            }
        });
    }
}

// Written out one by one, NOT generated by a macro: cbindgen does not expand macros,
// so a macro here compiles and exports from the cdylib while leaving the function out
// of the committed header — reachable in theory and unreachable by any C caller, which
// is the exact defect this session was added to remove.

/// Is the knowledge base consistent? See [`purrdf_entail_consistency`].
///
/// `*out_answer` and `*out_certificate` are exactly what [`purrdf_entail_consistency`] writes
/// for the same document. **Free BOTH with `purrdf_buffer_free`.**
///
/// # Safety
/// `reasoner` must be a live session handle; `out_answer` and `out_certificate` must be
/// writable pointers; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_reasoner_consistency(
    reasoner: *mut PurrdfReasoner,
    out_answer: *mut *mut PurrdfBuffer,
    out_certificate: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: the caller's contract above.
    unsafe {
        ffi_try!(out_error, {
            if reasoner.is_null() || out_answer.is_null() || out_certificate.is_null() {
                return Err(null_argument("purrdf_reasoner_consistency"));
            }
            store_answer((*reasoner).0.consistency(), out_answer, out_certificate)
        })
    }
}

/// The entailed subsumption hierarchy. See [`purrdf_entail_classify`].
///
/// `*out_answer` and `*out_certificate` are exactly what [`purrdf_entail_classify`] writes
/// for the same document. **Free BOTH with `purrdf_buffer_free`.**
///
/// # Safety
/// `reasoner` must be a live session handle; `out_answer` and `out_certificate` must be
/// writable pointers; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_reasoner_classify(
    reasoner: *mut PurrdfReasoner,
    out_answer: *mut *mut PurrdfBuffer,
    out_certificate: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: the caller's contract above.
    unsafe {
        ffi_try!(out_error, {
            if reasoner.is_null() || out_answer.is_null() || out_certificate.is_null() {
                return Err(null_argument("purrdf_reasoner_classify"));
            }
            store_answer((*reasoner).0.classify(), out_answer, out_certificate)
        })
    }
}

/// The entailed types of the named individuals. See [`purrdf_entail_realize`].
///
/// `*out_answer` and `*out_certificate` are exactly what [`purrdf_entail_realize`] writes
/// for the same document. **Free BOTH with `purrdf_buffer_free`.**
///
/// # Safety
/// `reasoner` must be a live session handle; `out_answer` and `out_certificate` must be
/// writable pointers; `out_error` must be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_reasoner_realize(
    reasoner: *mut PurrdfReasoner,
    out_answer: *mut *mut PurrdfBuffer,
    out_certificate: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: the caller's contract above.
    unsafe {
        ffi_try!(out_error, {
            if reasoner.is_null() || out_answer.is_null() || out_certificate.is_null() {
                return Err(null_argument("purrdf_reasoner_realize"));
            }
            store_answer((*reasoner).0.realize(), out_answer, out_certificate)
        })
    }
}

/// The individuals entailed to be instances of `class`. See [`purrdf_entail_instances`].
///
/// `*out_answer` and `*out_certificate` are exactly what [`purrdf_entail_instances`] writes for
/// the same document and `class`. **Free BOTH with `purrdf_buffer_free`.**
///
/// # Safety
/// `reasoner` must be a live session handle; `class` must be a non-null, NUL-terminated C
/// string; `out_answer` and `out_certificate` must be writable pointers; `out_error` must
/// be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_reasoner_instances(
    reasoner: *mut PurrdfReasoner,
    class: *const c_char,
    out_answer: *mut *mut PurrdfBuffer,
    out_certificate: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: the caller's contract above.
    unsafe {
        ffi_try!(out_error, {
            if reasoner.is_null()
                || class.is_null()
                || out_answer.is_null()
                || out_certificate.is_null()
            {
                return Err(null_argument("purrdf_reasoner_instances"));
            }
            let class = cstr_to_str(class)?;
            store_answer((*reasoner).0.instances(class), out_answer, out_certificate)
        })
    }
}

/// Does the ontology entail `axiom`? See [`purrdf_entail_entails`].
///
/// `*out_answer` and `*out_certificate` are exactly what [`purrdf_entail_entails`] writes for
/// the same document and `axiom`. **Free BOTH with `purrdf_buffer_free`.**
///
/// # Safety
/// `reasoner` must be a live session handle; `axiom` must be a non-null, NUL-terminated C
/// string; `out_answer` and `out_certificate` must be writable pointers; `out_error` must
/// be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_reasoner_entails(
    reasoner: *mut PurrdfReasoner,
    axiom: *const c_char,
    out_answer: *mut *mut PurrdfBuffer,
    out_certificate: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: the caller's contract above.
    unsafe {
        ffi_try!(out_error, {
            if reasoner.is_null()
                || axiom.is_null()
                || out_answer.is_null()
                || out_certificate.is_null()
            {
                return Err(null_argument("purrdf_reasoner_entails"));
            }
            let axiom = cstr_to_str(axiom)?;
            store_answer((*reasoner).0.entails(axiom), out_answer, out_certificate)
        })
    }
}

/// A justification for `axiom`. See [`purrdf_entail_justify`].
///
/// `*out_answer` and `*out_certificate` are exactly what [`purrdf_entail_justify`] writes for
/// the same document and `axiom`. **Free BOTH with `purrdf_buffer_free`.**
///
/// # Safety
/// `reasoner` must be a live session handle; `axiom` must be a non-null, NUL-terminated C
/// string; `out_answer` and `out_certificate` must be writable pointers; `out_error` must
/// be null or writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_reasoner_justify(
    reasoner: *mut PurrdfReasoner,
    axiom: *const c_char,
    out_answer: *mut *mut PurrdfBuffer,
    out_certificate: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: the caller's contract above.
    unsafe {
        ffi_try!(out_error, {
            if reasoner.is_null()
                || axiom.is_null()
                || out_answer.is_null()
                || out_certificate.is_null()
            {
                return Err(null_argument("purrdf_reasoner_justify"));
            }
            let axiom = cstr_to_str(axiom)?;
            store_answer((*reasoner).0.justify(axiom), out_answer, out_certificate)
        })
    }
}

/// Which OWL 2 profiles the ontology is provably in. See [`purrdf_entail_profile`].
///
/// Purely syntactic: never builds a knowledge base, so it answers even for an ontology
/// whose other services would fail. **Free BOTH buffers with `purrdf_buffer_free`.**
///
/// # Safety
/// As the other session services.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_reasoner_profile(
    reasoner: *const PurrdfReasoner,
    out_answer: *mut *mut PurrdfBuffer,
    out_certificate: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: the caller's contract above.
    unsafe {
        ffi_try!(out_error, {
            if reasoner.is_null() || out_answer.is_null() || out_certificate.is_null() {
                return Err(null_argument("purrdf_reasoner_profile"));
            }
            store_answer(Ok((*reasoner).0.profile()), out_answer, out_certificate)
        })
    }
}

/// A module for `signature` under `method`. See [`purrdf_entail_extract_module`].
///
/// **Free BOTH buffers with `purrdf_buffer_free`.**
///
/// # Safety
/// `reasoner` must be a live session handle; `signature` and `method` must be non-null,
/// NUL-terminated C strings; the out-params must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_reasoner_extract_module(
    reasoner: *const PurrdfReasoner,
    signature: *const c_char,
    method: *const c_char,
    out_answer: *mut *mut PurrdfBuffer,
    out_certificate: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: the caller's contract above.
    unsafe {
        ffi_try!(out_error, {
            if reasoner.is_null()
                || signature.is_null()
                || method.is_null()
                || out_answer.is_null()
                || out_certificate.is_null()
            {
                return Err(null_argument("purrdf_reasoner_extract_module"));
            }
            let signature = cstr_to_str(signature)?;
            let method = cstr_to_str(method)?;
            store_answer(
                (*reasoner).0.extract_module(signature, method),
                out_answer,
                out_certificate,
            )
        })
    }
}

/// Why `conclusion` holds under `regime`. See [`purrdf_entail_explain_conclusion`].
///
/// **Free BOTH buffers with `purrdf_buffer_free`.**
///
/// # Safety
/// `reasoner` must be a live session handle; `regime` and `conclusion` must be non-null,
/// NUL-terminated C strings; the out-params must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_reasoner_explain_conclusion(
    reasoner: *const PurrdfReasoner,
    regime: *const c_char,
    conclusion: *const c_char,
    out_answer: *mut *mut PurrdfBuffer,
    out_certificate: *mut *mut PurrdfBuffer,
    out_error: *mut *mut PurrdfError,
) -> i32 {
    // SAFETY: the caller's contract above.
    unsafe {
        ffi_try!(out_error, {
            if reasoner.is_null()
                || regime.is_null()
                || conclusion.is_null()
                || out_answer.is_null()
                || out_certificate.is_null()
            {
                return Err(null_argument("purrdf_reasoner_explain_conclusion"));
            }
            let regime = cstr_to_str(regime)?;
            let conclusion = cstr_to_str(conclusion)?;
            store_answer(
                (*reasoner).0.explain_conclusion(regime, conclusion),
                out_answer,
                out_certificate,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use purrdf_validate::regime::{
        check_absent_proof_is_not_verifiable, check_dl_proof_golden_vectors,
        check_inconsistent_refusal, check_regime_golden_vectors,
    };

    use super::*;

    /// `A ⊑ B` and one typed instance — enough for `rdfs9` to re-type it.
    const SCHEMA: &str = "\
<http://example.org/A> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/B> .
<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/A> .
";

    /// A normative RIF-in-XML rule document: `?x a ex:A` ⟹ `?x a ex:B`.
    ///
    /// `rif` is the one regime whose calculus is the CALLER's, so it is the one
    /// spelling whose `program` argument is a document rather than the empty string.
    const RIF_PROGRAM: &str = "<Document xmlns=\"http://www.w3.org/2007/rif#\"><payload><Group><sentence><Forall><declare><Var>x</Var></declare><formula><Implies><if><Frame><object><Var>x</Var></object><slot><Const type=\"http://www.w3.org/2007/rif#iri\">http://www.w3.org/1999/02/22-rdf-syntax-ns#type</Const><Const type=\"http://www.w3.org/2007/rif#iri\">http://example.org/A</Const></slot></Frame></if><then><Frame><object><Var>x</Var></object><slot><Const type=\"http://www.w3.org/2007/rif#iri\">http://www.w3.org/1999/02/22-rdf-syntax-ns#type</Const><Const type=\"http://www.w3.org/2007/rif#iri\">http://example.org/B</Const></slot></Frame></then></Implies></formula></Forall></sentence></Group></payload></Document>";

    /// The C-ABI host's leg of the tri-host assertion.
    ///
    /// The `purrdf-validate` test and the WASM host's `entailCheckGoldenVectors`
    /// call this SAME checker over the SAME committed artifact, so a host that
    /// produces different bytes fails here in the same words.
    #[test]
    fn the_golden_vector_matches() {
        check_regime_golden_vectors().expect("the regime golden vector");
    }

    /// The C-ABI host's leg of the OTHER tri-host assertion: an inconsistent input is
    /// refused WITH its certificate and its witness triples.
    ///
    /// The `purrdf-validate` test and the WASM host call this same checker. It is separate
    /// from the golden vector because a refusal has no closure to pair an input with, and
    /// it is shared for the same reason the vector is: the message a C caller reads is the
    /// only channel the evidence has.
    #[test]
    fn an_inconsistent_input_is_refused_with_its_report() {
        check_inconsistent_refusal().expect("the inconsistent refusal");
    }

    /// The C-ABI host's leg of the CROSS-HOST assertion for the PROOF surface.
    ///
    /// The `purrdf-validate` test, the PyO3 test and the WASM host's
    /// `entailCheckProofGoldenVectors` call this SAME checker over the SAME committed
    /// artifact. A rendered proof carries `ServiceProof::encode`'s canonical bytes, so a host
    /// producing different bytes has produced a different proof TERM.
    #[test]
    fn the_dl_proof_golden_vector_matches() {
        check_dl_proof_golden_vectors().expect("the DL proof golden vector");
    }

    /// The C-ABI host's leg of the availability assertion: an answer nobody asked to record
    /// is never presentable as a verified one.
    #[test]
    fn an_absent_proof_is_never_presented_as_a_verified_one() {
        check_absent_proof_is_not_verifiable().expect("the absent-proof refusal");
    }

    /// **THE GOLDEN ARTIFACT, THROUGH THE C SYMBOLS.** Every committed case reproduces
    /// byte for byte when produced and checked through the `extern "C"` entry points.
    ///
    /// `the_dl_proof_golden_vector_matches` runs the shared checker over the Rust boundary;
    /// this runs the same cases through the ABI a C consumer actually calls — C strings in,
    /// `PurrdfBuffer`s out. A framing bug that truncated a proof, or a host that produced
    /// different bytes, fails here on the case that moved rather than on a fixture only this
    /// crate has.
    #[test]
    fn the_golden_proof_bytes_survive_the_c_abi() {
        let cases = purrdf_validate::regime::dl_proof_golden_vectors().expect("the artifact");
        assert_eq!(cases.len(), 7, "one case per proof-bearing service");
        for case in &cases {
            let document = CString::new(case.input()).expect("no NUL");
            let service = CString::new(case.service()).expect("no NUL");
            let argument = CString::new(case.argument().trim_end()).expect("no NUL");
            let mut answer: *mut PurrdfBuffer = std::ptr::null_mut();
            let mut certificate: *mut PurrdfBuffer = std::ptr::null_mut();
            let mut proof: *mut PurrdfBuffer = std::ptr::null_mut();
            // SAFETY: live C strings and three writable out-params.
            let status = unsafe {
                purrdf_entail_prove(
                    document.as_ptr(),
                    service.as_ptr(),
                    argument.as_ptr(),
                    0,
                    0,
                    &raw mut answer,
                    &raw mut certificate,
                    &raw mut proof,
                    std::ptr::null_mut(),
                )
            };
            assert_eq!(status, PurrdfStatus::Ok as i32, "{}", case.name());
            // SAFETY: three live buffers, each freed exactly once.
            let (answer, certificate, proof) =
                unsafe { (take(answer), take(certificate), take(proof)) };
            assert_eq!(answer, case.answer(), "{}: answer", case.name());
            assert_eq!(proof, case.proof(), "{}: proof", case.name());

            let answer_c = CString::new(answer.as_str()).expect("no NUL");
            let certificate_c = CString::new(certificate.as_str()).expect("no NUL");
            let proof_c = CString::new(proof.as_str()).expect("no NUL");
            let mut report: *mut PurrdfBuffer = std::ptr::null_mut();
            // SAFETY: as above.
            let status = unsafe {
                purrdf_entail_check_proof(
                    document.as_ptr(),
                    service.as_ptr(),
                    argument.as_ptr(),
                    answer_c.as_ptr(),
                    certificate_c.as_ptr(),
                    proof_c.as_ptr(),
                    &raw mut report,
                    std::ptr::null_mut(),
                )
            };
            assert_eq!(status, PurrdfStatus::Ok as i32, "{}", case.name());
            // SAFETY: one live buffer, freed exactly once.
            assert_eq!(
                unsafe { take(report) },
                case.check(),
                "{}: check",
                case.name()
            );
        }
    }

    /// **THE C ENTRY POINTS THEMSELVES.** A proof produced through `purrdf_entail_prove`
    /// checks through `purrdf_entail_check_proof`, and the absence of one does not.
    ///
    /// Drives the actual `extern "C"` symbols with real C strings and real out-params rather
    /// than the Rust boundary underneath them, because the framing is this crate's whole job:
    /// a proof that never reaches the third buffer is a proof no C caller can hold.
    #[test]
    fn the_c_entry_points_produce_and_check_a_proof() {
        let document = CString::new(TAXONOMY).expect("no NUL");
        let service = CString::new("entails").expect("no NUL");
        let argument = CString::new(CHAIN_AXIOM).expect("no NUL");
        let empty = CString::new("").expect("no NUL");
        let mut answer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut certificate: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut proof: *mut PurrdfBuffer = std::ptr::null_mut();
        // SAFETY: every pointer is a live C string or a writable out-param.
        let status = unsafe {
            purrdf_entail_prove(
                document.as_ptr(),
                service.as_ptr(),
                argument.as_ptr(),
                0,
                0,
                &raw mut answer,
                &raw mut certificate,
                &raw mut proof,
                std::ptr::null_mut(),
            )
        };
        assert_eq!(status, PurrdfStatus::Ok as i32);
        // SAFETY: three live buffers, each freed exactly once.
        let (answer, certificate, proof) =
            unsafe { (take(answer), take(certificate), take(proof)) };
        assert!(answer.starts_with("entails true\n"), "{answer}");
        assert!(
            proof.starts_with("purrdf-dl-proof 1\nservice entails\navailability recorded\n"),
            "{proof}"
        );

        let answer_c = CString::new(answer.as_str()).expect("no NUL");
        let certificate_c = CString::new(certificate.as_str()).expect("no NUL");
        let proof_c = CString::new(proof.as_str()).expect("no NUL");
        let mut report: *mut PurrdfBuffer = std::ptr::null_mut();
        // SAFETY: as above.
        let status = unsafe {
            purrdf_entail_check_proof(
                document.as_ptr(),
                service.as_ptr(),
                argument.as_ptr(),
                answer_c.as_ptr(),
                certificate_c.as_ptr(),
                proof_c.as_ptr(),
                &raw mut report,
                std::ptr::null_mut(),
            )
        };
        assert_eq!(status, PurrdfStatus::Ok as i32);
        // SAFETY: one live buffer, freed exactly once.
        let report = unsafe { take(report) };
        assert!(
            report.starts_with("purrdf-dl-proof-check 1\nservice entails\n"),
            "{report}"
        );
        assert!(report.contains("\nanswer checked 1\n"), "{report}");

        // …and the ABSENT proof — what an ordinary `purrdf_entail_entails` answer has — is
        // REFUSED rather than reported as a verification of nothing.
        let absent =
            CString::new("purrdf-dl-proof 1\navailability not-recorded\n").expect("no NUL");
        let mut report: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        // SAFETY: as above; `out_error` is writable.
        let status = unsafe {
            purrdf_entail_check_proof(
                document.as_ptr(),
                service.as_ptr(),
                argument.as_ptr(),
                empty.as_ptr(),
                empty.as_ptr(),
                absent.as_ptr(),
                &raw mut report,
                &raw mut error,
            )
        };
        assert_ne!(status, PurrdfStatus::Ok as i32);
        assert!(report.is_null(), "a refused check writes no report to free");
        assert!(!error.is_null(), "and it names what it refused");
        // SAFETY: a live error handle, freed exactly once.
        unsafe { crate::error::purrdf_error_free(error) };
    }

    #[test]
    fn materialize_emits_closure_and_report() {
        let (nquads, report) =
            materialize_to_nquads_bytes(SCHEMA, "rdfs", "").expect("rdfs closure");
        let nquads = String::from_utf8(nquads).expect("utf8");
        let report = String::from_utf8(report).expect("utf8");
        assert!(nquads.contains(
            "<http://example.org/x> \
             <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/B> ."
        ));
        assert!(report.starts_with("purrdf-reasoning-report 4\n"));
        // The report says what the run could NOT do. Asserted as the invariant
        // rather than as a `sound-incomplete <n>` literal: the count moves every
        // time a rule lands, and the honesty gate does not.
        assert!(report.contains("\ncompleteness "));
        assert!(report.contains("\nboundary "));
        // The count of the conclusions the four existential rules reached and the answer
        // may not bind. It reaches the C ABI, which it did not before.
        assert!(report.contains("\nwithheld-surrogates "));
        assert!(report.ends_with("inconsistency none\n"));
    }

    #[test]
    fn an_unknown_regime_names_the_accepted_set() {
        for error in [
            materialize_to_nquads_bytes(SCHEMA, "RDFS", "").expect_err("case-sensitive"),
            rules_bytes("rdfs-plus").expect_err("unknown"),
            implemented_rules_bytes("rdfs-plus").expect_err("unknown"),
        ] {
            assert!(error.contains("accepted: simple, rdf, rdfs"), "{error}");
        }
    }

    /// EVERY accepted spelling materializes across the C ABI. None is refused.
    ///
    /// Falsifiable against the old behavior: `rif` and `owl-direct` were refused here
    /// with a message naming the five that were not.
    #[test]
    fn every_regime_spelling_materializes() {
        for (regime, program) in [
            ("simple", ""),
            ("rdf", ""),
            ("rdfs", ""),
            ("owl-rl", ""),
            ("owl-direct", ""),
            ("rif", RIF_PROGRAM),
            ("d", ""),
        ] {
            let (_, report) = materialize_to_nquads_bytes(SCHEMA, regime, program)
                .unwrap_or_else(|error| panic!("{regime}: {error}"));
            let report = String::from_utf8(report).expect("utf8");
            assert!(report.contains(&format!("\nregime {regime}\n")), "{report}");
        }
        // A rule document belongs to exactly one regime; passing one anywhere else is
        // refused rather than discarded.
        let error = materialize_to_nquads_bytes(SCHEMA, "rdfs", RIF_PROGRAM)
            .expect_err("a rule document for a rule-table regime");
        assert!(error.contains("takes no rule document"), "{error}");
    }

    #[test]
    fn a_malformed_document_is_an_error() {
        assert!(materialize_to_nquads_bytes("this is not n-quads\n", "rdfs", "").is_err());
    }

    #[test]
    fn the_inventories_are_the_specification_tables() {
        let rules = String::from_utf8(rules_bytes("owl-rl").expect("known")).expect("utf8");
        assert_eq!(rules.lines().count(), 78);
        let rdfs = String::from_utf8(rules_bytes("rdfs").expect("known")).expect("utf8");
        assert_eq!(rdfs.lines().count(), 18);
        let fired =
            String::from_utf8(implemented_rules_bytes("rdfs").expect("known")).expect("utf8");
        // The gap is MEASURED, never asserted as a literal: the implemented set is
        // a subsequence of the defined one, so the two ways of counting the gap
        // must agree — and it is legitimately empty once the table is complete.
        let missing = rdfs
            .lines()
            .filter(|rule| !fired.lines().any(|f| f == *rule))
            .count();
        assert_eq!(missing, rdfs.lines().count() - fired.lines().count());
        assert!(rules_bytes("simple").expect("known").is_empty());
    }

    // ── The pointer surface ─────────────────────────────────────────────────

    /// Read a buffer handle's bytes as a `String` and free the handle.
    ///
    /// # Safety
    /// `buf` must be a live buffer handle holding UTF-8, not already freed.
    unsafe fn take(buf: *mut PurrdfBuffer) -> String {
        // SAFETY: the caller's contract — `buf` is a live handle. `purrdf_buffer_data`
        // borrows its bytes for as long as the handle lives; the slice is copied
        // into an owned `String` BEFORE the handle is freed.
        unsafe {
            let mut ptr: *const u8 = std::ptr::null();
            let mut len: usize = 0;
            assert_eq!(
                crate::buffer::purrdf_buffer_data(buf, &raw mut ptr, &raw mut len),
                PurrdfStatus::Ok as i32
            );
            let text = String::from_utf8(std::slice::from_raw_parts(ptr, len).to_vec())
                .expect("the boundary emits UTF-8");
            crate::buffer::purrdf_buffer_free(buf);
            text
        }
    }

    #[test]
    fn the_pointer_surface_hands_out_two_buffers() {
        let document = CString::new(SCHEMA).expect("no interior NUL");
        let regime = CString::new("rdfs").expect("no interior NUL");
        let program = CString::new("").expect("no interior NUL");
        let mut nquads: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut report: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        // SAFETY: all three C strings are live for the call, and the three out-pointers
        // address live, writable locals.
        unsafe {
            assert_eq!(
                purrdf_entail_materialize_to_nquads(
                    document.as_ptr(),
                    regime.as_ptr(),
                    program.as_ptr(),
                    &raw mut nquads,
                    &raw mut report,
                    &raw mut error,
                ),
                PurrdfStatus::Ok as i32
            );
            assert!(error.is_null());
            assert!(take(nquads).contains(
                "<http://example.org/x> \
                 <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/B> ."
            ));
            assert!(take(report).starts_with("purrdf-reasoning-report 4\n"));
        }
    }

    #[test]
    fn the_pointer_surface_hands_out_the_inventories() {
        let regime = CString::new("rdfs").expect("no interior NUL");
        let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        // SAFETY: `regime` is live for both calls; `buffer`/`error` address live,
        // writable locals, and each handed-out buffer is freed by `take`.
        unsafe {
            assert_eq!(
                purrdf_entail_rules(regime.as_ptr(), &raw mut buffer, &raw mut error),
                PurrdfStatus::Ok as i32
            );
            // 18 is the SPECIFICATION's count and does not move.
            assert_eq!(take(buffer).lines().count(), 18);
            assert_eq!(
                purrdf_entail_implemented_rules(regime.as_ptr(), &raw mut buffer, &raw mut error),
                PurrdfStatus::Ok as i32
            );
            // The implemented count DOES move as rules land, so it is bounded, not
            // pinned: never empty, never more than the table it is a subset of.
            let implemented = take(buffer).lines().count();
            assert!((1..=18).contains(&implemented), "{implemented}");
            assert!(error.is_null());
        }
    }

    /// A failing call writes neither out-param, so there is nothing to free.
    ///
    /// The failure is now a MALFORMED DOCUMENT rather than a refused regime: every
    /// spelling materializes, so the error path is reached with bad bytes.
    #[test]
    fn a_failing_call_writes_no_buffer_to_free() {
        let document = CString::new("this is not n-quads\n").expect("no interior NUL");
        let regime = CString::new("rdfs").expect("no interior NUL");
        let program = CString::new("").expect("no interior NUL");
        let mut nquads: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut report: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        // SAFETY: all three C strings are live for the call, and the three out-pointers
        // address live, writable locals; the error handle is freed below.
        unsafe {
            assert_eq!(
                purrdf_entail_materialize_to_nquads(
                    document.as_ptr(),
                    regime.as_ptr(),
                    program.as_ptr(),
                    &raw mut nquads,
                    &raw mut report,
                    &raw mut error,
                ),
                PurrdfStatus::ParseError as i32
            );
            // Neither out-param was written, so the caller has nothing to free.
            assert!(nquads.is_null());
            assert!(report.is_null());
            assert!(!error.is_null());
            crate::error::purrdf_error_free(error);
        }
    }

    #[test]
    fn null_arguments_are_refused_not_dereferenced() {
        let mut nquads: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut report: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
        // SAFETY: every pointer passed is either null (the case under test) or
        // addresses a live, writable local; no error channel is requested, so the
        // status code is the whole observable result.
        unsafe {
            assert_eq!(
                purrdf_entail_materialize_to_nquads(
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    &raw mut nquads,
                    &raw mut report,
                    std::ptr::null_mut(),
                ),
                PurrdfStatus::NullPointer as i32
            );
            assert_eq!(
                purrdf_entail_rules(std::ptr::null(), &raw mut buffer, std::ptr::null_mut()),
                PurrdfStatus::NullPointer as i32
            );
            assert_eq!(
                purrdf_entail_implemented_rules(
                    std::ptr::null(),
                    &raw mut buffer,
                    std::ptr::null_mut(),
                ),
                PurrdfStatus::NullPointer as i32
            );
        }
    }

    // ── The Description-Logic reasoning services ────────────────────────────

    /// `A ⊑ B ⊑ C`, `D ⊑ C`, and one instance of `A`.
    const TAXONOMY: &str = "\
<http://example.org/A> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/B> .
<http://example.org/B> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C> .
<http://example.org/D> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C> .
<http://example.org/x> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/A> .
";

    /// `A ⊑ C` — entailed by the chain, asserted nowhere.
    const CHAIN_AXIOM: &str = "<http://example.org/A> \
<http://www.w3.org/2000/01/rdf-schema#subClassOf> <http://example.org/C> .\n";

    /// Call a two-out-param DL entry point and read BOTH buffers back.
    ///
    /// # Safety
    /// `call` must write two live buffer handles when it returns `Ok`.
    unsafe fn pair(
        call: impl FnOnce(*mut *mut PurrdfBuffer, *mut *mut PurrdfBuffer, *mut *mut PurrdfError) -> i32,
    ) -> (String, String) {
        let mut answer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut certificate: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        // SAFETY: the three out-pointers address live, writable locals, and both
        // handed-out buffers are freed by `take`.
        unsafe {
            assert_eq!(
                call(&raw mut answer, &raw mut certificate, &raw mut error),
                PurrdfStatus::Ok as i32
            );
            assert!(error.is_null());
            (take(answer), take(certificate))
        }
    }

    /// Every DL service reaches the POINTER surface, hands out two buffers, and
    /// every certificate names its own service and ends with its own gate.
    ///
    /// The tableau services (`consistency`, `classify`, `realize`, `instances`,
    /// `entails`) have no trailing gate LITERAL: their `purrdf-dl-certificate 1`
    /// block derives `completeness` from `boundary` on every render, so this test
    /// exercises that derivation directly rather than matching a constant that
    /// could only ever read `false`.
    #[test]
    fn every_dl_service_reaches_the_pointer_surface() {
        let document = CString::new(TAXONOMY).expect("no interior NUL");
        let axiom = CString::new(CHAIN_AXIOM).expect("no interior NUL");
        let class = CString::new("<http://example.org/C>").expect("no interior NUL");
        let signature = CString::new("<http://example.org/A>\n").expect("no interior NUL");
        let method = CString::new("star").expect("no interior NUL");
        let regime = CString::new("owl-rl").expect("no interior NUL");
        let conclusion = CString::new(
            "<http://example.org/x> \
             <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/C> .\n",
        )
        .expect("no interior NUL");

        // SAFETY: every C string is live for the whole block, and `pair` owns the
        // out-pointers and frees both buffers it reads.
        let produced: Vec<(&str, (String, String))> = unsafe {
            vec![
                (
                    "consistency",
                    pair(|a, c, e| purrdf_entail_consistency(document.as_ptr(), 0, 0, a, c, e)),
                ),
                (
                    "classify",
                    pair(|a, c, e| purrdf_entail_classify(document.as_ptr(), 0, 0, a, c, e)),
                ),
                (
                    "realize",
                    pair(|a, c, e| purrdf_entail_realize(document.as_ptr(), 0, 0, a, c, e)),
                ),
                (
                    "instances",
                    pair(|a, c, e| {
                        purrdf_entail_instances(document.as_ptr(), class.as_ptr(), 0, 0, a, c, e)
                    }),
                ),
                (
                    "entails",
                    pair(|a, c, e| {
                        purrdf_entail_entails(document.as_ptr(), axiom.as_ptr(), 0, 0, a, c, e)
                    }),
                ),
                (
                    "profile",
                    pair(|a, c, e| purrdf_entail_profile(document.as_ptr(), a, c, e)),
                ),
                (
                    "extract-module",
                    pair(|a, c, e| {
                        purrdf_entail_extract_module(
                            document.as_ptr(),
                            signature.as_ptr(),
                            method.as_ptr(),
                            a,
                            c,
                            e,
                        )
                    }),
                ),
                (
                    "justify",
                    pair(|a, c, e| {
                        purrdf_entail_justify(document.as_ptr(), axiom.as_ptr(), a, c, e)
                    }),
                ),
                (
                    "explain-conclusion",
                    pair(|a, c, e| {
                        purrdf_entail_explain_conclusion(
                            document.as_ptr(),
                            regime.as_ptr(),
                            conclusion.as_ptr(),
                            a,
                            c,
                            e,
                        )
                    }),
                ),
            ]
        };

        assert_eq!(produced.len(), 9);
        for (service, (_answer, certificate)) in produced {
            assert!(
                certificate.contains(&format!("\nservice {service}\n")),
                "{service}: {certificate}"
            );
            if certificate.starts_with("purrdf-dl-certificate 1\n") {
                let completeness = certificate
                    .lines()
                    .find_map(|line| line.strip_prefix("completeness "))
                    .unwrap_or_else(|| panic!("{service}: no completeness line: {certificate}"));
                let has_boundaries = certificate
                    .lines()
                    .any(|line| line.starts_with("boundary "));
                match completeness {
                    "decided" => assert!(!has_boundaries, "{service}: {certificate}"),
                    "decided-within-boundaries" => {
                        assert!(has_boundaries, "{service}: {certificate}");
                    }
                    "budget-exhausted" => {}
                    other => panic!("{service}: unknown completeness {other}"),
                }
            } else {
                let gate = certificate.lines().last().unwrap_or_default();
                assert!(
                    matches!(
                        gate,
                        "minimal true"
                            | "minimal false"
                            | "one-directional true"
                            | "conservative false"
                            | "conservative true"
                            | "checked true"
                            | "checked false"
                    ),
                    "{service}: {gate}"
                );
            }
        }
    }

    /// EVERY conclusion-directed service reaches the POINTER surface, hands out two
    /// buffers, and carries the run that answered.
    ///
    /// The sibling of the test above, and it exists for the same reason: a capability
    /// compiled into the shared object and absent from the committed header is a
    /// capability no C caller can reach. `scripts/check-entailment-surface.py` gates the
    /// header; this gates the behaviour, and the ABI test that scans for `no_mangle`
    /// entry points missing from `purrdf.h` gates the link.
    #[test]
    fn every_conclusion_directed_service_reaches_the_pointer_surface() {
        let regime = CString::new("owl-rl").expect("no interior NUL");
        let document = CString::new(TAXONOMY).expect("no interior NUL");
        let conclusion = CString::new(
            "<http://example.org/x> \
             <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/C> .\n",
        )
        .expect("no interior NUL");
        let pattern = CString::new(
            "<http://example.org/x> \
             <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> ?c .\n",
        )
        .expect("no interior NUL");

        // SAFETY: every C string is live for the whole block, and `pair` owns the
        // out-pointers and frees both buffers it reads.
        let produced: Vec<(&str, (String, String))> = unsafe {
            vec![
                (
                    "certain-answers",
                    pair(|a, c, e| {
                        purrdf_entail_certain_answers(
                            regime.as_ptr(),
                            document.as_ptr(),
                            pattern.as_ptr(),
                            std::ptr::null(),
                            std::ptr::null(),
                            0,
                            a,
                            c,
                            e,
                        )
                    }),
                ),
                (
                    "graph-entails",
                    pair(|a, c, e| {
                        purrdf_entail_graph_entails(
                            regime.as_ptr(),
                            document.as_ptr(),
                            conclusion.as_ptr(),
                            std::ptr::null(),
                            std::ptr::null(),
                            0,
                            a,
                            c,
                            e,
                        )
                    }),
                ),
                (
                    "verify-entailment",
                    pair(|a, c, e| {
                        purrdf_entail_verify_entailment(
                            regime.as_ptr(),
                            document.as_ptr(),
                            conclusion.as_ptr(),
                            std::ptr::null(),
                            std::ptr::null(),
                            0,
                            a,
                            c,
                            e,
                        )
                    }),
                ),
            ]
        };
        assert_eq!(produced.len(), 3);
        for (service, (answer, certificate)) in &produced {
            // The mechanism is the answer's FIRST line on every one of the three, and it
            // is the canonical spelling rather than an ordinal — an ordinal would be a
            // number whose meaning lives in a Rust file no C caller reads.
            assert_eq!(
                answer.lines().next(),
                Some("mechanism strict-table"),
                "{service}: {answer}"
            );
            assert!(
                certificate.starts_with("purrdf-reasoning-report 4\n"),
                "{service}: {certificate}"
            );
            assert!(
                certificate.contains("\nmechanism strict-table "),
                "{service}: {certificate}"
            );
        }
        // …and each said its own thing beyond the shared header.
        assert!(produced[0].1.0.contains("\nvar c\n"), "{}", produced[0].1.0);
        assert!(
            produced[0].1.0.contains("\nrow <http://example.org/C>\n"),
            "{}",
            produced[0].1.0
        );
        assert!(produced[1].1.0.contains("\nentailment entailed\n"));
        assert!(
            produced[2]
                .1
                .0
                .ends_with("warrant present\nverified true\n")
        );
    }

    /// A null argument to any of the three is REFUSED rather than dereferenced.
    #[test]
    fn the_conclusion_directed_services_refuse_null_arguments() {
        let mut answer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut certificate: *mut PurrdfBuffer = std::ptr::null_mut();
        // SAFETY: the out-pointers address live, writable locals; no error channel is
        // requested, so the status code is the whole observable result.
        unsafe {
            let null = std::ptr::null();
            for (service, status) in [
                (
                    "certain-answers",
                    purrdf_entail_certain_answers(
                        null,
                        null,
                        null,
                        std::ptr::null(),
                        std::ptr::null(),
                        0,
                        &raw mut answer,
                        &raw mut certificate,
                        std::ptr::null_mut(),
                    ),
                ),
                (
                    "graph-entails",
                    purrdf_entail_graph_entails(
                        null,
                        null,
                        null,
                        std::ptr::null(),
                        std::ptr::null(),
                        0,
                        &raw mut answer,
                        &raw mut certificate,
                        std::ptr::null_mut(),
                    ),
                ),
                (
                    "verify-entailment",
                    purrdf_entail_verify_entailment(
                        null,
                        null,
                        null,
                        std::ptr::null(),
                        std::ptr::null(),
                        0,
                        &raw mut answer,
                        &raw mut certificate,
                        std::ptr::null_mut(),
                    ),
                ),
            ] {
                assert_eq!(status, PurrdfStatus::NullPointer as i32, "{service}");
            }
        }
        assert!(answer.is_null());
        assert!(certificate.is_null());
    }

    /// A NULL import array with a NON-ZERO count is REFUSED rather than dereferenced —
    /// and two NULL arrays with a count of zero are the ordinary "imports nothing" call.
    ///
    /// The two halves belong in one test because the pair is the contract: refusing the
    /// second would make every ordinary call pass two dummy arrays, and dereferencing the
    /// first would be a segfault a C caller could reach by writing `1` for a table it had
    /// not built.
    #[test]
    fn a_null_import_array_with_a_non_zero_count_is_refused_not_dereferenced() {
        let regime = CString::new("owl-rl").expect("no interior NUL");
        let document = CString::new(TAXONOMY).expect("no interior NUL");
        let conclusion = CString::new(CHAIN_AXIOM).expect("no interior NUL");
        let iri = CString::new("http://example.org/schema").expect("no interior NUL");
        let iris: [*const c_char; 1] = [iri.as_ptr()];
        let mut answer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut certificate: *mut PurrdfBuffer = std::ptr::null_mut();
        // SAFETY: every C string and the one-element array are live for the whole block;
        // the out-pointers address live, writable locals. No error channel is requested,
        // so the status code is the whole observable result — and no buffer is handed out
        // on a refusal, which the two null assertions below check.
        unsafe {
            let (a, c) = (&raw mut answer, &raw mut certificate);
            for (service, status) in [
                (
                    "certain-answers: both arrays null",
                    purrdf_entail_certain_answers(
                        regime.as_ptr(),
                        document.as_ptr(),
                        conclusion.as_ptr(),
                        std::ptr::null(),
                        std::ptr::null(),
                        1,
                        a,
                        c,
                        std::ptr::null_mut(),
                    ),
                ),
                (
                    "graph-entails: the DOCUMENT array null",
                    purrdf_entail_graph_entails(
                        regime.as_ptr(),
                        document.as_ptr(),
                        conclusion.as_ptr(),
                        iris.as_ptr(),
                        std::ptr::null(),
                        1,
                        a,
                        c,
                        std::ptr::null_mut(),
                    ),
                ),
                (
                    "verify-entailment: the IRI array null",
                    purrdf_entail_verify_entailment(
                        regime.as_ptr(),
                        document.as_ptr(),
                        conclusion.as_ptr(),
                        std::ptr::null(),
                        iris.as_ptr(),
                        1,
                        a,
                        c,
                        std::ptr::null_mut(),
                    ),
                ),
            ] {
                assert_eq!(status, PurrdfStatus::NullPointer as i32, "{service}");
            }
        }
        assert!(answer.is_null());
        assert!(certificate.is_null());

        // …and a NULL ELEMENT inside a non-empty array is refused the same way.
        let nulls: [*const c_char; 1] = [std::ptr::null()];
        // SAFETY: as above; the one-element array holds a null the entry point must
        // refuse rather than pass to `CStr::from_ptr`.
        let status = unsafe {
            purrdf_entail_graph_entails(
                regime.as_ptr(),
                document.as_ptr(),
                conclusion.as_ptr(),
                nulls.as_ptr(),
                nulls.as_ptr(),
                1,
                &raw mut answer,
                &raw mut certificate,
                std::ptr::null_mut(),
            )
        };
        assert_eq!(status, PurrdfStatus::NullPointer as i32);
        assert!(answer.is_null());
        assert!(certificate.is_null());

        // The ZERO case is not an error: two null arrays and a count of zero is the
        // premise that imports nothing, which is the overwhelmingly common call.
        // SAFETY: the C strings are live; `pair` owns the out-pointers and frees both.
        let (decided, _) = unsafe {
            pair(|a, c, e| {
                purrdf_entail_graph_entails(
                    regime.as_ptr(),
                    document.as_ptr(),
                    conclusion.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    a,
                    c,
                    e,
                )
            })
        };
        assert!(decided.contains("\nentailment entailed\n"), "{decided}");
    }

    /// The step cap crosses the ABI and drives the third completeness state.
    ///
    /// `unknown` is never collapsed to `false` on the way through the C boundary,
    /// which is the one substitution a two-buffer surface could make silently.
    #[test]
    fn the_step_cap_crosses_the_abi() {
        let document = CString::new(TAXONOMY).expect("no interior NUL");
        let axiom = CString::new(CHAIN_AXIOM).expect("no interior NUL");
        // SAFETY: both C strings are live for the calls; `pair` owns the pointers.
        let (starved, certificate) = unsafe {
            pair(|a, c, e| purrdf_entail_entails(document.as_ptr(), axiom.as_ptr(), 1, 0, a, c, e))
        };
        assert_eq!(starved.lines().next(), Some("entails unknown"));
        assert!(certificate.contains("\ncompleteness budget-exhausted\n"));
        // SAFETY: as above.
        let (decided, certificate) = unsafe {
            pair(|a, c, e| purrdf_entail_entails(document.as_ptr(), axiom.as_ptr(), 0, 0, a, c, e))
        };
        assert!(decided.starts_with("entails true\n"));
        assert!(certificate.contains("\ncompleteness decided\n"));
    }

    /// A failing DL call writes NEITHER out-param, so there is nothing to free.
    #[test]
    fn a_failing_dl_call_writes_no_buffer_to_free() {
        let document = CString::new(TAXONOMY).expect("no interior NUL");
        let class = CString::new("not a term").expect("no interior NUL");
        let mut answer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut certificate: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        // SAFETY: both C strings are live for the call, the three out-pointers
        // address live, writable locals, and the error handle is freed below.
        unsafe {
            assert_eq!(
                purrdf_entail_instances(
                    document.as_ptr(),
                    class.as_ptr(),
                    0,
                    0,
                    &raw mut answer,
                    &raw mut certificate,
                    &raw mut error,
                ),
                PurrdfStatus::ParseError as i32
            );
            assert!(answer.is_null());
            assert!(certificate.is_null());
            assert!(!error.is_null());
            crate::error::purrdf_error_free(error);
        }
    }

    /// Null arguments to the DL entry points are refused, not dereferenced.
    #[test]
    fn null_dl_arguments_are_refused_not_dereferenced() {
        let mut answer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut certificate: *mut PurrdfBuffer = std::ptr::null_mut();
        let null = std::ptr::null();
        // SAFETY: every pointer passed is either null (the case under test) or
        // addresses a live, writable local; no error channel is requested, so the
        // status code is the whole observable result.
        unsafe {
            let (a, c) = (&raw mut answer, &raw mut certificate);
            for status in [
                purrdf_entail_consistency(null, 0, 0, a, c, std::ptr::null_mut()),
                purrdf_entail_classify(null, 0, 0, a, c, std::ptr::null_mut()),
                purrdf_entail_realize(null, 0, 0, a, c, std::ptr::null_mut()),
                purrdf_entail_instances(null, null, 0, 0, a, c, std::ptr::null_mut()),
                purrdf_entail_entails(null, null, 0, 0, a, c, std::ptr::null_mut()),
                purrdf_entail_profile(null, a, c, std::ptr::null_mut()),
                purrdf_entail_extract_module(null, null, null, a, c, std::ptr::null_mut()),
                purrdf_entail_justify(null, null, a, c, std::ptr::null_mut()),
                purrdf_entail_explain_conclusion(null, null, null, a, c, std::ptr::null_mut()),
            ] {
                assert_eq!(status, PurrdfStatus::NullPointer as i32);
            }
        }
    }

    // ── The vendored `owl:imports` case, driven through the pointer surface ──

    /// The W3C OWL 2 RL entailment corpus, as the conformance harness locates it.
    ///
    /// A path rather than a copy: `scripts/check-corpus-frozen.py` digests those bytes,
    /// so a fixture transcribing them here would be a second, un-digested corpus that
    /// could silently drift from the one the conformance scoreboard grades. This crate
    /// is `publish = false`, so reading a sibling crate's vendored tree from a test is a
    /// dev-only path and ships to nobody.
    fn corpus(relative: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../sparql-conformance/entailment-suite/w3c-owl2-rl")
            .join(relative)
    }

    /// One vendored RDF/XML document, converted to N-Quads THROUGH THE C ABI.
    ///
    /// `purrdf_parse` + `purrdf_serialize` rather than the Rust API: a C caller holding
    /// an RDF/XML ontology reaches the entailment services exactly this way, and a test
    /// that took a shortcut around the ABI would not be testing the host.
    ///
    /// No base IRI is passed, and none is needed — every document in the vendored tree
    /// either declares its own `xml:base` or uses only absolute IRIs, which the
    /// conformance harness asserts as a standing tripwire.
    fn corpus_nquads(relative: &str) -> String {
        let bytes = std::fs::read(corpus(relative)).expect("the vendored document");
        let media_type = CString::new("application/rdf+xml").expect("no interior NUL");
        let nquads_type = CString::new("application/n-quads").expect("no interior NUL");
        let mut dataset: *mut crate::handles::PurrdfDataset = std::ptr::null_mut();
        let mut buffer: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        // SAFETY: `bytes` is live for the parse call and its length is its own; both C
        // strings are live for the whole block; every out-pointer addresses a live,
        // writable local; the dataset handle and the buffer are each released once.
        unsafe {
            assert_eq!(
                crate::parse::purrdf_parse(
                    bytes.as_ptr(),
                    bytes.len(),
                    media_type.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    &raw mut dataset,
                    &raw mut error,
                ),
                PurrdfStatus::Ok as i32
            );
            assert!(error.is_null());
            assert_eq!(
                crate::serialize::purrdf_serialize(
                    dataset,
                    nquads_type.as_ptr(),
                    std::ptr::null(),
                    &raw mut buffer,
                    std::ptr::null_mut(),
                    &raw mut error,
                ),
                PurrdfStatus::Ok as i32
            );
            assert!(error.is_null());
            crate::handles::purrdf_dataset_free(dataset);
            take(buffer)
        }
    }

    /// `webont-imports-011` ANSWERS ON THIS HOST, from its own premise, `owl:imports`
    /// INTACT.
    ///
    /// The case the whole parameter exists for. Its premise says `Socrates a ont:Man`
    /// and `owl:imports <…/support011-A>`; `Man ⊑ Mortal` lives only in that support
    /// document, so the published answer — `Socrates a ont:Mortal` — is reachable only
    /// from the imports closure. Before this parameter existed the C ABI could not
    /// express the support document at all, and this premise was a permanent refusal.
    ///
    /// The premise is handed over UNMODIFIED: nothing is merged into it and the
    /// `owl:imports` triple is left exactly where W3C wrote it, which is the whole
    /// difference between resolving an import and being given a different premise.
    #[test]
    fn the_vendored_imports_case_answers_through_the_pointer_surface() {
        let premise_text = corpus_nquads("cases/webont-imports-011/premise.rdf");
        // The premise really does carry the import, so the test cannot pass by having
        // quietly been handed a document that needs none.
        assert!(
            premise_text.contains("<http://www.w3.org/2002/07/owl#imports>"),
            "{premise_text}"
        );
        let conclusion_text = corpus_nquads("cases/webont-imports-011/conclusion.rdf");
        let support_text = corpus_nquads("imports/support011-A.rdf");

        let regime = CString::new("owl-rl").expect("no interior NUL");
        let premise = CString::new(premise_text).expect("no interior NUL");
        let conclusion = CString::new(conclusion_text).expect("no interior NUL");
        let support = CString::new(support_text).expect("no interior NUL");
        // The ontology IRI the support document declares — the name the premise's
        // `owl:imports` object actually is, not the file it happens to live in.
        let iri = CString::new("http://www.w3.org/2002/03owlt/imports/support011-A")
            .expect("no interior NUL");
        let iris: [*const c_char; 1] = [iri.as_ptr()];
        let documents: [*const c_char; 1] = [support.as_ptr()];

        // SAFETY: every C string and both one-element arrays are live for the whole
        // block; `pair` owns the out-pointers and frees both buffers it reads.
        let (answer, certificate) = unsafe {
            pair(|a, c, e| {
                purrdf_entail_graph_entails(
                    regime.as_ptr(),
                    premise.as_ptr(),
                    conclusion.as_ptr(),
                    iris.as_ptr(),
                    documents.as_ptr(),
                    1,
                    a,
                    c,
                    e,
                )
            })
        };
        assert_eq!(answer.lines().next(), Some("mechanism strict-table"));
        assert!(answer.contains("\nentailment entailed\n"), "{answer}");
        assert!(certificate.starts_with("purrdf-reasoning-report 4\n"));

        // …and the SAME call with an empty table refuses by name rather than answering
        // from a premise that is missing the axioms it told the caller about.
        let mut answer_ptr: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut certificate_ptr: *mut PurrdfBuffer = std::ptr::null_mut();
        let mut error: *mut PurrdfError = std::ptr::null_mut();
        // SAFETY: as above; the error handle is read and freed below.
        unsafe {
            assert_eq!(
                purrdf_entail_graph_entails(
                    regime.as_ptr(),
                    premise.as_ptr(),
                    conclusion.as_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    &raw mut answer_ptr,
                    &raw mut certificate_ptr,
                    &raw mut error,
                ),
                PurrdfStatus::ParseError as i32
            );
            assert!(answer_ptr.is_null());
            assert!(certificate_ptr.is_null());
            assert!(!error.is_null());
            let message = crate::error::purrdf_error_message(error);
            assert!(!message.is_null());
            let message = std::ffi::CStr::from_ptr(message)
                .to_str()
                .expect("the boundary emits UTF-8");
            assert!(
                message.contains("http://www.w3.org/2002/03owlt/imports/support011-A"),
                "{message}"
            );
            crate::error::purrdf_error_free(error);
        }
    }
}
