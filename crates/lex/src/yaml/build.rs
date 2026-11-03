// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Products of the one YAML grammar, with eager Core-schema callbacks.

use super::admission::Allocations;
use super::read::{ErrorKind, Limits, Tag};
use super::resolve::{Plain, resolve};
use crate::allocation::{OrderedTable, StorageError};
use crate::json::{Number, Object, Value};
use core::mem;

pub(super) type Refusal = (ErrorKind, usize);

#[derive(Clone, Copy, Default)]
pub(super) struct PropertySpans {
    pub(super) anchor: Option<super::syntax::Span>,
    pub(super) tag: Option<super::syntax::Span>,
}
pub(super) trait Builder {
    const LEXICAL: bool = false;
    fn properties(&mut self, _: usize, _: bool, _: PropertySpans) -> Result<(), Refusal> {
        Ok(())
    }
    fn comment(&mut self, _: super::syntax::Span, _: bool) -> Result<(), Refusal> {
        Ok(())
    }
    fn directive(&mut self, _: super::syntax::Span) -> Result<(), Refusal> {
        Ok(())
    }
    fn escape(&mut self, _: super::syntax::SyntaxEscape) -> Result<(), Refusal> {
        Ok(())
    }
    fn question(&mut self, _: super::syntax::Span) -> Result<(), Refusal> {
        Ok(())
    }
    fn block(&mut self, _: usize, _: super::syntax::BlockHeader) -> Result<(), Refusal> {
        Ok(())
    }
    fn decorate(&mut self, _: &Self::Node, _: PropertySpans) -> Result<(), Refusal> {
        Ok(())
    }
    fn allocation_limit(&self) -> Option<usize>;
    fn allocations(&mut self) -> &mut Allocations;
    fn discard_text(&mut self, text: String) -> Result<(), Refusal>;
    type Node;
    type Key;
    fn copy_node(&mut self, node: &Self::Node, at: usize) -> Result<Self::Node, Refusal>;
    fn copy_key(&mut self, key: &Self::Key, at: usize) -> Result<Self::Key, Refusal>;
    fn discard_node(&mut self, node: Self::Node) -> Result<(), Refusal>;
    type Mapping;
    fn scalar(
        &mut self,
        text: String,
        plain: bool,
        tag: Option<(Tag, usize)>,
        at: usize,
        end: usize,
    ) -> Result<Self::Node, Refusal>;
    fn key_text(
        &mut self,
        text: String,
        plain: bool,
        tag: Option<(Tag, usize)>,
        at: usize,
        end: usize,
    ) -> Result<Self::Key, Refusal>;
    fn key_node(
        &mut self,
        key: &Self::Key,
        text: String,
        plain: bool,
        tag: Option<(Tag, usize)>,
        at: usize,
    ) -> Result<Self::Node, Refusal>;
    fn key_value(&mut self, key: Self::Key) -> Self::Node;
    fn key_of(
        &mut self,
        value: Self::Node,
        scalar_keys: bool,
        at: usize,
    ) -> Result<Option<Self::Key>, Refusal>;
    fn null(&mut self, at: usize) -> Result<Self::Node, Refusal>;
    fn sequence(
        &mut self,
        items: Vec<Self::Node>,
        at: usize,
        end: usize,
        flow: bool,
    ) -> Result<Self::Node, Refusal>;
    fn mapping(
        &mut self,
        map: Self::Mapping,
        at: usize,
        end: usize,
        flow: bool,
    ) -> Result<Self::Node, Refusal>;
    fn new_map(&mut self) -> Self::Mapping;
    fn insert(
        &mut self,
        map: &mut Self::Mapping,
        key: Self::Key,
        value: Self::Node,
        at: usize,
    ) -> Result<(), Refusal>;
    fn collection_tag(&self, tag: Option<(Tag, usize)>, kind: Tag) -> Result<(), Refusal>;
    fn size(&self, node: &Self::Node) -> u64;
    fn alias(
        &mut self,
        node: &Self::Node,
        name: &str,
        at: usize,
        end: usize,
    ) -> Result<Self::Node, Refusal>;
    fn alias_cost(&self, size: u64) -> u64;
}

pub(super) struct CoreBuilder {
    pub(super) limits: Limits,
    pub(super) allocations: Allocations,
}

/// One owned Core product. Payload bytes follow the value through parser
/// frames, anchor storage and native clones; inline parser storage is separate.
pub(super) struct CoreNode {
    pub(super) value: Value,
    owned_bytes: usize,
    semantic_size: u64,
}
pub(super) struct CoreMap {
    members: Vec<(String, CoreNode)>,
    keys: OrderedTable<String, ()>,
}

impl CoreBuilder {
    fn storage(&self, cause: StorageError, at: usize) -> Refusal {
        (
            match (cause, self.allocations.limit()) {
                (StorageError::AdmissionFailed, Some(limit)) => ErrorKind::Allocation { limit },
                _ => ErrorKind::Storage(cause),
            },
            at,
        )
    }
    fn leaf(value: Value, owned_bytes: usize) -> CoreNode {
        CoreNode {
            value,
            owned_bytes,
            semantic_size: 1,
        }
    }
    fn string(text: String) -> CoreNode {
        let bytes = text.capacity();
        Self::leaf(Value::String(text), bytes)
    }
    fn plain_value(&mut self, text: String, at: usize) -> Result<CoreNode, Refusal> {
        let node = match resolve(&text) {
            Plain::Null => Self::leaf(Value::Null, 0),
            Plain::Bool(value) => Self::leaf(Value::Bool(value), 0),
            Plain::Number(plan) => {
                let lexeme = self
                    .allocations
                    .format(&plan)
                    .map_err(|cause| self.storage(cause, at))?;
                let bytes = lexeme.capacity();
                let number = Number::from_lexeme(lexeme)
                    .map_err(|_| (ErrorKind::Syntax("a JSON-compatible number"), at))?;
                Self::leaf(Value::Number(number), bytes)
            }
            Plain::NonFinite => return Err((ErrorKind::NonFinite, at)),
            Plain::String => return Ok(Self::string(text)),
        };
        self.discard_text(text)?;
        Ok(node)
    }
    fn scalar(
        &mut self,
        text: String,
        plain: bool,
        tag: Option<(Tag, usize)>,
        at: usize,
    ) -> Result<CoreNode, Refusal> {
        let Some((tag, tag_at)) = tag else {
            return if plain {
                self.plain_value(text, at)
            } else {
                Ok(Self::string(text))
            };
        };
        match tag {
            Tag::Str | Tag::NonSpecific => return Ok(Self::string(text)),
            Tag::Seq | Tag::Map | Tag::Other => return Err((ErrorKind::Tag, tag_at)),
            Tag::Int | Tag::Float | Tag::Bool | Tag::Null => {}
        }
        let node = self.plain_value(text, at)?;
        let matches = match (tag, &node.value) {
            (Tag::Int, Value::Number(number)) => number.is_integer(),
            (Tag::Float, Value::Number(_))
            | (Tag::Bool, Value::Bool(_))
            | (Tag::Null, Value::Null) => true,
            _ => false,
        };
        if matches {
            Ok(node)
        } else {
            Err((ErrorKind::Tag, tag_at))
        }
    }
}

impl Builder for CoreBuilder {
    type Node = CoreNode;
    type Key = String;
    type Mapping = CoreMap;
    fn copy_node(&mut self, node: &CoreNode, at: usize) -> Result<CoreNode, Refusal> {
        if self.allocations.limit().is_none() {
            return Ok(CoreNode {
                value: node.value.clone(),
                owned_bytes: node.owned_bytes,
                semantic_size: node.semantic_size,
            });
        }
        let before = self.allocations.live();
        let value = self
            .allocations
            .memory(|memory| node.value.clone_with_memory(memory))
            .map_err(|cause| self.storage(cause, at))?;
        Ok(CoreNode {
            value,
            owned_bytes: self.allocations.live() - before,
            semantic_size: node.semantic_size,
        })
    }
    fn copy_key(&mut self, key: &String, at: usize) -> Result<String, Refusal> {
        self.allocations
            .string(key)
            .map_err(|cause| self.storage(cause, at))
    }
    fn discard_node(&mut self, node: CoreNode) -> Result<(), Refusal> {
        let bytes = node.owned_bytes;
        drop(node);
        self.allocations
            .release_bytes(bytes)
            .map_err(|cause| self.storage(cause, 0))
    }
    fn discard_text(&mut self, text: String) -> Result<(), Refusal> {
        self.allocations
            .discard_string(text)
            .map_err(|cause| self.storage(cause, 0))
    }
    fn allocation_limit(&self) -> Option<usize> {
        self.allocations.limit()
    }
    fn allocations(&mut self) -> &mut Allocations {
        &mut self.allocations
    }
    fn scalar(
        &mut self,
        text: String,
        plain: bool,
        tag: Option<(Tag, usize)>,
        at: usize,
        _: usize,
    ) -> Result<CoreNode, Refusal> {
        Self::scalar(self, text, plain, tag, at)
    }
    fn key_text(
        &mut self,
        text: String,
        plain: bool,
        tag: Option<(Tag, usize)>,
        at: usize,
        _: usize,
    ) -> Result<String, Refusal> {
        match tag {
            None if plain
                && !self.limits.scalar_keys
                && !matches!(resolve(&text), Plain::String) =>
            {
                Err((ErrorKind::NonStringKey, at))
            }
            None | Some((Tag::Str | Tag::NonSpecific, _)) => Ok(text),
            Some((tag @ (Tag::Int | Tag::Float | Tag::Bool | Tag::Null), tag_at))
                if self.limits.scalar_keys =>
            {
                let matches = match (tag, resolve(&text)) {
                    (Tag::Int, Plain::Number(plan)) => plan.is_integer(),
                    (Tag::Float, Plain::Number(_) | Plain::NonFinite)
                    | (Tag::Bool, Plain::Bool(_))
                    | (Tag::Null, Plain::Null) => true,
                    _ => false,
                };
                if matches {
                    Ok(text)
                } else {
                    Err((ErrorKind::Tag, tag_at))
                }
            }
            Some(_) => Err((ErrorKind::NonStringKey, at)),
        }
    }
    fn key_node(
        &mut self,
        _: &String,
        text: String,
        plain: bool,
        tag: Option<(Tag, usize)>,
        at: usize,
    ) -> Result<CoreNode, Refusal> {
        // The key callback already validated the tag. Only Core's deliberate
        // non-finite-key spelling needs a semantic fallback; storage failures
        // always propagate.
        if !self.limits.scalar_keys
            || (matches!(resolve(&text), Plain::NonFinite)
                && (plain && tag.is_none() || matches!(tag, Some((Tag::Float, _)))))
        {
            return Ok(Self::string(text));
        }
        Self::scalar(self, text, plain, tag, at)
    }
    fn key_value(&mut self, key: String) -> CoreNode {
        Self::string(key)
    }
    fn key_of(
        &mut self,
        mut node: CoreNode,
        scalar_keys: bool,
        at: usize,
    ) -> Result<Option<String>, Refusal> {
        if let Value::String(text) = &mut node.value {
            return Ok(Some(mem::take(text)));
        }
        let text = match &node.value {
            Value::Number(number) if scalar_keys => Some(
                self.allocations
                    .string(number.lexeme())
                    .map_err(|cause| self.storage(cause, at))?,
            ),
            Value::Bool(flag) if scalar_keys => Some(
                self.allocations
                    .string(if *flag { "true" } else { "false" })
                    .map_err(|cause| self.storage(cause, at))?,
            ),
            Value::Null if scalar_keys => Some(
                self.allocations
                    .string("null")
                    .map_err(|cause| self.storage(cause, at))?,
            ),
            _ => None,
        };
        self.discard_node(node)?;
        Ok(text)
    }
    fn null(&mut self, _: usize) -> Result<CoreNode, Refusal> {
        Ok(Self::leaf(Value::Null, 0))
    }
    fn sequence(
        &mut self,
        mut items: Vec<CoreNode>,
        at: usize,
        _: usize,
        _: bool,
    ) -> Result<CoreNode, Refusal> {
        let mut values = Vec::new();
        if self.allocations.limit().is_some() {
            self.allocations
                .memory(|memory| memory.reserve(&mut values, items.len()))
                .map_err(|cause| self.storage(cause, at))?;
        } else {
            values.reserve(items.len());
        }
        let (mut bytes, mut size) = (values.capacity() * size_of::<Value>(), 1_u64);
        #[expect(
            clippy::iter_with_drain,
            reason = "Retain the original staging allocation until discard_vec destroys it before refunding its admitted grant."
        )]
        for node in items.drain(..) {
            bytes = bytes
                .checked_add(node.owned_bytes)
                .ok_or((ErrorKind::Storage(StorageError::SizeOverflow), at))?;
            size = size.saturating_add(node.semantic_size);
            values.push(node.value);
        }
        self.allocations
            .discard_vec(items)
            .map_err(|cause| self.storage(cause, at))?;
        Ok(CoreNode {
            value: Value::Array(values),
            owned_bytes: bytes,
            semantic_size: size,
        })
    }
    fn mapping(
        &mut self,
        mut map: CoreMap,
        at: usize,
        _: usize,
        _: bool,
    ) -> Result<CoreNode, Refusal> {
        while let Some((name, ())) = map.keys.pop_first() {
            self.discard_text(name)?;
        }
        self.allocations
            .memory(|memory| map.keys.release(memory))
            .map_err(|cause| self.storage(cause, at))?;
        let mut members = Vec::new();
        if self.allocations.limit().is_some() {
            self.allocations
                .memory(|memory| memory.reserve(&mut members, map.members.len()))
                .map_err(|cause| self.storage(cause, at))?;
        } else {
            members.reserve(map.members.len());
        }
        let (mut bytes, mut size) = (members.capacity() * size_of::<(String, Value)>(), 1_u64);
        for (name, node) in map.members.drain(..) {
            bytes = bytes
                .checked_add(name.capacity())
                .and_then(|bytes| bytes.checked_add(node.owned_bytes))
                .ok_or((ErrorKind::Storage(StorageError::SizeOverflow), at))?;
            size = size.saturating_add(node.semantic_size);
            members.push((name, node.value));
        }
        self.allocations
            .discard_vec(map.members)
            .map_err(|cause| self.storage(cause, at))?;
        Ok(CoreNode {
            value: Value::Object(Object::from(members)),
            owned_bytes: bytes,
            semantic_size: size,
        })
    }
    fn new_map(&mut self) -> CoreMap {
        CoreMap {
            members: Vec::new(),
            keys: OrderedTable::default(),
        }
    }
    fn insert(
        &mut self,
        map: &mut CoreMap,
        key: String,
        value: CoreNode,
        at: usize,
    ) -> Result<(), Refusal> {
        if map.keys.get(&key).is_some() {
            return Err((ErrorKind::DuplicateKey, at));
        }
        let indexed = self
            .allocations
            .string(&key)
            .map_err(|cause| self.storage(cause, at))?;
        self.allocations
            .memory(|memory| map.keys.insert(indexed, (), memory))
            .map_err(|cause| self.storage(cause, at))?;
        self.allocations
            .push(&mut map.members, (key, value))
            .map_err(|cause| self.storage(cause, at))
    }
    fn collection_tag(&self, tag: Option<(Tag, usize)>, kind: Tag) -> Result<(), Refusal> {
        match tag {
            Some((tag, at)) if tag != kind && tag != Tag::NonSpecific => Err((ErrorKind::Tag, at)),
            _ => Ok(()),
        }
    }
    fn size(&self, node: &CoreNode) -> u64 {
        node.semantic_size
    }
    fn alias(
        &mut self,
        node: &CoreNode,
        _: &str,
        at: usize,
        _: usize,
    ) -> Result<CoreNode, Refusal> {
        self.copy_node(node, at)
    }
    fn alias_cost(&self, size: u64) -> u64 {
        size
    }
}
