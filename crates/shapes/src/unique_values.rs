// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `sh:uniqueValuesFor` (SHACL 1.2 Core §7.9.5): the one Core component whose
//! verdict for a value node depends on OTHER focus nodes.
//!
//! The textual definition, verbatim: "Let $targetNodes be the target nodes of S.
//! For each value node V for which there exists another node in $targetNodes that
//! has exactly the same values for all properties in $properties as V there is a
//! validation result. No result is produced if V has no values for any of the
//! properties in $properties." And the note beside it: "matching of literals needs
//! to be exact, e.g. "04"^^xsd:byte does not match "4"^^xsd:integer."
//!
//! # Binding time
//!
//! `$targetNodes` is a fact about the shape and the whole data graph, never about
//! the focus node being validated, so it is stage-1 work (see [`crate::plan`]):
//! the target set is resolved and grouped by value tuple ONCE per dataset binding,
//! on first use, and every value node is then answered by a hash lookup — linear
//! in the target set overall rather than quadratic in it.
//!
//! The grouping is built on first use rather than at bind for the reason the bind
//! is kept cheap at all: a binding whose shapes graph never reaches this component,
//! or a bounded request that never evaluates it, must not pay for enumerating a
//! target set. The validation entry points warm every grouping of the lowering
//! BEFORE they fan focus nodes out to workers ([`crate::plan::ShapePlan::warm_unique_values`]),
//! so the parallel path finds each one built and no two workers build the same one.
//!
//! # The bounded (change-path) requests
//!
//! `PreparedValidator::validate_focus_nodes` and `validate_focus_node_ids`
//! validate only the focus nodes a caller hands them, but "the target nodes of S"
//! is not the request: it is every target node of S in the bound data graph. A
//! focus node whose value tuple duplicates a target the caller did not name still
//! violates, so the grouping is always built from the shape's FULL target set.
//! That makes the first bounded request that reaches the component linear in the
//! target set; every later request against the same binding reuses the grouping
//! and pays only its lookups.
//!
//! # Exactness
//!
//! Values are compared as dataset identities. The interner is injective over RDF
//! terms, so two values collide exactly when they are the same RDF term — the
//! exact matching the note requires, with no value-space coercion.

use ::purrdf::{FastSet, IdSet, TermId};
use smallvec::SmallVec;

use crate::data::{GraphFilter, ShaclData, quads_for_pattern_ids};
use crate::data_view::ShaclRead;
use crate::plan::{ClassCatalog, DatasetBinding, TermSlot};
use crate::shapes::Target;

/// One `sh:uniqueValuesFor` constraint as the shapes-graph walk lowered it: what
/// its grouping is built from.
#[derive(Debug)]
pub(crate) struct UniqueSpec {
    /// The target declarations of the shape node that declares the constraint.
    pub(crate) targets: Box<[Target]>,
    /// The slot of each property in `$properties`, in the constraint's order.
    pub(crate) properties: Box<[TermSlot]>,
}

/// The values one node has for every property of `$properties`, as dataset
/// identities: each property's values sorted and deduplicated, and the length of
/// each property's run beside them so two tuples compare property by property.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ValueKey {
    lengths: SmallVec<[u32; 4]>,
    values: SmallVec<[TermId; 4]>,
}

/// Stage 1 of one `sh:uniqueValuesFor` constraint: its shape's target set in the
/// bound dataset, grouped by value tuple.
#[derive(Debug, Default)]
pub(crate) struct UniqueGroups {
    /// Each property's dataset identity; `None` for a property this data graph
    /// never interned, which no node has a value for.
    predicates: SmallVec<[Option<TermId>; 4]>,
    /// Every interned target node.
    members: IdSet,
    /// The target nodes that share their (non-empty) value tuple with another
    /// target node.
    colliding: IdSet,
    /// Every non-empty value tuple some target node has.
    keys: FastSet<ValueKey>,
}

impl UniqueGroups {
    /// Resolve `spec`'s target set against `data` and group it by value tuple.
    ///
    /// # Errors
    ///
    /// Returns an error when a target cannot be resolved (a SHACL-SPARQL target
    /// that fails to evaluate) or a slot the lowering never handed out is asked
    /// for.
    pub(crate) fn build(
        data: &ShaclData,
        spec: &UniqueSpec,
        binding: &DatasetBinding,
        classes: &ClassCatalog,
    ) -> Result<Self, String> {
        let predicates = spec
            .properties
            .iter()
            .map(|slot| binding.term(*slot))
            .collect::<Result<SmallVec<[Option<TermId>; 4]>, String>>()?;
        let mut groups = Self {
            predicates,
            ..Self::default()
        };
        // No target, or no property this data graph interns: no node has a value
        // tuple to share, so nothing can collide.
        if spec.targets.is_empty() || groups.predicates.iter().all(Option::is_none) {
            return Ok(groups);
        }
        let ds = data.core_view();
        let targets = crate::engine::resolve_focus_nodes(data, &spec.targets, binding, classes)?;
        // The first target seen with each tuple, and whether a second one has
        // been seen since — so the first is marked colliding exactly once.
        let mut first: ::purrdf::FastMap<ValueKey, (TermId, bool)> =
            ::purrdf::FastMap::with_capacity_and_hasher(
                targets.len(),
                ::purrdf::FastHasher::default(),
            );
        for focus in &targets {
            // A target the data graph does not intern is the subject of no
            // triple, so it has no values and can share no tuple.
            let Some(id) = focus.id() else {
                continue;
            };
            groups.members.insert(id);
            let Some(key) = value_key(ds, id, &groups.predicates) else {
                continue;
            };
            match first.entry(key) {
                std::collections::hash_map::Entry::Vacant(slot) => {
                    slot.insert((id, false));
                }
                std::collections::hash_map::Entry::Occupied(mut slot) => {
                    groups.colliding.insert(id);
                    let (earliest, marked) = slot.get_mut();
                    if !*marked {
                        groups.colliding.insert(*earliest);
                        *marked = true;
                    }
                }
            }
        }
        groups.keys = first.into_keys().collect();
        Ok(groups)
    }

    /// Whether the value node `value` has exactly the same (non-empty) values for
    /// every property as some OTHER target node.
    ///
    /// A target node is answered from the grouping alone, with no read and no
    /// allocation. A value node outside the target set (a property shape's value,
    /// or a shape reached through `sh:node`) has its tuple read once and looked
    /// up: every target it matches is "another node", since it is none of them.
    pub(crate) fn is_duplicated(&self, ds: &impl ShaclRead, value: TermId) -> bool {
        if self.members.contains(&value) {
            return self.colliding.contains(&value);
        }
        if self.keys.is_empty() {
            return false;
        }
        value_key(ds, value, &self.predicates).is_some_and(|key| self.keys.contains(&key))
    }
}

/// The value tuple of `node`, or `None` when it has no value for any property —
/// "No result is produced if V has no values for any of the properties".
fn value_key(ds: &impl ShaclRead, node: TermId, predicates: &[Option<TermId>]) -> Option<ValueKey> {
    let mut key = ValueKey {
        lengths: SmallVec::with_capacity(predicates.len()),
        values: SmallVec::new(),
    };
    for predicate in predicates {
        let mut run: SmallVec<[TermId; 4]> = match *predicate {
            Some(predicate) => quads_for_pattern_ids(
                ds,
                Some(node),
                Some(predicate),
                None,
                GraphFilter::DefaultGraph,
            )
            .map(|quad| quad.o)
            .collect(),
            None => SmallVec::new(),
        };
        run.sort_unstable_by_key(|id| id.index());
        run.dedup();
        // Property counts are bounded by the shapes graph and value runs by one
        // node's out-degree, both far below `u32::MAX`.
        key.lengths
            .push(u32::try_from(run.len()).unwrap_or(u32::MAX));
        key.values.extend(run);
    }
    (!key.values.is_empty()).then_some(key)
}
