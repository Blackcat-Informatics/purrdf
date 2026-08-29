// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `entails` subcommand: `Premise + Question → decide → Sink`.
//!
//! The conclusion-directed entailment service, on the command line. `reason` computes a
//! CLOSURE — everything the premise entails, as a document — which is the right shape for a
//! caller that will go on asking many questions of one premise and the wrong shape for a
//! caller with ONE question. Turning a closure into a verdict is not the membership test it
//! looks like: a conclusion's blank nodes are existentials that have to be MAPPED, an
//! inconsistent premise entails everything, and a failure to find a mapping means nothing
//! at all unless the rule set is complete for the premise it ran on. So this subcommand
//! does not post-process a `reason` output; it asks the question directly.
//!
//! # THE SAME BOUNDARY EVERY OTHER HOST CALLS
//!
//! There is no sixth copy of the fold here. Every question routes through
//! [`purrdf_validate::regime`] — the one string boundary the Python, WebAssembly and C-ABI
//! surfaces already reach — so a verdict this binary prints is byte-for-byte the verdict
//! those three print for the same documents:
//!
//! * `--conclusion FILE` → [`graph_entails_to_string`];
//! * `--conclusion FILE --verify` → [`verify_entailment_to_string`], which re-decides the
//!   warrant WITHOUT running a reasoner;
//! * `--pattern FILE` → [`certain_answers_to_string`], the certain answers of a basic graph
//!   pattern (the substitutions the premise ENTAILS the pattern under, not the ones present
//!   in one closure).
//!
//! # Five regimes, and the other two are refused BY NAME
//!
//! `--regime` is the same [`CliRegime`] value enum `reason` takes, all seven spellings, and
//! that is deliberate: the accepted set is one vocabulary across the binary. The service is
//! total over five of them. `owl-direct` is directed by a QUERY's class expressions and
//! `rif` entails under the caller's RULE DOCUMENT, and neither of those inputs is carried by
//! "premise, conclusion, regime" — so the boundary refuses both, naming the regime, and this
//! subcommand surfaces that refusal verbatim rather than falling back to a weaker regime and
//! labelling the answer with the one the operator asked for. A caller who wants those two
//! reaches `purrdf reason --regime owl-direct` / `--regime rif --rules FILE`, which carry the
//! input each is defined by.
//!
//! # `--import IRI=FILE` — the documents the premise says it is not all of
//!
//! OWL 2 defines an ontology's imports closure to BE the ontology, so a premise carrying an
//! `owl:imports` this command was not handed is a DIFFERENT premise from the one the operator
//! asked about. PurRDF fetches nothing and mints no vocabulary, so the closure is
//! caller-supplied configuration: each `--import` pair resolves one ontology IRI to one local
//! document. An `owl:imports` no pair resolves is refused BY NAME by the boundary — never a
//! silently truncated premise — and a malformed pair (no `=`) is a usage error here, never a
//! silently skipped one.
//!
//! # Formats: `--from`, and the ONE media type the boundary takes
//!
//! The boundary parses N-Quads (which accepts N-Triples unchanged). The CLI's own format
//! resolution therefore runs in FRONT of it: the premise, the conclusion and every
//! `--import` document are resolved exactly as `convert`/`reason` resolve `--from` (an
//! explicit choice wins, otherwise the path's extension is classified, and `-` has no
//! extension so it REQUIRES the explicit override), parsed with the native codecs, and
//! re-serialized into N-Quads. So a caller hands this command Turtle, RDF/XML, JSON-LD or a
//! verified pack, exactly as they would `reason`.
//!
//! That crossing is LOSSLESS by construction — N-Quads is the syntax that carries named
//! graphs, the RDF 1.2 statement layer and literal base direction, so the transcode matrix
//! records no contract loss into it from any of the nine — and a REALIZED drop is refused
//! rather than recorded, because a lossily transcoded premise is a different premise and the
//! verdict would be about that other one.
//!
//! `--pattern` is the exception, and it is not an RDF document: it is N-Triples with `?name`
//! (or `$name`) in any position, which no RDF parser accepts. It is therefore handed to the
//! boundary as the BYTES the file holds, with no format resolution and no transcode at all,
//! and `--from` says nothing about it. What the boundary then does with those bytes is not
//! "read them verbatim": it rewrites each `?name` to a term it has swept out of the caller's
//! own text, parses the result with the real N-Triples parser, and maps every such term back
//! to a variable — which is how a `?name` reaches a position RDF reserves for an IRI. The
//! rewrite is invisible on both sides, and [`purrdf_validate::regime`] states its full shape.
//! It is also escape-aware: the swept namespace is chosen against the pattern as the PARSER
//! will read it, so an IRI the caller wrote with a `UCHAR` escape is the same IRI as the one
//! they wrote plainly and neither is read back as a variable.
//!
//! A `?name` INSIDE an RDF 1.2 triple term is an ordinary variable — it binds, it is a
//! column, and one NAME is one VARIABLE wherever it was written, so
//! `?x <ex:p> <<( ?x <ex:q> <ex:r> )>>` is the join it reads as.
//!
//! # One stdin
//!
//! The premise, the question and each `--import` document may each be `-`, and at most ONE
//! of them may be: a process has one standard input and two streams reading it is incoherent.
//! Two `-` is a usage error naming both, never a silent mis-read of one document as two.
//!
//! # What comes out
//!
//! The ANSWER goes to the sink (`OUT`, default stdout) — the boundary's own line-oriented
//! grammar, unmodified. The run's certificate goes to `--report`, decoded through the same
//! [`ReportTarget`] tri-state `reason --report` uses, so the two never mix even when `OUT` is
//! `-`.

use purrdf_rdf::JsonLdSerializeOptions;
use purrdf_validate::regime::{
    ReasoningAnswer, certain_answers_to_string, graph_entails_to_string,
    verify_entailment_to_string,
};

use crate::cli::{CliRdfFormat, CliRegime, LedgerTarget, ReportTarget};
use crate::error::CliError;
use crate::format;
use crate::report;
use crate::sink;
use crate::source;

/// The question asked of the premise: a conclusion graph, or a basic graph pattern.
///
/// A closed enum rather than two `Option`s so the "exactly one" invariant is carried by the
/// type after clap has enforced it, and so [`run`] cannot be reached with neither or both.
#[derive(Debug, Clone, Copy)]
enum Question<'a> {
    /// `--conclusion FILE`: a conclusion GRAPH, whose answer is a verdict.
    ///
    /// The flag is `--verify`: re-decide the warrant of a `yes` without running a reasoner.
    Conclusion {
        /// The conclusion document's path, or `-`.
        path: &'a str,
        /// Whether `--verify` was set.
        verify: bool,
    },
    /// `--pattern FILE`: a basic graph pattern, whose answer is a relation.
    Pattern {
        /// The pattern document's path, or `-`.
        path: &'a str,
    },
}

impl<'a> Question<'a> {
    /// The path the question is read from — the second candidate for stdin.
    const fn path(self) -> &'a str {
        match self {
            Self::Conclusion { path, .. } | Self::Pattern { path } => path,
        }
    }

    /// The flag that named it, for a diagnostic.
    const fn flag(self) -> &'static str {
        match self {
            Self::Conclusion { .. } => "--conclusion",
            Self::Pattern { .. } => "--pattern",
        }
    }
}

/// The resolved `entails` flags.
///
/// Grouped for the reason [`crate::convert::ConvertOptions`] is: it keeps [`run`]'s
/// signature small enough to read, and every field is borrowed from the parsed command line.
pub(crate) struct EntailsOptions<'a> {
    /// `--regime`: the regime to decide under.
    pub(crate) regime: CliRegime,
    /// `--premise`: the premise document, or `-`.
    pub(crate) premise: &'a str,
    /// `--conclusion`: the conclusion graph, when the question is a verdict.
    pub(crate) conclusion: Option<&'a str>,
    /// `--pattern`: the basic graph pattern, when the question is a relation.
    pub(crate) pattern: Option<&'a str>,
    /// `--verify`: re-decide the warrant without running a reasoner.
    pub(crate) verify: bool,
    /// `--import IRI=FILE`, in the order the operator wrote them.
    pub(crate) imports: &'a [String],
    /// `--from`: the input-format override for every RDF document this command reads.
    pub(crate) from: Option<CliRdfFormat>,
    /// `--base`: the base IRI relative IRIs in those documents resolve against.
    pub(crate) base: Option<&'a str>,
    /// Explicit JSON-LD/YAML-LD serialization configuration, which this command refuses.
    pub(crate) jsonld_options: Option<&'a JsonLdSerializeOptions>,
}

/// Run the `entails` subcommand.
pub(crate) fn run(
    options: &EntailsOptions<'_>,
    output: &str,
    ledger_target: &LedgerTarget,
    report_target: &ReportTarget,
) -> Result<(), CliError> {
    refuse_document_flags(ledger_target, options.jsonld_options)?;
    let question = question(options)?;
    refuse_two_stdins(options, question)?;

    // Everything is read and transcoded BEFORE the boundary is called, so an unreadable
    // import fails against the file the operator named rather than as a refusal attributed
    // to the premise's `owl:imports`.
    let premise = read_as_nquads(options.premise, "--premise", options)?;
    let imports = read_imports(options)?;
    let table: Vec<(&str, &str)> = imports
        .iter()
        .map(|(iri, document)| (iri.as_str(), document.as_str()))
        .collect();
    let regime = purrdf_validate::regime::regime_name(options.regime.to_native());

    let answer: ReasoningAnswer = match question {
        Question::Conclusion { path, verify } => {
            let conclusion = read_as_nquads(path, "--conclusion", options)?;
            let decide = if verify {
                verify_entailment_to_string
            } else {
                graph_entails_to_string
            };
            decide(regime, &premise, &conclusion, &table).map_err(CliError::Runtime)?
        }
        Question::Pattern { path } => {
            let pattern = read_verbatim(path, "--pattern")?;
            certain_answers_to_string(regime, &premise, &pattern, &table)
                .map_err(CliError::Runtime)?
        }
    };

    // The verdict goes to the sink and the certificate goes to `--report`: this command
    // answers a question, and the evidence of what produced the answer is a second output
    // rather than a discarded one.
    report::surface_rendered(report_target, answer.certificate())?;
    sink::write_out(output, answer.answer().as_bytes())
}

/// The question the flags name, as the closed [`Question`].
///
/// Clap already enforces "exactly one of `--conclusion` / `--pattern`" (they conflict, and
/// `--conclusion` is required unless `--pattern` is present), so the impossible arm is an
/// internal inconsistency rather than a caller error — and it is still a named refusal
/// rather than a panic.
fn question<'a>(options: &EntailsOptions<'a>) -> Result<Question<'a>, CliError> {
    match (options.conclusion, options.pattern) {
        (Some(path), None) => Ok(Question::Conclusion {
            path,
            verify: options.verify,
        }),
        (None, Some(path)) => Ok(Question::Pattern { path }),
        _ => Err(CliError::Usage(
            "`entails` asks exactly one question: either `--conclusion FILE` (a conclusion \
             graph, whose answer is a verdict) or `--pattern FILE` (a basic graph pattern, \
             whose answer is its certain answers)"
                .to_owned(),
        )),
    }
}

/// Refuse the two global document flags, which name outputs this command does not produce.
///
/// `--loss-ledger` records what a CONVERSION dropped. This command converts nothing for the
/// operator: it reads documents, decides a question and writes a verdict. Its own crossing
/// into the boundary's N-Quads is lossless by construction and a realized drop is REFUSED
/// (see [`read_as_nquads`]) rather than recorded, so there is no ledger — and a flag that
/// silently wrote an empty one would be the no-op this repository refuses.
///
/// `--jsonld-options` configures a JSON-LD/YAML-LD serializer. The answer is a line-oriented
/// verdict in the boundary's own grammar, not RDF, so no serializer runs and the option has
/// nothing to configure.
fn refuse_document_flags(
    ledger_target: &LedgerTarget,
    jsonld_options: Option<&JsonLdSerializeOptions>,
) -> Result<(), CliError> {
    if !matches!(ledger_target, LedgerTarget::Silent) {
        return Err(CliError::Usage(
            "--loss-ledger records what a conversion dropped, and `entails` converts nothing \
             for you: it decides a question and writes a verdict. The documents it reads cross \
             into the boundary's N-Quads losslessly or the run is refused, so there is no \
             ledger to surface"
                .to_owned(),
        ));
    }
    if jsonld_options.is_some() {
        return Err(CliError::Usage(
            "--jsonld-options configures a JSON-LD/YAML-LD serializer, and `entails` runs \
             none: its answer is a line-oriented verdict, not RDF"
                .to_owned(),
        ));
    }
    Ok(())
}

/// Refuse a command line that reads standard input twice.
///
/// The premise, the question and every `--import` document may each be `-`. A process has
/// ONE standard input, so two of them naming it is not a command that reads one stream twice
/// — it is a command that reads half a document into each. Refused, naming both, rather than
/// mis-read.
fn refuse_two_stdins(options: &EntailsOptions<'_>, question: Question<'_>) -> Result<(), CliError> {
    let mut named: Vec<String> = Vec::new();
    if options.premise == "-" {
        named.push("--premise".to_owned());
    }
    if question.path() == "-" {
        named.push(question.flag().to_owned());
    }
    for spec in options.imports {
        if let Some((iri, path)) = split_import(spec)
            && path == "-"
        {
            named.push(format!("--import {iri}=-"));
        }
    }
    if named.len() > 1 {
        return Err(CliError::Usage(format!(
            "{} each read standard input, and there is only one: a process has a single stdin \
             stream, so two documents reading it would each get part of one. Give all but one \
             of them a path",
            named.join(" and ")
        )));
    }
    Ok(())
}

/// The `(ontology-iri, path)` halves of one `--import` argument, or `None` when it has no `=`.
///
/// The IRI is everything before the FIRST `=`, which is the conventional reading of a
/// `KEY=VALUE` argument; a path containing `=` therefore works and an ontology IRI containing
/// one does not, and that trade is stated rather than discovered.
fn split_import(spec: &str) -> Option<(&str, &str)> {
    spec.split_once('=')
}

/// Read every `--import IRI=FILE` pair, transcoding each document into the boundary's N-Quads.
///
/// A malformed pair is a usage error naming the argument, never a skipped import: a premise
/// answered without a document the operator supplied is answered over a different premise.
/// A DUPLICATE ontology IRI is not checked here — the boundary refuses it, and re-deciding
/// that in the CLI would be a second opinion about the same input.
fn read_imports(options: &EntailsOptions<'_>) -> Result<Vec<(String, String)>, CliError> {
    let mut resolved = Vec::with_capacity(options.imports.len());
    for spec in options.imports {
        let Some((iri, path)) = split_import(spec) else {
            return Err(CliError::Usage(format!(
                "--import {spec}: an import pair is `IRI=FILE` — the ontology IRI the premise \
                 declares, then the local document that resolves it — and this one has no `=`"
            )));
        };
        if iri.is_empty() || path.is_empty() {
            return Err(CliError::Usage(format!(
                "--import {spec}: both halves of `IRI=FILE` are required — the ontology IRI \
                 names what the premise imports, and the path names the document that is it"
            )));
        }
        let what = format!("--import {iri}");
        resolved.push((iri.to_owned(), read_as_nquads(path, &what, options)?));
    }
    Ok(resolved)
}

/// Read `path` through the CLI's own format resolution and re-serialize it as N-Quads.
///
/// This is where `--from` reaches the boundary: the resolution is `convert`'s and `reason`'s
/// ([`format::resolve`]), the parse is the native codecs' (including a verified pack), and
/// the re-serialization targets the one media type the boundary parses.
///
/// The serializer runs with NO base. `--base` is threaded into the PARSE, which is where a
/// relative IRI can appear; N-Quads has no relative-IRI syntax at all, so handing the
/// serializer a base could only ask it to emit something the boundary's parser would then
/// reject.
///
/// A REALIZED drop is refused. N-Quads carries named graphs, the RDF 1.2 statement layer and
/// literal base direction, so this crossing loses nothing from any of the nine syntaxes — and
/// if it ever did, the document the boundary decided over would not be the document the
/// operator named, which is a wrong answer rather than a recordable loss.
fn read_as_nquads(
    path: &str,
    what: &str,
    options: &EntailsOptions<'_>,
) -> Result<String, CliError> {
    let format = format::resolve(options.from, path)?;
    format::refuse_base_with_container(format, options.base, &format!("the {what} document"))?;
    // A pack crosses the N-Quads boundary as a zero-copy `PackView`, not a rebuilt
    // owned dataset; a text source parses to an `RdfDataset`.
    let outcome = source::serialize_input_to_nquads(path, format, options.base)?;
    if outcome.statement_rows_dropped > 0 || outcome.directional_literals_dropped > 0 {
        return Err(CliError::Runtime(format!(
            "{what} {path}: reading this document into the entailment boundary's N-Quads \
             dropped {} statement-layer row(s) and {} literal base direction(s). The decided \
             document would not be the one you named, so the run is refused rather than \
             answered about something else",
            outcome.statement_rows_dropped, outcome.directional_literals_dropped
        )));
    }
    String::from_utf8(outcome.bytes).map_err(|error| {
        CliError::Runtime(format!(
            "{what} {path}: N-Quads output is not UTF-8: {error}"
        ))
    })
}

/// Read `path` (or stdin) as text, with no format resolution.
///
/// The `--pattern` reader. A basic graph pattern is N-Triples with `?name` / `$name` in term
/// positions, which is not an RDF document and which no RDF parser accepts — so there is
/// nothing to resolve a format for, and the bytes go to the boundary's own pattern parser
/// exactly as written.
fn read_verbatim(path: &str, what: &str) -> Result<String, CliError> {
    let bytes = source::read_bytes(path)?;
    String::from_utf8(bytes)
        .map_err(|error| CliError::Runtime(format!("{what} {path}: not UTF-8 text: {error}")))
}
