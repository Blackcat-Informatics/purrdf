// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed pattern declarations through the model's existing traversal home.

use std::sync::Arc;

use purrdf_core::FastSet;
use purrdf_core::xsd_regex::xpath::{Error, Resource};

use crate::expression::CustomFunction;
use crate::shapes::link::{ModelWalk, ShapeIndexRef};
use crate::shapes::{Constraint, Shapes};

use super::Cause;

#[derive(Clone, Copy, Debug)]
pub(super) struct Declaration<'model> {
    pub(super) cell: usize,
    pub(super) pattern: &'model str,
    pub(super) flags: &'model str,
}

#[derive(Default)]
struct Inventory<'model> {
    depth: u32,
    seen: FastSet<(u8, usize)>,
    declarations: Vec<Declaration<'model>>,
}

impl Inventory<'_> {
    fn first(&mut self, kind: u8, address: usize) -> Result<bool, Cause> {
        let key = (kind, address);
        if self.seen.contains(&key) {
            return Ok(false);
        }
        self.seen.try_reserve(1).map_err(|_| allocation(1))?;
        Ok(self.seen.insert(key))
    }
}

impl<'model> ModelWalk<'model, Cause> for Inventory<'model> {
    fn first_owner(
        &mut self,
        owner: &'model Arc<crate::shapes::ModelOwner>,
    ) -> Result<bool, Cause> {
        self.first(3, Arc::as_ptr(owner).addr())
    }
    fn depth(&mut self) -> &mut u32 {
        &mut self.depth
    }

    fn first_declaration(&mut self, func: &'model Arc<CustomFunction>) -> Result<bool, Cause> {
        self.first(0, Arc::as_ptr(func).addr())
    }

    fn shape_index(&mut self, _index: &'model ShapeIndexRef) -> Result<(), Cause> {
        // ModelWalk::model visits the retained definitions through their owner.
        // Following a weak backreference cannot provide model-lived borrows.
        Ok(())
    }

    fn pattern(&mut self, declaration: &'model Constraint) -> Result<(), Cause> {
        let Constraint::Pattern {
            regex,
            flags,
            compiled,
        } = declaration
        else {
            unreachable!("the model traversal invokes this hook only for patterns")
        };
        self.declarations
            .try_reserve(1)
            .map_err(|_| allocation(1))?;
        self.declarations.push(Declaration {
            cell: Arc::as_ptr(compiled).addr(),
            pattern: regex,
            flags: flags.as_deref().unwrap_or_default(),
        });
        Ok(())
    }
}

fn allocation(units: u64) -> Cause {
    Cause::Pattern(Error::Allocation {
        resource: Resource::CompileSlots,
        units,
    })
}

pub(super) fn collect(shapes: &Shapes) -> Result<Vec<Declaration<'_>>, Cause> {
    let mut inventory = Inventory::default();
    for shape in &shapes.node_shapes {
        inventory.shape(shape)?;
    }
    for rule in &shapes.rules.global_rules {
        inventory.rule(rule)?;
    }
    inventory.declarations.sort_unstable_by(|left, right| {
        left.pattern
            .cmp(right.pattern)
            .then_with(|| left.flags.cmp(right.flags))
    });
    Ok(inventory.declarations)
}
