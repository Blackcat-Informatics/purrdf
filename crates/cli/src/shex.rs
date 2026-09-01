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
    ValidationOptions, check_structure, parse_shexc, parse_shexj, resolve_imports,
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

    let data_format = format::resolve(options.from, options.data)?;
    // No `--base` refusal here, unlike every other subcommand: see the module documentation
    // for why this run's base can never be inert.
    let data = source::load_dataset(options.data, data_format, options.base)?;

    let schema_base = schema_base(options.schema)?;
    let schema = read_schema(options.schema, options.schema_from, schema_base.as_deref())?;
    let schema = fold_imports(schema, options)?;
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
        // than about the map's own syntax, so the refusal names the other one.
        ShexError::UnknownShape(_) => CliError::Runtime(format!(
            "MAP: {error} (--schema {}). A shape map is validated AGAINST a schema, so a label \
             neither the schema nor its import closure declares names nothing to decide. \
             Reporting it as `nonconformant` would spend the result shape map's one word for a \
             data finding on a mistake the data had no part in",
            options.schema
        )),
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
fn schema_base(path: &str) -> Result<Option<String>, CliError> {
    if path == "-" {
        return Ok(None);
    }
    source::retrieval_base_iri(path).map(Some)
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
    let syntax = resolve_schema_format(explicit, path, "--schema")?;
    let text = read_text(path, "--schema")?;
    match syntax {
        CliShexFormat::Shexc => parse_shexc(&text, base),
        CliShexFormat::Shexj => parse_shexj(&text, base),
    }
    .map_err(|error| CliError::Runtime(format!("--schema {path}: {error}")))
}

/// Resolve a ShEx document's syntax from an explicit choice or its path extension.
fn resolve_schema_format(
    explicit: Option<CliShexFormat>,
    path: &str,
    what: &str,
) -> Result<CliShexFormat, CliError> {
    if let Some(choice) = explicit {
        return Ok(choice);
    }
    if path == "-" {
        return Err(CliError::Usage(format!(
            "{what} - reads standard input, which has no extension to infer a ShEx syntax from: \
             pass an explicit --schema-from (shexc or shexj)"
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
             extensions are `.shex`/`.shexc` (ShExC) and `.shexj`/`.json` (ShExJ); pass an \
             explicit --schema-from"
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
fn fold_imports(schema: Schema, options: &ShexOptions<'_>) -> Result<Schema, CliError> {
    let table = read_import_table(options)?;
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

    let requested = requested.into_inner();
    if let Some((iri, _)) = table.iter().find(|(iri, _)| !requested.contains(iri)) {
        return Err(CliError::Usage(format!(
            "--import {iri}=…: the schema's import closure never reaches <{iri}>, so this \
             document would be read and never used. Remove the pair, or import the IRI from \
             the schema"
        )));
    }
    Ok(resolved)
}

/// Parse every `--import IRI=FILE` pair into `(iri, schema)`.
///
/// A malformed pair is a usage error naming the argument, never a skipped import: a schema
/// folded without a document the operator supplied is a different schema. Each imported
/// document's syntax comes from its OWN extension, because `--schema-from` names the root
/// schema's syntax and an import closure may legitimately mix ShExC and ShExJ.
fn read_import_table(options: &ShexOptions<'_>) -> Result<Vec<(String, Schema)>, CliError> {
    let mut table: Vec<(String, Schema)> = Vec::with_capacity(options.imports.len());
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
        if table.iter().any(|(seen, _)| seen == iri) {
            return Err(CliError::Usage(format!(
                "--import {iri}=…: the IRI is named twice, and one IRI resolves to one \
                 document; the second pair would be read and never used"
            )));
        }
        // Parsed with the import IRI as its base, which is the per-document base resolution
        // `purrdf_shex::resolve_imports` documents its injection boundary as satisfying.
        let what = format!("--import {iri}");
        let syntax = resolve_schema_format(None, path, &what)?;
        let text = read_text(path, &what)?;
        let schema = match syntax {
            CliShexFormat::Shexc => parse_shexc(&text, Some(iri)),
            CliShexFormat::Shexj => parse_shexj(&text, Some(iri)),
        }
        .map_err(|error| CliError::Runtime(format!("{what} {path}: {error}")))?;
        table.push((iri.to_owned(), schema));
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
