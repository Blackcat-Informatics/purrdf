// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Checked eligibility for the borrowed recursive evaluator.

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
