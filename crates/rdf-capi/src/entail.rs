// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf_entail_*`: SPARQL entailment-**regime** materialization, the rule
//! inventories, and the OWL 2 Direct-Semantics reasoning services — all over the
//! shared [`PurrdfBuffer`].
//!
//! # Two lanes, two certificates
//!
//! [`purrdf_entail_materialize_to_nquads`] is the **chase**: it renders a
//! `purrdf-reasoning-report 2` block whose completeness is `exact` /
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
//!   materialization (`simple` / `rdf` / `rdfs` / `owl-rl`). It takes no shapes
//!   at all: it closes a document under the regime's own specification rule
//!   table.
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
    ReasoningAnswer, classify_to_string, consistency_to_string, entails_to_string,
    explain_conclusion_to_string, extract_module_to_string, implemented_rules_string,
    instances_to_string, justify_to_string, materialize_to_nquads_string, profile_to_string,
    realize_to_string, rules_string,
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
/// An empty buffer for a regime with no rule table (`simple`, and the two that
/// are not forward-materializable). `owl-rl` yields all 78 rules of OWL 2
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
                consistency_to_string(document, step_cap),
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
/// block. **Free BOTH with `purrdf_buffer_free`.** `step_cap` behaves exactly as in
/// [`purrdf_entail_consistency`].
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
                classify_to_string(document, step_cap),
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
/// `purrdf_buffer_free`.**
///
/// # Safety
/// As [`purrdf_entail_consistency`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn purrdf_entail_realize(
    document: *const c_char,
    step_cap: u32,
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
                realize_to_string(document, step_cap),
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
                instances_to_string(document, class, step_cap),
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
                entails_to_string(document, axiom, step_cap),
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

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use purrdf_validate::regime::{check_inconsistent_refusal, check_regime_golden_vectors};

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
        assert!(report.starts_with("purrdf-reasoning-report 2\n"));
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
            assert!(take(report).starts_with("purrdf-reasoning-report 2\n"));
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
                    pair(|a, c, e| purrdf_entail_consistency(document.as_ptr(), 0, a, c, e)),
                ),
                (
                    "classify",
                    pair(|a, c, e| purrdf_entail_classify(document.as_ptr(), 0, a, c, e)),
                ),
                (
                    "realize",
                    pair(|a, c, e| purrdf_entail_realize(document.as_ptr(), 0, a, c, e)),
                ),
                (
                    "instances",
                    pair(|a, c, e| {
                        purrdf_entail_instances(document.as_ptr(), class.as_ptr(), 0, a, c, e)
                    }),
                ),
                (
                    "entails",
                    pair(|a, c, e| {
                        purrdf_entail_entails(document.as_ptr(), axiom.as_ptr(), 0, a, c, e)
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
            let gate = certificate.lines().last().unwrap_or_default();
            assert!(
                matches!(
                    gate,
                    "overclaims false" | "one-directional true" | "conservative false"
                ),
                "{service}: {gate}"
            );
        }
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
            pair(|a, c, e| purrdf_entail_entails(document.as_ptr(), axiom.as_ptr(), 1, a, c, e))
        };
        assert_eq!(starved.lines().next(), Some("entails unknown"));
        assert!(certificate.contains("\ncompleteness budget-exhausted\n"));
        // SAFETY: as above.
        let (decided, certificate) = unsafe {
            pair(|a, c, e| purrdf_entail_entails(document.as_ptr(), axiom.as_ptr(), 0, a, c, e))
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
                purrdf_entail_consistency(null, 0, a, c, std::ptr::null_mut()),
                purrdf_entail_classify(null, 0, a, c, std::ptr::null_mut()),
                purrdf_entail_realize(null, 0, a, c, std::ptr::null_mut()),
                purrdf_entail_instances(null, null, 0, a, c, std::ptr::null_mut()),
                purrdf_entail_entails(null, null, 0, a, c, std::ptr::null_mut()),
                purrdf_entail_profile(null, a, c, std::ptr::null_mut()),
                purrdf_entail_extract_module(null, null, null, a, c, std::ptr::null_mut()),
                purrdf_entail_justify(null, null, a, c, std::ptr::null_mut()),
                purrdf_entail_explain_conclusion(null, null, null, a, c, std::ptr::null_mut()),
            ] {
                assert_eq!(status, PurrdfStatus::NullPointer as i32);
            }
        }
    }
}
