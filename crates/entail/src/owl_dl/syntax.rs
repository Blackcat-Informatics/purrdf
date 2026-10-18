// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Reads made by the native syntax owners, shared across memoized expressions.

use purrdf_lex::allocation::{Memory, OrderedTable, Resident, StorageError};
#[cfg(test)]
mod tests;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Space {
    #[default]
    Source,
    Query,
}

/// A completed native read with either no auxiliary syntax or one shared root.
/// Absence of this record means capture is missing, never a completed empty read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Capture {
    Empty,
    Root(usize),
}

impl Capture {
    #[cfg(test)]
    pub(crate) fn root(self) -> Option<usize> {
        match self {
            Self::Empty => None,
            Self::Root(root) => Some(root),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Kind {
    Recognition(u32),
    Expression(u32),
    Range(u32),
    Role(u32),
    List(u32),
    Group,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Read {
    Triple(u32, u32, u32),
    Child(usize),
}

pub(crate) struct Node {
    pub(crate) kind: Kind,
    pub(crate) reads: Vec<Read>,
    pub(crate) complete: bool,
}

impl std::fmt::Debug for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("kind", &self.kind)
            .field("reads", &self.reads)
            .field("complete", &self.complete)
            .finish()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Owner {
    #[default]
    Detached,
    Frame,
    Node(usize),
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Token {
    previous: Owner,
    node: Option<usize>,
}

#[derive(Default)]
pub(crate) struct Syntax {
    pub(crate) space: Space,
    roots: OrderedTable<Kind, usize>,
    pub(crate) nodes: Vec<Node>,
    tbox: Vec<Option<Capture>>,
    frame: Vec<Read>,
    current: Owner,
    incomplete: usize,
    storage: Resident,
    live: usize,
    pub(crate) refusal: Option<StorageError>,
    #[cfg(test)]
    pub(crate) injected_refusal: Option<StorageError>,
}

impl std::fmt::Debug for Syntax {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut value = f.debug_struct("Syntax");
        value
            .field("space", &self.space)
            .field("root_count", &self.roots.len())
            .field("nodes", &self.nodes)
            .field("tbox", &self.tbox)
            .field("frame", &self.frame)
            .field("current", &self.current)
            .field("incomplete", &self.incomplete)
            .field("storage", &self.storage)
            .field("live", &self.live)
            .field("refusal", &self.refusal);
        #[cfg(test)]
        value.field("injected_refusal", &self.injected_refusal);
        value.finish()
    }
}

impl Syntax {
    /// Traverse only retained native dependencies, including finite expression cycles.
    #[cfg(test)]
    pub(crate) fn selected(
        &self,
        roots: &[usize],
        stop: Option<&dyn purrdf_datalog::StopSignal>,
    ) -> Result<Vec<usize>, SelectionError> {
        if let Some(original) = self.refusal {
            return Err(SelectionError::Storage(original));
        }
        let mut storage = Resident;
        let mut memory = Memory::new(&mut storage);
        let mut pending = Vec::new();
        let mut selected = Vec::new();
        let mut visited = OrderedTable::default();
        memory.extend(&mut pending, roots.iter().copied())?;
        while let Some(id) = pending.pop() {
            if stop.is_some_and(purrdf_datalog::StopSignal::stopped) {
                return Err(SelectionError::Stopped);
            }
            if visited.insert(id, (), &mut memory)?.is_some() {
                continue;
            }
            let node = self
                .nodes
                .get(id)
                .filter(|node| node.complete)
                .ok_or(SelectionError::Missing(id))?;
            memory.push(&mut selected, id)?;
            for read in &node.reads {
                if let Read::Child(child) = read {
                    memory.push(&mut pending, *child)?;
                }
            }
        }
        Ok(selected)
    }
    pub(crate) fn query() -> Self {
        Self {
            space: Space::Query,
            ..Self::default()
        }
    }

    pub(crate) fn root(&self, kind: Kind) -> Option<usize> {
        self.roots.get(&kind).copied()
    }

    pub(crate) fn complete(&self) -> bool {
        self.current == Owner::Detached && self.incomplete == 0
    }

    fn completed(&mut self, id: usize) {
        if !self.nodes[id].complete {
            self.nodes[id].complete = true;
            self.incomplete -= 1;
        }
    }

    fn publish<T>(
        &mut self,
        body: impl FnOnce(&mut Self, &mut Memory<'_, Resident>) -> Result<T, StorageError>,
    ) -> Result<T, StorageError> {
        if let Some(original) = self.refusal {
            return Err(original);
        }
        #[cfg(test)]
        if let Some(original) = self.injected_refusal.take() {
            self.refusal = Some(original);
            return Err(original);
        }
        // The resident has no external callback; keep its owned buffers and live
        // grant together even when publication fails part way through a node.
        let mut storage = std::mem::take(&mut self.storage);
        let mut memory = Memory::resume(&mut storage, self.live);
        let result = body(self, &mut memory);
        self.live = memory.admitted_bytes();
        self.storage = storage;
        if let Err(original) = &result {
            self.refusal = Some(*original);
        }
        result
    }

    fn ensure(
        &mut self,
        kind: Kind,
        memory: &mut Memory<'_, Resident>,
    ) -> Result<usize, StorageError> {
        if let Some(root) = self.root(kind) {
            return Ok(root);
        }
        let id = self.nodes.len();
        memory.push(
            &mut self.nodes,
            Node {
                kind,
                reads: Vec::new(),
                complete: false,
            },
        )?;
        self.roots.insert(kind, id, memory)?;
        self.incomplete += 1;
        Ok(id)
    }

    fn append(
        &mut self,
        owner: Owner,
        read: Read,
        memory: &mut Memory<'_, Resident>,
    ) -> Result<(), StorageError> {
        let reads = match owner {
            Owner::Detached => return Ok(()),
            Owner::Frame => &mut self.frame,
            Owner::Node(id) => {
                if self.nodes[id].complete {
                    return Ok(());
                }
                &mut self.nodes[id].reads
            }
        };
        if reads.last() != Some(&read) {
            memory.push(reads, read)?;
        }
        Ok(())
    }

    pub(crate) fn enter(&mut self, kind: Kind) -> Token {
        let previous = self.current;
        let result = self.publish(|syntax, memory| {
            let node = syntax.ensure(kind, memory)?;
            syntax.append(previous, Read::Child(node), memory)?;
            Ok(node)
        });
        let node = result.ok();
        self.current = node.map_or(Owner::Detached, Owner::Node);
        Token { previous, node }
    }

    pub(crate) fn leave(&mut self, token: Token, complete: bool) {
        if let Some(node) = token.node
            && complete
        {
            self.completed(node);
        }
        self.current = token.previous;
    }

    pub(crate) fn triple(&mut self, s: u32, p: u32, o: u32) {
        let owner = self.current;
        let _ = self.publish(|syntax, memory| syntax.append(owner, Read::Triple(s, p, o), memory));
    }

    /// Cells share the original strict walk's two selected objects. No list is
    /// walked again to obtain its provenance.
    pub(crate) fn list_read(
        &mut self,
        cell: u32,
        predicate: u32,
        object: u32,
        rest: u32,
        nil: u32,
    ) {
        let _ = self.publish(|syntax, memory| {
            let id = syntax.ensure(Kind::List(cell), memory)?;
            syntax.append(
                Owner::Node(id),
                Read::Triple(cell, predicate, object),
                memory,
            )?;
            if predicate == rest {
                if object != nil {
                    let next = syntax.ensure(Kind::List(object), memory)?;
                    syntax.append(Owner::Node(id), Read::Child(next), memory)?;
                }
                syntax.completed(id);
            }
            Ok(())
        });
    }

    pub(crate) fn begin_axiom(&mut self) {
        assert_eq!(
            self.current,
            Owner::Detached,
            "one native axiom capture at a time"
        );
        self.frame.clear();
        self.current = Owner::Frame;
    }

    pub(crate) fn end_axiom(&mut self) -> Result<Option<usize>, StorageError> {
        self.current = Owner::Detached;
        self.publish(|syntax, memory| {
            let root = match syntax.frame.as_slice() {
                [] => None,
                [Read::Child(root)] => Some(*root),
                _ => {
                    let root = syntax.nodes.len();
                    let reads = std::mem::take(&mut syntax.frame);
                    memory.push(
                        &mut syntax.nodes,
                        Node {
                            kind: Kind::Group,
                            reads,
                            complete: true,
                        },
                    )?;
                    Some(root)
                }
            };
            syntax.frame.clear();
            Ok(root)
        })
    }

    pub(crate) fn bind_tbox(
        &mut self,
        start: usize,
        end: usize,
        root: Option<usize>,
    ) -> Result<(), StorageError> {
        if start == end {
            return Ok(());
        }
        for index in start..end {
            assert!(
                self.tbox_root(index).is_none(),
                "one syntax capture per normalized input inclusion"
            );
        }
        self.publish(|syntax, memory| {
            if end > syntax.tbox.capacity() {
                let capacity = purrdf_lex::allocation::growth_capacity::<Option<Capture>>(
                    syntax.tbox.capacity(),
                    end,
                )?;
                memory.reserve(&mut syntax.tbox, capacity)?;
            }
            syntax.tbox.resize(end.max(syntax.tbox.len()), None);
            syntax.tbox[start..end].fill(Some(root.map_or(Capture::Empty, Capture::Root)));
            Ok(())
        })
    }

    pub(crate) fn tbox_root(&self, index: usize) -> Option<Capture> {
        self.tbox.get(index).copied().flatten()
    }
}

#[cfg(test)]
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SelectionError {
    Storage(StorageError),
    Stopped,
    Missing(usize),
}

#[cfg(test)]
purrdf_lex::variant_from!(SelectionError { Storage(StorageError) });
