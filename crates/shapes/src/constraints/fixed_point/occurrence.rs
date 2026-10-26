// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Resolve original occurrences while their owning model is retained.

use std::sync::Arc;

use purrdf_lex::allocation::{Memory, OrderedTable, Resident, Shared, SharedView, StorageError};
use purrdf_rdf::FastSet;

use crate::error::ShapesError;
use crate::expression::CustomFunction;
use crate::product::ShapesProductError;
use crate::shapes::link::{ModelWalk, ShapeIndexRef};
use crate::shapes::{Constraint, ModelOwner, Shape};

pub(crate) fn property(
    owner: &Arc<ModelOwner>,
    address: usize,
) -> Result<&Constraint, ShapesError> {
    Find::search(owner, Position::Property(address))?.constraint.ok_or_else(|| {
        ShapesError::internal(
            "recursive property work is outside its original model; retain the source model with its property edge",
        )
    })
}

pub(crate) fn shape(owner: &Arc<ModelOwner>, address: usize) -> Result<&Shape, ShapesError> {
    maybe_shape(owner, address)?.ok_or_else(|| {
        ShapesError::internal(
            "recursive local shape work is outside its original model; retain the source model with its local handle",
        )
    })
}

pub(crate) fn maybe_shape(
    owner: &Arc<ModelOwner>,
    address: usize,
) -> Result<Option<&Shape>, ShapesError> {
    Ok(Find::search(owner, Position::Shape(address))?.shape)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Position {
    Shape(usize),
    Property(usize),
}

struct Find<'model> {
    position: Position,
    shape: Option<&'model Shape>,
    constraint: Option<&'model Constraint>,
    depth: u32,
    owners: FastSet<usize>,
    functions: FastSet<usize>,
}

impl<'model> Find<'model> {
    fn search(owner: &'model Arc<ModelOwner>, position: Position) -> Result<Self, ShapesError> {
        let mut find = Self {
            position,
            shape: None,
            constraint: None,
            depth: 0,
            owners: FastSet::default(),
            functions: FastSet::default(),
        };
        find.model(owner).map_err(|error| {
            error.preparation_refusal().cloned().map_or_else(
                || ShapesError::internal(format!("original model occurrence: {error}")),
                ShapesError::Preparation,
            )
        })?;
        Ok(find)
    }

    fn finished(&self) -> bool {
        self.shape.is_some() || self.constraint.is_some()
    }
}

impl<'model> ModelWalk<'model> for Find<'model> {
    fn depth(&mut self) -> &mut u32 {
        &mut self.depth
    }

    fn enter(&mut self) -> Result<(), ShapesProductError> {
        if purrdf_stack::is_low() {
            return Err(crate::PreparationRefusal::StackMargin.into());
        }
        self.depth = self
            .depth
            .checked_add(1)
            .ok_or(crate::PreparationRefusal::Storage(
                StorageError::SizeOverflow,
            ))?;
        Ok(())
    }
    fn model(&mut self, owner: &'model Arc<ModelOwner>) -> Result<(), ShapesProductError> {
        if self.first_owner(owner)? {
            for function in &owner.functions {
                self.declare(function)?;
            }
            for definition in owner.definitions() {
                self.shape(definition)?;
            }
        }
        Ok(())
    }
    fn first_owner(&mut self, owner: &'model Arc<ModelOwner>) -> Result<bool, ShapesProductError> {
        if self.finished() {
            return Ok(false);
        }
        self.owners
            .try_reserve(1)
            .map_err(|_| crate::PreparationRefusal::Storage(StorageError::AllocationFailed))?;
        Ok(self.owners.insert(Arc::as_ptr(owner).addr()))
    }

    fn first_declaration(
        &mut self,
        function: &'model Arc<CustomFunction>,
    ) -> Result<bool, ShapesProductError> {
        if self.finished() {
            return Ok(false);
        }
        self.functions
            .try_reserve(1)
            .map_err(|_| crate::PreparationRefusal::Storage(StorageError::AllocationFailed))?;
        Ok(self.functions.insert(Arc::as_ptr(function).addr()))
    }

    fn shape_index(&mut self, _: &'model ShapeIndexRef) -> Result<(), ShapesProductError> {
        Ok(())
    }

    fn shape(&mut self, shape: &'model Shape) -> Result<(), ShapesProductError> {
        if self.finished() {
            return Ok(());
        }
        if self.position == Position::Shape(std::ptr::from_ref(shape).addr()) {
            self.shape = Some(shape);
        } else {
            self.walk_shape(shape)?;
        }
        Ok(())
    }

    fn constraint(&mut self, constraint: &'model Constraint) -> Result<(), ShapesProductError> {
        if self.finished() {
            return Ok(());
        }
        if let Constraint::PropertyReference(edge) = constraint
            && self.position == Position::Property(std::ptr::from_ref(edge).addr())
        {
            self.constraint = Some(constraint);
        } else {
            self.walk_constraint(constraint)?;
        }
        Ok(())
    }
}

pub(crate) type ShapeOccurrence = SharedView<Arc<ModelOwner>, Shape>;
pub(crate) type PropertyOccurrence = SharedView<Arc<ModelOwner>, Constraint>;

#[derive(Default)]
pub(super) struct Index {
    owners: OrderedTable<usize, SharedView<Arc<ModelOwner>, Arc<ModelOwner>>>,
    shapes: OrderedTable<(usize, usize), ShapeOccurrence>,
    properties: OrderedTable<(usize, usize), PropertyOccurrence>,
    live: usize,
}

impl Index {
    fn owner(
        &mut self,
        owner: &Arc<ModelOwner>,
    ) -> Result<SharedView<Arc<ModelOwner>, Arc<ModelOwner>>, ShapesError> {
        let key = Arc::as_ptr(owner).addr();
        if let Some(found) = self.owners.get(&key) {
            return Ok(found.clone());
        }
        let shared = Shared::try_new(Arc::clone(owner))
            .map_err(|_| StorageError::AllocationFailed)?
            .into_view(|owner| owner);
        let mut resident = Resident;
        let mut memory = Memory::resume(&mut resident, self.live);
        let result = self.owners.try_insert_with(
            key,
            &mut memory,
            |a, b| Ok::<_, ShapesError>(a.cmp(b)),
            |_, _, _| Ok::<_, ShapesError>(shared.clone()),
        );
        self.live = memory.admitted_bytes();
        result?;
        Ok(shared)
    }
    pub(super) fn shape(
        &mut self,
        owner: &Arc<ModelOwner>,
        address: usize,
    ) -> Result<ShapeOccurrence, ShapesError> {
        let key = (Arc::as_ptr(owner).addr(), address);
        if let Some(found) = self.shapes.get(&key) {
            return Ok(found.clone());
        }
        let owner = self.owner(owner)?;
        let mut failure = None;
        let found = owner.try_project(|owner| match shape(owner, address) {
            Ok(found) => Some(found),
            Err(error) => {
                failure = Some(error);
                None
            }
        });
        if let Some(error) = failure {
            return Err(error);
        }
        let found =
            found.ok_or_else(|| ShapesError::internal("original shape projection disappeared"))?;
        let mut resident = Resident;
        let mut memory = Memory::resume(&mut resident, self.live);
        let result = self.shapes.try_insert_with(
            key,
            &mut memory,
            |a, b| Ok::<_, ShapesError>(a.cmp(b)),
            |_, _, _| Ok::<_, ShapesError>(found.clone()),
        );
        self.live = memory.admitted_bytes();
        result?;
        Ok(found)
    }
    pub(super) fn property(
        &mut self,
        owner: &Arc<ModelOwner>,
        address: usize,
    ) -> Result<PropertyOccurrence, ShapesError> {
        let key = (Arc::as_ptr(owner).addr(), address);
        if let Some(found) = self.properties.get(&key) {
            return Ok(found.clone());
        }
        let owner = self.owner(owner)?;
        let mut failure = None;
        let found = owner.try_project(|owner| match property(owner, address) {
            Ok(found) => Some(found),
            Err(error) => {
                failure = Some(error);
                None
            }
        });
        if let Some(error) = failure {
            return Err(error);
        }
        let found = found
            .ok_or_else(|| ShapesError::internal("original property projection disappeared"))?;
        let mut resident = Resident;
        let mut memory = Memory::resume(&mut resident, self.live);
        let result = self.properties.try_insert_with(
            key,
            &mut memory,
            |a, b| Ok::<_, ShapesError>(a.cmp(b)),
            |_, _, _| Ok::<_, ShapesError>(found.clone()),
        );
        self.live = memory.admitted_bytes();
        result?;
        Ok(found)
    }
}
