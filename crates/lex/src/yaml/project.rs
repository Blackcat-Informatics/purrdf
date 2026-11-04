// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Consuming semantic projection of the admitted stream, using the Core builder.

use super::{
    admission::Allocations,
    build::{Builder, CoreBuilder, CoreNode, Refusal},
    read::{Error, ErrorKind, Limits, Tag},
    resolve::Plain,
    syntax::{ExplicitTag, NodeId, NodeKind, ScalarStyle, SyntaxNode, SyntaxStream},
};
use crate::{
    allocation::{OrderedTable, StorageError},
    json::Value,
};

/// Which document values are constructed after whole-stream validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyntaxSelection {
    /// Construct the first document; an absent stream produces null.
    First,
    /// Construct every document, in one array; an absent stream produces `[]`.
    All,
}

/// Document root forms admitted by a semantic consumer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyntaxRootPolicy {
    /// Every Core value is a possible root.
    Any,
    /// Each root must be a mapping, sequence or resolved null scalar.
    CollectionsOrNull,
}

/// Aggregate controls for consuming a stream into strict JSON-shaped values.
/// Mapping keys must resolve to strings. Legacy scalar-key controls are unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyntaxProjectionLimits {
    /// Additional live storage, including the retained source arena and metadata.
    pub max_allocation_bytes: usize,
    /// Expanded selected YAML nodes, including keys and alias copies.
    pub max_nodes: u64,
    /// Maximum selected collection depth after alias expansion.
    pub max_depth: usize,
    /// One monotone classification and numeric-conversion work allowance.
    pub max_numeric_work: u64,
    /// Newly materialized numeric spelling bytes; alias copies do not spend it.
    pub max_numeric_bytes: usize,
}
impl SyntaxProjectionLimits {
    /// 128 MiB live storage and the standard Core node, depth and numeric bounds.
    pub const DEFAULT: Self = Self {
        max_allocation_bytes: 128 << 20,
        max_nodes: Limits::DEFAULT.max_nodes,
        max_depth: Limits::DEFAULT.max_depth,
        max_numeric_work: Limits::DEFAULT.max_numeric_work,
        max_numeric_bytes: Limits::DEFAULT.max_numeric_bytes,
    };
}
impl Default for SyntaxProjectionLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

fn tag(node: &SyntaxNode) -> Option<(Tag, usize)> {
    node.explicit_tag.as_ref().map(|tag| {
        (
            match tag {
                ExplicitTag::NonSpecific => Tag::NonSpecific,
                ExplicitTag::Specific(name) => Tag::specific(name),
            },
            node.tag
                .expect("an explicit identity has an authored property")
                .start,
        )
    })
}
fn plain(style: ScalarStyle) -> bool {
    matches!(style, ScalarStyle::Plain | ScalarStyle::Empty)
}
fn target(nodes: &[SyntaxNode], mut id: NodeId) -> &SyntaxNode {
    loop {
        match nodes[id.0].kind {
            NodeKind::Alias { target, .. } => id = target,
            _ => return &nodes[id.0],
        }
    }
}
fn storage(error: StorageError, limit: usize, at: usize) -> Refusal {
    (
        if error == StorageError::AdmissionFailed {
            ErrorKind::Allocation { limit }
        } else {
            ErrorKind::Storage(error)
        },
        at,
    )
}

impl SyntaxStream<'_> {
    /// Validate every document's roots and resolved string keys, then construct
    /// only the selection through the existing Core builder. The exact retained
    /// account is resumed while source occurrences and projection products coexist.
    ///
    /// # Errors
    /// Returns an original positioned YAML or resource refusal, with no partial
    /// value. Unselected scalar values are not semantically constructed.
    pub fn into_core(
        self,
        selection: SyntaxSelection,
        roots: SyntaxRootPolicy,
        limits: SyntaxProjectionLimits,
    ) -> Result<Value, Error> {
        let source = self.source();
        let allocations =
            Allocations::resume(limits.max_allocation_bytes, self.arena.retained_bytes).map_err(
                |cause| Error::storage(source, Some(limits.max_allocation_bytes), cause, 0),
            )?;
        let core_limits = Limits {
            max_depth: limits.max_depth,
            max_nodes: limits.max_nodes,
            max_numeric_work: limits.max_numeric_work,
            max_numeric_bytes: limits.max_numeric_bytes,
            ..Limits::DEFAULT
        };
        let mut builder = CoreBuilder::new(core_limits, allocations);
        self.project(selection, roots, limits, &mut builder)
            .map_err(|(kind, at)| Error::new(source, kind, at))
    }

    fn project(
        self,
        selection: SyntaxSelection,
        roots: SyntaxRootPolicy,
        limits: SyntaxProjectionLimits,
        builder: &mut CoreBuilder,
    ) -> Result<Value, Refusal> {
        let limit = limits.max_allocation_bytes;
        let nodes = &self.arena.nodes;
        // These checks inspect the complete representation without constructing
        // scalar values. In particular, later numeric and non-finite values are
        // left untouched when only the first document is selected.
        for doc in &self.documents {
            let root = target(nodes, doc.root);
            if roots == SyntaxRootPolicy::CollectionsOrNull {
                let valid = match &root.kind {
                    NodeKind::Mapping { .. } | NodeKind::Sequence { .. } => true,
                    NodeKind::Scalar(s)
                        if matches!(tag(root), None | Some((Tag::Null, _)))
                            && (plain(s.style) || matches!(tag(root), Some((Tag::Null, _)))) =>
                    {
                        matches!(builder.classify(&s.text, root.span.start)?, Plain::Null)
                    }
                    _ => false,
                };
                if !valid {
                    return Err((ErrorKind::InvalidRoot, root.span.start));
                }
            }
        }
        for node in nodes {
            if let NodeKind::Mapping { entries, .. } = &node.kind {
                let mut keys = OrderedTable::default();
                for entry in entries {
                    let key = target(nodes, entry.key);
                    let at = nodes[entry.key.0].span.start;
                    let NodeKind::Scalar(scalar) = &key.kind else {
                        return Err((ErrorKind::NonStringKey, at));
                    };
                    builder.validate_key(&scalar.text, plain(scalar.style), tag(key), at)?;
                    if keys.get(&scalar.text.as_str()).is_some() {
                        return Err((ErrorKind::DuplicateKey, at));
                    }
                    builder
                        .allocations
                        .memory(|memory| keys.insert(scalar.text.as_str(), (), memory))
                        .map_err(|cause| storage(cause, limit, key.span.start))?;
                }
                builder
                    .allocations
                    .memory(|memory| keys.release(memory))
                    .map_err(|cause| storage(cause, limit, node.span.start))?;
            }
        }
        let documents = match selection {
            SyntaxSelection::First => &self.documents[..self.documents.len().min(1)],
            SyntaxSelection::All => &self.documents,
        };
        let count = documents.last().map_or(0, |doc| doc.root.0 + 1);
        let mut metrics = Vec::new();
        builder
            .allocations
            .memory(|memory| memory.reserve(&mut metrics, count))
            .map_err(|cause| storage(cause, limit, 0))?;
        for node in &nodes[..count] {
            let (mut size, mut height) = (1_u64, 0_usize);
            let mut overflow = false;
            let mut include = |id: NodeId| {
                let (child_size, child_height): (u64, usize) = metrics[id.0];
                size = size.checked_add(child_size).unwrap_or_else(|| {
                    overflow = true;
                    u64::MAX
                });
                height = height.max(child_height);
            };
            match &node.kind {
                NodeKind::Scalar(_) => {}
                NodeKind::Alias { target, .. } => {
                    (size, height) = metrics[target.0];
                }
                NodeKind::Sequence { items, .. } => {
                    for &id in items {
                        include(id);
                    }
                    height = height.saturating_add(1);
                }
                NodeKind::Mapping { entries, .. } => {
                    for entry in entries {
                        include(entry.key);
                        include(entry.value);
                    }
                    height = height.saturating_add(1);
                }
            }
            if overflow {
                return Err((
                    ErrorKind::Nodes {
                        limit: limits.max_nodes,
                    },
                    node.span.start,
                ));
            }
            metrics.push((size, height));
        }
        let mut total = 0_u64;
        for doc in documents {
            let (size, height) = metrics[doc.root.0];
            total = total.checked_add(size).ok_or((
                ErrorKind::Nodes {
                    limit: limits.max_nodes,
                },
                nodes[doc.root.0].span.start,
            ))?;
            if total > limits.max_nodes {
                return Err((
                    ErrorKind::Nodes {
                        limit: limits.max_nodes,
                    },
                    nodes[doc.root.0].span.start,
                ));
            }
            if height > limits.max_depth {
                return Err((
                    ErrorKind::Depth {
                        limit: limits.max_depth,
                    },
                    nodes[doc.root.0].span.start,
                ));
            }
        }
        builder
            .allocations
            .discard_vec(metrics)
            .map_err(|cause| storage(cause, limit, 0))?;
        let mut cache: Vec<Option<CoreNode>> = Vec::new();
        builder
            .allocations
            .memory(|memory| memory.reserve(&mut cache, count))
            .map_err(|cause| storage(cause, limit, 0))?;
        for node in &nodes[..count] {
            let at = node.span.start;
            let value = match &node.kind {
                NodeKind::Scalar(scalar) => {
                    let text = builder
                        .allocations
                        .string(&scalar.text)
                        .map_err(|cause| storage(cause, limit, at))?;
                    builder.scalar(text, plain(scalar.style), tag(node), at, scalar.raw.end)?
                }
                NodeKind::Alias { target, .. } => copy(builder, &cache, *target, at)?,
                NodeKind::Sequence { items, style } => {
                    builder.collection_tag(tag(node), Tag::Seq)?;
                    let mut children = Vec::new();
                    for &id in items {
                        let child = copy(builder, &cache, id, at)?;
                        builder
                            .allocations
                            .push(&mut children, child)
                            .map_err(|cause| storage(cause, limit, at))?;
                    }
                    builder.sequence(
                        children,
                        at,
                        node.span.end,
                        *style == super::syntax::CollectionStyle::Flow,
                    )?
                }
                NodeKind::Mapping { entries, style } => {
                    builder.collection_tag(tag(node), Tag::Map)?;
                    let mut map = builder.new_map();
                    for entry in entries {
                        let key = &cache[entry.key.0].as_ref().expect("a completed key").value;
                        let Value::String(text) = key else {
                            return Err((ErrorKind::NonStringKey, nodes[entry.key.0].span.start));
                        };
                        let key = builder.copy_key(text, at)?;
                        let value = copy(builder, &cache, entry.value, at)?;
                        builder.insert(&mut map, key, value, nodes[entry.key.0].span.start)?;
                    }
                    builder.mapping(
                        map,
                        at,
                        node.span.end,
                        *style == super::syntax::CollectionStyle::Flow,
                    )?
                }
            };
            cache.push(Some(value));
        }
        let mut output = Vec::new();
        for doc in documents {
            let value = cache[doc.root.0].take().expect("a constructed document");
            builder
                .allocations
                .push(&mut output, value)
                .map_err(|cause| storage(cause, limit, 0))?;
        }
        let value = match selection {
            SyntaxSelection::All => builder.sequence(output, 0, self.source().len(), false)?,
            SyntaxSelection::First => {
                let value = match output.pop() {
                    Some(value) => value,
                    None => builder.null(0)?,
                };
                builder
                    .allocations
                    .discard_vec(output)
                    .map_err(|cause| storage(cause, limit, 0))?;
                value
            }
        };
        for node in &mut cache {
            if let Some(node) = node.take() {
                builder.discard_node(node)?;
            }
        }
        builder
            .allocations
            .discard_vec(cache)
            .map_err(|cause| storage(cause, limit, 0))?;
        let retained = self.arena.retained_bytes;
        drop(self);
        builder
            .allocations
            .release_bytes(retained)
            .map_err(|cause| storage(cause, limit, 0))?;
        Ok(value.value)
    }
}

fn copy(
    builder: &mut CoreBuilder,
    cache: &[Option<CoreNode>],
    id: NodeId,
    at: usize,
) -> Result<CoreNode, Refusal> {
    builder.copy_node(
        cache[id.0]
            .as_ref()
            .expect("an earlier completed occurrence"),
        at,
    )
}
