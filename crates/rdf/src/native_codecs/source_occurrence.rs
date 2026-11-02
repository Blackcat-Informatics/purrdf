// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native Turtle/TriG source occurrences before statement-layer folding.

use super::acquisition::NativeParseError;
use crate::TermId;
use core::ops::Range;
use purrdf_lex::allocation::{Admission, Memory, StorageError};

/// A source origin in the same [`SourceOccurrences`] table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceOriginId(pub(crate) usize);

/// The role of one native occurrence, including declarations of empty graphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceRole {
    /// The subject of an emitted statement.
    Subject,
    /// The predicate of an emitted statement.
    Predicate,
    /// The object of an emitted statement.
    Object,
    /// The inherited graph context of an emitted statement.
    Graph,
    /// A graph label in a graph-block header, including an empty block.
    GraphDeclaration,
}

/// The native grammar rule responsible for a generated semantic term.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceRule {
    /// An anonymous blank-node token.
    AnonymousBlank,
    /// The subject introduced by a blank-node property list.
    PropertyListSubject,
    /// A cell introduced by the collection expansion, in member order.
    CollectionCell {
        /// Zero-based member index within the source collection.
        index: usize,
    },
    /// The collection's `rdf:first` predicate.
    CollectionFirst,
    /// The collection's `rdf:rest` predicate.
    CollectionRest,
    /// The collection's `rdf:nil` terminator.
    CollectionNil,
    /// An unnamed reifier introduced by reification syntax.
    Reifier,
    /// The `rdf:reifies` predicate introduced by reification syntax.
    ReifiesPredicate,
    /// The triple-term object constructed by reification syntax.
    ReifiedTriple,
    /// The reference to the reifier denoted by a reifying expression.
    ReifierReference,
}

/// The original native source authority for one semantic term occurrence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceOriginKind {
    /// This term is written in the original source at the half-open byte range.
    Written(Range<usize>),
    /// This grammar position reuses a term established by another origin.
    Inherited {
        /// The original source origin, in this same table.
        from: SourceOriginId,
    },
    /// This term is introduced by a native grammar expansion.
    Generated {
        /// The grammar expansion that introduces the term.
        rule: SourceRule,
        /// The real source syntax that triggers the expansion, not a spelling
        /// fabricated for the generated term.
        anchor: Range<usize>,
        /// Original input origins, in grammar order; unused positions are empty.
        inputs: [Option<SourceOriginId>; 4],
    },
}

/// A semantic reference observed in the original native lowering/fold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceSemanticReference {
    /// A term actually interned by the native dataset construction.
    Term(TermId),
    /// A native grammar constant that need not have its own native term row.
    ConstantIri(&'static str),
    /// A native triple construction whose components retain their own origins.
    ConstructedTriple([SourceOriginId; 3]),
}

/// An inconsistent association produced by native lowering or statement folding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceAssociationError {
    /// A source statement ordinal has no recorded roots.
    MissingStatement {
        /// Original native statement ordinal.
        statement: usize,
    },
    /// A referenced origin is absent from the source table.
    MissingOrigin {
        /// Original source origin.
        origin: SourceOriginId,
    },
    /// A lowered graph declaration has no original source declaration.
    MissingDeclaration {
        /// Original named graph declaration ordinal.
        declaration: usize,
    },
    /// One origin was associated with different native terms.
    ConflictingNative {
        /// Original source origin.
        origin: SourceOriginId,
        /// Previously established association.
        previous: usize,
        /// Incompatible incoming association.
        incoming: usize,
    },
    /// One origin was associated with incompatible semantic references.
    ConflictingSemantic {
        /// Original source origin.
        origin: SourceOriginId,
        /// Previously established association.
        previous: SourceSemanticReference,
        /// Incompatible incoming association.
        incoming: SourceSemanticReference,
    },
    /// One native term was associated with different frozen terms.
    ConflictingFinal {
        /// Original native term index.
        native: usize,
        /// Previously established association.
        previous: TermId,
        /// Incompatible incoming association.
        incoming: TermId,
    },
    /// Original source presence disagrees with the folded statement's term slot.
    PresenceMismatch {
        /// Original native statement ordinal.
        statement: usize,
        /// Statement slot whose presence disagrees.
        role: SourceRole,
        /// Original source origin.
        origin: Option<SourceOriginId>,
        /// Term actually observed by native folding.
        term: Option<TermId>,
    },
}
impl core::fmt::Display for SourceAssociationError {
    fn fmt(&self, output: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(output, "inconsistent native source association: {self:?}")
    }
}
impl std::error::Error for SourceAssociationError {}

impl purrdf_hash::error::StructuredError for SourceSemanticReference {
    purrdf_hash::error_visit! {
        Term(term) => "source-reference-term" { "term" => Unsigned(term.index() as u128) },
        ConstantIri(iri) => "source-reference-constant" { "iri" => Text(iri) },
        ConstructedTriple(origins) => "source-reference-triple" {
            "subject" => Unsigned(origins[0].0 as u128),
            "predicate" => Unsigned(origins[1].0 as u128),
            "object" => Unsigned(origins[2].0 as u128)
        },
    }
}
impl purrdf_hash::error::StructuredError for SourceAssociationError {
    purrdf_hash::error_visit! {
        MissingStatement { statement } => "source-missing-statement" { "statement" => Unsigned(*statement as u128) },
        MissingOrigin { origin } => "source-missing-origin" { "origin" => Unsigned(origin.0 as u128) },
        MissingDeclaration { declaration } => "source-missing-declaration" { "declaration" => Unsigned(*declaration as u128) },
        ConflictingNative { origin, previous, incoming } => "source-conflicting-native" {
            "origin" => Unsigned(origin.0 as u128), "previous" => Unsigned(*previous as u128), "incoming" => Unsigned(*incoming as u128)
        },
        ConflictingSemantic { origin, previous, incoming } => "source-conflicting-semantic" {
            "origin" => Unsigned(origin.0 as u128), "previous" => Record(previous), "incoming" => Record(incoming)
        },
        ConflictingFinal { native, previous, incoming } => "source-conflicting-final" {
            "native" => Unsigned(*native as u128), "previous" => Unsigned(previous.index() as u128), "incoming" => Unsigned(incoming.index() as u128)
        },
        PresenceMismatch { statement, role, origin, term } => "source-presence-mismatch" {
            "statement" => Unsigned(*statement as u128),
            "role" => Code(match role {
                SourceRole::Subject => "subject", SourceRole::Predicate => "predicate", SourceRole::Object => "object", SourceRole::Graph => "graph", SourceRole::GraphDeclaration => "graph-declaration"
            }),
            "origin" => Value(origin.map_or(purrdf_hash::error::ErrorValue::Absent, |origin| purrdf_hash::error::ErrorValue::Unsigned(origin.0 as u128))),
            "term" => Value(term.map_or(purrdf_hash::error::ErrorValue::Absent, |term| purrdf_hash::error::ErrorValue::Unsigned(term.index() as u128)))
        },
    }
}

/// The native statement layer selected by the original two-pass fold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceStatementKind {
    /// An ordinary data quad, including duplicate source assertions.
    Quad,
    /// A reifier binding, including duplicate source bindings.
    Reifier,
    /// An annotation of a reifier declared in the same graph.
    Annotation,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum NativeSourceRow {
    Quad(usize),
    Reifier(usize),
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct SourceStatementOrigins {
    pub(crate) roots: [Option<SourceOriginId>; 4],
    pub(crate) kind: Option<SourceStatementKind>,
}
#[derive(Debug, Default, Clone)]
struct NativeAssociations {
    terms: Vec<Vec<SourceOriginId>>,
    final_terms: Vec<Option<TermId>>,
    quads: Vec<Vec<usize>>,
    reifiers: Vec<Vec<usize>>,
}

/// A term's origin and, for a triple term, the exact component origins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceTermOrigin {
    pub(crate) kind: SourceOriginKind,
    pub(crate) components: Option<[SourceOriginId; 3]>,
    pub(crate) semantic: Option<SourceSemanticReference>,
    pub(crate) native_term: Option<usize>,
}
impl SourceTermOrigin {
    /// The source spelling, inherited reference, or native generation rule.
    pub fn kind(&self) -> &SourceOriginKind {
        &self.kind
    }
    /// The semantic term, constant or construction observed by native lowering.
    /// No term is interned solely to fill this association.
    pub fn semantic(&self) -> Option<SourceSemanticReference> {
        self.semantic
    }
    /// The subject, predicate and object origins of a nested triple term.
    pub fn components(&self) -> Option<[SourceOriginId; 3]> {
        self.components
    }
}

/// One ordered native occurrence, preserved before diversion and deduplication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceOccurrence {
    pub(crate) statement: Option<usize>,
    pub(crate) role: SourceRole,
    pub(crate) origin: SourceOriginId,
}
impl SourceOccurrence {
    /// Native emission ordinal, before statement folding and deduplication.
    /// Graph declarations have no statement ordinal. This is not a frozen quad ID.
    pub fn statement(&self) -> Option<usize> {
        self.statement
    }
    /// Subject, predicate, object, graph context, or graph declaration.
    pub fn role(&self) -> SourceRole {
        self.role
    }
    /// This occurrence's origin in its containing source table.
    pub fn origin(&self) -> SourceOriginId {
        self.origin
    }
}

/// Exact ordered Turtle/TriG occurrences and their native source derivations.
///
/// Each statement contributes subject, predicate, object and optional graph in
/// that order. Duplicate statements remain distinct; nested triple-term origins
/// retain their ordered components. The source byte ranges refer to the original
/// input, which a caller of the borrowed parse API keeps separately.
///
/// ```
/// use purrdf_rdf::{ParseOptions, SourceRole, parse_dataset_with};
/// let parsed = parse_dataset_with(b"<urn:s> <urn:p> <urn:o> .", "text/turtle", None,
///     &ParseOptions { track_source_spans: true })?;
/// let source = parsed.spans.as_ref().expect("requested spans").source_occurrences();
/// assert_eq!(source.iter().next().expect("subject").role(), SourceRole::Subject);
/// # Ok::<(), purrdf_rdf::RdfDiagnostic>(())
/// ```
#[derive(Debug, Default, Clone)]
pub struct SourceOccurrences {
    pub(crate) origins: Vec<SourceTermOrigin>,
    pub(crate) occurrences: Vec<SourceOccurrence>,
    pub(crate) statements: Vec<SourceStatementOrigins>,
    pub(crate) declarations: Vec<SourceOriginId>,
    native: NativeAssociations,
}
impl SourceOccurrences {
    /// Borrow an original source derivation without copying its owner.
    pub fn origin(&self, id: SourceOriginId) -> Option<&SourceTermOrigin> {
        self.origins.get(id.0)
    }
    /// The statement layer selected for a native emission ordinal.
    pub fn statement_kind(&self, statement: usize) -> Option<SourceStatementKind> {
        self.statements
            .get(statement)
            .and_then(|source| source.kind)
    }

    pub(crate) fn is_active(&self) -> bool {
        !self.statements.is_empty() || !self.declarations.is_empty()
    }
    pub(crate) fn roots(
        &self,
        statement: usize,
    ) -> Result<[Option<SourceOriginId>; 4], SourceAssociationError> {
        self.statements
            .get(statement)
            .map(|source| source.roots)
            .ok_or(SourceAssociationError::MissingStatement { statement })
    }
    pub(crate) fn components(&self, origin: Option<SourceOriginId>) -> Option<[SourceOriginId; 3]> {
        origin
            .and_then(|origin| self.origins.get(origin.0))
            .and_then(|origin| origin.components)
    }
    pub(crate) fn bind_native<S: Admission + ?Sized>(
        &mut self,
        origin: Option<SourceOriginId>,
        native: usize,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), NativeParseError> {
        let mut current = origin;
        while let Some(origin) = current {
            memory.admission_mut().checkpoint(1)?;
            let record = self
                .origins
                .get_mut(origin.0)
                .ok_or(SourceAssociationError::MissingOrigin { origin })?;
            if let Some(previous) = record.native_term {
                return if previous == native {
                    Ok(())
                } else {
                    Err(SourceAssociationError::ConflictingNative {
                        origin,
                        previous,
                        incoming: native,
                    }
                    .into())
                };
            }
            record.native_term = Some(native);
            let inherited = match record.kind {
                SourceOriginKind::Inherited { from } => Some(from),
                _ => None,
            };
            while self.native.terms.len() <= native {
                memory.admission_mut().checkpoint(1)?;
                memory.push(&mut self.native.terms, Vec::new())?;
            }
            memory.push(&mut self.native.terms[native], origin)?;
            current = inherited;
        }
        Ok(())
    }
    pub(crate) fn bind_reference<S: Admission + ?Sized>(
        &mut self,
        origin: Option<SourceOriginId>,
        semantic: SourceSemanticReference,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), NativeParseError> {
        let mut current = origin;
        while let Some(origin) = current {
            memory.admission_mut().checkpoint(1)?;
            let record = self
                .origins
                .get_mut(origin.0)
                .ok_or(SourceAssociationError::MissingOrigin { origin })?;
            if let Some(previous) = record.semantic {
                if previous != semantic {
                    return Err(SourceAssociationError::ConflictingSemantic {
                        origin,
                        previous,
                        incoming: semantic,
                    }
                    .into());
                }
            } else {
                record.semantic = Some(semantic);
            }
            // Construction inputs belong to this occurrence. An inherited triple
            // has its own component origins, so only a source-independent constant
            // can be propagated back through the inheritance edge here.
            if let SourceSemanticReference::ConstantIri(_) = semantic
                && let SourceOriginKind::Inherited { from } = record.kind
            {
                current = Some(from);
            } else {
                current = None;
            }
        }
        Ok(())
    }
    fn bind_term<S: Admission + ?Sized>(
        &mut self,
        origin: Option<SourceOriginId>,
        term: TermId,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), NativeParseError> {
        let mut current = origin;
        while let Some(origin) = current {
            memory.admission_mut().checkpoint(1)?;
            let record = self
                .origins
                .get_mut(origin.0)
                .ok_or(SourceAssociationError::MissingOrigin { origin })?;
            if let Some(SourceSemanticReference::Term(previous)) = record.semantic {
                if previous != term {
                    return Err(SourceAssociationError::ConflictingSemantic {
                        origin,
                        previous: SourceSemanticReference::Term(previous),
                        incoming: SourceSemanticReference::Term(term),
                    }
                    .into());
                }
            }
            record.semantic = Some(SourceSemanticReference::Term(term));
            let inherited = match record.kind {
                SourceOriginKind::Inherited { from } => Some(from),
                _ => None,
            };
            current = inherited;
        }
        Ok(())
    }
    pub(crate) fn bind_final_native<S: Admission + ?Sized>(
        &mut self,
        native: usize,
        term: TermId,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), NativeParseError> {
        while self.native.final_terms.len() <= native {
            memory.admission_mut().checkpoint(1)?;
            memory.push(&mut self.native.final_terms, None)?;
        }
        memory.admission_mut().checkpoint(1)?;
        if let Some(previous) = self.native.final_terms[native] {
            return if previous == term {
                Ok(())
            } else {
                Err(SourceAssociationError::ConflictingFinal {
                    native,
                    previous,
                    incoming: term,
                }
                .into())
            };
        }
        self.native.final_terms[native] = Some(term);
        if let Some(origins) = self.native.terms.get(native) {
            // Each association is visited only when this native ID is first
            // observed by actual final interning; repeated cache hits are O(1).
            for index in 0..origins.len() {
                let origin = self.native.terms[native][index];
                self.bind_term(Some(origin), term, memory)?;
            }
        }
        Ok(())
    }
    pub(crate) fn associate_row<S: Admission + ?Sized>(
        &mut self,
        row: NativeSourceRow,
        statement: usize,
        memory: &mut Memory<'_, S>,
    ) -> Result<(), StorageError> {
        let (groups, index) = match row {
            NativeSourceRow::Quad(index) => (&mut self.native.quads, index),
            NativeSourceRow::Reifier(index) => (&mut self.native.reifiers, index),
        };
        while groups.len() <= index {
            memory.admission_mut().checkpoint(1)?;
            memory.push(groups, Vec::new())?;
        }
        memory.admission_mut().checkpoint(1)?;
        memory.push(&mut groups[index], statement)
    }
    pub(crate) fn bind_row<S: Admission + ?Sized>(
        &mut self,
        row: NativeSourceRow,
        kind: SourceStatementKind,
        terms: [Option<TermId>; 4],
        memory: &mut Memory<'_, S>,
    ) -> Result<(), NativeParseError> {
        let (groups, index) = match row {
            NativeSourceRow::Quad(index) => (&self.native.quads, index),
            NativeSourceRow::Reifier(index) => (&self.native.reifiers, index),
        };
        let count = groups.get(index).map_or(0, Vec::len);
        for index_in_group in 0..count {
            memory.admission_mut().checkpoint(1)?;
            let statement = match row {
                NativeSourceRow::Quad(index) => self.native.quads[index][index_in_group],
                NativeSourceRow::Reifier(index) => self.native.reifiers[index][index_in_group],
            };
            let roots = self.roots(statement)?;
            self.statements[statement].kind = Some(kind);
            for ((origin, term), role) in roots.into_iter().zip(terms).zip([
                SourceRole::Subject,
                SourceRole::Predicate,
                SourceRole::Object,
                SourceRole::Graph,
            ]) {
                match (origin, term) {
                    (Some(origin), Some(term)) => self.bind_term(Some(origin), term, memory)?,
                    (None, None) => {}
                    _ => {
                        return Err(SourceAssociationError::PresenceMismatch {
                            statement,
                            role,
                            origin,
                            term,
                        }
                        .into());
                    }
                }
            }
        }
        Ok(())
    }

    /// Every native occurrence in emission order, including duplicates.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &SourceOccurrence> {
        self.occurrences.iter()
    }
}
