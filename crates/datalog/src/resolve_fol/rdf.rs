// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original native RDF operands at the legacy clause transport boundary.

use super::{ClauseTerm, FolError, NodeId, TermDag};
use purrdf_core::TermValue;
use purrdf_lex::allocation::StorageError;

/// Original RDF values addressed by the source store's injective transport
/// identity, with explicit admission for registered non-RDF transport constants.
/// This is not a human-display lookup or a parser for arbitrary literal strings.
/// Implementations preserve every literal component, blank scope and quoted
/// term and must use their original source dictionary.
pub trait FolRdfSource {
    /// Resolve an original constant identity from that source's clause/store
    /// transport. Absence is a hard lowering error unless a literal transport
    /// operand is explicitly admitted by [`Self::is_opaque_transport`].
    fn resolve(&self, original_identity: &str) -> Option<&TermValue>;

    /// Admit a registered non-RDF relation or coordinate from the original source.
    ///
    /// The default refuses every opaque operand. Lowering consults this only for
    /// [`ClauseTerm::Literal`] identities absent from [`Self::resolve`]; an IRI
    /// always requires its original native RDF value. Implementations must use
    /// their exact source inventory rather than infer membership from spelling.
    #[must_use]
    fn is_opaque_transport(&self, _original_identity: &str) -> bool {
        false
    }
}

/// An RDF operand absent from its declared original source dictionary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolRdfError {
    identity: String,
}

impl FolRdfError {
    /// The exact original source identity that could not be resolved.
    #[must_use]
    pub fn identity(&self) -> &str {
        &self.identity
    }
}

impl core::fmt::Display for FolRdfError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "no original RDF operand for source identity {:?}",
            self.identity
        )
    }
}
impl std::error::Error for FolRdfError {}
impl purrdf_hash::error::StructuredError for FolRdfError {
    purrdf_hash::error_visit! {
        @record self { identity } => "fol-rdf-original-missing" { "identity" => Text(self.identity()) }
    }
}

pub(super) enum Constants<'a> {
    Opaque,
    Rdf(&'a dyn FolRdfSource),
}

impl Constants<'_> {
    pub(super) fn lower(&self, dag: &mut TermDag, term: &ClauseTerm) -> Result<NodeId, FolError> {
        match (self, term) {
            (_, ClauseTerm::DefaultGraph) => Ok(dag.intern_leaf("")?),
            (Self::Rdf(source), ClauseTerm::Iri(_) | ClauseTerm::Literal(_)) => {
                let identity = term
                    .surface()
                    .expect("an RDF operand has its original transport identity");
                if let Some(value) = source.resolve(&identity) {
                    let value = value
                        .try_clone()
                        .map_err(|_| StorageError::AllocationFailed)?;
                    Ok(dag.intern_rdf(value)?)
                } else if matches!(term, ClauseTerm::Literal(_))
                    && source.is_opaque_transport(&identity)
                {
                    Ok(dag.intern_leaf(&identity)?)
                } else {
                    Err(FolRdfError { identity }.into())
                }
            }
            (Self::Opaque, other) => {
                let identity = other
                    .surface()
                    .expect("the lowering scope handles variables first");
                Ok(dag.intern_leaf(&identity)?)
            }
            (Self::Rdf(_), ClauseTerm::Var(_)) => {
                unreachable!("the lowering scope handles variables first")
            }
        }
    }
}
