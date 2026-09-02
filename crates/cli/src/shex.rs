// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `shex` subcommand: `Schema + Data + Shape map → decide → result shape map`.
//!
//! ShEx 2.1 validation on the command line, over the same `purrdf_shex` engine the library
//! exposes. The schema parser, the structural checker, the import folder and the shape-map
//! validator are all the library's; this module resolves formats, reads documents, refuses the
//! command lines that would produce a verdict nobody can act on, and serializes what comes
//! back.
//!
//! # The output is the ShapeMap specification's result shape map, and there is no `--format`
//!
//! [`ResultShapeMap::to_result_json`](purrdf_shex::ResultShapeMap::to_result_json) is
//! `purrdf-shex`'s single rendered form: a JSON array of `{"node","shape","status","reason"?}`
//! objects with a fixed field order, in the engine's own deterministic association order
//! (query selectors de-duplicate their matches and sort by term string). A `--format` flag
//! would have exactly one legal value, and a second renderer would be a second opinion about
//! the same verdict — so there is neither.
//!
//! # Both verdicts exit 0, and the verdict is on stderr
//!
//! Identical to [`validate`](crate::validate)'s contract and for the identical reason: a
//! nonconformant node is a DECIDED verdict, the artifact on stdout is the answer either way,
//! and the summary goes to stderr (`shex conformant true|false`, `shex entries N`, `shex
//! nonconformant N`) so stdout stays a well-formed JSON document. There is no exit **3**
//! here — the ShEx engine takes no execution governors, so there is no budget to trip.
//!
//! # Four refusals, because four kinds of "unavailable semantics" otherwise become verdicts
//!
//! `purrdf-shex` is honest about what it cannot decide, and each of its honest fallbacks
//! becomes a LIE the moment it is printed as a conformance verdict rather than acted on:
//!
//! * an **unresolved `IMPORT`** would leave the imported labels dangling, so the schema being
//!   validated is not the schema the operator wrote. Resolution is caller-supplied
//!   (`--import IRI=FILE`, the same shape `entails --import` takes), because PurRDF fetches
//!   nothing and reads only the files it was handed. An import no pair resolves is refused by
//!   name — and a pair the import closure never reaches is refused too, rather than accepted
//!   and silently unused.
//! * an **`EXTERNAL` shape** has its semantics defined outside the schema. With no resolver,
//!   the library documents that it "fails every node" — a definite `nonconformant` derived
//!   from semantics nobody supplied. The resolver is a host Rust closure, which cannot cross a
//!   command-line boundary as a string (the same reason `query --aggregate-namespace` carries
//!   no surface for a caller-defined aggregate), so the schema is refused by name instead.
//! * a **semantic action** dispatches to an extension the caller registers. The default
//!   registry is empty and every action is then "an inert success" — so a schema that says
//!   "and this custom check also passes" would report `conformant` without the check ever
//!   running. Extensions are host closures too, so this is refused by name for the same
//!   reason.
//! * a **`MAP` naming a shape the schema does not declare** has nothing to decide against.
//!   Neither the ShapeMap specification nor ShEx 2.1 defines the case — §5.7's reference
//!   requirement binds a `shapeExprRef` written inside a SCHEMA (which `check_structure`
//!   enforces above), and `satisfies` is defined only where the label resolves to a shape
//!   expression — and the result shape map's `status` vocabulary is `conformant` /
//!   `nonconformant` with no third value meaning "not evaluated". So `nonconformant` would
//!   spend the one word the format has for a finding about the DATA on a mistake the data had
//!   no part in. Refused by name, against the schema that was searched.
//!
//! All four are exit **1** and name the construct, never a weaker answer labelled as the one
//! that was asked for. Exit **1** rather than **2** because each is a property of the
//! DOCUMENTS, discovered by reading them, rather than of the command line — which is what
//! exit **2** is for in this binary.
//!
//! # The command line is decided FIRST, and its faults are exit 2
//!
//! [`crate::error`] draws the exit line at when a fault is KNOWABLE: exit **2** is a malformed
//! command line (matching clap's own code), exit **1** is a failure discovered by reading a
//! document. Two of this subcommand's arguments used to fall on the wrong side of it, because
//! nothing decided them until a document had already been opened:
//!
//! * the **`MAP` argument** is command-line text with its own grammar. A prefixed name, or a
//!   relative IRI with no `--base`, is a fault in the argument and needs no schema and no data
//!   graph to see — yet it surfaced from inside `validate_shape_map`, after both documents had
//!   been read, as exit 1. [`check_map_syntax`] now parses it through the SAME
//!   [`purrdf_shex::parse_shape_map`] the validator uses, before anything is opened, and
//!   reports exit 2. A relative reference there also gets a remedy naming `--base`, because the
//!   library's own remedy names `@base`/`xml:base` document directives and a `MAP` is argv text
//!   no document reaches.
//! * the **`--import` ontology-IRI half** is matched against the schema's `IMPORT` IRIs, which
//!   are absolute, AND is the base its document parses under — so a relative or malformed half
//!   can do neither job. It used to surface as exit 1 from the imported document's parser,
//!   after that document had been read. [`resolve_import_pairs`] now decides every pair's whole
//!   argument shape — the `=`, both halves, `-`, duplicates, the syntax its extension implies,
//!   and the half's absoluteness through the shared [`purrdf_iri::BaseIri`] — with no I/O at
//!   all, so the FIRST malformed pair is reported before the FIRST file is opened.
//!
//! What deliberately did NOT move is [`purrdf_shex::ShexError::UnknownShape`]: a `MAP` naming a
//! label the schema does not declare cannot be detected until the schema is loaded and its
//! import closure walked, so it sits on the runtime side of the same line and stays exit 1.
//! The two used to share one error channel out of `validate_shape_map`; they are split rather
//! than made to follow each other, with the syntax half decided in front and the
//! schema-dependent half left exactly where it was.
//!
//! # What "RDF 1.2" means to a ShEx neighbourhood
//!
//! ShEx 2.1 predates RDF 1.2 and its data model is arcs, so the two halves of PurRDF's
//! statement support reach a shape map differently and it is worth saying which is which
//! rather than leaving a caller to infer it from an empty result.
//!
//! A **triple term** is an ordinary RDF term: `ex:claim ex:states <<( ex:a ex:p ex:b )>>` is a
//! quad, so a triple constraint matches that arc, `NONLITERAL` accepts the object, and a
//! shape map may name the triple term itself as a focus node (`{_ <ex:states> FOCUS}`, or the
//! literal `<< … >>` term syntax) and validate its — empty — neighbourhood.
//!
//! The **statement layer** (a reifier's `rdf:reifies` binding and the annotations hung off it,
//! what Turtle's `{| … |}` mints) is a separate side table in the IR rather than quads, and
//! `purrdf_shex` reads it alongside the quad table. So a shape map selector over an annotation
//! predicate selects the reifier that carries it, and a shape whose focus is a reifier sees a
//! neighbourhood that is the union of its ordinary arcs, its `rdf:reifies` arc, and its
//! annotations. ShEx 2.1 predates RDF 1.2 and describes only arcs; PurRDF extends the data
//! model rather than inheriting the gap, so `shex`, `validate` (SHACL) and `query` (SPARQL)
//! all answer alike over the same document.
//!
//! # Three documents, three bases — `--base` names the data graph, not the schema
//!
//! This command reads THREE independent documents (a data graph, a schema, a query shape
//! map), and a base IRI is a property of one document rather than of the invocation. Handing
//! one document's base to another silently re-homes its relative terms onto a namespace its
//! author never wrote, which is exactly the failure `validate` refuses when it declines to
//! pass `--base` to the shapes graph.
//!
//! * the **data graph** takes `--base` when given, else its own RFC-8089 `file://` retrieval
//!   IRI (RFC-3986 §5.1.2 then §5.1.3), through the same [`source::effective_base`] seam
//!   every other RDF input in this binary takes;
//! * the **schema** takes its OWN retrieval IRI and never `--base`. `<S1> { <p1> . }` in
//!   `s.shex` therefore means `<…/S1>` and `<…/p1>` relative to `s.shex`, which is what an
//!   author writing a self-contained schema means — and supplying `--base` to fix a relative
//!   data graph can no longer move the schema's shapes to the data document's namespace. A
//!   `BASE` directive inside the schema still wins over the retrieval IRI (§5.1.1);
//! * each **`--import IRI=FILE`** document is parsed under the IMPORT IRI, because that is
//!   the name the importing schema gave it — the per-document base
//!   [`purrdf_shex::resolve_imports`] documents as its injection boundary;
//! * the **shape map** is command-line text with no retrieval IRI at all, so `--base` is the
//!   only base it can ever have and it legitimately takes one. That asymmetry is stated on
//!   `--base`'s help rather than left to be discovered.
//!
//! `--schema -` keeps the hard error: a piped schema has no retrieval IRI, so a relative
//! reference in it is the RFC-3986 §5.1.4 failure naming `BASE`, never a vacuous constraint.
//!
//! ## `--base` is never inert here, so it is never refused
//!
//! Every other subcommand refuses a `--base` no leg of the run can spend
//! ([`crate::format::refuse_unconsumable_base`], decided off the format registry's
//! `admits_relative_iri` / `emits_base` columns). `shex` does not, because its base has a
//! consumer no format row can name and that consumer is always present: `MAP` is a REQUIRED
//! argument, it is command-line text with no retrieval IRI, and the ShapeMap grammar admits
//! relative IRI references (`<alice>@<UserShape>`). So a base handed to this command always
//! has somewhere to be spent, whatever `--data`'s syntax is.
//!
//! That includes a **pack or GTS** data graph. A container stores fully-resolved terms and
//! cannot use a base, but the shape map still can — the same one-live-leg reasoning that
//! keeps `convert --base X --to ntriples` working from a relative-admitting source. This lane
//! used to refuse that pairing on the data source's syntax alone, which rejected the entirely
//! legitimate "verified pack plus a relative shape map" and was justified by a base reaching
//! the SCHEMA — a leg that no longer exists now that the schema carries its own retrieval IRI.
//!
//! # No ledger, and nothing transcoded
//!
//! The data graph is read through the pipeline's own format resolution straight into the
//! frozen IR — any of the nine native syntaxes, or a verified pack — and the validator reads
//! that IR. Nothing crosses a serializer in either direction, so `--loss-ledger` has no
//! conversion to record and `--jsonld-options` has no serializer to configure; both are
//! refused by name rather than accepted and ignored.

use std::cell::RefCell;
use std::collections::BTreeSet;

use purrdf::shex::{
    ResultShapeMap, Schema, SemAct, Shape, ShapeExpr, ShexError, TripleExpr, TripleExprGroup,
    ValidationOptions, check_structure, parse_shape_map, parse_shexc, parse_shexj, resolve_imports,
    validate_shape_map,
};
use purrdf_rdf::JsonLdSerializeOptions;

use crate::cli::{CliRdfFormat, CliShexFormat, LedgerTarget};
use crate::error::CliError;
use crate::{format, sink, source};

/// The resolved `shex` flags.
pub(crate) struct ShexOptions<'a> {
    /// `--schema`: the ShEx schema document, or `-`.
    pub(crate) schema: &'a str,
    /// `--schema-from`: the schema syntax override.
    pub(crate) schema_from: Option<CliShexFormat>,
    /// `--import IRI=FILE`, in the order the operator wrote them.
    pub(crate) imports: &'a [String],
    /// `--data`: the data graph, or `-`.
    pub(crate) data: &'a str,
    /// `--from`: the data-graph format override.
    pub(crate) from: Option<CliRdfFormat>,
    /// `--base`: the base relative IRIs in the DATA graph and the shape map resolve against.
    ///
    /// Deliberately NOT the schema's: see the module documentation for why each of this
    /// command's three documents gets its own base.
    pub(crate) base: Option<&'a str>,
    /// The query shape map, verbatim from the command line.
    pub(crate) map: &'a str,
    /// The result-shape-map path `OUT`, or `-`.
    pub(crate) output: &'a str,
    /// Explicit JSON-LD/YAML-LD serialization configuration, which this command refuses.
    pub(crate) jsonld_options: Option<&'a JsonLdSerializeOptions>,
}

/// Run the `shex` subcommand.
pub(crate) fn run(options: &ShexOptions<'_>, ledger_target: &LedgerTarget) -> Result<(), CliError> {
    refuse_document_flags(ledger_target, options.jsonld_options)?;
    refuse_two_stdins(options)?;
    // EVERY command-line decision happens here, before a single document is opened, so a fault
    // in an ARGUMENT is reported against the argument (exit 2) rather than surfacing later out
    // of a parser that no longer knows the value came from the command line.
    let import_pairs = resolve_import_pairs(options)?;
    check_map_syntax(options)?;

    let data_format = format::resolve(options.from, options.data)?;
    // No `--base` refusal here, unlike every other subcommand: see the module documentation
    // for why this run's base can never be inert.
    let data = source::load_dataset(options.data, data_format, options.base)?;

    let schema_base = schema_base(options.schema)?;
    let schema = read_schema(options.schema, options.schema_from, schema_base.as_deref())?;
    let schema = fold_imports(schema, &import_pairs, options)?;
    // Structure BEFORE the unavailable-semantics survey: a schema with a dangling reference
    // is malformed outright, and reporting an `EXTERNAL` inside it would describe a shape
    // that may not even be reachable.
    check_structure(&schema).map_err(|errors| {
        CliError::Runtime(format!(
            "--schema {}: the schema violates the ShEx 2.1 §5.7 structural requirements:\n{}",
            options.schema,
            errors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n")
        ))
    })?;
    refuse_unavailable_semantics(&schema, options.schema)?;

    let result = validate_shape_map(
        &schema,
        &data,
        options.map,
        options.base,
        &ValidationOptions::default(),
    )
    .map_err(|error| match error {
        // The one map failure that is about the PAIRING of two documents rather
        // than about the map's own syntax, so the refusal names the other one. It stays a
        // RUNTIME failure (exit 1) precisely because it cannot be seen from the command line:
        // deciding it needs the schema loaded and its import closure walked.
        ShexError::UnknownShape(_) => CliError::Runtime(format!(
            "MAP: {error} (--schema {}). A shape map is validated AGAINST a schema, so a label \
             neither the schema nor its import closure declares names nothing to decide. \
             Reporting it as `nonconformant` would spend the result shape map's one word for a \
             data finding on a mistake the data had no part in",
            options.schema
        )),
        // Defensive, and expected to be unreachable: `validate_shape_map` is
        // `parse_shape_map` → undeclared-shape check → `validate_with`, the first of which
        // `check_map_syntax` already ran over the same text and base (so it would have failed
        // there, as exit 2) and the last of which returns no `Err`. Reported rather than
        // unwrapped, because an unreachable panic in a CLI is a crash report.
        other => CliError::Runtime(format!("MAP: {other}")),
    })?;

    let mut json = result.to_result_json();
    json.push('\n');
    sink::write_out(options.output, json.as_bytes())?;
    surface_verdict(&result);
    Ok(())
}

/// Write the three-line verdict summary to stderr.
///
/// After the artifact, so a `> file` redirect has the result shape map on disk by the time the
/// operator reads the lines describing it. `nonconformant` is stated as its own count rather
/// than left to be derived, because "how many failed" is the number a shell branches on.
fn surface_verdict(result: &ResultShapeMap) {
    let nonconformant = result
        .entries
        .iter()
        .filter(|entry| entry.status == purrdf::shex::ConformanceStatus::Nonconformant)
        .count();
    eprintln!("shex conformant {}", result.all_conformant());
    eprintln!("shex entries {}", result.entries.len());
    eprintln!("shex nonconformant {nonconformant}");
}

/// Decide the `MAP` argument's own grammar, before any document is opened.
///
/// A `MAP` is command-line text: its syntax and its IRI resolution depend on the argument and
/// on `--base`, and on nothing this run reads. So a fault in it is a fault in the COMMAND LINE
/// (exit 2), and it is reported before the schema and the data graph are read rather than from
/// inside the validator afterwards.
///
/// This is [`parse_shape_map`] — the very function [`validate_shape_map`] calls first — over
/// the same text and the same base, so the classification here cannot come to disagree with
/// the parse that decides. The parsed map is discarded: the validator owns the decision, and
/// keeping a second copy of it would be the second opinion this binary refuses. The one
/// failure NOT decided here is [`ShexError::UnknownShape`], which needs the schema.
fn check_map_syntax(options: &ShexOptions<'_>) -> Result<(), CliError> {
    parse_shape_map(options.map, options.base)
        .map(|_| ())
        .map_err(|error| map_refusal(options.map, &error))
}

/// The refusal for a `MAP` argument that does not parse.
///
/// A relative IRI reference with no base gets a remedy naming `--base`, and NOT the library's
/// own: that one names `@base`/`BASE`/`xml:base`, which are DOCUMENT directives, and a `MAP` is
/// argv text that no document reaches — the identical wrong-remedy shape `describe --iri`,
/// `validate --shapes-graph` and `entails --import` each fixed for their own surface. The
/// shared [`purrdf_iri::IriError::diagnostic_code`] still travels, inside the library's
/// rendered reason, so the failure groups with every other IRI failure in this toolkit.
fn map_refusal(map: &str, error: &ShexError) -> CliError {
    if let ShexError::Iri { lexical, reason } = error
        && reason.starts_with(RELATIVE_NO_BASE)
    {
        // `reason` is the library's `{code}: {IriError}`, and `IriError`'s `Display` ENDS with
        // its own remedy — the `@base`/`xml:base` one. Interpolating it here and then adding
        // the remedy that actually applies would print two remedies, one of them wrong, which
        // is the exact defect `validate --shapes-graph` was fixed for. So the condition is
        // restated from the parts and the library's sentence is not used at all; only the
        // shared CODE travels, which is the part every consumer switches on.
        return CliError::Usage(format!(
            "MAP `{map}`: {RELATIVE_NO_BASE}: the reference `{lexical}` is a relative IRI \
             reference with no base in scope, so it denotes no node. A shape map is a \
             command-line value with no document of its own, so no `@base`/`BASE` you write \
             anywhere resolves it: pass --base <IRI> — which this command spends on the shape \
             map and the data graph — or write the reference in absolute form"
        ));
    }
    CliError::Usage(format!("MAP `{map}`: {error}"))
}

/// The one `purrdf-iri` code whose library remedy names DOCUMENT directives, and so has to be
/// re-worded for a command-line value.
///
/// Single-owned here so the branch condition and the rendered code cannot drift apart: every
/// other `iri-*` code's remedy ("write the IRI in absolute form, with a scheme") already fits
/// an argument and is passed through verbatim.
const RELATIVE_NO_BASE: &str = "iri-relative-no-base";

/// The base the SCHEMA document parses under: its own RFC-8089 `file://` retrieval IRI.
///
/// `--base` is deliberately absent from this derivation. It names the base of the DATA graph,
/// and the schema is an independent document — the identical rule `validate` states for its
/// shapes graph, applied to the identical situation. Reusing it here would mean that
/// supplying a base to resolve `<alice>` in the data silently moved every `<S1>` in the
/// schema onto the data document's namespace.
///
/// `-` (stdin) gets NO base and keeps the hard error: a piped schema has no retrieval IRI to
/// derive one from, so there is no honest answer and the parse refuses (RFC-3986 §5.1.4). A
/// `BASE` directive inside a ShExC schema still overrides what this returns (§5.1.1).
///
/// The shared derivation's failure is RE-WORDED here, and only here. [`source::retrieval_base_iri`]
/// ends its message with "pass --base explicitly", which is the right escape hatch everywhere a
/// derived retrieval IRI would have become the parse base — but not in this command, where
/// `--base` is deliberately never applied to the schema. Passing it would change nothing, so
/// offering it would send an operator whose real problem is an unreadable path to try a flag
/// that cannot help. The exit code stays **1**: a path that does not resolve is a fact about
/// the document, not about the shape of the command line.
fn schema_base(path: &str) -> Result<Option<String>, CliError> {
    if path == "-" {
        return Ok(None);
    }
    source::retrieval_base_iri(path).map(Some).map_err(|error| {
        let rendered = error.to_string();
        let without_base_hint = rendered
            .strip_suffix("; pass --base explicitly")
            .unwrap_or(&rendered);
        CliError::Runtime(format!(
            "--schema {without_base_hint}. The schema resolves its relative IRIs against its \
             OWN retrieval IRI, and `--base` names the DATA graph's base here, so passing one \
             would not supply this: give --schema a readable path, or write a `BASE` directive \
             in the schema"
        ))
    })
}

/// Read and parse the root schema.
///
/// The syntax is resolved the way every other format in this binary is: an explicit
/// `--schema-from` wins, otherwise the path's extension classifies it, and `-` (stdin) has no
/// extension so it REQUIRES the override. `base` reaches BOTH syntaxes: ShExC resolves its
/// IRIREFs against the `BASE` in force, and ShExJ is a JSON-LD dialect whose IRI-valued
/// members are document-relative in exactly the same way, so one base decides one schema
/// whichever spelling it arrived in.
fn read_schema(
    path: &str,
    explicit: Option<CliShexFormat>,
    base: Option<&str>,
) -> Result<Schema, CliError> {
    let syntax = resolve_schema_format(explicit, path, "--schema", SyntaxOverride::SchemaFrom)?;
    let text = read_text(path, "--schema")?;
    match syntax {
        CliShexFormat::Shexc => parse_shexc(&text, base),
        CliShexFormat::Shexj => parse_shexj(&text, base),
    }
    .map_err(|error| CliError::Runtime(format!("--schema {path}: {error}")))
}

/// Resolve a ShEx document's syntax from an explicit choice or its path extension.
/// Whether the document whose syntax is being resolved has a flag that can override it.
///
/// Only the ROOT schema does. `--schema-from` names the root schema's syntax and nothing
/// else — an import closure may legitimately mix ShExC and ShExJ, so each imported document is
/// classified by its own extension. Telling an operator with an unclassifiable IMPORT path to
/// "pass an explicit --schema-from" named a flag that would not have applied to it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SyntaxOverride {
    /// The root schema: `--schema-from` overrides the inference.
    SchemaFrom,
    /// An imported document: its own extension is the only classifier there is.
    None,
}

fn resolve_schema_format(
    explicit: Option<CliShexFormat>,
    path: &str,
    what: &str,
    override_flag: SyntaxOverride,
) -> Result<CliShexFormat, CliError> {
    if let Some(choice) = explicit {
        return Ok(choice);
    }
    let remedy = match override_flag {
        SyntaxOverride::SchemaFrom => "pass an explicit --schema-from",
        SyntaxOverride::None => {
            "an imported document is classified by its OWN extension, because --schema-from \
             names the root schema's syntax; give it a recognized one"
        }
    };
    if path == "-" {
        return Err(CliError::Usage(format!(
            "{what} - reads standard input, which has no extension to infer a ShEx syntax from: \
             {remedy}"
        )));
    }
    let extension = std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase);
    match extension.as_deref() {
        Some("shex" | "shexc") => Ok(CliShexFormat::Shexc),
        Some("shexj" | "json") => Ok(CliShexFormat::Shexj),
        _ => Err(CliError::Usage(format!(
            "{what} {path}: cannot infer a ShEx syntax from this path — the recognized \
             extensions are `.shex`/`.shexc` (ShExC) and `.shexj`/`.json` (ShExJ); {remedy}"
        ))),
    }
}

/// Fold the schema's transitive `IMPORT` closure in from the `--import IRI=FILE` table.
///
/// Every pair is parsed BEFORE resolution starts, so an unreadable or malformed imported
/// document fails against the file the operator named rather than as a refusal attributed to
/// the root schema's `IMPORT` directive. The resolver records every IRI it is ASKED for, which
/// is what makes both halves of the contract enforceable afterwards: an import with no pair is
/// refused by name, and a pair the closure never reached is refused as unused rather than
/// silently ignored.
fn fold_imports(
    schema: Schema,
    pairs: &[ImportPair<'_>],
    options: &ShexOptions<'_>,
) -> Result<Schema, CliError> {
    let table = read_import_table(pairs)?;
    let requested: RefCell<BTreeSet<String>> = RefCell::new(BTreeSet::new());

    let resolved = {
        let resolver = |iri: &str| -> Result<Schema, ShexError> {
            requested.borrow_mut().insert(iri.to_owned());
            table
                .iter()
                .find(|(pair_iri, _)| pair_iri == iri)
                .map(|(_, schema)| schema.clone())
                // Never rendered: the CLI replaces the whole error below with one that names
                // the flag the operator has to write.
                .ok_or_else(|| ShexError::shexj("no --import pair resolves this IRI"))
        };
        resolve_imports(schema, &resolver)
    };

    let resolved = resolved.map_err(|error| match error {
        ShexError::Import { iri, .. } => CliError::Runtime(format!(
            "the schema imports <{iri}>, and no `--import <{iri}>=FILE` pair resolves it. PurRDF \
             fetches nothing the operator did not name, so an unresolved import is refused \
             rather than folded in as an empty schema"
        )),
        other => CliError::Runtime(format!("--schema {}: {other}", options.schema)),
    })?;

    // A pair the closure never reached, quoted back exactly as the operator wrote it. This
    // one refusal is a USAGE error (exit 2) that nevertheless needs the schema to decide,
    // which is not a contradiction: the fault is in the command line — a pair that should not
    // have been written — and only its DETECTION needs the closure walked. Reporting it as a
    // runtime failure would blame the documents for an argument nobody asked for.
    let requested = requested.into_inner();
    if let Some(pair) = pairs.iter().find(|pair| !requested.contains(pair.iri)) {
        return Err(CliError::Usage(format!(
            "--import {}: the schema's import closure never reaches <{}>, so this document \
             would be read and never used. Remove the pair, or import the IRI from the schema",
            pair.spec, pair.iri
        )));
    }
    Ok(resolved)
}

/// One `--import IRI=FILE` argument, fully DECIDED but not yet read.
struct ImportPair<'a> {
    /// The pair exactly as the operator wrote it, so a diagnostic can quote it back.
    spec: &'a str,
    /// The ontology-IRI half, checked absolute.
    iri: &'a str,
    /// The document-path half.
    path: &'a str,
    /// The syntax the path's own extension classifies it as.
    syntax: CliShexFormat,
}

/// Decide every `--import IRI=FILE` ARGUMENT, with no I/O at all.
///
/// A malformed pair is a usage error naming the argument, never a skipped import: a schema
/// folded without a document the operator supplied is a different schema. Because nothing here
/// touches the filesystem, the FIRST bad pair is reported before the FIRST file is opened —
/// where the old shape read pair one from disk before noticing that pair two had no `=`.
///
/// # The ontology-IRI half must be absolute
///
/// It does two jobs and neither admits a relative reference: it is MATCHED against the schema's
/// `IMPORT` IRIs, which are absolute by the time the schema parser is done with them, and it is
/// the BASE the imported document parses under (the per-document base
/// [`purrdf_shex::resolve_imports`] documents as its injection boundary). So it is checked with
/// [`purrdf_iri::BaseIri::parse`] — the workspace's shared "valid IRI with a scheme" primitive,
/// the same gate `--base` passes at the clap boundary — and a failure carries
/// [`purrdf_iri::IriError::diagnostic_code`]. It used to be discovered by the imported
/// document's parser instead, which meant reading a file to learn that an ARGUMENT was
/// malformed, and reporting it as a runtime failure rather than a usage one.
fn resolve_import_pairs<'a>(options: &ShexOptions<'a>) -> Result<Vec<ImportPair<'a>>, CliError> {
    let mut pairs: Vec<ImportPair<'a>> = Vec::with_capacity(options.imports.len());
    for spec in options.imports {
        let Some((iri, path)) = spec.split_once('=') else {
            return Err(CliError::Usage(format!(
                "--import {spec}: an import pair is `IRI=FILE` — the schema IRI the document \
                 imports, then the local document that resolves it — and this one has no `=`"
            )));
        };
        if iri.is_empty() || path.is_empty() {
            return Err(CliError::Usage(format!(
                "--import {spec}: both halves of `IRI=FILE` are required — the IRI names what \
                 the schema imports, and the path names the document that is it"
            )));
        }
        if path == "-" {
            return Err(CliError::Usage(format!(
                "--import {spec}: an imported schema's syntax is inferred from its own path \
                 extension, and `-` has none. Write the document to a `.shex`/`.shexj` path"
            )));
        }
        if let Err(error) = purrdf_iri::BaseIri::parse(iri) {
            return Err(import_iri_refusal(spec, iri, &error));
        }
        if pairs.iter().any(|seen| seen.iri == iri) {
            return Err(CliError::Usage(format!(
                "--import {iri}=…: the IRI is named twice, and one IRI resolves to one \
                 document; the second pair would be read and never used"
            )));
        }
        let syntax =
            resolve_schema_format(None, path, &format!("--import {iri}"), SyntaxOverride::None)?;
        pairs.push(ImportPair {
            spec,
            iri,
            path,
            syntax,
        });
    }
    Ok(pairs)
}

/// The refusal for an `--import` whose own ontology-IRI half is not an absolute IRI.
///
/// It names the FLAG, the pair as written and the offending half, and carries the shared
/// [`purrdf_iri::IriError::diagnostic_code`]. Exit **2**: nothing was read to discover it.
fn import_iri_refusal(spec: &str, iri: &str, error: &purrdf_iri::IriError) -> CliError {
    let code = error.diagnostic_code();
    if code == "iri-non-absolute-base" {
        return CliError::Usage(format!(
            "--import {spec}: {code}: the ontology-IRI half `{iri}` is a relative IRI reference. \
             It is matched against the schema's `IMPORT` IRIs, which are absolute, and it is \
             also the base the imported document parses under — a relative reference can do \
             neither. This is a command-line value, so no `BASE` in any document reaches it: \
             write the half as the absolute IRI the schema's `IMPORT` names"
        ));
    }
    CliError::Usage(format!(
        "--import {spec}: {code}: the ontology-IRI half `{iri}` is not a usable IRI: {error}"
    ))
}

/// Read and parse each DECIDED `--import` pair into `(iri, schema)`.
///
/// Every argument-level decision was already made by [`resolve_import_pairs`], so what remains
/// here is I/O and parsing: nothing in this function refuses a command line, and every failure
/// it can report is a property of a document (exit 1).
fn read_import_table(pairs: &[ImportPair<'_>]) -> Result<Vec<(String, Schema)>, CliError> {
    let mut table: Vec<(String, Schema)> = Vec::with_capacity(pairs.len());
    for pair in pairs {
        // Parsed with the import IRI as its base, which is the per-document base resolution
        // `purrdf_shex::resolve_imports` documents its injection boundary as satisfying.
        let what = format!("--import {}", pair.iri);
        let text = read_text(pair.path, &what)?;
        let schema = match pair.syntax {
            CliShexFormat::Shexc => parse_shexc(&text, Some(pair.iri)),
            CliShexFormat::Shexj => parse_shexj(&text, Some(pair.iri)),
        }
        .map_err(|error| CliError::Runtime(format!("{what} {}: {error}", pair.path)))?;
        table.push((pair.iri.to_owned(), schema));
    }
    Ok(table)
}

/// Read `path` (or stdin) as UTF-8 text.
fn read_text(path: &str, what: &str) -> Result<String, CliError> {
    let bytes = source::read_bytes(path)?;
    String::from_utf8(bytes)
        .map_err(|error| CliError::Runtime(format!("{what} {path}: not UTF-8 text: {error}")))
}

/// Refuse a schema whose verdict would rest on semantics this boundary cannot supply.
///
/// See the module documentation for why `EXTERNAL` and semantic actions are refused rather
/// than reported: each has a documented fallback in the engine (fail every node / inert
/// success) that is honest as a library behavior and a wrong ANSWER once printed as a
/// conformance verdict.
fn refuse_unavailable_semantics(schema: &Schema, path: &str) -> Result<(), CliError> {
    let mut survey = Survey::default();
    survey.schema(schema);

    if survey.external {
        return Err(CliError::Runtime(format!(
            "--schema {path}: the schema declares an EXTERNAL shape, whose semantics are \
             defined outside it. Resolving one is a host callback rather than a document, so it \
             cannot cross this command-line boundary — and validating without it would report \
             every node NONCONFORMANT against a shape nobody supplied. Reach the resolver from \
             the Rust API (`purrdf_shex::ValidationOptions::external_resolver`)"
        )));
    }
    if let Some(name) = survey.sem_acts.first() {
        return Err(CliError::Runtime(format!(
            "--schema {path}: the schema carries the semantic action <{name}>, which dispatches \
             to a caller-registered extension. Extensions are host code rather than documents, \
             so none can be registered from a command line — and the empty registry treats \
             every action as an INERT SUCCESS, which would report conformance a check never \
             granted. Reach the registry from the Rust API \
             (`purrdf_shex::ValidationOptions::sem_acts`)"
        )));
    }
    Ok(())
}

/// What a full walk of a schema found that this boundary cannot supply semantics for.
#[derive(Default)]
struct Survey {
    /// Whether any `EXTERNAL` shape expression occurs anywhere in the schema.
    external: bool,
    /// Every semantic-action extension IRI, in first-encounter order.
    sem_acts: Vec<String>,
}

impl Survey {
    /// Walk the whole schema: `start`, every declaration, and every nested shape and triple
    /// expression, so a construct buried inside a triple constraint's value expression is
    /// found exactly as a top-level one is.
    fn schema(&mut self, schema: &Schema) {
        self.acts(&schema.start_acts);
        if let Some(start) = &schema.start {
            self.expr(start);
        }
        for decl in &schema.shapes {
            self.expr(&decl.expr);
        }
    }

    /// Record the extension IRI of every semantic action in `acts`.
    fn acts(&mut self, acts: &[SemAct]) {
        for act in acts {
            if !self.sem_acts.iter().any(|seen| seen == &act.name) {
                self.sem_acts.push(act.name.clone());
            }
        }
    }

    /// Walk a shape expression.
    fn expr(&mut self, expr: &ShapeExpr) {
        match expr {
            ShapeExpr::External => self.external = true,
            ShapeExpr::And(list) | ShapeExpr::Or(list) => {
                for child in list {
                    self.expr(child);
                }
            }
            ShapeExpr::Not(inner) => self.expr(inner),
            ShapeExpr::Shape(shape) => self.shape(shape),
            ShapeExpr::Node(_) | ShapeExpr::Ref(_) => {}
        }
    }

    /// Walk a shape's body and its trailing semantic actions.
    fn shape(&mut self, shape: &Shape) {
        self.acts(&shape.sem_acts);
        if let Some(expression) = &shape.expression {
            self.triple(expression);
        }
    }

    /// Walk a triple expression.
    fn triple(&mut self, expr: &TripleExpr) {
        match expr {
            TripleExpr::EachOf(group) | TripleExpr::OneOf(group) => self.group(group),
            TripleExpr::TripleConstraint(constraint) => {
                self.acts(&constraint.sem_acts);
                if let Some(value) = &constraint.value_expr {
                    self.expr(value);
                }
            }
            TripleExpr::Ref(_) => {}
        }
    }

    /// Walk an `EachOf`/`OneOf` group.
    fn group(&mut self, group: &TripleExprGroup) {
        self.acts(&group.sem_acts);
        for member in &group.expressions {
            self.triple(member);
        }
    }
}

/// Refuse a command line that reads standard input twice.
///
/// `--schema` and `--data` may each be `-`, and at most one of them may be — the same
/// one-stdin invariant `entails` and `validate` enforce. `--import` paths cannot be `-` at all
/// (their syntax comes from the path extension), so they are refused earlier and separately.
fn refuse_two_stdins(options: &ShexOptions<'_>) -> Result<(), CliError> {
    if options.schema == "-" && options.data == "-" {
        return Err(CliError::Usage(
            "--schema and --data both read standard input, and there is only one: a process has \
             a single stdin stream, so the schema and the data graph would each get part of one \
             document. Give one of them a path"
                .to_owned(),
        ));
    }
    Ok(())
}

/// Refuse the two global document flags, which name outputs this command does not produce.
///
/// `--loss-ledger` records what a CONVERSION dropped, and this command converts nothing: the
/// data graph is parsed straight into the IR the validator reads, and the answer is a result
/// shape map. `--jsonld-options` configures a JSON-LD/YAML-LD serializer, and none runs — the
/// result shape map is the ShapeMap specification's own JSON, not JSON-LD. Both flags are
/// GLOBAL, so an unrefused one would be accepted and silently do nothing.
fn refuse_document_flags(
    ledger_target: &LedgerTarget,
    jsonld_options: Option<&JsonLdSerializeOptions>,
) -> Result<(), CliError> {
    if ledger_target.is_requested() {
        return Err(CliError::Usage(
            "--loss-ledger records what a conversion dropped, and `shex` converts nothing for \
             you: the data graph is parsed straight into the IR the validator reads, and the \
             answer is a result shape map. There is no ledger to surface"
                .to_owned(),
        ));
    }
    if jsonld_options.is_some() {
        return Err(CliError::Usage(
            "--jsonld-options configures a JSON-LD/YAML-LD serializer, and `shex` runs none: \
             its answer is the ShapeMap specification's result shape map, not JSON-LD"
                .to_owned(),
        ));
    }
    Ok(())
}
