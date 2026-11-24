// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use crate::FastMap;
use crate::v2::fields::push;
use crate::v2::{FoldError, RdfFold, Term, ValueName};

use super::{DefinitionKey, ReferenceForm, RowScope, SegmentDictionary};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Shape<'a> {
    Triple([u64; 3]),
    Literal(&'a [u8], u64),
    Reference(ValueName, u64),
}

impl Shape<'_> {
    fn map_ids(self, map: impl Fn(u64) -> u64) -> Self {
        match self {
            Self::Triple(ids) => Self::Triple(ids.map(map)),
            Self::Literal(text, datatype) => Self::Literal(text, map(datatype)),
            Self::Reference(value, datatype) => Self::Reference(value, map(datatype)),
        }
    }

    const fn dependencies(self) -> [Option<u64>; 3] {
        match self {
            Self::Triple([s, p, o]) => [Some(s), Some(p), Some(o)],
            Self::Literal(_, datatype) | Self::Reference(_, datatype) => {
                [Some(datatype), None, None]
            }
        }
    }
}

struct Definition<'a> {
    id: u64,
    shape: Shape<'a>,
    key: Option<Shape<'a>>,
    parents: Vec<usize>,
    root: usize,
    members: usize,
}

/// Reconsider written shapes only when a definition can name a later alias.
/// A merge revisits the shapes that depend on the smaller class. Shapes
/// sharing an unresolved ID compare by that ID until a definition is in scope.
pub(super) struct Aliases<'a> {
    definitions: Vec<Definition<'a>>,
    index: FastMap<u64, usize>,
}

impl<'a> Aliases<'a> {
    pub(super) fn new(
        dictionary: &'a SegmentDictionary,
        fold: &'a RdfFold,
        scope: RowScope,
    ) -> Result<Self, FoldError> {
        let normalize = |id| dictionary.scoped_definition_id(id, scope);
        let mut definitions = Vec::new();
        for (key, &id) in &dictionary.definitions {
            if !scope.defines(id) {
                continue;
            }
            let shape = match key {
                DefinitionKey::Triple(ids) => Shape::Triple(*ids),
                DefinitionKey::UnreadableLiteral(text, ReferenceForm::Typed(datatype)) => {
                    Shape::Literal(text, *datatype)
                }
                DefinitionKey::Atom(2, handle) => {
                    let Some(Term::Typed { lexical, datatype }) = fold.term(*handle) else {
                        continue;
                    };
                    let datatype = dictionary
                        .definitions
                        .get(&DefinitionKey::Atom(1, *datatype))
                        .ok_or(FoldError::Definitions)?;
                    Shape::Literal(lexical.as_bytes(), *datatype)
                }
                _ => continue,
            };
            Self::record(&mut definitions, id, shape.map_ids(normalize))?;
        }
        for (&(value, form), &id) in &dictionary.references {
            if let ReferenceForm::Typed(datatype) = form
                && scope.defines(id)
            {
                Self::record(
                    &mut definitions,
                    id,
                    Shape::Reference(value, normalize(datatype)),
                )?;
            }
        }
        let mut index = FastMap::default();
        index.try_reserve(definitions.len())?;
        for (at, definition) in definitions.iter().enumerate() {
            index.insert(definition.id, at);
        }
        for at in 0..definitions.len() {
            for id in definitions[at].shape.dependencies().into_iter().flatten() {
                if let Some(&child) = index.get(&id) {
                    push(&mut definitions[child].parents, at)?;
                }
            }
        }
        let mut aliases = Self { definitions, index };
        aliases.settle()?;
        Ok(aliases)
    }

    fn record(
        definitions: &mut Vec<Definition<'a>>,
        id: u64,
        shape: Shape<'a>,
    ) -> Result<(), FoldError> {
        let root = definitions.len();
        push(
            definitions,
            Definition {
                id,
                shape,
                key: None,
                parents: Vec::new(),
                root,
                members: 1,
            },
        )?;
        Ok(())
    }

    pub(super) fn representatives(self) -> Result<FastMap<u64, u64>, FoldError> {
        let mut representatives = FastMap::default();
        for definition in &self.definitions {
            let root = self.representative(definition.id);
            if root != definition.id {
                representatives.try_reserve(1)?;
                representatives.insert(definition.id, root);
            }
        }
        Ok(representatives)
    }

    fn representative(&self, id: u64) -> u64 {
        self.index
            .get(&id)
            .map_or(id, |&at| self.definitions[self.root(at)].id)
    }

    fn root(&self, mut at: usize) -> usize {
        while self.definitions[at].root != at {
            at = self.definitions[at].root;
        }
        at
    }

    fn settle(&mut self) -> Result<(), FoldError> {
        let len = self.definitions.len();
        let mut pending = Vec::new();
        pending.try_reserve_exact(len)?;
        pending.extend((0..len).rev());
        let mut keys = FastMap::default();
        keys.try_reserve(len)?;
        while let Some(at) = pending.pop() {
            let key = self.definitions[at]
                .shape
                .map_ids(|id| self.representative(id));
            if let Some(old) = self.definitions[at].key.replace(key)
                && keys.get(&old) == Some(&at)
            {
                keys.remove(&old);
            }
            let &mut other = keys.entry(key).or_insert(at);
            let mut left = self.root(at);
            let mut right = self.root(other);
            if left == right {
                continue;
            }
            if self.definitions[left].members < self.definitions[right].members {
                core::mem::swap(&mut left, &mut right);
            }
            let mut parents = core::mem::take(&mut self.definitions[right].parents);
            pending.try_reserve(parents.len())?;
            pending.extend(parents.iter().copied());
            let members = self.definitions[right].members;
            self.definitions[left].parents.try_reserve(parents.len())?;
            self.definitions[left].parents.append(&mut parents);
            self.definitions[left].members += members;
            self.definitions[right].root = left;
        }
        Ok(())
    }
}
