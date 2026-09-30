// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The three shapes-graph tools beside `validate`: `rules`, `node-expr` and `shapes lint`.
//!
//! Each is a thin lane over one library entry point every other PurRDF host reaches too —
//! `purrdf_shapes::infer` / `purrdf_shapes::srl::infer`,
//! `purrdf_shapes::free_expression::evaluate` and `purrdf_shapes::lint::lint` — so the
//! command line cannot disagree with Python, WebAssembly or the C ABI about what a rule
//! infers, what an expression evaluates to, or whether a shapes graph is well-formed. The
//! shapes document is read through [`crate::shapes_source`], the seam `validate` reads it
//! through, so `--import` folds the same `owl:imports` closure here as there.
//!
//! # `rules` writes the inference graph, not the closure
//!
//! `purrdf reason` materializes a CLOSURE — the input plus everything entailed. `rules`
//! writes the INFERENCE GRAPH, the triples the rule set inferred and nothing else: SHACL
//! 1.2 Inference Rules calls them "the inferred triples" and SPARQL 1.2 RL's `infer` is
//! defined to output exactly that graph. A caller who wants the union merges the two
//! documents; a caller who wants to know what the rules added would otherwise have to
//! subtract the input back out.
//!
//! # `shapes lint` exits like the certify verb it is
//!
//! A non-conforming `validate` exits **0**: the data graph is the thing judged, and "it does
//! not conform" is the answer. `shapes lint` judges the SHAPES graph, as a precondition of
//! every validation run against it, and a finding is a refusal of that graph — the position
//! `shacl verify` holds for a product that fails its certification. So a clean report
//! exits **0** and a report with a finding exits **1**, with the report written either way.

use std::sync::Arc;

use purrdf::shapes::data::ShaclData;
use purrdf::shapes::free_expression::{self, FreeExpression};
use purrdf::shapes::srl::{self, InferOptions};
use purrdf::shapes::{Inference, RuleOptions, engine, lint};
use purrdf_rdf::{JsonLdSerializeOptions, SourceFormat};
use purrdf_validate::ExprSelector;

use crate::cli::{CliRdfFormat, LedgerTarget, ReportTarget, refuse_document_flags};
use crate::error::CliError;
use crate::shapes_source::{
    read_shapes_document, resolve_shapes_graph, shapes_error, shapes_imports,
};
use crate::{format, ledger, report, sink, source};

/// The resolved `rules` flags.
pub(crate) struct RulesOptions<'a> {
    /// `--shapes`: the shapes graph whose rules run, or `-`.
    pub(crate) shapes: Option<&'a str>,
    /// `--shapes-from`.
    pub(crate) shapes_from: Option<CliRdfFormat>,
    /// `--shapes-base`.
    pub(crate) shapes_base: Option<&'a str>,
    /// `--shapes-graph`: the IRI the shapes graph is exposed under to SPARQL rules.
    pub(crate) shapes_graph: Option<&'a str>,
    /// `--srl`: the SPARQL 1.2 RL rule set, or `-`.
    pub(crate) srl: Option<&'a str>,
    /// `--srl-base`.
    pub(crate) srl_base: Option<&'a str>,
    /// `--import IRI=FILE`, repeatable: the rule source's imports.
    pub(crate) imports: &'a [String],
    /// `--check[=LEVEL]`: check the `--srl` rule set to this level and evaluate nothing.
    pub(crate) check: Option<srl::CheckLevel>,
    /// `--explain`: where the proof goes, if anywhere.
    pub(crate) explain: ReportTarget,
    /// `--max-term-generating-rounds`.
    pub(crate) max_term_generating_rounds: Option<u64>,
    /// `--max-generated-terms`.
    pub(crate) max_generated_terms: Option<u64>,
    /// `--max-stored-facts`.
    pub(crate) max_stored_facts: Option<u64>,
    /// `--max-join-steps`.
    pub(crate) max_join_steps: Option<u64>,
    /// `--from`: the data-graph format override.
    pub(crate) from: Option<CliRdfFormat>,
    /// `--to`: the output format override.
    pub(crate) to: Option<CliRdfFormat>,
    /// `--base`: the data graph's parse base and the output's base.
    pub(crate) base: Option<&'a str>,
    /// The data-graph path `IN`, or `-`.
    pub(crate) input: &'a str,
    /// The output path `OUT`, or `-`.
    pub(crate) output: &'a str,
    /// Explicit JSON-LD/YAML-LD serialization configuration for a JSON-LD/YAML-LD `--to`.
    pub(crate) jsonld_options: Option<&'a JsonLdSerializeOptions>,
}

/// The rule source `rules` runs, decided from the command line before anything is read.
enum RuleSource<'a> {
    /// A SHACL shapes graph, and the IRI `--shapes-graph` exposes it under.
    Shapes {
        path: &'a str,
        format: SourceFormat,
        base: Option<String>,
        shapes_graph: Option<String>,
    },
    /// A SPARQL 1.2 RL rule set.
    Srl { path: &'a str, base: Option<String> },
}

/// Refuse a command line that reads standard input twice: a process has one stdin.
fn refuse_two_stdins(readers: &[(&str, Option<&str>)]) -> Result<(), CliError> {
    let named: Vec<&str> = readers
        .iter()
        .filter(|(_, path)| *path == Some("-"))
        .map(|(flag, _)| *flag)
        .collect();
    if named.len() > 1 {
        return Err(CliError::Usage(format!(
            "{} all read standard input, and there is only one: a process has a single stdin \
             stream, so each document would get part of one. Give all but one of them a path",
            named.join(" and ")
        )));
    }
    Ok(())
}

/// Run the `rules` subcommand.
pub(crate) fn run_rules(
    options: &RulesOptions<'_>,
    ledger_target: &LedgerTarget,
) -> Result<(), CliError> {
    if options.shapes_graph.is_some() && options.shapes.is_none() {
        return Err(CliError::Usage(
            "--shapes-graph names the graph a SHACL shapes graph's rules see the shapes graph \
             under, and a --srl rule set has no shapes graph: drop --shapes-graph, or run a \
             --shapes graph"
                .to_owned(),
        ));
    }
    if let Some(level) = options.check {
        return run_srl_check(options, level, ledger_target);
    }
    refuse_two_stdins(&[
        ("IN", Some(options.input)),
        ("--shapes", options.shapes),
        ("--srl", options.srl),
    ])?;
    let data_format = format::resolve(options.from, options.input)?;
    let target_format = format::resolve_target(options.to, options.output, "the --to target")?;
    format::refuse_unconsumable_base(
        options.base,
        &[
            format::BaseUse::parse(data_format, "the --from data graph"),
            format::BaseUse::serialize(target_format, "the --to target"),
        ],
    )?;
    sink::validate_jsonld_options(target_format, options.jsonld_options)?;
    let rule_source = match (options.shapes, options.srl) {
        (Some(path), None) => {
            let format = format::resolve(options.shapes_from, path)?;
            let base = crate::validate::shapes_document_base(path, format, options.shapes_base)?;
            let shapes_graph = resolve_shapes_graph(options.shapes_graph, base.as_deref())?;
            RuleSource::Shapes {
                path,
                format,
                base,
                shapes_graph,
            }
        }
        (None, Some(path)) => RuleSource::Srl {
            path,
            base: srl_base(path, options.srl_base)?,
        },
        // clap makes exactly one of the two required; reported rather than unwrapped,
        // because an unreachable panic in a CLI is a crash report.
        _ => {
            return Err(CliError::Usage(
                "exactly one of --shapes (a SHACL shapes graph) or --srl (a SPARQL 1.2 RL rule \
                 set) is required"
                    .to_owned(),
            ));
        }
    };
    let srl_pairs = match rule_source {
        RuleSource::Srl { .. } => import_pairs(options.imports)?,
        RuleSource::Shapes { .. } => Vec::new(),
    };

    let data = source::load_dataset(options.input, data_format, options.base)?;
    let mut diagnostics: Vec<lint::MandatoryDiagnostic> = Vec::new();
    let inference: Inference = match &rule_source {
        RuleSource::Shapes {
            path,
            format,
            base,
            shapes_graph,
        } => {
            let root = read_shapes_document(path, *format, base.as_deref(), "--shapes")?;
            let table = shapes_imports(&root, options.imports)?;
            let shapes = purrdf::shapes::shapes::from_dataset_with_base(
                &root.dataset,
                None,
                &root.prefixes,
                None,
                shapes_graph.clone(),
                &table,
            )
            .map_err(|error| {
                shapes_error(error, &format!("--shapes {path}"), &root, "--shapes-base")
            })?;
            diagnostics = shapes.mandatory_diagnostics().to_vec();
            let projected = engine::project_dataset(data.as_ref()).map_err(CliError::Runtime)?;
            let holder = ShaclData::new(Arc::clone(&projected), projected, None);
            let mut rule_options = RuleOptions::default().with_limit_knobs(cli_limit_knobs());
            if let Some(rounds) = options.max_term_generating_rounds {
                rule_options = rule_options.with_max_term_generating_rounds(rounds);
            }
            if let Some(terms) = options.max_generated_terms {
                rule_options = rule_options.with_max_generated_terms(terms);
            }
            if let Some(facts) = options.max_stored_facts {
                rule_options = rule_options.with_max_stored_facts(facts);
            }
            if let Some(steps) = options.max_join_steps {
                rule_options = rule_options.with_max_join_steps(steps);
            }
            purrdf::shapes::infer(&holder, &shapes, &rule_options)
                .map_err(|error| CliError::Runtime(format!("--shapes {path}: {error}")))?
        }
        RuleSource::Srl { path, base } => {
            let document = check_srl(
                path,
                base.as_deref(),
                &srl_pairs,
                srl::CheckLevel::Stratified,
            )?
            .into_document();
            let mut infer_options = InferOptions::default().with_limit_knobs(cli_limit_knobs());
            if let Some(rounds) = options.max_term_generating_rounds {
                infer_options = infer_options.with_max_term_generating_rounds(rounds);
            }
            if let Some(terms) = options.max_generated_terms {
                infer_options = infer_options.with_max_generated_terms(terms);
            }
            if let Some(facts) = options.max_stored_facts {
                infer_options = infer_options.with_max_stored_facts(facts);
            }
            if let Some(steps) = options.max_join_steps {
                infer_options = infer_options.with_max_join_steps(steps);
            }
            srl::infer(&document, data.as_ref(), &infer_options)
                .map_err(|error| CliError::Runtime(format!("--srl {path}: {error}")))?
        }
    };

    let inferred = inference.inferred_dataset().map_err(CliError::Runtime)?;
    let ledger = sink::write_rdf(
        &*inferred,
        options.output,
        target_format,
        options.base,
        data_format.loss_codec_name(),
        options.jsonld_options,
    )?;
    eprintln!("rules inferred {}", inference.inferred().len());
    // A shapes graph's mandatory diagnostics — every empty `sh:in` / `sh:xone` list — as
    // every run reports them (`validate` writes the same lines); a SPARQL 1.2 RL rule set
    // has no shapes graph and none.
    for diagnostic in &diagnostics {
        eprintln!("shacl diagnostic {diagnostic}");
    }
    if options.explain.is_requested() {
        report::surface_rendered(&options.explain, &inference.proof_text())?;
    }
    ledger::surface(ledger_target, &ledger)
}

/// The rule-evaluation limits' knobs, as this command spells them.
fn cli_limit_knobs() -> purrdf::shapes::LimitKnobs {
    purrdf::shapes::LimitKnobs::new(
        "--max-term-generating-rounds",
        "--max-generated-terms",
        "--max-stored-facts",
        "--max-join-steps",
    )
}

/// Read `path` (or stdin) as UTF-8 text.
fn utf8_text(path: &str, flag: &str) -> Result<String, CliError> {
    String::from_utf8(source::read_bytes(path)?)
        .map_err(|error| CliError::Runtime(format!("{flag} {path}: not UTF-8 text: {error}")))
}

/// One `--import IRI=FILE` pair for a SPARQL 1.2 RL rule set: `(spec, iri, path)`.
type ImportPair<'a> = (&'a str, &'a str, &'a str);

/// Decide every `--import IRI=FILE` argument with no I/O: a malformed pair is a usage error
/// naming the argument, before any file is opened. The IRI half must be absolute — it is
/// matched against the rule set's `IMPORTS` IRIs and is the base the imported document
/// parses under.
fn import_pairs(specs: &[String]) -> Result<Vec<ImportPair<'_>>, CliError> {
    specs
        .iter()
        .map(|spec| {
            let (iri, path) = spec.split_once('=').ok_or_else(|| {
                CliError::Usage(format!(
                    "--import {spec}: an import pair is `IRI=FILE` — the IRI the rule set \
                     imports, then the local document that resolves it — and this one has no `=`"
                ))
            })?;
            if !purrdf_iri::is_absolute(iri).unwrap_or(false) {
                return Err(CliError::Usage(format!(
                    "--import {spec}: the IRI half `{iri}` must be an absolute IRI, the one the \
                     rule set's IMPORTS names"
                )));
            }
            Ok((spec.as_str(), iri, path))
        })
        .collect()
}

/// The base a `--srl` rule set parses under: `--srl-base`, else the file's `file://`
/// retrieval IRI; stdin has none.
fn srl_base(path: &str, srl_base: Option<&str>) -> Result<Option<String>, CliError> {
    Ok(match srl_base {
        Some(base) => Some(base.to_owned()),
        None if path == "-" => None,
        None => Some(source::retrieval_base_iri(path)?),
    })
}

/// `rules --srl FILE --check[=LEVEL]`: check the rule set to `level` through the one
/// check-only entry point every PurRDF host exposes (`purrdf_shapes::srl::check`), with
/// the `--import` table a run would use, read no data graph, evaluate nothing, and write
/// the one-line summary to stdout.
fn run_srl_check(
    options: &RulesOptions<'_>,
    level: srl::CheckLevel,
    ledger_target: &LedgerTarget,
) -> Result<(), CliError> {
    refuse_text_report_flags("rules --check", ledger_target, options.jsonld_options)?;
    // clap makes `--check` require `--srl`; reported rather than unwrapped, because an
    // unreachable panic in a CLI is a crash report.
    let Some(path) = options.srl else {
        return Err(CliError::Usage(
            "--check checks a SPARQL 1.2 RL rule set, and names none without --srl; a SHACL \
             shapes graph is checked with `purrdf shapes lint`"
                .to_owned(),
        ));
    };
    let pairs = import_pairs(options.imports)?;
    let base = srl_base(path, options.srl_base)?;
    let checked = check_srl(path, base.as_deref(), &pairs, level)?;
    println!("--srl {path}: {}", checked.summary());
    Ok(())
}

/// Read the `--srl` rule set and its `--import` pairs, and check it to `level` through
/// `purrdf_shapes::srl::check` — the static checks a run applies before it evaluates
/// ([`srl::CheckLevel::Stratified`]) and the check-only entry point alike, so `--check`
/// refuses exactly what a run refuses. Every pair's file is read and the table they make
/// is resolved exactly as the Python, WebAssembly and C hosts resolve theirs. An import no
/// pair names is refused (exit 1) with the pair that resolves it, and a pair the closure
/// never reaches is refused as unused (exit 2): it would be read and never used.
fn check_srl(
    path: &str,
    base: Option<&str>,
    pairs: &[ImportPair<'_>],
    level: srl::CheckLevel,
) -> Result<srl::CheckedRuleSet, CliError> {
    let text = utf8_text(path, "--srl")?;
    let texts = pairs
        .iter()
        .map(|(_, iri, file)| utf8_text(file, &format!("--import {iri}")))
        .collect::<Result<Vec<String>, CliError>>()?;
    let table: Vec<(&str, &str)> = pairs
        .iter()
        .zip(&texts)
        .map(|((_, iri, _), text)| (*iri, text.as_str()))
        .collect();
    srl::check(&text, base, &table, level).map_err(|error| match error {
        srl::SrlError::UnreachedImports { iris } => {
            let specs: Vec<&str> = pairs
                .iter()
                .filter(|(_, iri, _)| iris.iter().any(|unreached| unreached == iri))
                .map(|(spec, ..)| *spec)
                .collect();
            CliError::Usage(format!(
                "--import {}: the rule set's import closure never reaches {}, so {} would be \
                 read and never used. Remove {}",
                specs.join(" --import "),
                iris.iter()
                    .map(|iri| format!("<{iri}>"))
                    .collect::<Vec<_>>()
                    .join(", "),
                if iris.len() == 1 {
                    "this document"
                } else {
                    "these documents"
                },
                if iris.len() == 1 {
                    "the pair"
                } else {
                    "the pairs"
                },
            ))
        }
        srl::SrlError::Import { iri, message } if message == srl::UNRESOLVED_IMPORT_MESSAGE => {
            CliError::Runtime(format!(
                "--srl {path}: SPARQL 1.2 RL import <{iri}> failed: no --import pair resolves \
                 it, and PurRDF fetches nothing the operator did not name; pass \
                 `--import {iri}=FILE`"
            ))
        }
        other => CliError::Runtime(format!("--srl {path}: {other}")),
    })
}

/// The resolved `node-expr` flags.
pub(crate) struct NodeExprOptions<'a> {
    /// `--shapes`: the shapes graph carrying the expression, or `-`.
    pub(crate) shapes: &'a str,
    /// `--shapes-from`.
    pub(crate) shapes_from: Option<CliRdfFormat>,
    /// `--shapes-base`.
    pub(crate) shapes_base: Option<&'a str>,
    /// `--import IRI=FILE`, repeatable.
    pub(crate) imports: &'a [String],
    /// The expression selector: `--expr`, `--expr-at`/`--expr-via`, `--expr-turtle` or
    /// `--expr-turtle-file`.
    pub(crate) expr: ExprFlags<'a>,
    /// `--focus`.
    pub(crate) focus: &'a str,
    /// `--scope NAME=TERM`, repeatable.
    pub(crate) scope: &'a [String],
    /// `--from`.
    pub(crate) from: Option<CliRdfFormat>,
    /// `--base`.
    pub(crate) base: Option<&'a str>,
    /// The data-graph path `IN`, or `-`.
    pub(crate) input: &'a str,
    /// The output path `OUT`, or `-`.
    pub(crate) output: &'a str,
}

/// The `node-expr` expression-selector flags, of which clap admits exactly one form.
pub(crate) struct ExprFlags<'a> {
    /// `--expr`.
    pub(crate) expr: Option<&'a str>,
    /// `--expr-at`.
    pub(crate) expr_at: Option<&'a str>,
    /// `--expr-via`, repeatable, in order.
    pub(crate) expr_via: &'a [String],
    /// `--expr-turtle`.
    pub(crate) expr_turtle: Option<&'a str>,
    /// `--expr-turtle-file`.
    pub(crate) expr_turtle_file: Option<&'a str>,
}

impl ExprFlags<'_> {
    /// The flags as the operator wrote them, for a diagnostic's context.
    fn spelled(&self) -> String {
        if let Some(expr) = self.expr {
            return format!("--expr {expr}");
        }
        if let Some(node) = self.expr_at {
            let mut out = format!("--expr-at {node}");
            for predicate in self.expr_via {
                out.push_str(" --expr-via ");
                out.push_str(predicate);
            }
            return out;
        }
        if let Some(path) = self.expr_turtle_file {
            return format!("--expr-turtle-file {path}");
        }
        "--expr-turtle".to_owned()
    }
}

/// Run the `node-expr` subcommand.
pub(crate) fn run_node_expr(
    options: &NodeExprOptions<'_>,
    ledger_target: &LedgerTarget,
    jsonld_options: Option<&JsonLdSerializeOptions>,
) -> Result<(), CliError> {
    refuse_text_report_flags("node-expr", ledger_target, jsonld_options)?;
    refuse_two_stdins(&[
        ("IN", Some(options.input)),
        ("--shapes", Some(options.shapes)),
    ])?;
    let data_format = format::resolve(options.from, options.input)?;
    format::refuse_unconsumable_base(
        options.base,
        &[format::BaseUse::parse(data_format, "the --from data graph")],
    )?;
    let shapes_format = format::resolve(options.shapes_from, options.shapes)?;
    let shapes_base =
        crate::validate::shapes_document_base(options.shapes, shapes_format, options.shapes_base)?;
    // Every argv term is decided before a document is read: a malformed one is the command
    // line's fault.
    let context = options.expr.spelled();
    let via: Vec<&str> = options.expr.expr_via.iter().map(String::as_str).collect();
    let argv_selector = match (
        options.expr.expr,
        options.expr.expr_at,
        options.expr.expr_turtle,
    ) {
        (Some(expr), _, _) => Some(ExprSelector::Node(expr)),
        (None, Some(node), _) => Some(ExprSelector::At { node, via: &via }),
        (None, None, Some(text)) => Some(ExprSelector::Turtle(text)),
        (None, None, None) => None,
    };
    let argv_selector = argv_selector
        .map(|selector| selector.parse())
        .transpose()
        .map_err(|error| CliError::Usage(format!("{context}: {error}")))?;
    if options.expr.expr_turtle_file == Some("-") {
        return Err(CliError::Usage(
            "--expr-turtle-file -: stdin is IN's or --shapes'; write the expression to a \
             file, or pass it with --expr-turtle"
                .to_owned(),
        ));
    }
    let focus = free_expression::parse_term(options.focus)
        .map_err(|error| CliError::Usage(format!("--focus {error}")))?;
    let scope = options
        .scope
        .iter()
        .map(|binding| {
            let (name, term) = purrdf_validate::parse_scope_binding(binding)
                .map_err(|error| CliError::Usage(format!("--scope {error}")))?;
            let term = free_expression::parse_term(term)
                .map_err(|error| CliError::Usage(format!("--scope {binding}: {error}")))?;
            Ok((name.to_owned(), term))
        })
        .collect::<Result<Vec<_>, CliError>>()?;

    let root_document = read_shapes_document(
        options.shapes,
        shapes_format,
        shapes_base.as_deref(),
        "--shapes",
    )?;
    let table = shapes_imports(&root_document, options.imports)?;
    let data = source::load_dataset(options.input, data_format, options.base)?;
    let selector = match (argv_selector, options.expr.expr_turtle_file) {
        (Some(selector), _) => selector,
        (None, Some(path)) => {
            let text = String::from_utf8(source::read_bytes(path)?).map_err(|error| {
                CliError::Runtime(format!(
                    "--expr-turtle-file {path}: not UTF-8 text: {error}"
                ))
            })?;
            ExprSelector::Turtle(&text)
                .parse()
                .map_err(|error| CliError::Usage(format!("{context}: {error}")))?
        }
        (None, None) => {
            return Err(CliError::Usage(
                "name the expression with --expr, --expr-at, --expr-turtle or \
                 --expr-turtle-file"
                    .to_owned(),
            ));
        }
    };
    let selected = selector
        .select(
            &root_document.dataset,
            &root_document.prefixes,
            root_document.loaded.last().map(String::as_str),
        )
        .map_err(|error| CliError::Runtime(format!("{context}: {error}")))?;
    let evaluated = free_expression::evaluate(&FreeExpression {
        shapes: &selected.shapes,
        prefixes: &root_document.prefixes,
        root: &selected.root,
        data: data.as_ref(),
        focus: &focus,
        scope: &scope,
        imports: &table,
    })
    .map_err(|error| shapes_error(error, &context, &root_document, "--shapes-base"))?;
    let mut text = String::new();
    for term in &evaluated.outputs {
        text.push_str(&term.to_string());
        text.push('\n');
    }
    sink::write_out(options.output, text.as_bytes())?;
    eprintln!("node-expr outputs {}", evaluated.outputs.len());
    // The shapes graph's mandatory diagnostics, which every run reports: one `shacl
    // diagnostic RULE SHAPE` line each, exactly as `validate` and `rules` print them. They
    // change no output — the shapes graph is well-formed — so they go to stderr beside the
    // count, never into the output document.
    for diagnostic in &evaluated.diagnostics {
        eprintln!("shacl diagnostic {diagnostic}");
    }
    Ok(())
}

/// The resolved `shapes lint` flags.
pub(crate) struct LintOptions<'a> {
    /// `--from`: the shapes-graph format override.
    pub(crate) from: Option<CliRdfFormat>,
    /// `--base`: the shapes document's base.
    pub(crate) base: Option<&'a str>,
    /// `--import IRI=FILE`, repeatable.
    pub(crate) imports: &'a [String],
    /// `--box-role-vocab`.
    pub(crate) box_role_vocab: Option<&'a str>,
    /// `--shapes-graph`.
    pub(crate) shapes_graph: Option<&'a str>,
    /// The shapes-graph path, or `-`.
    pub(crate) input: &'a str,
    /// The report path `OUT`, or `-`.
    pub(crate) output: &'a str,
}

/// Run the `shapes lint` subcommand.
pub(crate) fn run_lint(
    options: &LintOptions<'_>,
    ledger_target: &LedgerTarget,
    jsonld_options: Option<&JsonLdSerializeOptions>,
) -> Result<(), CliError> {
    refuse_text_report_flags("shapes lint", ledger_target, jsonld_options)?;
    let format = format::resolve(options.from, options.input)?;
    let base = match format {
        SourceFormat::Native(native) => {
            source::effective_base(options.input, native, options.base)?
        }
        SourceFormat::Pack | SourceFormat::Gts => match options.base {
            None => None,
            Some(base) => {
                return Err(CliError::Usage(format!(
                    "--base {base}: the shapes input {} is a container, which stores resolved \
                     IRIs and has no document base to set. Drop --base, or give an RDF text \
                     document",
                    options.input
                )));
            }
        },
    };
    let shapes_graph = resolve_shapes_graph(options.shapes_graph, base.as_deref())?;
    let root = read_shapes_document(options.input, format, base.as_deref(), "shapes lint")?;
    let table = shapes_imports(&root, options.imports)?;
    let report = lint::lint(
        &root.dataset,
        &root.prefixes,
        options
            .box_role_vocab
            .map(purrdf::shapes::model::BoxRoleVocab::for_namespace),
        shapes_graph,
        &table,
    )
    .map_err(|error| shapes_error(error, "shapes lint", &root, "--base"))?;
    sink::write_out(options.output, report.render().as_bytes())?;
    eprintln!("shapes lint clean {}", report.is_clean());
    eprintln!("shapes lint findings {}", report.findings());
    if report.is_clean() {
        return Ok(());
    }
    Err(CliError::Runtime(format!(
        "{}: the shapes graph carries {} finding(s) — see the report",
        options.input,
        report.findings()
    )))
}

/// Refuse the two global document flags, which name an RDF serialization a text-report
/// command does not run: an unrefused one would be accepted and silently do nothing.
fn refuse_text_report_flags(
    command: &str,
    ledger_target: &LedgerTarget,
    jsonld_options: Option<&JsonLdSerializeOptions>,
) -> Result<(), CliError> {
    refuse_document_flags(
        ledger_target,
        jsonld_options,
        &format!(
            "--loss-ledger records what an RDF serialization dropped, and `{command}` runs none: \
             its answer is line-oriented text. There is no ledger to surface"
        ),
        &format!(
            "--jsonld-options configures a JSON-LD/YAML-LD serializer, and `{command}` runs \
             none: its answer is line-oriented text, not JSON-LD"
        ),
    )
}
