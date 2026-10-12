// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native document-load ingress shared by the Python Store and MutableDataset.
//! No interpreter state participates in scope minting or native mutation.

use purrdf_core::ir::MutableDataset;
use purrdf_core::{BlankScope, DatasetMut, QuadValues, RdfDiagnostic, RdfQuad, TermValue};
use purrdf_lex::allocation::{CapacityError, CapacitySpace, StorageError};

/// Preserve the two existing host error boundaries: mutation IRI errors and
/// declaration diagnostics have different message conventions.
#[derive(Debug)]
pub(super) enum LoadError {
    Insert(purrdf::iri::IriReadError),
    Declaration(RdfDiagnostic),
}

purrdf_lex::variant_from!(LoadError {
    Insert(purrdf::iri::IriReadError),
    Declaration(RdfDiagnostic),
});

impl From<StorageError> for LoadError {
    fn from(error: StorageError) -> Self {
        Self::Insert(error.into())
    }
}

/// Convert a native owned [`RdfQuad`] into [`QuadValues`], tagging every blank node
/// with `scope` (the per-load isolation scope) through
/// [`TermValue::from_rdf_term_in_scope`].
pub(super) fn rdf_quad_to_values_scoped(quad: &RdfQuad, scope: BlankScope) -> QuadValues {
    QuadValues {
        s: TermValue::from_rdf_term_in_scope(&quad.subject, scope),
        p: TermValue::Iri(quad.predicate.clone()),
        o: TermValue::from_rdf_term_in_scope(&quad.object, scope),
        g: quad
            .graph_name
            .as_ref()
            .map(|g| TermValue::from_rdf_term_in_scope(g, scope)),
    }
}

/// Mint one per-document scope while holding the store's exclusive Python borrow.
/// The exhausted ordinal remains representable, so repeated refusal cannot reuse a
/// prior document's blank-node identity.
fn next_load_scope(next: &mut u64) -> Result<BlankScope, StorageError> {
    CapacityError::check(
        CapacitySpace::ImportBlankScopes,
        u128::from(*next),
        u128::from(u32::MAX),
    )?;
    let ordinal = u32::try_from(*next).map_err(|_| StorageError::SizeOverflow)?;
    *next += 1;
    Ok(BlankScope(ordinal))
}

/// Insert one parsed document through the shared native mutation boundary. Mint a
/// scope only if a direct, nested or composite blank identity needs it, admitting
/// that scope before any document mutation. Blank-free documents and declarations
/// do not consume the bounded scope space.
/// Shared by `Store.load` and `MutableDataset.load`.
pub(super) fn insert_loaded_document(
    inner: &mut MutableDataset,
    quads: &[RdfQuad],
    declared: Vec<TermValue>,
    next_scope: &mut u64,
) -> Result<(), LoadError> {
    use purrdf_lex::allocation::{Memory, Resident};
    use std::ops::ControlFlow;

    // Scope refusal must precede even a blank-free prefix's insertion. This scan
    // uses the original term/blank visitor; it retains no document copy. Terms
    // inspected here are converted again during insertion, including blank-free
    // documents, so this is not an allocation-neutral preflight.
    let scope = 'document: {
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        for quad in quads {
            let values = rdf_quad_to_values_scoped(quad, BlankScope::DEFAULT);
            for term in [&values.s, &values.o].into_iter().chain(values.g.iter()) {
                if term
                    .visit_blank_identities_with_memory(
                        |_, _, _| ControlFlow::Break(()),
                        &mut memory,
                    )?
                    .is_break()
                {
                    break 'document next_load_scope(next_scope)?;
                }
            }
        }
        if declared
            .iter()
            .any(|graph| matches!(graph, TermValue::Blank { .. }))
        {
            next_load_scope(next_scope)?
        } else {
            BlankScope::DEFAULT
        }
    };
    for quad in quads {
        inner.insert(rdf_quad_to_values_scoped(quad, scope))?;
    }
    for graph in declared {
        let term = graph
            .to_rdf_term()
            .expect("a declared graph name has an owned form");
        inner.declare_named_graph(TermValue::from_rdf_term_in_scope(&term, scope))?;
    }
    Ok(())
}
