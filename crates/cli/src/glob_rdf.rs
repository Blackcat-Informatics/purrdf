// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! RDF codecs surround the public matcher; only a completed result reaches stdout.
use crate::cli::{CliNativeRdfFormat, GlobRdfArgs};
use crate::error::CliError;
use purrdf::glob::rdf::{self, PreparedPlan, Vocabulary};
use purrdf_core::{DatasetHandle, DatasetView, GraphMatch, RdfDiagnostic, TermId};
use purrdf_lex::allocation::{Memory, Resident};
use std::io::Write as _;

/// Native refusals retain their original payload and RDF snapshot owners.
#[derive(Debug)]
pub enum Failure {
    /// A malformed RDF document, including the parser's directive context.
    Parse(Box<purrdf_rdf::ParseFailure>),
    /// A serializer refused the completed native dataset.
    Serialize(RdfDiagnostic),
    /// A file or standard stream failed.
    Io(std::io::Error),
    /// Compilation or execution refused with its original source context.
    Plan(rdf::Error),
    /// Export refused while retaining its original admission owner.
    Export(rdf::ExportError<Resident>),
    /// An explicit vocabulary argument was refused.
    Vocabulary {
        /// Original closed reason from the native vocabulary boundary.
        reason: &'static str,
        /// Exact original refused argument field.
        value: String,
    },
    /// A requested root or input object did not have exactly one selection.
    Selection {
        /// Original snapshot in which the selection was attempted.
        dataset: DatasetHandle,
        /// The selection that failed.
        role: &'static str,
        /// Original selected subject or root spelling.
        subject: String,
        /// Original selected predicate, when selecting an input object.
        predicate: Option<String>,
        /// Number of objects found; root absence has zero.
        count: usize,
    },
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(error) => error.diagnostic.fmt(f),
            Self::Serialize(error) => error.fmt(f),
            Self::Io(error) => error.fmt(f),
            Self::Plan(error) => error.fmt(f),
            Self::Export(error) => error.fmt(f),
            Self::Vocabulary { reason, value } => {
                write!(f, "glob RDF vocabulary {reason}: {value}")
            }
            Self::Selection {
                role,
                subject,
                count,
                ..
            } => {
                write!(
                    f,
                    "glob RDF {role} {subject}: expected one selection, found {count}"
                )
            }
        }
    }
}
impl std::error::Error for Failure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(error) => Some(&error.diagnostic),
            Self::Serialize(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::Plan(error) => Some(error),
            Self::Export(error) => Some(error),
            Self::Vocabulary { .. } | Self::Selection { .. } => None,
        }
    }
}

fn refusal(value: Failure) -> CliError {
    Memory::new(&mut Resident)
        .boxed(value)
        .map_or_else(CliError::GlobRdfStorage, CliError::GlobRdf)
}

fn format(value: CliNativeRdfFormat) -> Result<purrdf_rdf::NativeRdfFormat, CliError> {
    match value {
        CliNativeRdfFormat::Turtle | CliNativeRdfFormat::Ntriples => Ok(value.to_native()),
        _ => Err(CliError::Usage(
            "glob-rdf requires turtle or ntriples".to_owned(),
        )),
    }
}

fn load(path: &str, format: purrdf_rdf::NativeRdfFormat) -> Result<DatasetHandle, CliError> {
    let bytes = crate::source::read_raw_bytes(path).map_err(|error| refusal(Failure::Io(error)))?;
    let parsed = purrdf_rdf::parse_dataset_reporting_failure(
        &bytes,
        format.media_type(),
        None,
        &purrdf_rdf::ParseOptions::default(),
    )
    .map_err(|error| refusal(Failure::Parse(error)))?;
    // The existing codec owns ingress allocation. Generated output uses native admission.
    Ok(DatasetHandle::from(parsed.dataset))
}

fn selection(
    dataset: DatasetHandle,
    role: &'static str,
    subject: &str,
    predicate: Option<&str>,
    count: usize,
) -> CliError {
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let owned = (|| {
        Ok(Failure::Selection {
            dataset,
            role,
            subject: memory.string(subject)?,
            predicate: predicate.map(|value| memory.string(value)).transpose()?,
            count,
        })
    })();
    owned.map_or_else(CliError::GlobRdfStorage, refusal)
}

pub(crate) fn run(args: &GlobRdfArgs) -> Result<(), CliError> {
    let from = format(args.from)?;
    let to = format(args.to)?;
    crate::argv_documents::refuse_shared_stdin(&[
        ("plan".to_owned(), args.plan.as_str()),
        ("input".to_owned(), args.input.as_str()),
    ])?;
    for value in [&args.root, &args.input_subject, &args.input_predicate] {
        if purrdf_iri::absolute_verdict(value) != Some(true) {
            return Err(CliError::Usage(format!(
                "glob-rdf requires an absolute IRI: {value}"
            )));
        }
    }
    let mut entries = Vec::new();
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    memory
        .reserve(&mut entries, args.vocabulary.len())
        .map_err(CliError::GlobRdfStorage)?;
    for entry in &args.vocabulary {
        entries.push(
            entry
                .split_once('=')
                .ok_or_else(|| CliError::Usage("--vocabulary requires ROLE=IRI".to_owned()))?,
        );
    }
    let vocabulary = Vocabulary::new(&entries).map_err(|error| {
        Memory::new(&mut Resident).string(error.value).map_or_else(
            CliError::GlobRdfStorage,
            |value| {
                refusal(Failure::Vocabulary {
                    reason: error.reason,
                    value,
                })
            },
        )
    })?;
    memory
        .release_vec(entries)
        .map_err(CliError::GlobRdfStorage)?;
    let source = load(&args.plan, from)?;
    let Some(root) = source.term_id_by_iri(&args.root) else {
        return Err(selection(source, "root", &args.root, None, 0));
    };
    let plan = PreparedPlan::compile(source, root, &vocabulary, Resident)
        .map_err(|error| refusal(Failure::Plan(error)))?;
    let input = load(&args.input, from)?;
    let mut object: Option<TermId> = None;
    let mut count = 0;
    if let (Some(subject), Some(predicate)) = (
        input.term_id_by_iri(&args.input_subject),
        input.term_id_by_iri(&args.input_predicate),
    ) {
        for quad in input.quads_for_pattern(Some(subject), Some(predicate), None, GraphMatch::Any) {
            if quad.g.is_none() {
                object = Some(quad.o);
                count += 1;
            }
        }
    }
    if count != 1 {
        return Err(selection(
            input,
            "input object",
            &args.input_subject,
            Some(&args.input_predicate),
            count,
        ));
    }
    let execution = plan
        .execute(
            input,
            object.expect("one selected object"),
            args.max_work,
            Resident,
        )
        .map_err(|error| refusal(Failure::Plan(error)))?;
    let output = execution
        .export(&args.evaluation, Resident)
        .map_err(|error| refusal(Failure::Export(error)))?;
    let bytes = purrdf_rdf::serialize_dataset_to_format(&*output, to, None)
        .map_err(|error| refusal(Failure::Serialize(error)))?
        .bytes;
    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(&bytes)
        .and_then(|()| stdout.flush())
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::BrokenPipe {
                CliError::DownstreamClosed
            } else {
                refusal(Failure::Io(error))
            }
        })
}
