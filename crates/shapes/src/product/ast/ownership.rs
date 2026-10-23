// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Admit strong ownership before filling any decoded function or index cell.
//! Logical calls and computed references are non-owning and add no graph edge.

use super::{
    Arc, BTreeMap, CustomFunction, ModelOwner, ModelWalk, NodeExpr, Shape, ShapeIndex,
    ShapeIndexRef, ShapesProductError, malformed,
};
use purrdf_core::graph::{GraphError, tarjan_scc_with_memory};
use purrdf_lex::allocation::{Memory, Resident};

pub(super) struct Ownership {
    functions: BTreeMap<usize, usize>,
    indexes: BTreeMap<usize, usize>,
    adjacency: Vec<Vec<usize>>,
    parent: usize,
    depth: u32,
}

impl Ownership {
    pub(super) fn new(functions: &[Arc<CustomFunction>], indexes: &[ShapeIndex]) -> Self {
        Self {
            functions: functions
                .iter()
                .enumerate()
                .map(|(id, value)| (Arc::as_ptr(value).addr(), id))
                .collect(),
            indexes: indexes
                .iter()
                .enumerate()
                .map(|(id, value)| (Arc::as_ptr(value).addr(), functions.len() + id))
                .collect(),
            adjacency: vec![Vec::new(); functions.len() + indexes.len()],
            parent: 0,
            depth: 0,
        }
    }

    pub(super) fn body(
        &mut self,
        function: &Arc<CustomFunction>,
        body: &NodeExpr,
    ) -> Result<(), ShapesProductError> {
        self.parent = *self
            .functions
            .get(&Arc::as_ptr(function).addr())
            .ok_or_else(|| malformed("unregistered function owner"))?;
        self.node_expr(body)
    }

    pub(super) fn definitions<'a>(
        &mut self,
        index: &ShapeIndex,
        definitions: impl IntoIterator<Item = &'a Shape>,
    ) -> Result<(), ShapesProductError> {
        self.parent = *self
            .indexes
            .get(&Arc::as_ptr(index).addr())
            .ok_or_else(|| malformed("unregistered definition owner"))?;
        for definition in definitions {
            self.shape(definition)?;
        }
        Ok(())
    }

    pub(super) fn finish(mut self) -> Result<(), ShapesProductError> {
        for row in &mut self.adjacency {
            row.sort_unstable();
            row.dedup();
        }
        let components = tarjan_scc_with_memory(&self.adjacency, &mut Memory::new(&mut Resident))
            .map_err(|error| match error {
            GraphError::Storage(error) => {
                ShapesProductError::from(crate::PreparationRefusal::Storage(error))
            }
            GraphError::InvalidEdge { from, to } => {
                malformed(format!("invalid strong-ownership edge {from} to {to}"))
            }
        })?;
        if components.iter().any(|component| {
            component.len() > 1
                || component
                    .first()
                    .is_some_and(|&node| self.adjacency[node].contains(&node))
        }) {
            return Err(malformed(
                "retained function bodies and definitions form a strong ownership cycle; use logical function and shape references",
            ));
        }
        Ok(())
    }
}

impl ModelWalk<'_> for Ownership {
    fn depth(&mut self) -> &mut u32 {
        &mut self.depth
    }

    fn first_owner(&mut self, owner: &Arc<ModelOwner>) -> Result<bool, ShapesProductError> {
        let index = *self
            .indexes
            .get(&Arc::as_ptr(&owner.index).addr())
            .ok_or_else(|| malformed("unregistered retained definition domain"))?;
        self.adjacency[self.parent].push(index);
        for function in &owner.functions {
            let id = *self
                .functions
                .get(&Arc::as_ptr(function).addr())
                .ok_or_else(|| malformed("unregistered retained declaration"))?;
            self.adjacency[self.parent].push(id);
        }
        // Headers are inventoried separately. Following their cells here would
        // confuse semantic recursion with the strong edges being checked.
        Ok(false)
    }

    fn first_declaration(
        &mut self,
        _function: &Arc<CustomFunction>,
    ) -> Result<bool, ShapesProductError> {
        Ok(false)
    }

    fn shape_index(&mut self, _index: &ShapeIndexRef) -> Result<(), ShapesProductError> {
        Ok(())
    }
}
