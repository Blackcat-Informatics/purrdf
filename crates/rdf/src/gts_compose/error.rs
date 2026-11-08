// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Owned composition refusals and bounded diagnostic term evidence.

use super::FrameSlot;
use crate::{BlankScope, DatasetView, RdfTextDirection, TermRef};
use purrdf_core::dataset_view::TermGuard as _;
use purrdf_hash::error::{ErrorValue, StructuredError};
use purrdf_lex::allocation::{Memory, NativeCause, Resident, StorageError, retain_native_cause};

/// A bounded diagnostic snapshot, not a complete RDF value.
///
/// The source is followed in its existing diagnostic order, at most eight terms
/// deep. Cyclic or deeper sources end in `Cutoff`. A language-tagged literal's
/// datatype is not followed, matching the diagnostic traversal's original policy.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum OffendingTerm {
    /// The remaining diagnostic depth was exhausted; no source lookup occurred.
    Cutoff,
    /// Original IRI text.
    Iri(String),
    /// Original blank label and dataset scope, before rendering qualification.
    Blank {
        /// Original blank label.
        label: String,
        /// Original dataset scope.
        scope: BlankScope,
    },
    /// Original literal components. Datatype evidence is absent when language
    /// suppresses its diagnostic lookup, not a claim that the source lacked one.
    Literal {
        /// Original lexical form.
        lexical: String,
        /// Bounded datatype evidence, absent when language suppresses its lookup.
        datatype: Option<Box<Self>>,
        /// Original language tag, when present.
        language: Option<String>,
        /// Original RDF base direction, when present.
        direction: Option<RdfTextDirection>,
    },
    /// Original constituent evidence, in subject/predicate/object lookup order.
    Triple {
        /// Subject evidence.
        s: Box<Self>,
        /// Predicate evidence.
        p: Box<Self>,
        /// Object evidence.
        o: Box<Self>,
    },
}

impl StructuredError for OffendingTerm {
    purrdf_hash::error_visit! {
        Cutoff => "gts-term-cutoff" {},
        Iri(iri) => "gts-term-iri" { "iri" => Iri(iri) },
        Blank { label, scope } => "gts-term-blank" { "label" => Text(label), "scope" => Unsigned(u128::from(scope.ordinal())) },
        Literal { lexical, datatype, language, direction } => "gts-term-literal" {
            "lexical" => Text(lexical),
            "datatype" => Value(datatype.as_ref().map_or(ErrorValue::Absent, |term| ErrorValue::Record(&**term))),
            "language" => Value(language.as_deref().map_or(ErrorValue::Absent, ErrorValue::Text)),
            "direction" => Value(direction.map_or(ErrorValue::Absent, |direction| ErrorValue::Code(direction.as_str()))),
        },
        Triple { s, p, o } => "gts-term-triple" { "subject" => Record(&**s), "predicate" => Record(&**p), "object" => Record(&**o) },
    }
}

impl std::fmt::Display for OffendingTerm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cutoff => f.write_str("…"),
            Self::Iri(iri) => f.write_str(iri),
            Self::Blank { label, scope } => write!(f, "_:{}", scope.qualify_label(label)),
            Self::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                write!(f, "{lexical:?}")?;
                match (language, direction) {
                    (Some(language), Some(direction)) => {
                        write!(f, "@{language}--{}", direction.as_str())
                    }
                    (Some(language), None) => write!(f, "@{language}"),
                    (None, _) => match datatype {
                        Some(datatype) => write!(f, "^^<{datatype}>"),
                        None => Ok(()),
                    },
                }
            }
            Self::Triple { s, p, o } => write!(f, "<<( {s} {p} {o} )>>"),
        }
    }
}

pub(super) fn term_evidence<D: DatasetView<ReadError: StructuredError>>(
    view: &D,
    id: D::Id,
) -> Result<OffendingTerm, GtsIngestError> {
    let mut resident = Resident;
    evidence_within(view, id, 8, &mut Memory::new(&mut resident))
}

fn boxed_evidence<D: DatasetView<ReadError: StructuredError>>(
    view: &D,
    id: D::Id,
    depth: usize,
    memory: &mut Memory<'_, Resident>,
) -> Result<Box<OffendingTerm>, GtsIngestError> {
    let child = evidence_within(view, id, depth, memory)?;
    Ok(memory.boxed(child)?)
}

fn evidence_within<D: DatasetView<ReadError: StructuredError>>(
    view: &D,
    id: D::Id,
    depth: usize,
    memory: &mut Memory<'_, Resident>,
) -> Result<OffendingTerm, GtsIngestError> {
    let Some(next) = depth.checked_sub(1) else {
        return Ok(OffendingTerm::Cutoff);
    };
    let guard = view.resolve(id).map_err(GtsIngestError::source_read)?;
    Ok(match guard.term() {
        TermRef::Iri(iri) => OffendingTerm::Iri(memory.string(iri)?),
        TermRef::Blank { label, scope } => OffendingTerm::Blank {
            label: memory.string(label)?,
            scope,
        },
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => OffendingTerm::Literal {
            lexical: memory.string(lexical)?,
            datatype: if language.is_none() {
                Some(boxed_evidence(view, datatype, next, memory)?)
            } else {
                None
            },
            language: language.map(|text| memory.string(text)).transpose()?,
            direction,
        },
        TermRef::Triple { s, p, o } => OffendingTerm::Triple {
            s: boxed_evidence(view, s, next, memory)?,
            p: boxed_evidence(view, p, next, memory)?,
            o: boxed_evidence(view, o, next, memory)?,
        },
    })
}

/// Which ingestion boundary sampled a fallible view's operational status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum IngestCheckpoint {
    /// Before consuming the first row.
    BeforeRows,
    /// After consuming ordinary, reifier and annotation rows.
    AfterRows,
}
impl StructuredError for IngestCheckpoint {
    purrdf_hash::error_visit! {
        BeforeRows => "gts-ingest-before-rows" {},
        AfterRows => "gts-ingest-after-rows" {},
    }
}
impl std::fmt::Display for IngestCheckpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::BeforeRows => "before row consumption",
            Self::AfterRows => "after row consumption",
        })
    }
}

/// A terminal ingestion refusal. Failed ingestion rolls back to the last fully
/// accepted state and poisons the builder; that accepted subset cannot be emitted.
/// Native erased causes are shared on Clone and have no invented value equality.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum GtsIngestError {
    /// Retaining native evidence physically failed.
    Storage(StorageError),
    /// Forward or reverse source dictionary access failed.
    SourceRead {
        /// Original source read failure.
        cause: NativeCause,
    },
    /// The source reported a sticky failure at the named boundary.
    ViewNotReady {
        /// Boundary that observed the source failure.
        checkpoint: IngestCheckpoint,
        /// Original owned native failure.
        cause: NativeCause,
    },
    /// An RDF position or nesting restriction was violated.
    UnrepresentableTerm {
        /// RDF position that refused the term.
        position: &'static str,
        /// Bounded evidence of the offending source term.
        term: OffendingTerm,
    },
    /// Distinct blank intern keys map to the same unchanged wire spelling.
    BlankWireCollision {
        /// The unchanged wire spelling shared by distinct keys.
        wire_value: String,
        /// Caller scope of the previously accepted key.
        held_scope: Option<String>,
        /// Label of the previously accepted key.
        held_label: String,
        /// Caller scope of the colliding key.
        incoming_scope: Option<String>,
        /// Label of the colliding key.
        incoming_label: String,
    },
    /// An earlier ingestion failed; no later ingestion or emission is permitted.
    Poisoned {
        /// Original earlier ingestion refusal.
        cause: NativeCause,
    },
}
impl GtsIngestError {
    pub(super) fn source_read(error: impl purrdf_hash::error::NativeError + 'static) -> Self {
        match retain_native_cause(error) {
            Ok(cause) => Self::SourceRead { cause },
            Err(cause) => Self::Storage(cause),
        }
    }
    pub(super) fn poisoned(error: &Self) -> Self {
        match retain_native_cause(error.clone()) {
            Ok(cause) => Self::Poisoned { cause },
            Err(cause) => Self::Storage(cause),
        }
    }
    pub(super) fn not_ready(
        checkpoint: IngestCheckpoint,
        error: impl purrdf_hash::error::NativeError + 'static,
    ) -> Self {
        match retain_native_cause(error) {
            Ok(cause) => Self::ViewNotReady { checkpoint, cause },
            Err(cause) => Self::Storage(cause),
        }
    }
}
purrdf_lex::variant_from!(GtsIngestError { Storage(StorageError) });
impl StructuredError for GtsIngestError {
    purrdf_hash::error_visit! {
        Storage(cause) => "gts-ingest-storage" { "cause" => Cause(cause) },
        SourceRead { cause } => "gts-ingest-source-read" { "cause" => Cause(&***cause) },
        ViewNotReady { checkpoint, cause } => "gts-ingest-view-not-ready" { "checkpoint" => Record(checkpoint), "cause" => Cause(&***cause) },
        UnrepresentableTerm { position, term } => "gts-ingest-unrepresentable-term" { "position" => Text(position), "term" => Record(term) },
        BlankWireCollision { wire_value, held_scope, held_label, incoming_scope, incoming_label } => "gts-ingest-blank-wire-collision" {
            "wire_value" => Text(wire_value), "held_scope" => Value(held_scope.as_deref().map_or(ErrorValue::Absent, ErrorValue::Text)), "held_label" => Text(held_label),
            "incoming_scope" => Value(incoming_scope.as_deref().map_or(ErrorValue::Absent, ErrorValue::Text)), "incoming_label" => Text(incoming_label),
        },
        Poisoned { cause } => "gts-ingest-poisoned" { "cause" => Cause(&***cause) },
    }
}
impl std::fmt::Display for GtsIngestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(cause) => cause.fmt(f),
            Self::SourceRead { cause } => write!(f, "snapshot source read failed: {}", &***cause),
            Self::ViewNotReady { checkpoint, cause } => write!(
                f,
                "the view reported an operational failure {checkpoint}, so the rows read are a truncation rather than the view: {}",
                &***cause
            ),
            Self::UnrepresentableTerm { position, term } => write!(
                f,
                "carrier {position} is not directly representable in the gts snapshot frame: {term}"
            ),
            Self::BlankWireCollision {
                wire_value,
                held_scope,
                held_label,
                incoming_scope,
                incoming_label,
            } => write!(
                f,
                "two distinct blank-node intern keys encode onto the single wire value {wire_value:?}: ({held_scope:?}, {held_label:?}) already holds it and ({incoming_scope:?}, {incoming_label:?}) would mint an indistinguishable second term row"
            ),
            Self::Poisoned { cause } => write!(
                f,
                "the snapshot builder is poisoned by an earlier ingestion failure and can neither ingest nor publish: {}",
                &***cause
            ),
        }
    }
}
impl std::error::Error for GtsIngestError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Storage(cause) => Some(cause),
            Self::SourceRead { cause }
            | Self::ViewNotReady { cause, .. }
            | Self::Poisoned { cause } => Some(&***cause),
            _ => None,
        }
    }
}

/// Why snapshot emission could not finish. Configuration fields contain presence
/// only for signing material; secret bytes are never diagnostic evidence.
#[derive(Debug)]
#[non_exhaustive]
pub enum GtsEmitError {
    /// Retaining the original poison cause physically failed.
    Storage(StorageError),
    /// An earlier ingestion failure prohibited emission.
    Poisoned(NativeCause),
    /// Native writer construction or frame authoring failed.
    Writer(purrdf_gts::writer::WriterError),
    /// The signing fields were supplied partially.
    SigningFields {
        /// Whether a secret was supplied; its bytes are never retained here.
        secret_present: bool,
        /// Whether a key identifier was supplied.
        kid_present: bool,
        /// Whether a public key was supplied.
        armor_present: bool,
    },
    /// A zstd level was supplied without a zstd-family transform.
    ZstdLevelTransform {
        /// Original caller-selected level.
        level: i32,
        /// Effective transform chain.
        transform: Vec<String>,
    },
    /// In-band dictionaries were supplied without a zstd-family transform.
    DictionariesTransform {
        /// Number of dictionaries pinned by the plan.
        dictionaries: usize,
        /// The original effective transform chain.
        transform: Vec<String>,
    },
    /// A dictionary-bearing plan omitted an authored frame slot.
    MissingAssignment {
        /// Number of dictionaries pinned by the plan.
        dictionaries: usize,
        /// The authored frame slot missing an explicit assignment.
        slot: FrameSlot,
    },
}
impl StructuredError for FrameSlot {
    purrdf_hash::error_visit! {
        Snapshot => "gts-frame-slot-snapshot" {},
        Blob(rep) => "gts-frame-slot-blob" { "representation" => Text(rep) },
    }
}
impl StructuredError for GtsEmitError {
    purrdf_hash::error_visit! {
        Storage(cause) => "gts-emit-storage" { "cause" => Cause(cause) },
        Poisoned(cause) => "gts-emit-poisoned" { "cause" => Cause(&***cause) },
        Writer(cause) => "gts-emit-writer" { "cause" => Cause(cause) },
        SigningFields { secret_present, kid_present, armor_present } => "gts-emit-signing-fields" { "secret_present" => Boolean(*secret_present), "kid_present" => Boolean(*kid_present), "armor_present" => Boolean(*armor_present) },
        ZstdLevelTransform { level, transform } => "gts-emit-zstd-level-transform" { "level" => Signed(i128::from(*level)), "transform" => List(transform.iter().map(|name| ErrorValue::Text(name))) },
        DictionariesTransform { dictionaries, transform } => "gts-emit-dictionaries-transform" { "dictionaries" => Unsigned(*dictionaries as u128), "transform" => List(transform.iter().map(|name| ErrorValue::Text(name))) },
        MissingAssignment { dictionaries, slot } => "gts-emit-missing-assignment" { "dictionaries" => Unsigned(*dictionaries as u128), "slot" => Record(slot) },
    }
}
impl std::fmt::Display for GtsEmitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(cause) => cause.fmt(f),
            Self::Poisoned(cause) => write!(f, "the snapshot builder is poisoned by an earlier ingestion failure and can neither ingest nor publish: {}", &***cause),
            Self::Writer(cause) => cause.fmt(f),
            Self::SigningFields { .. } => f.write_str("signing requires signer_secret, signer_kid, and public_key_armor together (all three or none)"),
            Self::ZstdLevelTransform { level, transform } => write!(f, "the medium plan declares zstd level {:?} but the transform chain {transform:?} carries no zstd-family codec", Some(*level)),
            Self::DictionariesTransform { transform, .. } => write!(f, "the medium plan pins in-band dictionaries but the transform chain {transform:?} carries no zstd-family codec to prime"),
            Self::MissingAssignment { dictionaries, slot } => write!(f, "the medium plan pins {dictionaries} dictionar(y/ies) but assigns none to {slot:?}; every frame slot needs an explicit DictSelection"),
        }
    }
}
purrdf_lex::variant_from!(GtsEmitError { Writer(purrdf_gts::writer::WriterError) });
impl std::error::Error for GtsEmitError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Storage(cause) => Some(cause),
            Self::Poisoned(cause) => Some(&***cause),
            Self::Writer(cause) => Some(cause),
            _ => None,
        }
    }
}
