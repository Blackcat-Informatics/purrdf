// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Checked eligibility for the borrowed recursive evaluator.

use crate::expression::{NodeExpr, ShapeArg};
use crate::shapes::{
    ModelOwner,
    link::{ShapeIndex, ShapeIndexRef},
};
use crate::term::Term;
use purrdf_lex::allocation::{Shared, StorageError};
use std::sync::{Arc, Weak};

struct Fields {
    definition: Term,
    parameter: &'static str,
    model: Option<Arc<ModelOwner>>,
    _index_owner: Option<ShapeIndex>,
    index: Option<ShapeIndexRef>,
}

/// An original operation whose recursive dependency scope has not been proved.
/// Published refusals retain their original model and index. They are never
/// installed in the weak-key admission cache.
#[derive(Clone)]
pub struct ClosedRecursionRefusal {
    fields: Shared<Fields>,
}

impl ClosedRecursionRefusal {
    pub(crate) fn new(
        definition: &Term,
        parameter: &'static str,
        model: Option<&Arc<ModelOwner>>,
        index: Option<&ShapeIndexRef>,
    ) -> Result<Self, StorageError> {
        let fields = Fields {
            definition: definition.try_clone()?,
            parameter,
            model: model.cloned(),
            _index_owner: index.and_then(Weak::upgrade),
            index: index.cloned(),
        };
        Shared::try_new(fields)
            .map(|fields| Self { fields })
            .map_err(|_| StorageError::AllocationFailed)
    }
    /// The original containing definition.
    #[must_use]
    pub fn definition(&self) -> &Term {
        &self.fields.definition
    }
    /// The original parameter or native model operation.
    #[must_use]
    pub fn parameter(&self) -> &'static str {
        self.fields.parameter
    }
    /// The original model, retained for inspection after the caller drops it.
    #[must_use]
    pub fn model(&self) -> Option<&Arc<ModelOwner>> {
        self.fields.model.as_ref()
    }
    /// The original computed-reference index scope, if present.
    #[must_use]
    pub fn index(&self) -> Option<&ShapeIndexRef> {
        self.fields.index.as_ref()
    }
}

impl PartialEq for ClosedRecursionRefusal {
    fn eq(&self, other: &Self) -> bool {
        fn same<T>(a: Option<&Weak<T>>, b: Option<&Weak<T>>) -> bool {
            match (a, b) {
                (Some(a), Some(b)) => Weak::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            }
        }
        self.definition() == other.definition()
            && self.parameter() == other.parameter()
            && match (self.model(), other.model()) {
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            }
            && same(self.index(), other.index())
    }
}
impl Eq for ClosedRecursionRefusal {}
impl std::fmt::Debug for ClosedRecursionRefusal {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.debug_struct("ClosedRecursionRefusal")
            .field("definition", self.definition())
            .field("parameter", &self.parameter())
            .finish_non_exhaustive()
    }
}
impl std::fmt::Display for ClosedRecursionRefusal {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            out,
            "recursive definition {} has no closed dependency proof for {}; retain its original model/index and use a supported structural catalog",
            self.definition(),
            self.parameter()
        )
    }
}
impl std::error::Error for ClosedRecursionRefusal {}

/// Constructed only after the original catalog and occurrence are checked.
pub(crate) struct ClosedProof(pub(super) ());

/// A conformance question whose focus and target cannot depend on the current
/// recursive valuation. Only a canonical reference into the same original
/// catalog is admitted: neither a local replacement body nor another model's
/// equal-named shape may acquire this catalog's proof or borrowed binding.
pub(crate) fn original_conformance<'a>(
    expression: &'a NodeExpr,
    owner: Option<&Arc<ModelOwner>>,
) -> Option<&'a crate::shapes::Shape> {
    let NodeExpr::ConformsToShape {
        node,
        shape: ShapeArg::Named(shape),
    } = expression
    else {
        return None;
    };
    matches!(node.as_ref(), NodeExpr::This)
        .then(|| original_reference(shape, owner))
        .flatten()
}

/// Resolve only canonical references in their original declaration domain.
/// Blank labels carry RDF scope, while the owner distinguishes independent
/// catalogs even when their native terms have the same spelling.
fn original_reference<'a>(
    shape: &'a crate::shapes::Shape,
    owner: Option<&Arc<ModelOwner>>,
) -> Option<&'a crate::shapes::Shape> {
    let owner = owner?;
    (matches!(shape.id, Term::NamedNode(_) | Term::BlankNode(_))
        && shape
            .owner
            .as_ref()
            .is_none_or(|target| Arc::ptr_eq(owner, target))
        && super::model::canonical_handle(shape)
        && owner.definition(&shape.id).is_some())
    .then_some(shape)
}

/// A where operand is the original bare reference, not an exported root with
/// its own targets. Check that representation before the general root proof:
/// its ownerless branch checks every metadata field without descending into a
/// caller-built tree of owned where targets.
pub(crate) fn where_reference<'a>(
    shape: &'a crate::shapes::Shape,
    owner: Option<&Arc<ModelOwner>>,
) -> Option<&'a crate::shapes::Shape> {
    if shape.owner.is_some() {
        return None;
    }
    original_reference(shape, owner)
}

/// The original kind of an admitted dependency-graph occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RecursionDefinitionKind {
    /// Original node/definition shape body.
    Shape,
    /// Original local property body.
    Property,
    /// Original named expression declaration in unselected admission evidence.
    Function,
}

/// An original definition retained by a published recursive-component refusal.
#[derive(Clone)]
pub struct RecursionDefinition {
    fields: Shared<DefinitionFields>,
}
struct DefinitionFields {
    definition: Shared<Term>,
    model: Option<Arc<ModelOwner>>,
    occurrence: usize,
    kind: RecursionDefinitionKind,
}
purrdf_hash::debug_non_exhaustive!(RecursionDefinition {});
impl RecursionDefinition {
    /// The authored definition identity, without rendering or scope folding.
    #[must_use]
    pub fn definition(&self) -> &Term {
        &self.fields.definition
    }
    /// Its original model domain, retained for inspection.
    #[must_use]
    pub fn model(&self) -> Option<&Arc<ModelOwner>> {
        self.fields.model.as_ref()
    }
    /// Original occurrence identity within that retained model domain.
    #[must_use]
    pub fn occurrence(&self) -> usize {
        self.fields.occurrence
    }
    /// The original shape/property/declaration kind, independently of its name.
    #[must_use]
    pub fn kind(&self) -> RecursionDefinitionKind {
        self.fields.kind
    }
}
impl PartialEq for RecursionDefinition {
    fn eq(&self, other: &Self) -> bool {
        self.definition() == other.definition()
            && self.occurrence() == other.occurrence()
            && self.kind() == other.kind()
            && match (self.model(), other.model()) {
                (Some(a), Some(b)) => Arc::ptr_eq(a, b),
                (None, None) => true,
                _ => false,
            }
    }
}
impl Eq for RecursionDefinition {}

pub(crate) struct WeakDefinition {
    definition: Shared<Term>,
    model: Option<Weak<ModelOwner>>,
    occurrence: usize,
    kind: RecursionDefinitionKind,
}
impl WeakDefinition {
    pub(crate) fn new(
        definition: &Term,
        model: Option<&Arc<ModelOwner>>,
        occurrence: usize,
        kind: RecursionDefinitionKind,
    ) -> Result<Self, StorageError> {
        Ok(Self {
            definition: Shared::try_new(definition.try_clone()?)
                .map_err(|_| StorageError::AllocationFailed)?,
            model: model.map(Arc::downgrade),
            occurrence,
            kind,
        })
    }
    pub(crate) fn native(&self) -> &Term {
        &self.definition
    }
    pub(crate) fn model_key(&self) -> Option<usize> {
        self.model.as_ref().map(|model| model.as_ptr().addr())
    }
    pub(crate) fn occurrence(&self) -> usize {
        self.occurrence
    }
    pub(crate) fn kind(&self) -> RecursionDefinitionKind {
        self.kind
    }
    pub(crate) fn publish(&self) -> Result<RecursionDefinition, crate::error::ShapesError> {
        let model = self
            .model
            .as_ref()
            .map(|model| {
                model
                    .upgrade()
                    .ok_or(crate::PreparationRefusal::UnboundModel)
            })
            .transpose()?;
        let fields = Shared::try_new(DefinitionFields {
            definition: self.definition.clone(),
            model,
            occurrence: self.occurrence,
            kind: self.kind,
        })
        .map_err(|_| StorageError::AllocationFailed)?;
        Ok(RecursionDefinition { fields })
    }
}
