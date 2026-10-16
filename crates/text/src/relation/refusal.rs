// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native invocation failures, retaining admitted input and query owners.

use super::{
    OCCURRENCE_MODE, OCCURRENCE_TERM, SEARCH_CANDIDATE_MODE, SEARCH_MODE, SEARCH_NEEDLE,
    SEARCH_RANK,
};
use crate::query_workspace::QueryString;
use purrdf_sparql_eval::{
    AdmittedVec, EvalError, NativeDiagnostic, NativeDiagnosticKind, PfArity, WorkspaceCapability,
    WorkspaceTerm,
};

#[derive(Debug)]
pub(crate) enum RelationRefusal {
    NeedleDatatype {
        position: usize,
        datatype: QueryString,
    },
    NeedleKind {
        position: usize,
        value: WorkspaceTerm,
    },
    LanguageKind {
        position: usize,
        value: WorkspaceTerm,
    },
    RankKind {
        value: WorkspaceTerm,
    },
    RankDatatype {
        datatype: QueryString,
    },
    RankLexical {
        lexical: QueryString,
    },
    RankNonpositive {
        lexical: QueryString,
        negative: bool,
        zero: bool,
    },
    Arity {
        relation: &'static str,
        declared: PfArity,
        supplied: PfArity,
    },
    SearchUnbound,
    OccurrenceUnbound,
    FieldScope {
        predicate: WorkspaceTerm,
    },
    OccurrenceTerms {
        text: QueryString,
        analyzed: AdmittedVec<QueryString>,
    },
    Scope(purrdf_core::query_scope::LexicalScopeError),
    DictionaryAnalyzer {
        expected: [u8; 32],
        actual: [u8; 32],
    },
    PhraseArity {
        supplied: PfArity,
    },
    PhraseBinding {
        value: Option<WorkspaceTerm>,
    },
    PhrasePredicate {
        predicate: WorkspaceTerm,
    },
    Invariant {
        reason: Invariant,
        document: u32,
    },
    ScoredDocument {
        document: u32,
    },
    PostingDocument {
        document: u32,
    },
}

#[derive(Debug)]
pub(crate) enum Invariant {
    DocumentPartition,
    CandidateDocument,
    ScopedPartition,
    ScopedDeclaration,
    HoldingDocument,
    CursorDeclaration,
    RejectedCandidate,
}

impl purrdf_hash::error::StructuredError for Invariant {
    purrdf_hash::error_visit! {
        DocumentPartition => "text-invariant-document-partition" {},
        CandidateDocument => "text-invariant-candidate-document" {},
        ScopedPartition => "text-invariant-scoped-partition" {},
        ScopedDeclaration => "text-invariant-scoped-declaration" {},
        HoldingDocument => "text-invariant-holding-document" {},
        CursorDeclaration => "text-invariant-cursor-declaration" {},
        RejectedCandidate => "text-invariant-rejected-candidate" {},
    }
}

impl RelationRefusal {
    pub(crate) fn into_eval(self, workspace: &WorkspaceCapability) -> EvalError {
        let kind = match &self {
            Self::Invariant { .. } => NativeDiagnosticKind::Internal,
            Self::DictionaryAnalyzer { .. } | Self::PhrasePredicate { .. } => {
                NativeDiagnosticKind::Config
            }
            Self::ScoredDocument { .. } | Self::PostingDocument { .. } => {
                NativeDiagnosticKind::Data
            }
            _ => NativeDiagnosticKind::Function,
        };
        match NativeDiagnostic::render_cause(kind, self, workspace) {
            Ok(diagnostic) => EvalError::NativeDiagnostic(diagnostic),
            Err(error) => error,
        }
    }
}

impl purrdf_hash::error::StructuredError for RelationRefusal {
    purrdf_hash::error_visit! {
        NeedleDatatype { position, datatype } => "text-call-needle-datatype" { "position" => Unsigned(*position as u128), "datatype" => Iri(datatype) },
        NeedleKind { position, value } => "text-call-needle-kind" { "position" => Unsigned(*position as u128), "value" => Term(&**value) },
        LanguageKind { position, value } => "text-call-language-kind" { "position" => Unsigned(*position as u128), "value" => Term(&**value) },
        RankKind { value } => "text-call-rank-kind" { "position" => Unsigned(SEARCH_RANK as u128), "value" => Term(&**value) },
        RankDatatype { datatype } => "text-call-rank-datatype" { "position" => Unsigned(SEARCH_RANK as u128), "datatype" => Iri(datatype) },
        RankLexical { lexical } => "text-call-rank-lexical" { "position" => Unsigned(SEARCH_RANK as u128), "lexical" => Text(lexical) },
        RankNonpositive { lexical, negative, zero } => "text-call-rank-nonpositive" { "position" => Unsigned(SEARCH_RANK as u128), "lexical" => Text(lexical), "negative" => Boolean(*negative), "zero" => Boolean(*zero) },
        Arity { relation, declared, supplied } => "text-call-arity" { "relation" => Code(relation), "declared" => Record(declared), "supplied" => Record(supplied) },
        SearchUnbound => "text-call-search-unbound" { "position" => Unsigned(SEARCH_NEEDLE as u128), "mode" => Code(SEARCH_MODE), "candidate_mode" => Code(SEARCH_CANDIDATE_MODE) },
        OccurrenceUnbound => "text-call-occurrence-unbound" { "position" => Unsigned(OCCURRENCE_TERM as u128), "mode" => Code(OCCURRENCE_MODE) },
        FieldScope { predicate } => "text-call-field-scope" { "predicate" => Term(&**predicate) },
        OccurrenceTerms { text, analyzed } => "text-call-occurrence-terms" { "text" => Text(text), "count" => Unsigned(analyzed.len() as u128), "terms" => List(analyzed.iter().map(|term| purrdf_hash::error::ErrorValue::Text(term.as_str()))) },
        Scope(error) => "text-call-lexical-scope" { "cause" => Cause(error) },
        DictionaryAnalyzer { expected, actual } => "text-call-dictionary-analyzer" { "expected" => Bytes(expected), "actual" => Bytes(actual) },
        PhraseArity { supplied } => "text-call-phrase-arity" { "declared_subject" => Unsigned(1), "declared_object" => Unsigned(4), "supplied" => Record(supplied) },
        PhraseBinding { value } => "text-call-phrase-binding" { "position" => Unsigned(1), "value" => Value(value.as_ref().map_or(purrdf_hash::error::ErrorValue::Absent, |term| purrdf_hash::error::ErrorValue::Term(&**term))) },
        PhrasePredicate { predicate } => "text-call-phrase-predicate" { "predicate" => Term(&**predicate) },
        Invariant { reason, document } => "text-call-invariant" { "reason" => Record(reason), "document" => Unsigned(u128::from(*document)) },
        ScoredDocument { document } => "text-call-scored-document" { "document" => Unsigned(u128::from(*document)) },
        PostingDocument { document } => "text-call-posting-document" { "document" => Unsigned(u128::from(*document)) },
    }
}

impl core::fmt::Display for RelationRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NeedleDatatype { position, datatype } => write!(
                f,
                "the needle at position {position} is a literal of datatype <{}>; a text search reads a string, so the needle must be an xsd:string, rdf:langString or rdf:dirLangString",
                datatype.as_str()
            ),
            Self::NeedleKind { position, value } => write!(
                f,
                "the needle at position {position} is {}; only a literal carries text, so an IRI, a blank node or a triple term names nothing this relation can search for",
                value.kind_description()
            ),
            Self::LanguageKind { position, value } => write!(
                f,
                "the language at position {position} is {}; this relation emits an xsd:string there — the tag itself, or the empty string for an untagged document — so nothing else can name a language",
                value.kind_description()
            ),
            Self::RankKind { value } => write!(
                f,
                "the rank at position {SEARCH_RANK} is {}; a rank is a 1-based position, which only an xsd:integer literal can name",
                value.kind_description()
            ),
            Self::RankDatatype { datatype } => write!(
                f,
                "the rank at position {SEARCH_RANK} is a literal of datatype <{}>; this relation emits an xsd:integer there, so only an xsd:integer can name a rank",
                datatype.as_str()
            ),
            Self::RankLexical { lexical } => write!(
                f,
                "the rank at position {SEARCH_RANK} has lexical form {:?}, which is not in the lexical space of xsd:integer (an optional sign followed by one or more digits)",
                lexical.as_str()
            ),
            Self::RankNonpositive { lexical, .. } => write!(
                f,
                "the rank at position {SEARCH_RANK} is {}; ranks are 1-based, so zero and every negative value are outside the domain rather than an empty answer within it",
                lexical.as_str()
            ),
            Self::Arity {
                relation,
                declared,
                supplied,
            } => write!(
                f,
                "the {relation} relation expects {declared} argument(s), got {supplied}"
            ),
            Self::SearchUnbound => write!(
                f,
                "the needle at position {SEARCH_NEEDLE} is free; this relation retrieves documents for a needle and cannot enumerate needles for a document, which is why both of its declared modes — `{SEARCH_MODE}` and `{SEARCH_CANDIDATE_MODE}` — demand it"
            ),
            Self::OccurrenceUnbound => write!(
                f,
                "the term at position {OCCURRENCE_TERM} is free; this relation enumerates the occurrences of a term and cannot enumerate terms, which is why its only declared mode is `{OCCURRENCE_MODE}`"
            ),
            Self::FieldScope { .. } => {
                f.write_str("field scope predicate is not selected by this text index")
            }
            Self::OccurrenceTerms { text, analyzed } => write!(
                f,
                "the term at position {OCCURRENCE_TERM} is {:?}, which analyzes to {} terms ({:?}); this relation matches ONE term per invocation by contract, so a multi-term needle is written as one call per term joined on the document position",
                text.as_str(),
                analyzed.len(),
                &**analyzed
            ),
            Self::Scope(error) => error.fmt(f),
            Self::DictionaryAnalyzer { .. } => {
                f.write_str("dictionary analyzer differs from index analyzer")
            }
            Self::PhraseArity { .. } => {
                f.write_str("phrase occurrence expects one subject and four object arguments")
            }
            Self::PhraseBinding { .. } => {
                f.write_str("phrase occurrence requires a bound xsd:string phrase")
            }
            Self::PhrasePredicate { .. } => {
                f.write_str("phrase predicate is not configured in the index")
            }
            Self::Invariant { reason, document } => match reason {
                Invariant::DocumentPartition => {
                    write!(f, "text index document {document} has no partition")
                }
                Invariant::CandidateDocument => {
                    f.write_str("text candidate is absent from its original index")
                }
                Invariant::ScopedPartition => {
                    f.write_str("scoped text row lost its original partition")
                }
                Invariant::ScopedDeclaration => {
                    f.write_str("scoped text row lost its original declaration")
                }
                Invariant::HoldingDocument => {
                    f.write_str("bound text holding lost its original document")
                }
                Invariant::CursorDeclaration => {
                    f.write_str("text cursor lost its original scoped declaration")
                }
                Invariant::RejectedCandidate => {
                    f.write_str("text cursor published a rejected scoped candidate")
                }
            },
            Self::ScoredDocument { document } => write!(
                f,
                "a scored row named document {document}, which the index does not hold"
            ),
            Self::PostingDocument { document } => write!(
                f,
                "a posting named document {document}, which the index does not hold"
            ),
        }
    }
}

impl std::error::Error for RelationRefusal {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Scope(error) => Some(error),
            _ => None,
        }
    }
}
