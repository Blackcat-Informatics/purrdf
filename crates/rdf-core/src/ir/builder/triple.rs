// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Graph-free row ingestion over the original lexical interner and validator.

use super::{
    BlankScope, ContentIdScheme, HashTable, LocalCapacity, NativeBuildError, RdfDatasetBuilder,
    RdfLiteral, RdfTerm, TermId, compute_capabilities, store_once_with_dependency,
};
use crate::ir::dataset::QuadRow;
use crate::ir::term::TermValue;
use crate::ir::triple::{TripleDataset, TripleIds, TripleReifierRow, TripleTable};
use purrdf_lex::allocation::{Admission, CapacitySpace, Memory, Resident, StorageError};
use std::hash::Hash;

#[derive(Debug)]
struct CompactRows<T> {
    rows: Vec<T>,
    index: HashTable<u32>,
}

impl<T> Default for CompactRows<T> {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            index: HashTable::new(),
        }
    }
}

impl<T: Hash + Eq + Ord> CompactRows<T> {
    fn insert<S: Admission + ?Sized>(
        &mut self,
        value: T,
        space: CapacitySpace,
        capacity: LocalCapacity,
        memory: &mut Memory<'_, S>,
        dependency: impl FnOnce(&mut Memory<'_, S>) -> Result<(), StorageError>,
    ) -> Result<(), StorageError> {
        store_once_with_dependency(
            &mut self.rows,
            &mut self.index,
            value,
            capacity,
            space,
            memory,
            dependency,
        )?;
        Ok(())
    }

    fn retained_bytes(&self) -> Result<usize, StorageError> {
        std::alloc::Layout::array::<T>(self.rows.capacity())
            .map_err(|_| StorageError::SizeOverflow)?
            .size()
            .checked_add(
                crate::hash::hash_table_retained_bound(&self.index)
                    .ok_or(StorageError::SizeOverflow)?,
            )
            .ok_or(StorageError::SizeOverflow)
    }

    fn freeze<S: Admission + ?Sized>(
        mut self,
        memory: &mut Memory<'_, S>,
    ) -> Result<Box<[T]>, StorageError> {
        let bytes = crate::hash::hash_table_retained_bound(&self.index)
            .ok_or(StorageError::SizeOverflow)?;
        drop(self.index);
        memory.release_bytes(bytes)?;
        self.rows.sort_unstable();
        memory.boxed_slice(self.rows)
    }
}

/// Explicit graph-free ingestion. Row methods have no graph argument.
///
/// The private lexical control uses the existing interner, scope and CDT homes;
/// none of its quad, reifier, annotation or named-graph tables is populated.
/// Each statement exists only in its compact table below.
#[derive(Debug, Default)]
pub struct TripleDatasetBuilder {
    lexical: RdfDatasetBuilder,
    triples: CompactRows<TripleIds>,
    reifiers: CompactRows<TripleReifierRow>,
    annotations: CompactRows<TripleIds>,
}

macro_rules! lexical_ingress {
    ($( $(#[$meta:meta])* fn $name:ident($($argument:ident: $ty:ty),*) ;)*) => {
        $(
            $(#[$meta])*
            pub fn $name(&mut self, $($argument: $ty),*) -> Result<TermId, StorageError> {
                self.lexical.$name($($argument),*)
            }
        )*
    };
}

macro_rules! resident_row_ingress {
    ($( $(#[$meta:meta])* fn $name:ident => $admitted:ident($($argument:ident: $ty:ty),*) ;)*) => {
        $(
            $(#[$meta])*
            pub fn $name(&mut self, $($argument: $ty),*) -> Result<(), StorageError> {
                let mut resident = Resident;
                let mut memory = Memory::resume(&mut resident, self.native_buffer_bytes()?);
                self.$admitted($($argument,)* &mut memory)
            }
        )*
    };
}

macro_rules! lexical_memory_ingress {
    ($( $(#[$meta:meta])* fn $name:ident($($argument:ident: $ty:ty),*) ;)*) => {
        $(
            $(#[$meta])*
            pub fn $name<S: Admission + ?Sized>(&mut self, $($argument: $ty,)*
                memory: &mut Memory<'_, S>) -> Result<TermId, StorageError> {
                self.lexical.$name($($argument,)* memory)
            }
        )*
    };
}

impl TripleDatasetBuilder {
    /// Fix the original content-recognition and derivation configuration before
    /// any term is interned. No vocabulary is selected implicitly.
    #[must_use]
    pub fn with_content_addressing(
        scheme: ContentIdScheme,
        derivation_predicate: Option<String>,
    ) -> Self {
        Self {
            lexical: RdfDatasetBuilder::with_content_addressing(scheme, derivation_predicate),
            ..Self::default()
        }
    }

    lexical_ingress! {
        /// Intern an IRI through the original identity home.
        fn intern_iri(iri: &str);
        /// Intern a blank node with its exact supplied scope.
        fn intern_blank(label: &str, scope: BlankScope);
        /// Intern a literal through the original datatype/language/direction law.
        fn intern_literal(literal: RdfLiteral);
        /// Intern an RDF 1.2 quoted triple from local IDs.
        fn intern_triple(s: TermId, p: TermId, o: TermId);
        /// Intern an owned RDF term without changing its blank identity.
        fn intern_owned_term(term: &RdfTerm);
        /// Intern an owned term under the original recursive scoped binding law.
        fn intern_owned_term_scoped(term: &RdfTerm, scope: BlankScope);
    }

    lexical_memory_ingress! {
        /// Intern borrowed lexical bytes under their original account.
        fn intern_iri_with_memory(iri: &str);
        /// Intern a scoped blank under its original account.
        fn intern_blank_with_memory(label: &str, scope: BlankScope);
        /// Intern a quoted triple under its original account.
        fn intern_triple_with_memory(s: TermId, p: TermId, o: TermId);
        /// Intern a borrowed value, including nested/CDT lexical scratch,
        /// preserving original typed capacity and physical refusal.
        fn intern_value_with_memory(value: &TermValue);
    }

    resident_row_ingress! {
        /// Insert one asserted triple; exact duplicates do not consume capacity.
        fn push_triple => push_triple_with_memory(s: TermId, p: TermId, o: TermId);
        /// Bind a reifier to a quoted triple without a graph slot.
        fn push_reifier => push_reifier_with_memory(reifier: TermId, triple: TermId);
        /// Insert an annotation in its compact statement table.
        fn push_annotation => push_annotation_with_memory(reifier: TermId, p: TermId, o: TermId);
    }

    /// Admit and insert a compact base row before publication.
    ///
    /// # Errors
    /// Returns row capacity, admission or allocator refusal.
    pub fn push_triple_with_memory<S: Admission + ?Sized>(
        &mut self,
        s: TermId,
        p: TermId,
        o: TermId,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        self.triples.insert(
            TripleIds { s, p, o },
            CapacitySpace::Triples,
            self.lexical.interner.capacity,
            memory,
            |_| Ok(()),
        )
    }

    /// Admit the binding before its implicit `rdf:reifies` dependency, then
    /// publish only when both succeed, through the original store-once home.
    ///
    /// # Errors
    /// Returns the first row, term or physical storage refusal.
    pub fn push_reifier_with_memory<S: Admission + ?Sized>(
        &mut self,
        reifier: TermId,
        triple: TermId,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        let interner = &mut self.lexical.interner;
        let predicate = &mut self.lexical.reifies_predicate;
        self.reifiers.insert(
            (reifier, triple),
            CapacitySpace::Reifiers,
            interner.capacity,
            memory,
            |memory| interner.ensure_reifies(predicate, memory),
        )
    }

    /// Admit and insert a compact annotation without a graph slot.
    ///
    /// # Errors
    /// Returns row capacity, admission or allocator refusal.
    pub fn push_annotation_with_memory<S: Admission + ?Sized>(
        &mut self,
        reifier: TermId,
        p: TermId,
        o: TermId,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        self.annotations.insert(
            TripleIds { s: reifier, p, o },
            CapacitySpace::Annotations,
            self.lexical.interner.capacity,
            memory,
            |_| Ok(()),
        )
    }

    /// All currently retained native buffers, for explicit admission adoption.
    ///
    /// # Errors
    /// Refuses an unrepresentable physical layout.
    pub fn native_buffer_bytes(&self) -> Result<usize, StorageError> {
        [
            self.lexical.native_buffer_bytes()?,
            self.triples.retained_bytes()?,
            self.reifiers.retained_bytes()?,
            self.annotations.retained_bytes()?,
        ]
        .into_iter()
        .try_fold(0usize, |sum, bytes| {
            sum.checked_add(bytes).ok_or(StorageError::SizeOverflow)
        })
    }

    /// Validate all three roles through the same native RDF validator and freeze
    /// without ever constructing a quad dataset or a quad row allocation.
    ///
    /// # Errors
    /// Returns the original RDF diagnostic or typed storage refusal.
    pub fn freeze(self) -> Result<TripleDataset, NativeBuildError> {
        let mut resident = Resident;
        let mut memory = Memory::resume(&mut resident, self.native_buffer_bytes()?);
        self.freeze_with_memory(&mut memory)
    }

    /// Freeze using the account that admitted the original buffers.
    ///
    /// # Errors
    /// Returns the first native validation or storage error.
    pub fn freeze_with_memory<S: Admission + ?Sized>(
        self,
        memory: &mut Memory<'_, S>,
    ) -> Result<TripleDataset, NativeBuildError> {
        let bytes = self.native_buffer_bytes()?;
        let valid = crate::ir::validate::validate_rows_with_memory(
            &self.lexical,
            self.triples.rows.iter().map(|row| QuadRow {
                s: row.s,
                p: row.p,
                o: row.o,
                g: None,
            }),
            self.reifiers.rows.iter().map(|&(r, t)| (r, t, None)),
            self.annotations
                .rows
                .iter()
                .map(|row| (row.s, row.p, row.o, None)),
            memory,
        );
        if let Err(error) = valid {
            drop(self);
            memory.release_bytes(bytes)?;
            return Err(error);
        }
        let caps = compute_capabilities(
            &self.lexical.interner.terms,
            false,
            &self.reifiers.rows,
            &self.annotations.rows,
            false,
        );
        let RdfDatasetBuilder {
            interner,
            used_scopes,
            scope_journal,
            derivation_predicate,
            ..
        } = self.lexical;
        memory.release_vec(used_scopes)?;
        memory.release_vec(scope_journal)?;
        let dictionary = interner.freeze_with_memory(derivation_predicate, memory)?;
        let statistics = super::super::statistics::LocalStatistics::build(
            &dictionary,
            (self.triples.rows.len(), |index| {
                let row = self.triples.rows[index];
                super::super::dataset::QuadIds {
                    s: row.s,
                    p: row.p,
                    o: row.o,
                    g: None,
                }
            }),
            &(self.reifiers.rows.len(), |index| {
                let (s, o) = self.reifiers.rows[index];
                (s, o, None)
            }),
            (self.annotations.rows.len(), |index| {
                let row = self.annotations.rows[index];
                super::super::dataset::QuadIds {
                    s: row.s,
                    p: row.p,
                    o: row.o,
                    g: None,
                }
            }),
            memory,
        )?;
        let snapshot_identity = crate::source::resident_snapshot(
            statistics.source_directory().content_records(),
            purrdf_hash::multiset::Multiset256::default(),
            dictionary.snapshot_fingerprint(memory)?,
        );
        Ok(TripleDataset {
            snapshot_identity,
            dictionary,
            statistics,
            triples: TripleTable::new(self.triples.freeze(memory)?),
            reifiers: self.reifiers.freeze(memory)?,
            reifier_targets: std::sync::OnceLock::new(),
            reifier_components: std::array::from_fn(|_| std::sync::OnceLock::new()),
            annotations: TripleTable::new(self.annotations.freeze(memory)?),
            locations: Box::default(),
            caps,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_misses_refuse_at_original_named_capacity_without_quad_staging() {
        let mut builder = TripleDatasetBuilder::default();
        let s = builder.intern_iri("urn:s").unwrap();
        let p = builder.intern_iri("urn:p").unwrap();
        let o = builder.intern_iri("urn:o").unwrap();
        let t = builder.intern_triple(s, p, o).unwrap();
        builder.lexical.interner.capacity = LocalCapacity::Reduced {
            terms: 32,
            text: 2048,
            rows: 1,
        };
        builder.push_triple(s, p, o).unwrap();
        builder.push_reifier(s, t).unwrap();
        builder.push_annotation(s, p, o).unwrap();
        for _ in 0..2 {
            builder.push_triple(s, p, o).unwrap();
            builder.push_reifier(s, t).unwrap();
            builder.push_annotation(s, p, o).unwrap();
            for (error, space) in [
                (
                    builder.push_triple(o, p, s).unwrap_err(),
                    CapacitySpace::Triples,
                ),
                (
                    builder.push_reifier(o, t).unwrap_err(),
                    CapacitySpace::Reifiers,
                ),
                (
                    builder.push_annotation(o, p, s).unwrap_err(),
                    CapacitySpace::Annotations,
                ),
            ] {
                let StorageError::Capacity(error) = error else {
                    panic!("logical row capacity")
                };
                assert_eq!(
                    (error.space, error.attempted(), error.limit()),
                    (space, 2, 1)
                );
            }
        }
        assert_eq!(builder.lexical.quads.len(), 0);
        assert_eq!(builder.lexical.reifiers.len(), 0);
        assert_eq!(builder.lexical.annotations.len(), 0);
        assert_eq!(builder.lexical.declared_graphs.len(), 0);
        let dataset = builder.freeze().unwrap();
        assert_eq!(dataset.len(), 1);
        assert_eq!(dataset.reifiers().count(), 1);
        assert_eq!(dataset.annotations().count(), 1);
    }

    #[test]
    fn compact_rows_use_original_first_lexical_diagnostic() {
        let mut compact = TripleDatasetBuilder::default();
        let mut quad = RdfDatasetBuilder::new();
        let cs = compact.intern_iri("relative").unwrap();
        let qs = quad.intern_iri("relative").unwrap();
        let cp = compact
            .intern_literal(RdfLiteral::simple("invalid predicate"))
            .unwrap();
        let qp = quad
            .intern_literal(RdfLiteral::simple("invalid predicate"))
            .unwrap();
        compact.push_triple(cs, cp, cs).unwrap();
        quad.push_quad(qs, qp, qs, None).unwrap();
        let NativeBuildError::Diagnostic(compact) = compact.freeze().unwrap_err() else {
            panic!("original RDF diagnostic")
        };
        let quad = quad.freeze().unwrap_err();
        assert_eq!(compact.code, quad.code);
        assert_eq!(compact.message, quad.message);
    }
}
