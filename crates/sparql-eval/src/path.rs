// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! SPARQL property-path evaluation — the wasm-safe in-engine runtime.
//!
//! A property path constrains two endpoints by a *relation* between them rather
//! than a single triple. This module evaluates the `Path` graph pattern entirely in
//! interned [`TermId`](purrdf_core::TermId) space over the same indexed
//! [`DatasetView::quads_for_pattern`] surface the BGP hot path uses, returning a
//! [`SolutionSeq`] over the path's variable endpoint(s) that composes through the
//! existing join machinery unchanged.
//!
//! ## The program
//!
//! The path expression is compiled once per evaluation into a [`PathProgram`]: one
//! [`PathOp`] per node of the expression, numbered in pre-order, each naming its
//! children by index, with every predicate IRI and every negated set resolved to
//! dataset ids on the way. Whether a node admits the zero-length identity and whether
//! a repetition operator occurs under it are folded up beside each op, so the two
//! questions `eval_path` asks of the whole path are table reads.
//!
//! ## The reachability primitive
//!
//! Evaluation follows the SPARQL 1.1 §18.1.7 ALP (arbitrary-length-path) shape: a
//! single direction-parameterised primitive [`reach`], where
//! `reach(op, node, forward)` returns the set of nodes `y` such that `(node, y)` is in
//! the relation of the path node `op` (forward), or `(y, node)` is (backward). Every
//! operator is one kind of [`Frame`] of an explicit machine: a frame asks for the
//! sub-relations it composes through a work list of frames ([`Step::Call`]) and is
//! resumed with each answer, so a path nested to any depth is evaluated on the heap
//! and the machine stack is the same for every path.
//!
//! - `^p` (`Reverse`) flips the direction flag, and shares its inner op's memo entry.
//! - `p1/p2/…` (`Sequence`, one node however long the chain) steps a frontier through
//!   each element in turn, `reach(p2, ·)` over each `reach(p1, node)` and so on (in
//!   reverse element order under backward evaluation, so predecessors compose
//!   correctly).
//! - `p1|p2|…` (`Alternative`) unions every element's sub-relation, in source order.
//! - `p?` (`ZeroOrOne`) adds the zero-length identity `{node}`.
//! - `p*`/`p+` (`ZeroOrMore`/`OneOrMore`) take the transitive closure with a
//!   **visited-set guard on the endpoint frontier**, so cyclic graphs terminate.
//! - `p{n,m}` (`Range`, a PurRDF extension) is **k-fold composition unioned over
//!   `[n, m]`**, re-entrant per `k` (NOT one global visited set across `k`) so a
//!   node reachable at several repetition counts is reported for each — a single
//!   visited-guarded level-BFS would be *wrong* on cyclic graphs.
//! - `!(…)` (`NegatedPropertySet`) and `<any>`/`<any:ns>` (`Wildcard`, a PurRDF
//!   extension) scan any-predicate edges, filtering by the excluded set or the
//!   namespace prefix respectively.
//!
//! Determinism: endpoint sets stay ordered by `TermId`, so the materialised
//! solution order is the dataset's `TermId` order over the frozen dataset — the
//! same canonical discipline the rest of the evaluator follows.

use std::cell::{Cell, RefCell};
#[cfg(test)]
use std::collections::{BTreeMap, BTreeSet};
#[cfg(test)]
use std::rc::Rc;
#[cfg(test)]
use std::sync::Arc;

#[cfg(test)]
use purrdf_core::TermValue;
use purrdf_core::{DatasetView, TermId, TermRef, ViewTermId};
use purrdf_sparql_algebra::{
    NamedNode, NegatedPathElement, PropertyPathExpression, TermPattern, Variable,
};

#[cfg(test)]
use crate::convert::named_node_to_value;
use crate::convert::{ground_term_pattern_to_workspace_value, named_node_to_workspace_value};
use crate::dataset_spec::GraphScope;
use crate::error::EvalError;
use crate::eval::EvalCtx;
use crate::scratch::SolutionTerm;
use crate::solution::{SolutionSeq, VarSchema};
#[cfg(test)]
use crate::{DetHashMap, DetHashSet};

/// The path machine's local stop channel. Native containers delegate physical
/// growth to the common admitted homes and move the first failure here. A stopped
/// callback cannot allocate; the machine checks this channel before publication.
struct PathStorage {
    workspace: crate::WorkspaceCapability,
    error: RefCell<Option<EvalError>>,
    failed: Cell<bool>,
    _control: crate::WorkspaceAllocation,
}

type PathStorageOwner = purrdf_core::small::Shared<PathStorage>;

impl PathStorage {
    fn new(workspace: &crate::WorkspaceCapability) -> Result<PathStorageOwner, EvalError> {
        let control = workspace.charge(crate::workspace::shared_layout::<Self>()?)?;
        purrdf_core::small::Shared::try_new(Self {
            workspace: workspace.clone(),
            error: RefCell::new(None),
            failed: Cell::new(false),
            _control: control,
        })
        .map_err(|_| EvalError::AllocationFailed {
            construct: "property-path owner",
        })
    }

    fn fail(&self, error: EvalError) {
        if !self.failed.replace(true) {
            *self.error.borrow_mut() = Some(error);
        }
    }

    fn check(&self) -> Result<(), EvalError> {
        if let Some(error) = self.error.borrow_mut().take() {
            return Err(error);
        }
        if self.failed.get() {
            return Err(EvalError::WorkspaceStopped);
        }
        Ok(())
    }
}

struct PathVec<T> {
    values: crate::AdmittedVec<T>,
    storage: PathStorageOwner,
}

impl<T> PathVec<T> {
    fn new(storage: &PathStorageOwner) -> Self {
        Self {
            values: crate::AdmittedVec::new(&storage.workspace),
            storage: storage.clone(),
        }
    }
    fn from_iter(values: impl IntoIterator<Item = T>, storage: &PathStorageOwner) -> Self {
        let mut result = Self::new(storage);
        result.extend(values);
        result
    }
    fn push(&mut self, value: T) {
        if self.storage.failed.get() {
            return;
        }
        if let Err(error) = self.values.push(value) {
            self.storage.fail(error);
        }
    }
    fn extend(&mut self, values: impl IntoIterator<Item = T>) {
        for value in values {
            self.push(value);
            if self.storage.failed.get() {
                break;
            }
        }
    }
    fn pop(&mut self) -> Option<T> {
        self.values.pop()
    }
    fn take(&mut self) -> Self {
        let storage = self.storage.clone();
        std::mem::replace(self, Self::new(&storage))
    }
    fn into_admitted(self) -> crate::AdmittedVec<T> {
        self.values
    }
}
impl<T> std::ops::Deref for PathVec<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        &self.values
    }
}
impl<T> std::ops::DerefMut for PathVec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        self.values.as_mut_slice()
    }
}
impl<T> IntoIterator for PathVec<T> {
    type Item = T;
    type IntoIter = crate::workspace::AdmittedVecIntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

/// Canonically ordered endpoint ownership. Binary search does not allocate;
/// insertion grows the common admitted vector before moving any existing cell.
/// Set insertion removes duplicates; the separate vector preserves bag order.
struct PathSet<I: ViewTermId> {
    values: PathVec<I>,
}
impl<I: ViewTermId> PathSet<I> {
    fn new(storage: &PathStorageOwner) -> Self {
        Self {
            values: PathVec::new(storage),
        }
    }
    fn from_iter(values: impl IntoIterator<Item = I>, storage: &PathStorageOwner) -> Self {
        let mut set = Self::new(storage);
        set.extend(values);
        set
    }
    fn insert(&mut self, value: I) -> bool {
        if self.values.storage.failed.get() {
            return false;
        }
        let Err(index) = self.values.binary_search(&value) else {
            return false;
        };
        self.values.push(value);
        if self.values.storage.failed.get() {
            return false;
        }
        self.values[index..].rotate_right(1);
        true
    }
    fn extend(&mut self, values: impl IntoIterator<Item = I>) {
        for value in values {
            self.insert(value);
            if self.values.storage.failed.get() {
                break;
            }
        }
    }
    fn contains(&self, value: &I) -> bool {
        self.values.binary_search(value).is_ok()
    }
    fn take(&mut self) -> Self {
        Self {
            values: self.values.take(),
        }
    }
    fn into_vec(self) -> PathVec<I> {
        self.values
    }
}
impl<I: ViewTermId> Clone for PathSet<I> {
    fn clone(&self) -> Self {
        Self::from_iter(self.iter().copied(), &self.values.storage)
    }
}
impl<I: ViewTermId> std::ops::Deref for PathSet<I> {
    type Target = [I];
    fn deref(&self) -> &[I] {
        &self.values
    }
}
impl<I: ViewTermId> IntoIterator for PathSet<I> {
    type Item = I;
    type IntoIter = crate::workspace::AdmittedVecIntoIter<I>;
    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

struct SetPayload<I: ViewTermId> {
    set: PathSet<I>,
    _control: crate::WorkspaceAllocation,
}
enum SetRef<I: ViewTermId> {
    Shared(purrdf_core::small::Shared<SetPayload<I>>),
    Empty(PathSet<I>),
}
impl<I: ViewTermId> SetRef<I> {
    fn new(set: PathSet<I>) -> Self {
        let storage = set.values.storage.clone();
        if storage.failed.get() {
            return Self::Empty(PathSet::new(&storage));
        }
        let control = match crate::workspace::shared_layout::<SetPayload<I>>()
            .and_then(|bytes| storage.workspace.charge(bytes))
        {
            Ok(control) => control,
            Err(error) => {
                storage.fail(error);
                return Self::Empty(PathSet::new(&storage));
            }
        };
        match purrdf_core::small::Shared::try_new(SetPayload {
            set,
            _control: control,
        }) {
            Ok(owner) => Self::Shared(owner),
            Err(_) => {
                storage.fail(EvalError::AllocationFailed {
                    construct: "path reach-set owner",
                });
                Self::Empty(PathSet::new(&storage))
            }
        }
    }
    fn as_ref(&self) -> &PathSet<I> {
        self
    }
}
impl<I: ViewTermId> Clone for SetRef<I> {
    fn clone(&self) -> Self {
        match self {
            Self::Shared(owner) => Self::Shared(owner.clone()),
            Self::Empty(set) => Self::Empty(set.clone()),
        }
    }
}
impl<I: ViewTermId> std::ops::Deref for SetRef<I> {
    type Target = PathSet<I>;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Shared(owner) => &owner.set,
            Self::Empty(set) => set,
        }
    }
}

struct PathMap<K, V> {
    entries: crate::AdmittedMap<K, V>,
    storage: PathStorageOwner,
}
impl<K: Eq + std::hash::Hash, V> PathMap<K, V> {
    fn new(storage: &PathStorageOwner) -> Self {
        Self {
            entries: crate::AdmittedMap::default(),
            storage: storage.clone(),
        }
    }
    fn get(&self, key: &K) -> Option<&V> {
        self.entries.get(key)
    }
    fn insert(&mut self, key: K, value: V) {
        if self.storage.failed.get() {
            return;
        }
        if let Err(error) = self
            .entries
            .insert_admitted(key, value, &self.storage.workspace)
        {
            self.storage.fail(error);
        }
    }
}

struct Visited<I: ViewTermId>(PathMap<I, ()>);
impl<I: ViewTermId> Visited<I> {
    fn new(storage: &PathStorageOwner) -> Self {
        Self(PathMap::new(storage))
    }
    fn contains(&self, node: &I) -> bool {
        self.0.get(node).is_some()
    }
    fn insert(&mut self, node: I) -> bool {
        if self.0.storage.failed.get() || self.contains(&node) {
            return false;
        }
        self.0.insert(node, ());
        !self.0.storage.failed.get()
    }
}

struct PowerMemo<I: ViewTermId> {
    entries: RefCell<PathMap<(u32, I), SetRef<I>>>,
    _control: crate::WorkspaceAllocation,
}
type PowerMemoOwner<I> = purrdf_core::small::Shared<PowerMemo<I>>;
impl<I: ViewTermId> std::ops::Deref for PowerMemo<I> {
    type Target = RefCell<PathMap<(u32, I), SetRef<I>>>;
    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}
fn power_memo<I: ViewTermId>(storage: &PathStorageOwner) -> Option<PowerMemoOwner<I>> {
    if storage.failed.get() {
        return None;
    }
    let control = match crate::workspace::shared_layout::<PowerMemo<I>>()
        .and_then(|bytes| storage.workspace.charge(bytes))
    {
        Ok(control) => control,
        Err(error) => {
            storage.fail(error);
            return None;
        }
    };
    match purrdf_core::small::Shared::try_new(PowerMemo {
        entries: RefCell::new(PathMap::new(storage)),
        _control: control,
    }) {
        Ok(owner) => Some(owner),
        Err(_) => {
            storage.fail(EvalError::AllocationFailed {
                construct: "path power memo",
            });
            None
        }
    }
}

/// The pre-resolved exclusion sets for one `NegatedPropertySet`, split by
/// element direction (SPARQL 1.1 §18.2/§18.3): `!(p1|^p2|...)` decomposes into
/// a forward-only negated step over the plain elements and a reverse-only
/// negated step over the `^`-elements, unioned. Each side is `None` when that
/// direction has NO elements at all — a direction with zero listed elements
/// contributes NOTHING to the union (it is omitted, not treated as "excludes
/// nothing so everything forward-matches"); `Some(empty set)` cannot occur
/// because an empty `TermId` set would only arise from a non-empty element
/// list whose IRIs are simply absent from the dataset, which still
/// legitimately participates (excluding nothing that occurs).
struct NegatedSets<I: ViewTermId = TermId> {
    /// Predicates excluded from a **forward** hop (the plain, non-`^` elements),
    /// or `None` if the set has no plain elements.
    forward: Option<crate::AdmittedVec<I>>,
    /// Predicates excluded from a **reverse** hop (the `^`-prefixed elements),
    /// or `None` if the set has no inverted elements.
    inverse: Option<crate::AdmittedVec<I>>,
}

/// `elements`' excluded predicates resolved to dataset ids, each element looked up in
/// element order and filed under the direction it excludes.
fn resolve_negated<D: DatasetView + Sync>(
    elements: &[NegatedPathElement],
    dataset: &D,
    workspace: &crate::WorkspaceCapability,
    source_error: impl Fn(D::ReadError) -> EvalError,
) -> Result<NegatedSets<D::Id>, EvalError> {
    let mut forward = None;
    let mut inverse = None;
    for element in elements {
        let target = if element.inverse {
            inverse.get_or_insert_with(|| crate::AdmittedVec::new(workspace))
        } else {
            forward.get_or_insert_with(|| crate::AdmittedVec::new(workspace))
        };
        let term = named_node_to_workspace_value(&element.predicate, workspace)?;
        if let Some(id) = dataset.term_id_by_value(&term).map_err(&source_error)?
            && let Err(index) = target.binary_search(&id)
        {
            target.push(id)?;
            target.as_mut_slice()[index..].rotate_right(1);
        }
    }
    Ok(NegatedSets { forward, inverse })
}

/// One node of a compiled path expression ([`PathProgram`]): the operator, with its
/// children named by their index in the program and its leaf payload resolved against
/// the dataset.
enum PathOp<'p, I: ViewTermId = TermId> {
    /// A single predicate: its dataset id, or `None` when the dataset has no such term
    /// (the hop then reaches nothing).
    Predicate(Option<I>),
    /// `^inner`: the inner relation with its direction flipped.
    Reverse(usize),
    /// `e1 / e2 / …`, the elements in source order.
    Sequence(crate::AdmittedVec<usize>),
    /// `e1 | e2 | …`, the elements in source order.
    Alternative(crate::AdmittedVec<usize>),
    /// `inner?`.
    ZeroOrOne(usize),
    /// `inner*`.
    ZeroOrMore(usize),
    /// `inner+`.
    OneOrMore(usize),
    /// `inner{min,max}`.
    Range {
        inner: usize,
        min: u32,
        max: Option<u32>,
    },
    /// `!(…)`, its excluded predicates resolved per direction.
    Negated(NegatedSets<I>),
    /// `<any>` / `<any:ns>`, with the namespace prefix when one is written.
    Wildcard(Option<&'p NamedNode>),
}

/// The index of a program's root op: the first entered, so the first numbered.
const ROOT_OP: usize = 0;

/// A path expression compiled to a flat program: one [`PathOp`] per node, numbered in
/// pre-order (the root is [`ROOT_OP`]), with each node's two static properties —
/// whether it admits the zero-length identity and whether a repetition operator occurs
/// in it — folded up beside it.
struct PathProgram<'p, I: ViewTermId = TermId> {
    ops: crate::AdmittedVec<PathOp<'p, I>>,
    /// Per op, whether `reach(op, n, …)` contains `n` for every `n` regardless of the
    /// graph: `*` and `?` unconditionally, `{min,…}` when `min == 0`, `^inner` as its
    /// inner, a sequence when every element is, an alternative when any element is.
    reflexive: crate::AdmittedVec<bool>,
    /// Per op, whether `*`, `+`, `?` or `{n,m}` occurs at or under it — the SPARQL 1.1
    /// §18.3 dividing line between bag and set evaluation.
    repetition: crate::AdmittedVec<bool>,
}

impl<'p, I: ViewTermId> PathProgram<'p, I> {
    /// Compile `path`, consulting `dataset` for every predicate IRI and negated set in
    /// pre-order.
    ///
    /// The walk keeps its own stack of steps: entering a node numbers it and resolves
    /// a leaf's payload; exiting it, after its children, links the children's indices
    /// and folds their static properties.
    #[cfg(test)]
    fn compile<D: DatasetView<Id = I> + Sync>(
        path: &'p PropertyPathExpression,
        dataset: &D,
    ) -> Self {
        Self::compile_admitted(
            path,
            dataset,
            &crate::WorkspaceCapability::default(),
            EvalError::source_read,
        )
        .expect("resident path program allocation")
    }

    /// Compile the same program while each buffer retains its actual capacity
    /// admission. Lookup failures remain typed at the query's source checkpoint.
    fn compile_admitted<D: DatasetView<Id = I> + Sync>(
        path: &'p PropertyPathExpression,
        dataset: &D,
        workspace: &crate::WorkspaceCapability,
        source_error: impl Fn(D::ReadError) -> EvalError,
    ) -> Result<Self, EvalError> {
        enum Build<'p> {
            Enter(&'p PropertyPathExpression),
            Exit(usize, &'p PropertyPathExpression),
        }
        use PropertyPathExpression as P;
        let mut ops = crate::AdmittedVec::new(workspace);
        let mut reflexive = crate::AdmittedVec::new(workspace);
        let mut repetition = crate::AdmittedVec::new(workspace);
        // The indices of the nodes whose exit step has run, the latest on top: a
        // parent's exit pops its children's, last child first.
        let mut built = crate::AdmittedVec::new(workspace);
        let mut pending = crate::AdmittedVec::new(workspace);
        pending.push(Build::Enter(path))?;
        while let Some(step) = pending.pop() {
            match step {
                Build::Enter(node) => {
                    let index = ops.len();
                    ops.push(match node {
                        P::NamedNode(predicate) => {
                            let term = named_node_to_workspace_value(predicate, workspace)?;
                            PathOp::Predicate(
                                dataset.term_id_by_value(&term).map_err(&source_error)?,
                            )
                        }
                        P::NegatedPropertySet(elements) => PathOp::Negated(resolve_negated(
                            elements,
                            dataset,
                            workspace,
                            &source_error,
                        )?),
                        P::Wildcard { namespace } => PathOp::Wildcard(namespace.as_ref()),
                        P::Reverse(_)
                        | P::Sequence(_)
                        | P::Alternative(_)
                        | P::ZeroOrMore(_)
                        | P::OneOrMore(_)
                        | P::ZeroOrOne(_)
                        | P::Range { .. } => PathOp::Predicate(None),
                    })?;
                    reflexive.push(false)?;
                    repetition.push(false)?;
                    pending.push(Build::Exit(index, node))?;
                    // Children are pushed last first, so they are entered in source order.
                    match node {
                        P::Reverse(inner)
                        | P::ZeroOrMore(inner)
                        | P::OneOrMore(inner)
                        | P::ZeroOrOne(inner)
                        | P::Range { inner, .. } => pending.push(Build::Enter(inner))?,
                        P::Sequence(elements) | P::Alternative(elements) => {
                            for element in elements.iter().rev() {
                                pending.push(Build::Enter(element))?;
                            }
                        }
                        P::NamedNode(_) | P::NegatedPropertySet(_) | P::Wildcard { .. } => {}
                    }
                }
                Build::Exit(index, node) => {
                    match node {
                        P::NamedNode(_) | P::NegatedPropertySet(_) | P::Wildcard { .. } => {}
                        P::Reverse(_) => {
                            let inner = pop_child(&mut built);
                            let child_reflexive = reflexive[inner];
                            let child_repetition = repetition[inner];
                            reflexive.as_mut_slice()[index] = child_reflexive;
                            repetition.as_mut_slice()[index] = child_repetition;
                            ops.as_mut_slice()[index] = PathOp::Reverse(inner);
                        }
                        P::ZeroOrOne(_) => {
                            let inner = pop_child(&mut built);
                            reflexive.as_mut_slice()[index] = true;
                            repetition.as_mut_slice()[index] = true;
                            ops.as_mut_slice()[index] = PathOp::ZeroOrOne(inner);
                        }
                        P::ZeroOrMore(_) => {
                            let inner = pop_child(&mut built);
                            reflexive.as_mut_slice()[index] = true;
                            repetition.as_mut_slice()[index] = true;
                            ops.as_mut_slice()[index] = PathOp::ZeroOrMore(inner);
                        }
                        P::OneOrMore(_) => {
                            let inner = pop_child(&mut built);
                            repetition.as_mut_slice()[index] = true;
                            ops.as_mut_slice()[index] = PathOp::OneOrMore(inner);
                        }
                        P::Range { min, max, .. } => {
                            let inner = pop_child(&mut built);
                            reflexive.as_mut_slice()[index] = *min == 0;
                            repetition.as_mut_slice()[index] = true;
                            ops.as_mut_slice()[index] = PathOp::Range {
                                inner,
                                min: *min,
                                max: *max,
                            };
                        }
                        P::Sequence(elements) => {
                            let children = pop_children(&mut built, elements.len(), workspace)?;
                            let child_reflexive = children.iter().all(|&child| reflexive[child]);
                            let child_repetition = children.iter().any(|&child| repetition[child]);
                            reflexive.as_mut_slice()[index] = child_reflexive;
                            repetition.as_mut_slice()[index] = child_repetition;
                            ops.as_mut_slice()[index] = PathOp::Sequence(children);
                        }
                        P::Alternative(elements) => {
                            let children = pop_children(&mut built, elements.len(), workspace)?;
                            let child_reflexive = children.iter().any(|&child| reflexive[child]);
                            let child_repetition = children.iter().any(|&child| repetition[child]);
                            reflexive.as_mut_slice()[index] = child_reflexive;
                            repetition.as_mut_slice()[index] = child_repetition;
                            ops.as_mut_slice()[index] = PathOp::Alternative(children);
                        }
                    }
                    built.push(index)?;
                }
            }
        }
        Ok(Self {
            ops,
            reflexive,
            repetition,
        })
    }

    /// Whether the whole path admits the zero-length identity, i.e. `reach(root, n, …)`
    /// always contains `n` itself regardless of the graph.
    fn is_reflexive(&self) -> bool {
        self.reflexive[ROOT_OP]
    }

    /// Whether the whole path contains a repetition operator (`*`, `+`, `?`, `{n,m}`)
    /// anywhere — the SPARQL 1.1 §18.3 dividing line between the two evaluation
    /// strategies `eval_path` dispatches on:
    ///
    /// - `false` (a "simple" path of only `/`, `|`, `^`, `!(…)`, a single
    ///   predicate, or `<any>`): evaluates as if unrolled into a BGP, so a node
    ///   pair reachable via several distinct triple combinations is a MULTISET —
    ///   one result row per derivation (`pp11`, `pp31`). See [`bag_reach`].
    /// - `true`: evaluates via the ALP fixpoint ([`reach`]), a SET of reachable
    ///   nodes with no duplicates — required for termination on cyclic/infinite
    ///   graphs, and mandated even when the repetition is nested under a combinator
    ///   (e.g. `(:p/:q)+`).
    fn has_repetition(&self) -> bool {
        self.repetition[ROOT_OP]
    }
}

/// The index of the child whose exit step ran last.
fn pop_child(built: &mut crate::AdmittedVec<usize>) -> usize {
    built
        .pop()
        .expect("a child's exit step runs before its parent's")
}

/// The indices of the last `count` children built, in source order.
fn pop_children(
    built: &mut crate::AdmittedVec<usize>,
    count: usize,
    workspace: &crate::WorkspaceCapability,
) -> Result<crate::AdmittedVec<usize>, EvalError> {
    let mut children = crate::AdmittedVec::with_capacity(count, workspace)?;
    for _ in 0..count {
        children.push(pop_child(built))?;
    }
    children.as_mut_slice().reverse();
    Ok(children)
}

/// A reach memo key: the op, the start node, and the direction.
type ReachKey<I = TermId> = (usize, I, bool);
type ReachCache<I = TermId> = RefCell<PathMap<ReachKey<I>, SetRef<I>>>;
#[cfg(test)]
type ReferenceReachCache<I = TermId> = RefCell<DetHashMap<ReachKey<I>, Rc<BTreeSet<I>>>>;
#[cfg(test)]
type PowerReachCache<I = TermId> = BTreeMap<(u32, I), Rc<BTreeSet<I>>>;

/// The immutable, traversal-wide context every frame of the reach machine reads: the
/// frozen dataset, the active dataset graph scope (§13: a single graph, or a
/// `FROM`/`USING`-merged default graph), the compiled program, and a per-evaluation
/// reachability memo.
struct PathCtx<'a, D: DatasetView + Sync> {
    dataset: &'a D,
    scope: GraphScope<D::Id>,
    program: PathProgram<'a, D::Id>,
    growth: crate::WorkspaceCapability,
    storage: PathStorageOwner,
    reach_cache: ReachCache<D::Id>,
    /// The live governor accounting of the execution this traversal belongs to, shared
    /// by [`Arc`] with the evaluation context rather than copied — a traversal that
    /// spent its own copy of the budget would let a query with `N` property paths spend
    /// `N` budgets.
    governors: Option<crate::workspace::SharedWorkspace<crate::governor::GovernorState>>,
    /// The per-node charge ledger and the ordinal of the path node being evaluated, when
    /// one is installed. A traversal's fuel is spent well below the operator boundary, so
    /// without this the single most graph-dependent cost in the evaluator would be the one
    /// cost an EXPLAIN could not attribute.
    ledger: Option<(
        crate::workspace::SharedWorkspace<crate::governor::ledger::ChargeLedger>,
        crate::plan::NodeId,
    )>,
}

impl<D: DatasetView + Sync> PathCtx<'_, D> {
    /// The `path-frontier-expansion` charge point: one unit per candidate relation edge
    /// examined.
    ///
    /// A property path is the one place in the evaluator where the work is unbounded in
    /// the *graph* rather than in the query. Charging only once for a frontier node makes a
    /// million-edge star cost the same as a leaf; charging each indexed quad candidate and
    /// each relation-composition candidate bounds the work that fan-out actually creates.
    /// Returns `false` when execution has stopped. Traversal loops then retain only nodes
    /// already proven reachable, which is a sound lower bound.
    #[inline]
    fn charge_candidate(&self) -> bool {
        if self.storage.failed.get() {
            return false;
        }
        let Some(state) = self.governors.as_ref() else {
            return true;
        };
        if state.tripped().is_some() || state.poll_stop().is_some() {
            return false;
        }
        let point = crate::governor::ChargePoint::PathFrontierExpansion;
        let charged = state.charge_point_if_engaged(point).is_ok();
        if charged && let Some((ledger, node)) = self.ledger.as_ref() {
            ledger.record_fuel(*node, point, point.cost());
        }
        charged
    }

    /// Whether a governor has already stopped this execution, so that anything computed
    /// from here on is a partial view of the graph rather than a finished one.
    ///
    /// One null test on an ungoverned traversal — which is every traversal that did not
    /// ask for a budget — and one write-once cell read on a governed one.
    #[inline]
    fn stopped(&self) -> bool {
        self.storage.failed.get()
            || self
                .governors
                .as_ref()
                .is_some_and(|state| state.tripped().is_some() || state.poll_stop().is_some())
    }
}

#[cfg(test)]
impl<'a, D: DatasetView + Sync> PathCtx<'a, D> {
    fn resident(
        dataset: &'a D,
        scope: GraphScope<D::Id>,
        program: PathProgram<'a, D::Id>,
        governors: Option<crate::workspace::SharedWorkspace<crate::GovernorState>>,
    ) -> Self {
        let growth = crate::WorkspaceCapability::default();
        let storage = PathStorage::new(&growth).expect("resident path control");
        Self {
            dataset,
            scope,
            program,
            growth,
            reach_cache: RefCell::new(PathMap::new(&storage)),
            storage,
            governors,
            ledger: None,
        }
    }
}

/// Evaluate a property-path constraint `subject path object` to a multiset of
/// solutions over its variable endpoint(s).
///
/// The result schema is the variable endpoints in subject-then-object order
/// (deduplicated, so `?x p+ ?x` is a single column). A blank-node endpoint is an
/// anonymous variable that is projected away (like BGP, SPARQL §4.1.4).
///
/// A ground endpoint absent from the dataset (it is never the subject or object
/// of any quad — e.g. the whole graph is empty) is NOT automatically an empty
/// result: SPARQL 1.1's zero-length-path identity (`?`, `*`, `{0,…}`) matches a
/// node to itself regardless of whether that node happens to appear in any
/// triple, so `:o :p* :o` and `?s :p* :o` both still admit the trivial
/// self-pairing (W3C `property-path/zero_or_more_set_start` /
/// `zero_or_more_set_end`). A non-reflexive path cannot connect an absent node
/// to anything else (it has no edges to traverse), so it correctly stays empty.
/// # A leaf of the partial-lift channel
///
/// A property path has no sub-pattern, so there is no child truncation to compose: like a
/// basic graph pattern, it is where a truncation ORIGINATES rather than somewhere one
/// passes through, and the dispatch in [`crate::eval::eval`] wraps its result directly.
// Out of line by design: see the thin-dispatcher invariant on `eval::eval_node`.
#[inline(never)]
pub(crate) fn eval_path<D: DatasetView + Sync>(
    subject: &TermPattern,
    path: &PropertyPathExpression,
    object: &TermPattern,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<SolutionSeq<D::Id>, EvalError> {
    let dataset = ctx.dataset;
    let scope = ctx.active_dataset.scope_for(ctx.active_graph);

    // The output schema is fixed by which endpoints are *visible* variables, and is
    // independent of whether a ground endpoint happens to be absent — so an empty
    // result still carries the right columns for downstream joins.
    let schema = path_schema(subject, object, &ctx.growth)?;
    let width = schema.len();
    let s_col = visible_var(subject).and_then(|v| schema.index_of(&v));
    let o_col = visible_var(object).and_then(|v| schema.index_of(&v));

    let s_end = resolve_end(subject, dataset, &ctx.growth, |error| {
        ctx.workspace.source_error(error)
    })?;
    let o_end = resolve_end(object, dataset, &ctx.growth, |error| {
        ctx.workspace.source_error(error)
    })?;

    // The path compiled once for this evaluation: every predicate IRI and every
    // negated set resolved to ids, so no traversal step repeats a lookup.
    let storage = PathStorage::new(&ctx.growth)?;
    let pctx = PathCtx {
        dataset,
        scope,
        program: PathProgram::compile_admitted(path, dataset, &ctx.growth, |error| {
            ctx.workspace.source_error(error)
        })?,
        growth: ctx.growth.clone(),
        reach_cache: RefCell::new(PathMap::new(&storage)),
        storage,
        governors: ctx.governor_state().cloned(),
        ledger: ctx
            .charge_ledger()
            .map(|ledger| (ledger.clone(), ctx.ledger_node)),
    };

    // SPARQL 1.1 §18.3: a path with NO repetition operator (`*`/`+`/`?`/`{n,m}`)
    // anywhere in its tree evaluates as if unrolled into a BGP — each distinct
    // combination of matching triples is its own solution, so a node reachable
    // by several derivations (e.g. `pp11`'s `:p1/:p2` through two different
    // intermediates) surfaces as that many DUPLICATE result rows (a MULTISET).
    // A path containing repetition anywhere instead uses the ALP fixpoint
    // semantics (`reach`), which is a SET of reachable nodes — no duplicates, and
    // required for termination on cyclic/infinite graphs. Both shapes are unified
    // behind `node_reach`, which returns a `Vec` either way (with genuine
    // duplicates in the bag case, and none in the set case).
    let bag = !pctx.program.has_repetition();
    let node_reach = |node: D::Id, forward: bool| -> Result<crate::AdmittedVec<D::Id>, EvalError> {
        if bag {
            bag_reach(ROOT_OP, node, forward, &pctx)
        } else {
            let reached = reach(ROOT_OP, node, forward, &pctx)?;
            let mut nodes = crate::AdmittedVec::with_capacity(reached.len(), &pctx.growth)?;
            for &node in reached.iter() {
                nodes.push(node)?;
            }
            Ok(nodes)
        }
    };

    // The answer-cap / `LIMIT` pushdown's verdict for this path node: the number of output
    // rows past which nothing it produces can reach the query's answer. A path's emission
    // order is the traversal order below, and the loops that can be stopped are stopped at
    // the top of an iteration — so what is skipped is whole `node_reach` traversals, which
    // is where a path's cost lives, not merely the row that would have been pushed.
    let semantic_ceiling = ctx.row_ceiling();
    let cell_ceiling = ctx.cell_row_ceiling(width);

    let mut rows = crate::solution::RowsBuilder::new(&ctx.growth);
    let push_pair = |ctx: &EvalCtx<'_, D>,
                     rows: &mut crate::solution::RowsBuilder<D::Id>,
                     s_id: Option<SolutionTerm<D::Id>>,
                     o_id: Option<SolutionTerm<D::Id>>|
     -> Result<bool, EvalError> {
        // LIMIT / answer-cap pushdown proves rows at and beyond this point irrelevant to
        // the query and may stop exactly at its bound. A cell ceiling is inclusive and
        // therefore stops only when a further qualifying row exists; record that attempted
        // row before constructing its `SmallVec`, then decline the allocation.
        if semantic_ceiling.is_some_and(|cap| rows.len() >= cap) {
            return Ok(false);
        }
        if cell_ceiling.is_some_and(|cap| rows.len() >= cap) {
            let _ = ctx.observe_cells(rows.len().saturating_add(1), width);
            return Ok(false);
        }
        let row = crate::solution::RetainedRow::try_build(width, &ctx.growth, |row| {
            if let (Some(c), Some(id)) = (s_col, s_id) {
                row[c] = Some(id);
            }
            if let (Some(c), Some(id)) = (o_col, o_id) {
                row[c] = Some(id);
            }
            Ok(())
        })?;
        rows.push_row(row)?;
        Ok(true)
    };

    match (s_end, o_end) {
        // Both ground: an ASK-shaped membership test. The schema is empty, so
        // each derivation reaching `oid` is its own unit solution (one empty
        // row) — for a SET path (`bag == false`) that count is always 0 or 1.
        (Endpoint::Bound(sid), Endpoint::Bound(oid)) => {
            // SET case: the reach set is a `BTreeSet` (elements unique), so the
            // derivation count is a single O(log n) membership probe on the memoized
            // set — no O(n) collect-into-`Vec` + filter pass. The BAG case keeps its
            // multiset walk, where duplicates ARE the answer.
            let count = if bag {
                node_reach(sid, true)?.iter().filter(|&&y| y == oid).count()
            } else {
                usize::from(reach(ROOT_OP, sid, true, &pctx)?.contains(&oid))
            };
            for _ in 0..count {
                if !push_pair(ctx, &mut rows, None, None)? {
                    break;
                }
            }
        }
        // Both ground but absent from the dataset entirely: the only way they can
        // ever connect is the reflexive zero-length identity, when they are the
        // SAME term (an absent node has no edges to traverse for anything else).
        (Endpoint::BoundAbsent(sval), Endpoint::BoundAbsent(oval)) => {
            if ctx.growth.terms_equal(&sval, &oval)? && pctx.program.is_reflexive() {
                let _ = push_pair(ctx, &mut rows, None, None)?;
            }
        }
        // One side present in the dataset, the other absent: they cannot be the
        // same term (an equal value would have resolved to the same `TermId` on
        // both sides), and an absent node has no edges — so no path connects them.
        (Endpoint::Bound(_), Endpoint::BoundAbsent(_))
        | (Endpoint::BoundAbsent(_), Endpoint::Bound(_)) => {}
        // Subject ground, object variable: walk forward from the subject.
        (Endpoint::Bound(sid), Endpoint::Free { .. }) => {
            for y in node_reach(sid, true)? {
                if !push_pair(
                    ctx,
                    &mut rows,
                    Some(SolutionTerm::Existing(sid)),
                    Some(SolutionTerm::Existing(y)),
                )? {
                    break;
                }
            }
        }
        // Subject ground but absent from the dataset, object variable: only the
        // zero-length reflexive pair (subject bound to itself) can ever match.
        (Endpoint::BoundAbsent(sval), Endpoint::Free { .. }) => {
            if pctx.program.is_reflexive() {
                // The PLAIN door, deliberately. SPARQL 1.1 §18.5.1 makes the
                // zero-length pair `(x, x)` a solution for a ground `x` whether
                // or not `x` occurs in the graph, so this row is REQUIRED — and
                // there is no binding to give up short of deleting the whole
                // row, which no refusal in this crate is allowed to cost. A path
                // endpoint is an algebra ground term, gated before it arrives:
                // query text by the SPARQL parser, a hand-built `Query` by
                // `purrdf_sparql_algebra`'s algebra validator on this same
                // profile. See `ScratchInterner::intern`.
                let term = ctx.intern_workspace_term(sval)?;
                let _ = push_pair(ctx, &mut rows, term, term)?;
            }
        }
        // Object ground, subject variable: walk backward from the object.
        (Endpoint::Free { .. }, Endpoint::Bound(oid)) => {
            for x in node_reach(oid, false)? {
                if !push_pair(
                    ctx,
                    &mut rows,
                    Some(SolutionTerm::Existing(x)),
                    Some(SolutionTerm::Existing(oid)),
                )? {
                    break;
                }
            }
        }
        // Object ground but absent from the dataset, subject variable: symmetric
        // to the subject-absent case above.
        (Endpoint::Free { .. }, Endpoint::BoundAbsent(oval)) => {
            if pctx.program.is_reflexive() {
                let term = ctx.intern_workspace_term(oval)?;
                let _ = push_pair(ctx, &mut rows, term, term)?;
            }
        }
        // Both variable: enumerate the node universe (so zero-length `*`/`?`/`{0,…}`
        // pairs isolated nodes with themselves) and walk forward from each. When the
        // two endpoints are the *same* variable, keep only the reflexive pairs.
        (Endpoint::Free { var: sv }, Endpoint::Free { var: ov }) => {
            let same = sv == ov;
            if same {
                // Reflexive paths (p*, p?, p{0,m}) admit the zero-length identity, so
                // every node trivially reaches itself — skip the reach call entirely
                // (and note `reflexive` is only ever true for a repetition path, i.e.
                // `bag == false`, so the multiset-count branch below never double-
                // counts a reflexive path's zero-length step). Non-reflexive paths
                // require an actual traversal to discover whether x cycles back to
                // itself — and for a bag path, count EACH derivation as its own row.
                let reflexive = pctx.program.is_reflexive();
                'nodes: for x in node_universe(&pctx)? {
                    if reflexive {
                        if !push_pair(
                            ctx,
                            &mut rows,
                            Some(SolutionTerm::Existing(x)),
                            Some(SolutionTerm::Existing(x)),
                        )? {
                            break;
                        }
                    } else {
                        let count = node_reach(x, true)?.into_iter().filter(|&y| y == x).count();
                        for _ in 0..count {
                            if !push_pair(
                                ctx,
                                &mut rows,
                                Some(SolutionTerm::Existing(x)),
                                Some(SolutionTerm::Existing(x)),
                            )? {
                                break 'nodes;
                            }
                        }
                    }
                }
            } else {
                // PINNED: spec-mandated distinct-var enumeration — enumerate every node
                // in the universe and materialise all forward reachability. DO NOT alter.
                'nodes: for x in node_universe(&pctx)? {
                    for y in node_reach(x, true)? {
                        if !push_pair(
                            ctx,
                            &mut rows,
                            Some(SolutionTerm::Existing(x)),
                            Some(SolutionTerm::Existing(y)),
                        )? {
                            break 'nodes;
                        }
                    }
                }
            }
        }
    }

    pctx.storage.check()?;
    Ok(SolutionSeq {
        schema: schema.shared_admitted(&ctx.growth)?,
        rows: rows.finish()?,
    })
}

/// A resolved path endpoint: a ground dataset id, a ground term absent from the
/// dataset, or a free (variable / blank) position.
enum Endpoint<I: ViewTermId = TermId> {
    /// A ground constant resolved to its dataset id.
    Bound(I),
    /// A ground constant that is not the subject or object of any quad in the
    /// dataset. Still a valid RDF term for the zero-length reflexive identity
    /// (see [`eval_path`]'s doc comment) — just not reachable by any real hop.
    BoundAbsent(crate::WorkspaceTerm),
    /// A free position — a real variable, or a blank node treated as an anonymous
    /// (projected-away) variable. The variable identity is carried so two free
    /// endpoints sharing a name evaluate the reflexive `?x p ?x` case.
    Free { var: Variable },
}

/// Resolve an endpoint term to a [`Endpoint`].
fn resolve_end<D: DatasetView + Sync>(
    term: &TermPattern,
    dataset: &D,
    workspace: &crate::WorkspaceCapability,
    source_error: impl Fn(D::ReadError) -> EvalError,
) -> Result<Endpoint<D::Id>, EvalError> {
    match term {
        TermPattern::Variable(v) => Ok(Endpoint::Free { var: v.clone() }),
        // A blank node in a path endpoint is an anonymous variable (SPARQL §4.1.4):
        // give it a NUL-prefixed synthetic name (the grammar can never produce one),
        // so two distinct blank labels are distinct vars and a repeated label
        // co-refers, exactly as in a BGP.
        TermPattern::BlankNode(b) => Ok(Endpoint::Free {
            var: crate::bgp::blank_var_admitted(b.as_str(), workspace)?,
        }),
        // `crate::bgp` supports a variable *inside* a quoted-triple-term BGP
        // position (`Pos::Triple`'s structural match), so a reader could expect
        // the same for a path endpoint — it is NOT the same feature here. BGP's
        // structural match runs inside the row/binding join: a candidate quoted
        // triple's components resolve against the SAME schema the rest of the
        // BGP's slots share. A path endpoint has no such row to bind into — path
        // evaluation resolves each endpoint to a single node identity (or leaves
        // it free) and then walks reachability over THAT node id
        // ([`Endpoint::Bound`]/[`Endpoint::Free`]); a quoted triple term with a
        // variable component names more than one node, and `path_schema` (below)
        // gives an endpoint exactly one output column, not one per component. Would
        // need its own node identity AND its own multi-variable schema/output
        // contract before "support" could mean anything more than the refusal an
        // inaccurate message used to hide — genuinely new evaluation semantics,
        // not a wiring gap this conversion helper's `site` parameter can close.
        other => {
            let value = ground_term_pattern_to_workspace_value(
                other,
                "a property-path endpoint",
                workspace,
            )?;
            Ok(
                match dataset.term_id_by_value(&value).map_err(source_error)? {
                    Some(id) => Endpoint::Bound(id),
                    None => Endpoint::BoundAbsent(value),
                },
            )
        }
    }
}

/// The output schema: the visible variable endpoints in subject-then-object order,
/// deduplicated (a repeated variable is one column).
fn path_schema(
    subject: &TermPattern,
    object: &TermPattern,
    workspace: &crate::WorkspaceCapability,
) -> Result<VarSchema, EvalError> {
    VarSchema::from_vars_admitted(
        [visible_var(subject), visible_var(object)]
            .into_iter()
            .flatten(),
        workspace,
    )
}

/// The projectable variable an endpoint exposes, if any. Blank nodes (anonymous
/// variables) and ground terms expose none.
fn visible_var(term: &TermPattern) -> Option<Variable> {
    match term {
        TermPattern::Variable(v) => Some(v.clone()),
        _ => None,
    }
}

/// All terms that appear as a subject or object of a quad in the active-dataset scope
/// — the node universe for a both-endpoints-variable path (SPARQL §18.1.7). The
/// `BTreeSet` de-dupes endpoints, so a `FROM`-merged scope needs no extra triple dedup.
fn node_universe<D: DatasetView + Sync>(ctx: &PathCtx<'_, D>) -> Result<PathSet<D::Id>, EvalError> {
    let mut out = PathSet::new(&ctx.storage);
    // The node universe is a full scan of the active scope, and it seeds every
    // both-endpoints-variable path — so it is charged per candidate endpoint examined,
    // exactly like a frontier expansion. Stopping early yields a smaller universe, hence
    // a subset of the true solutions: a sound lower bound.
    let mut stopped = false;
    ctx.scope.for_each_quad(ctx.dataset, None, None, None, |q| {
        if stopped {
            return;
        }
        if !ctx.charge_candidate() {
            stopped = true;
            return;
        }
        out.insert(q.s);
        out.insert(q.o);
    });
    ctx.storage.check()?;
    Ok(out)
}

// ---------------------------------------------------------------------------
// The reach machine (set semantics)
// ---------------------------------------------------------------------------

/// The nodes `op`'s relation reaches from `node` — `(node, y)` forward, `(y, node)`
/// backward — as a set, through the traversal's memo.
///
/// The machine runs a work list of [`Frame`]s: the top frame is stepped, and either
/// pushes the frame whose answer it needs or returns its own value to the frame
/// beneath it. The root frame's value is the answer.
fn reach<D: DatasetView + Sync>(
    op: usize,
    node: D::Id,
    forward: bool,
    ctx: &PathCtx<'_, D>,
) -> Result<SetRef<D::Id>, EvalError> {
    let mut frames = PathVec::from_iter(
        [Frame::Reach(ReachFrame::new(op, node, forward))],
        &ctx.storage,
    );
    let mut returned: Option<Value<D::Id>> = None;
    loop {
        ctx.storage.check()?;
        let top = frames
            .last_mut()
            .expect("the root frame's return ends the loop before the list empties");
        let step = top.step(returned.take(), ctx);
        ctx.storage.check()?;
        match step {
            Step::Call(frame) => frames.push(frame),
            Step::Return(value) => {
                let _ = frames.pop();
                if frames.is_empty() {
                    ctx.storage.check()?;
                    return Ok(value.into_reach());
                }
                returned = Some(value);
            }
        }
    }
}

/// What one frame of the reach machine hands back to the frame beneath it.
enum Value<I: ViewTermId> {
    /// A reach set, shared with the memo entry it may also be.
    Reach(SetRef<I>),
    /// A set an operator frame built.
    Built(PathSet<I>),
    /// One frontier level, or `None` when a governor stopped the walk inside it.
    Level(Option<PathSet<I>>),
}

impl<I: ViewTermId> Value<I> {
    /// The answer to a reach request.
    fn into_reach(self) -> SetRef<I> {
        match self {
            Self::Reach(set) => set,
            Self::Built(_) | Self::Level(_) => {
                unreachable!("a reach request is answered with a shared set")
            }
        }
    }

    /// The answer to an operator's body: a reach set copied from its shared owner, or a set
    /// the operator built.
    fn into_set(self) -> PathSet<I> {
        match self {
            Self::Reach(set) => set.as_ref().clone(),
            Self::Built(set) => set,
            Self::Level(_) => unreachable!("an operator's body is answered with a set"),
        }
    }

    /// The answer to a level request.
    fn into_level(self) -> Option<PathSet<I>> {
        match self {
            Self::Level(level) => level,
            Self::Reach(_) | Self::Built(_) => {
                unreachable!("a level request is answered with a level")
            }
        }
    }
}

/// What a frame asks of the machine after one step.
#[expect(
    clippy::large_enum_variant,
    reason = "the next path-machine step stays inline; boxing a frame would add a separate allocation beyond the original admitted frame array"
)]
enum Step<I: ViewTermId> {
    /// Run `frame` to completion, then resume this frame with its value.
    Call(Frame<I>),
    /// This frame is finished with this value.
    Return(Value<I>),
}

/// One suspended operator of the reach machine, holding the loop state its
/// computation has between two sub-requests. A frame names the program op it
/// evaluates and reads the op's children from the program on every step, so it holds
/// no borrow of the program.
enum Frame<I: ViewTermId> {
    Reach(ReachFrame<I>),
    Sequence(SequenceFrame<I>),
    Alternative(AlternativeFrame<I>),
    Closure(ClosureFrame<I>),
    Range(RangeFrame<I>),
    StepLevel(StepLevelFrame<I>),
    ApplyPower(ApplyPowerFrame<I>),
    PowerReach(PowerReachFrame<I>),
}

impl<I: ViewTermId> Frame<I> {
    /// Advance this frame: with `returned`, the value of the frame it last called, or
    /// with `None` on its first step.
    fn step<D: DatasetView<Id = I> + Sync>(
        &mut self,
        returned: Option<Value<I>>,
        ctx: &PathCtx<'_, D>,
    ) -> Step<I> {
        match self {
            Self::Reach(frame) => frame.step(returned, ctx),
            Self::Sequence(frame) => frame.step(returned, ctx),
            Self::Alternative(frame) => frame.step(returned, ctx),
            Self::Closure(frame) => frame.step(returned, ctx),
            Self::Range(frame) => frame.step(returned),
            Self::StepLevel(frame) => frame.step(returned, ctx),
            Self::ApplyPower(frame) => frame.step(returned, ctx),
            Self::PowerReach(frame) => frame.step(returned, ctx),
        }
    }
}

/// The elements of the sequence or alternative op `op`.
fn elements_of<'a, I: ViewTermId>(program: &'a PathProgram<'_, I>, op: usize) -> &'a [usize] {
    match &program.ops[op] {
        PathOp::Sequence(elements) | PathOp::Alternative(elements) => elements,
        PathOp::Predicate(_)
        | PathOp::Reverse(_)
        | PathOp::ZeroOrOne(_)
        | PathOp::ZeroOrMore(_)
        | PathOp::OneOrMore(_)
        | PathOp::Range { .. }
        | PathOp::Negated(_)
        | PathOp::Wildcard(_) => {
            unreachable!("a sequence or alternative frame is opened only at such an op")
        }
    }
}

/// A reach request: the memo in front of the operator's body.
///
/// **A set the budget cut short is never memoized.** The closure, the range and the
/// level frames all return the frontier reached so far when a governor stops them, and
/// that partial set is a *lower* bound on the path relation, not the relation — writing
/// it under this key would let a later lookup read it as though the traversal had
/// finished. The write is therefore gated on the traversal having completed, which is
/// the same rule [`crate::expr::exists`] applies to its inner-pattern memo, and it
/// makes the memo's soundness local to this frame rather than a consequence of trips
/// being latched somewhere else.
struct ReachFrame<I: ViewTermId> {
    op: usize,
    node: I,
    forward: bool,
    /// Whether the operator adds `node` itself — the zero-length step — to what its
    /// body reaches (`?` and `*`).
    adds_node: bool,
}

impl<I: ViewTermId> ReachFrame<I> {
    const fn new(op: usize, node: I, forward: bool) -> Self {
        Self {
            op,
            node,
            forward,
            adds_node: false,
        }
    }

    fn key(&self) -> ReachKey<I> {
        (self.op, self.node, self.forward)
    }

    fn step<D: DatasetView<Id = I> + Sync>(
        &mut self,
        returned: Option<Value<I>>,
        ctx: &PathCtx<'_, D>,
    ) -> Step<I> {
        if let Some(value) = returned {
            let mut set = value.into_set();
            if self.adds_node {
                set.insert(self.node);
            }
            return self.finish(set, ctx);
        }
        // Nothing computed from here on is a finished view of the graph: an empty set
        // stands in, and is not memoized.
        if ctx.stopped() {
            return Step::Return(Value::Reach(SetRef::new(PathSet::new(&ctx.storage))));
        }
        // `^inner` only flips the direction flag: its reach set IS `inner`'s set for the
        // opposite direction, so that memoized owner is shared instead of deep-cloning
        // endpoint storage into a second entry under the `Reverse` op's own key.
        while let PathOp::Reverse(inner) = &ctx.program.ops[self.op] {
            self.op = *inner;
            self.forward = !self.forward;
        }
        if let Some(cached) = ctx.reach_cache.borrow().get(&self.key()) {
            return Step::Return(Value::Reach(Clone::clone(cached)));
        }
        match &ctx.program.ops[self.op] {
            PathOp::Predicate(predicate) => self.finish(
                step_predicate(*predicate, self.node, self.forward, ctx),
                ctx,
            ),
            PathOp::Negated(sets) => {
                self.finish(step_negated(sets, self.node, self.forward, ctx), ctx)
            }
            PathOp::Wildcard(namespace) => {
                self.finish(step_wildcard(*namespace, self.node, self.forward, ctx), ctx)
            }
            PathOp::Sequence(_) => Step::Call(Frame::Sequence(SequenceFrame::new(
                self.op,
                self.node,
                self.forward,
                &ctx.storage,
            ))),
            PathOp::Alternative(_) => Step::Call(Frame::Alternative(AlternativeFrame::new(
                self.op,
                self.node,
                self.forward,
                &ctx.storage,
            ))),
            PathOp::ZeroOrOne(inner) => {
                self.adds_node = true;
                Step::Call(Frame::Reach(Self::new(*inner, self.node, self.forward)))
            }
            PathOp::ZeroOrMore(inner) => {
                self.adds_node = true;
                Step::Call(Frame::Closure(ClosureFrame::new(
                    *inner,
                    PathVec::from_iter([self.node], &ctx.storage),
                    self.forward,
                )))
            }
            PathOp::OneOrMore(inner) => Step::Call(Frame::Closure(ClosureFrame::new(
                *inner,
                PathVec::from_iter([self.node], &ctx.storage),
                self.forward,
            ))),
            PathOp::Range { inner, min, max } => Step::Call(Frame::Range(RangeFrame::new(
                *inner,
                self.node,
                self.forward,
                *min,
                *max,
                &ctx.storage,
            ))),
            PathOp::Reverse(_) => {
                unreachable!("every reverse at this op was resolved into the direction flag")
            }
        }
    }

    /// Return `set`, memoized under this request's key unless the budget cut it short.
    fn finish<D: DatasetView<Id = I> + Sync>(
        &self,
        set: PathSet<I>,
        ctx: &PathCtx<'_, D>,
    ) -> Step<I> {
        let result = SetRef::new(set);
        if !ctx.stopped() {
            ctx.reach_cache
                .borrow_mut()
                .insert(self.key(), Clone::clone(&result));
        }
        Step::Return(Value::Reach(result))
    }
}

/// The set a [`PathOp::Sequence`] reaches from a node: the frontier `{node}` stepped
/// through each element in turn, forward in source order and backward (predecessors)
/// in reverse order.
///
/// This is the left-nested binary chain's reach exactly. `Sequence(Sequence(a, b), c)`
/// reaches the union over `mid ∈ reach(Sequence(a, b), node)` of `reach(c, mid)`, and
/// unfolding the inner node the same way leaves `a`, `b`, `c` applied in turn to the
/// frontier each produced; backward, the binary arm stepped its right side first, which
/// unfolds to the elements in reverse. The element reaches it asks for are the ones the
/// binary tree asked for, through the same memo, so the graph is read and the budget
/// charged exactly as before. What the chain does not have is a memo entry per prefix
/// node: a backward walk whose frontiers meet at the same node for two start nodes
/// merges that node's memoized element sets again rather than reading one prefix set.
/// That is set work over sets already in memory, never another graph read, and the
/// whole chain's own entry still answers any start node it has seen.
struct SequenceFrame<I: ViewTermId> {
    op: usize,
    forward: bool,
    /// How many elements the frontier has been stepped through.
    k: usize,
    /// The frontier after `k` elements, in ascending order.
    frontier: PathVec<I>,
    /// How many frontier nodes have been stepped through the current element.
    pos: usize,
    /// What the frontier has reached so far through the current element.
    next: PathSet<I>,
}

impl<I: ViewTermId> SequenceFrame<I> {
    fn new(op: usize, node: I, forward: bool, storage: &PathStorageOwner) -> Self {
        Self {
            op,
            forward,
            k: 0,
            frontier: PathVec::from_iter([node], storage),
            pos: 0,
            next: PathSet::new(storage),
        }
    }

    fn step<D: DatasetView<Id = I> + Sync>(
        &mut self,
        returned: Option<Value<I>>,
        ctx: &PathCtx<'_, D>,
    ) -> Step<I> {
        if let Some(value) = returned {
            self.next.extend(value.into_reach().iter().copied());
            self.pos += 1;
        }
        let elements = elements_of(&ctx.program, self.op);
        loop {
            if self.k == elements.len() {
                return Step::Return(Value::Built(PathSet::from_iter(
                    self.frontier.iter().copied(),
                    &ctx.storage,
                )));
            }
            // A frontier that died stays dead: no later element can reach anything from it.
            if self.frontier.is_empty() {
                return Step::Return(Value::Built(PathSet::new(&ctx.storage)));
            }
            if let Some(&mid) = self.frontier.get(self.pos) {
                let element = elements[if self.forward {
                    self.k
                } else {
                    elements.len() - 1 - self.k
                }];
                return Step::Call(Frame::Reach(ReachFrame::new(element, mid, self.forward)));
            }
            self.frontier = self.next.take().into_vec();
            self.pos = 0;
            self.k += 1;
        }
    }
}

/// The set a [`PathOp::Alternative`] reaches from a node: each element's set in source
/// order, unioned.
struct AlternativeFrame<I: ViewTermId> {
    op: usize,
    node: I,
    forward: bool,
    /// How many elements have been asked.
    i: usize,
    out: PathSet<I>,
}

impl<I: ViewTermId> AlternativeFrame<I> {
    fn new(op: usize, node: I, forward: bool, storage: &PathStorageOwner) -> Self {
        Self {
            op,
            node,
            forward,
            i: 0,
            out: PathSet::new(storage),
        }
    }

    fn step<D: DatasetView<Id = I> + Sync>(
        &mut self,
        returned: Option<Value<I>>,
        ctx: &PathCtx<'_, D>,
    ) -> Step<I> {
        if let Some(value) = returned {
            self.out.extend(value.into_reach().iter().copied());
            self.i += 1;
        }
        match elements_of(&ctx.program, self.op).get(self.i) {
            Some(&element) => Step::Call(Frame::Reach(ReachFrame::new(
                element,
                self.node,
                self.forward,
            ))),
            None => Step::Return(Value::Built(self.out.take())),
        }
    }
}

/// The one-or-more transitive closure of `inner` from every seed, in one joint
/// traversal: every node reachable by applying `inner` at least once from any seed.
/// The visited set guards the endpoint frontier so cyclic graphs terminate; a seed
/// itself appears iff it is reachable from a seed via a cycle (the SPARQL `+`
/// behaviour). Over several seeds this equals the union of each seed's closure, and
/// visits each node at most once (O(V+E), not O(|seeds|·(V+E))).
///
/// `result` stays ordered — it is the returned set, and its iteration
/// order determines solution-row order — while `visited` is a membership-only guard,
/// never iterated into output, so it uses the fixed-key admitted table.
struct ClosureFrame<I: ViewTermId> {
    inner: usize,
    forward: bool,
    /// The seeds whose one-step reaches make the initial frontier, in ascending order.
    seeds: PathVec<I>,
    /// How many seeds have been expanded into the frontier.
    seeded: usize,
    /// Whether every seed is expanded and the walk over the frontier has begun.
    walking: bool,
    frontier: PathVec<I>,
    visited: Visited<I>,
    result: PathSet<I>,
}

impl<I: ViewTermId> ClosureFrame<I> {
    fn new(inner: usize, seeds: PathVec<I>, forward: bool) -> Self {
        let storage = seeds.storage.clone();
        Self {
            inner,
            forward,
            seeds,
            seeded: 0,
            walking: false,
            frontier: PathVec::new(&storage),
            visited: Visited::new(&storage),
            result: PathSet::new(&storage),
        }
    }

    fn step<D: DatasetView<Id = I> + Sync>(
        &mut self,
        returned: Option<Value<I>>,
        ctx: &PathCtx<'_, D>,
    ) -> Step<I> {
        if let Some(value) = returned {
            let reached = value.into_reach();
            if self.walking {
                for next in reached.iter().copied() {
                    if !self.visited.contains(&next) {
                        self.frontier.push(next);
                    }
                }
            } else {
                self.frontier.extend(reached.iter().copied());
                self.seeded += 1;
            }
        }
        if !self.walking {
            if let Some(&seed) = self.seeds.get(self.seeded) {
                return Step::Call(Frame::Reach(ReachFrame::new(
                    self.inner,
                    seed,
                    self.forward,
                )));
            }
            self.walking = true;
        }
        while let Some(n) = self.frontier.pop() {
            if !self.visited.insert(n) {
                continue;
            }
            if ctx.stopped() {
                break;
            }
            self.result.insert(n);
            return Step::Call(Frame::Reach(ReachFrame::new(self.inner, n, self.forward)));
        }
        Step::Return(Value::Built(self.result.take()))
    }
}

/// Where a [`RangeFrame`] stands.
#[derive(Clone, Copy)]
enum RangePhase {
    /// Before the first step.
    Start,
    /// Walking the prefix one level at a time.
    Linear,
    /// Composing the prefix from binary relation powers.
    Binary,
    /// Accumulating the levels in `[min, max]`.
    Accumulate,
    /// Awaiting the `*` closure of the exactly-`min` frontier, for an open tail.
    Tail,
}

/// `inner{min,max}` — the union over `k ∈ [min, max]` of the nodes reachable in
/// **exactly** `k` applications of `inner`. The per-level frontier is a fresh set, so a
/// node reachable at several repetition counts remains present in the set result.
/// `max == None` (`{n,}`) applies `inner` exactly `min` times and then takes the `*`
/// closure of that frontier.
///
/// The prefix cannot be walked literally: `min` is a `u32`, and a hostile
/// `p{4000000000}` must not perform four billion graph levels. Above
/// [`LINEAR_RANGE_PREFIX`] the prefix is composed from binary relation powers (power `b`
/// denotes `inner^(2^b)`), applied for the set bits of `min`. For a reachable node
/// universe `V`, the power memo has at most `|V| * 32` entries, every entry is a subset
/// of `V`, and composition is polynomial (`O(|V|^3 log min)` time, `O(|V|^2 log min)`
/// stored ids), rather than cycle detection over the exponential `2^|V|` space of
/// frontier sets.
///
/// Once level `min` is reached, the accumulating walk is already graph-bounded. If a
/// level adds no new node to the union, distributivity of relation composition proves no
/// later level can add one either; every continuing level therefore adds a member of `V`.
struct RangeFrame<I: ViewTermId> {
    inner: usize,
    node: I,
    forward: bool,
    min: u32,
    max: Option<u32>,
    /// The frontier at the level the walk stands at.
    current: PathSet<I>,
    out: PathSet<I>,
    /// Levels left to walk in the linear prefix.
    left: u32,
    /// The part of `min` the binary prefix has not applied yet, and the power its lowest
    /// bit denotes.
    remaining: u32,
    bit: u32,
    /// The level `current` stands at during accumulation.
    level: u32,
    /// The binary prefix's power memo, shared with the frames that fill it.
    powers: Option<PowerMemoOwner<I>>,
    phase: RangePhase,
}

/// Small prefixes are cheaper to walk directly than to populate a power memo. Above this
/// constant, the direct lane's query-text bound gives way to the polynomial relation lane.
const LINEAR_RANGE_PREFIX: u32 = 64;

impl<I: ViewTermId> RangeFrame<I> {
    fn new(
        inner: usize,
        node: I,
        forward: bool,
        min: u32,
        max: Option<u32>,
        storage: &PathStorageOwner,
    ) -> Self {
        Self {
            inner,
            node,
            forward,
            min,
            max,
            current: PathSet::new(storage),
            out: PathSet::new(storage),
            left: 0,
            remaining: 0,
            bit: 0,
            level: 0,
            powers: None,
            phase: RangePhase::Start,
        }
    }

    /// Return what has been accumulated.
    fn finished(&mut self) -> Step<I> {
        Step::Return(Value::Built(self.out.take()))
    }

    /// The prefix died or was stopped: no level at or above `min` is reachable, and the
    /// whole range is empty.
    fn died(&self) -> Step<I> {
        Step::Return(Value::Built(PathSet::new(&self.out.values.storage)))
    }

    /// One frontier advance from `current`.
    fn advance(&mut self) -> Step<I> {
        Step::Call(Frame::StepLevel(StepLevelFrame::new(
            self.inner,
            self.current.take(),
            self.forward,
        )))
    }

    fn step(&mut self, returned: Option<Value<I>>) -> Step<I> {
        match returned {
            Some(value) => {
                if let Some(early) = self.resume(value) {
                    return early;
                }
            }
            None => {
                // An empty repetition window (`max < min`) admits no `k` at all.
                if self.max.is_some_and(|m| m < self.min) {
                    return self.died();
                }
                self.current.insert(self.node);
                if self.min <= LINEAR_RANGE_PREFIX {
                    self.left = self.min;
                    self.phase = RangePhase::Linear;
                } else {
                    self.remaining = self.min;
                    self.bit = 0;
                    self.phase = RangePhase::Binary;
                }
            }
        }
        self.drive()
    }

    /// Take in the value of the frame this one last called. `Some` ends the frame with
    /// that step.
    fn resume(&mut self, value: Value<I>) -> Option<Step<I>> {
        match self.phase {
            // A prefix level that died or was stopped ends the range: no level at or
            // above `min` is reachable either.
            RangePhase::Linear => {
                let Some(next) = value.into_level().filter(|next| !next.is_empty()) else {
                    return Some(self.died());
                };
                self.current = next;
                self.left -= 1;
            }
            RangePhase::Binary => {
                let Some(next) = value.into_level().filter(|next| !next.is_empty()) else {
                    return Some(self.died());
                };
                self.current = next;
                self.remaining >>= 1;
                self.bit += 1;
            }
            RangePhase::Accumulate => {
                // The budget stopped the walk mid-level: what has been reached so far is
                // a subset of the truly reachable nodes, and is the answer.
                let Some(next) = value.into_level() else {
                    return Some(self.finished());
                };
                self.current = next;
                self.level += 1;
            }
            RangePhase::Tail => {
                self.out.extend(value.into_set());
                return Some(self.finished());
            }
            RangePhase::Start => unreachable!("nothing is requested before the first step"),
        }
        None
    }

    /// Run the phase the frame stands in up to its next request or its end.
    fn drive(&mut self) -> Step<I> {
        loop {
            match self.phase {
                // The prefix walk: reach level `min`. Levels below it contribute nothing
                // to the output.
                RangePhase::Linear => {
                    if self.left != 0 {
                        return self.advance();
                    }
                    self.level = self.min;
                    self.phase = RangePhase::Accumulate;
                }
                RangePhase::Binary => {
                    if self.remaining == 0 {
                        self.level = self.min;
                        self.phase = RangePhase::Accumulate;
                    } else if self.remaining & 1 == 1 {
                        if self.powers.is_none() {
                            self.powers = power_memo(&self.out.values.storage);
                        }
                        let Some(powers) = self.powers.as_ref() else {
                            return self.died();
                        };
                        return Step::Call(Frame::ApplyPower(ApplyPowerFrame::new(
                            self.inner,
                            self.current.take(),
                            self.forward,
                            self.bit,
                            Clone::clone(powers),
                        )));
                    } else {
                        self.remaining >>= 1;
                        self.bit += 1;
                    }
                }
                // The accumulating walk: union the levels in `[min, max]`.
                RangePhase::Accumulate => {
                    let before = self.out.len();
                    self.out.extend(self.current.iter().copied());
                    let grew = self.out.len() != before;
                    match self.max {
                        // The window closes at this level.
                        Some(m) if self.level >= m => return self.finished(),
                        // Unbounded tail: `*`-close from the exactly-`min` frontier in a
                        // single joint traversal. Only ever reached at `level == min`,
                        // since this arm always finishes.
                        None => {
                            self.phase = RangePhase::Tail;
                            return Step::Call(Frame::Closure(ClosureFrame::new(
                                self.inner,
                                self.current.take().into_vec(),
                                self.forward,
                            )));
                        }
                        Some(_) => {}
                    }
                    // This level contributed nothing, so no later level can either.
                    if !grew {
                        return self.finished();
                    }
                    return self.advance();
                }
                RangePhase::Start | RangePhase::Tail => {
                    unreachable!("the range frame drives only its prefix and accumulation phases")
                }
            }
        }
    }
}

/// One frontier advance: the nodes reachable in one further application of `inner` from
/// every node of `current`. Answers [`Value::Level`]: `None` means the execution's budget
/// stopped the walk mid-level, which every caller treats as "return what has been reached
/// so far" — a partially expanded frontier reaches a subset of the truly reachable nodes.
struct StepLevelFrame<I: ViewTermId> {
    inner: usize,
    forward: bool,
    /// The frontier being advanced, in ascending order.
    current: PathVec<I>,
    /// How many of its nodes have been expanded.
    pos: usize,
    next: PathSet<I>,
}

impl<I: ViewTermId> StepLevelFrame<I> {
    fn new(inner: usize, current: PathSet<I>, forward: bool) -> Self {
        let storage = current.values.storage.clone();
        Self {
            inner,
            forward,
            current: current.into_vec(),
            pos: 0,
            next: PathSet::new(&storage),
        }
    }

    fn step<D: DatasetView<Id = I> + Sync>(
        &mut self,
        returned: Option<Value<I>>,
        ctx: &PathCtx<'_, D>,
    ) -> Step<I> {
        match returned {
            None => note_level_advance(),
            Some(value) => {
                let reached = value.into_reach();
                if ctx.stopped() {
                    return Step::Return(Value::Level(None));
                }
                // A cached relation still has to be composed into this level. Charge each
                // candidate edge consumed, not merely the source frontier node, so fan-out
                // is represented in deterministic fuel.
                for target in reached.iter().copied() {
                    if !ctx.charge_candidate() {
                        return Step::Return(Value::Level(None));
                    }
                    self.next.insert(target);
                }
                self.pos += 1;
            }
        }
        match self.current.get(self.pos) {
            Some(&n) => Step::Call(Frame::Reach(ReachFrame::new(self.inner, n, self.forward))),
            None => Step::Return(Value::Level(Some(self.next.take()))),
        }
    }
}

/// Apply `inner^(2^bit)` to every source in `current`. Answers [`Value::Level`], `None`
/// when the budget stopped the composition.
struct ApplyPowerFrame<I: ViewTermId> {
    inner: usize,
    forward: bool,
    bit: u32,
    /// The sources, in ascending order.
    current: PathVec<I>,
    /// How many of them have been applied.
    pos: usize,
    out: PathSet<I>,
    powers: PowerMemoOwner<I>,
}

impl<I: ViewTermId> ApplyPowerFrame<I> {
    fn new(
        inner: usize,
        current: PathSet<I>,
        forward: bool,
        bit: u32,
        powers: PowerMemoOwner<I>,
    ) -> Self {
        let storage = current.values.storage.clone();
        Self {
            inner,
            forward,
            bit,
            current: current.into_vec(),
            pos: 0,
            out: PathSet::new(&storage),
            powers,
        }
    }

    fn step<D: DatasetView<Id = I> + Sync>(
        &mut self,
        returned: Option<Value<I>>,
        ctx: &PathCtx<'_, D>,
    ) -> Step<I> {
        if let Some(value) = returned {
            let reached = value.into_reach();
            if ctx.stopped() {
                return Step::Return(Value::Level(None));
            }
            for target in reached.iter().copied() {
                if !ctx.charge_candidate() {
                    return Step::Return(Value::Level(None));
                }
                self.out.insert(target);
            }
            self.pos += 1;
        }
        match self.current.get(self.pos) {
            Some(&source) => Step::Call(Frame::PowerReach(PowerReachFrame::new(
                self.inner,
                source,
                self.forward,
                self.bit,
                Clone::clone(&self.powers),
            ))),
            None => Step::Return(Value::Level(Some(self.out.take()))),
        }
    }
}

/// Where a [`PowerReachFrame`] stands.
enum PowerPhase<I: ViewTermId> {
    /// Before the first step.
    Start,
    /// Awaiting `inner`'s own reach (power zero).
    Base,
    /// Awaiting the first half, `inner^(2^(bit-1))` from the node.
    First,
    /// Composing the second half from each node of the first: `mids` are those nodes,
    /// `pos` how many have been composed, `out` what they reach.
    Mids {
        mids: PathVec<I>,
        pos: usize,
        out: PathSet<I>,
    },
}

/// The nodes reachable from `node` by exactly `2^bit` applications of `inner`.
///
/// Each complete entry is memoized in the power memo. A governor-cut entry is returned
/// only to its caller and never cached as though it were the full relation.
struct PowerReachFrame<I: ViewTermId> {
    inner: usize,
    node: I,
    forward: bool,
    bit: u32,
    powers: PowerMemoOwner<I>,
    phase: PowerPhase<I>,
}

impl<I: ViewTermId> PowerReachFrame<I> {
    const fn new(
        inner: usize,
        node: I,
        forward: bool,
        bit: u32,
        powers: PowerMemoOwner<I>,
    ) -> Self {
        Self {
            inner,
            node,
            forward,
            bit,
            powers,
            phase: PowerPhase::Start,
        }
    }

    /// The request for `inner^(2^(bit-1))` from `node`.
    fn half(&self, node: I) -> Step<I> {
        Step::Call(Frame::PowerReach(Self::new(
            self.inner,
            node,
            self.forward,
            self.bit - 1,
            Clone::clone(&self.powers),
        )))
    }

    /// Return `reached`, memoized unless the budget cut it short.
    fn finish<D: DatasetView<Id = I> + Sync>(
        &self,
        reached: SetRef<I>,
        ctx: &PathCtx<'_, D>,
    ) -> Step<I> {
        if !ctx.stopped() {
            self.powers
                .borrow_mut()
                .insert((self.bit, self.node), Clone::clone(&reached));
        }
        Step::Return(Value::Reach(reached))
    }

    /// The first step: the memo probe, then the request for the base relation or the
    /// first half.
    fn start(&mut self) -> Step<I> {
        if let Some(cached) = self.powers.borrow().get(&(self.bit, self.node)) {
            return Step::Return(Value::Reach(Clone::clone(cached)));
        }
        note_power_expansion();
        if self.bit == 0 {
            self.phase = PowerPhase::Base;
            return Step::Call(Frame::Reach(ReachFrame::new(
                self.inner,
                self.node,
                self.forward,
            )));
        }
        self.phase = PowerPhase::First;
        self.half(self.node)
    }

    fn step<D: DatasetView<Id = I> + Sync>(
        &mut self,
        returned: Option<Value<I>>,
        ctx: &PathCtx<'_, D>,
    ) -> Step<I> {
        let Some(value) = returned else {
            return self.start();
        };
        let reached = value.into_reach();
        match &mut self.phase {
            PowerPhase::Base => return self.finish(reached, ctx),
            PowerPhase::First => {
                if ctx.stopped() {
                    return Step::Return(Value::Reach(SetRef::new(PathSet::new(&ctx.storage))));
                }
                self.phase = PowerPhase::Mids {
                    mids: PathVec::from_iter(reached.iter().copied(), &ctx.storage),
                    pos: 0,
                    out: PathSet::new(&ctx.storage),
                };
            }
            PowerPhase::Mids { pos, out, .. } => {
                if ctx.stopped() {
                    return Step::Return(Value::Reach(SetRef::new(out.take())));
                }
                for target in reached.iter().copied() {
                    if !ctx.charge_candidate() {
                        return Step::Return(Value::Reach(SetRef::new(out.take())));
                    }
                    out.insert(target);
                }
                *pos += 1;
            }
            PowerPhase::Start => unreachable!("nothing is requested before the first step"),
        }
        let PowerPhase::Mids { mids, pos, out } = &mut self.phase else {
            unreachable!("the second half is composed only after the first is known")
        };
        match mids.get(*pos) {
            Some(&mid) => self.half(mid),
            None => {
                let reached = SetRef::new(out.take());
                self.finish(reached, ctx)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The bag machine (multiset semantics)
// ---------------------------------------------------------------------------

/// The MULTISET of nodes `y` such that `(node,y)` (forward) or `(y,node)` (backward) is
/// in `op`'s relation, for a path with NO repetition operator (see
/// [`PathProgram::has_repetition`]) — one entry per distinct underlying triple
/// combination, so a `Sequence`/`Alternative` that can be satisfied several ways yields
/// that many entries. Deterministic: every leaf step iterates the dataset's `TermId`
/// order (via the same `step_*` primitives [`reach`] uses) and `Sequence`/`Alternative`
/// compose that order structurally, so row order is stable run-to-run. Never asked of
/// a path containing repetition (`eval_path` reads the program first).
///
/// The same work-list machine as [`reach`], over [`BagFrame`]s and with no memo.
fn bag_reach<D: DatasetView + Sync>(
    op: usize,
    node: D::Id,
    forward: bool,
    ctx: &PathCtx<'_, D>,
) -> Result<crate::AdmittedVec<D::Id>, EvalError> {
    let mut frames = crate::AdmittedVec::new(&ctx.growth);
    frames.push(BagFrame::Reach { op, node, forward })?;
    let mut returned = None;
    loop {
        ctx.storage.check()?;
        let top = frames
            .as_mut_slice()
            .last_mut()
            .expect("the root frame's return ends the loop before the list empties");
        let step = top.step(returned.take(), ctx)?;
        ctx.storage.check()?;
        match step {
            BagStep::Call(frame) => frames.push(frame)?,
            BagStep::Replace(frame) => *top = frame,
            BagStep::Return(bag) => {
                let _ = frames.pop();
                if frames.is_empty() {
                    return Ok(bag);
                }
                returned = Some(bag);
            }
        }
    }
}

/// What a [`BagFrame`] asks of the machine after one step.
enum BagStep<I: ViewTermId> {
    /// Run `frame` to completion, then resume this frame with its bag.
    Call(BagFrame<I>),
    /// This request becomes `frame`, which answers in its place.
    Replace(BagFrame<I>),
    /// This frame is finished with this bag.
    Return(crate::AdmittedVec<I>),
}

/// One suspended operator of the bag machine.
enum BagFrame<I: ViewTermId> {
    /// A request: the derivations of `op` from `node`.
    Reach { op: usize, node: I, forward: bool },
    /// The frontier `[node]` stepped through each element in turn — forward in source
    /// order, backward in reverse, the direction swap the set machine's sequence
    /// applies — keeping one entry per derivation. Stepping every entry of a frontier in
    /// order and concatenating what each reaches is the left-nested binary chain's
    /// nested loops unfolded: the same entries, with the same multiplicities, in the
    /// same order (each derivation ordered by its intermediate nodes, first hop first).
    Sequence {
        op: usize,
        forward: bool,
        k: usize,
        frontier: crate::AdmittedVec<I>,
        pos: usize,
        next: crate::AdmittedVec<I>,
    },
    /// Each element's derivations in turn, in source order: the left-nested chain's bag
    /// union, `(a ⊎ b) ⊎ c`, concatenated the same way.
    Alternative {
        op: usize,
        node: I,
        forward: bool,
        i: usize,
        out: crate::AdmittedVec<I>,
    },
}

impl<I: ViewTermId> BagFrame<I> {
    fn step<D: DatasetView<Id = I> + Sync>(
        &mut self,
        returned: Option<crate::AdmittedVec<I>>,
        ctx: &PathCtx<'_, D>,
    ) -> Result<BagStep<I>, EvalError> {
        Ok(match self {
            Self::Reach { op, node, forward } => Self::request(op, *node, forward, ctx)?,
            Self::Sequence {
                op,
                forward,
                k,
                frontier,
                pos,
                next,
            } => {
                if let Some(bag) = returned {
                    for node in bag {
                        next.push(node)?;
                    }
                    *pos += 1;
                }
                let elements = elements_of(&ctx.program, *op);
                loop {
                    if *k == elements.len() {
                        return Ok(BagStep::Return(std::mem::replace(
                            frontier,
                            crate::AdmittedVec::new(&ctx.growth),
                        )));
                    }
                    if frontier.is_empty() {
                        return Ok(BagStep::Return(crate::AdmittedVec::new(&ctx.growth)));
                    }
                    if let Some(&mid) = frontier.get(*pos) {
                        let element = elements[if *forward {
                            *k
                        } else {
                            elements.len() - 1 - *k
                        }];
                        return Ok(BagStep::Call(Self::Reach {
                            op: element,
                            node: mid,
                            forward: *forward,
                        }));
                    }
                    *frontier = std::mem::replace(next, crate::AdmittedVec::new(&ctx.growth));
                    *pos = 0;
                    *k += 1;
                }
            }
            Self::Alternative {
                op,
                node,
                forward,
                i,
                out,
            } => {
                if let Some(bag) = returned {
                    for node in bag {
                        out.push(node)?;
                    }
                    *i += 1;
                }
                match elements_of(&ctx.program, *op).get(*i) {
                    Some(&element) => BagStep::Call(Self::Reach {
                        op: element,
                        node: *node,
                        forward: *forward,
                    }),
                    None => BagStep::Return(std::mem::replace(
                        out,
                        crate::AdmittedVec::new(&ctx.growth),
                    )),
                }
            }
        })
    }

    /// Answer a request: a leaf is stepped here; a sequence or alternative becomes its
    /// operator frame. Every `^` at the op is resolved into the direction flag first.
    fn request<D: DatasetView<Id = I> + Sync>(
        op: &mut usize,
        node: I,
        forward: &mut bool,
        ctx: &PathCtx<'_, D>,
    ) -> Result<BagStep<I>, EvalError> {
        while let PathOp::Reverse(inner) = &ctx.program.ops[*op] {
            *op = *inner;
            *forward = !*forward;
        }
        let leaf = match &ctx.program.ops[*op] {
            PathOp::Predicate(predicate) => Some(step_predicate(*predicate, node, *forward, ctx)),
            PathOp::Negated(sets) => Some(step_negated(sets, node, *forward, ctx)),
            PathOp::Wildcard(namespace) => Some(step_wildcard(*namespace, node, *forward, ctx)),
            _ => None,
        };
        if let Some(leaf) = leaf {
            ctx.storage.check()?;
            return Ok(BagStep::Return(leaf.into_vec().into_admitted()));
        }
        Ok(match &ctx.program.ops[*op] {
            PathOp::Sequence(_) => BagStep::Replace(Self::Sequence {
                op: *op,
                forward: *forward,
                k: 0,
                frontier: {
                    let mut frontier = crate::AdmittedVec::with_capacity(1, &ctx.growth)?;
                    frontier.push(node)?;
                    frontier
                },
                pos: 0,
                next: crate::AdmittedVec::new(&ctx.growth),
            }),
            PathOp::Alternative(_) => BagStep::Replace(Self::Alternative {
                op: *op,
                node,
                forward: *forward,
                i: 0,
                out: crate::AdmittedVec::new(&ctx.growth),
            }),
            PathOp::ZeroOrMore(_)
            | PathOp::OneOrMore(_)
            | PathOp::ZeroOrOne(_)
            | PathOp::Range { .. } => unreachable!(
                "a bag traversal is asked only of a path with no repetition operator; \
                 eval_path reads the program first"
            ),
            PathOp::Reverse(_) => {
                unreachable!("every reverse at this op was resolved into the direction flag")
            }
            PathOp::Predicate(_) | PathOp::Negated(_) | PathOp::Wildcard(_) => {
                unreachable!("leaf paths returned above")
            }
        })
    }
}

// ---------------------------------------------------------------------------
// Leaf steps
// ---------------------------------------------------------------------------

/// One predicate hop. Forward: objects of `(node, p, ?)`; backward: subjects of
/// `(?, p, node)`. A predicate absent from the dataset (`None`) yields nothing.
fn step_predicate<D: DatasetView + Sync>(
    predicate: Option<D::Id>,
    node: D::Id,
    forward: bool,
    ctx: &PathCtx<'_, D>,
) -> PathSet<D::Id> {
    let Some(pid) = predicate else {
        return PathSet::new(&ctx.storage);
    };
    let mut out = PathSet::new(&ctx.storage);
    let mut stopped = false;
    if forward {
        ctx.scope
            .for_each_quad(ctx.dataset, Some(node), Some(pid), None, |q| {
                if stopped {
                    return;
                }
                if !ctx.charge_candidate() {
                    stopped = true;
                    return;
                }
                out.insert(q.o);
            });
    } else {
        ctx.scope
            .for_each_quad(ctx.dataset, None, Some(pid), Some(node), |q| {
                if stopped {
                    return;
                }
                if !ctx.charge_candidate() {
                    stopped = true;
                    return;
                }
                out.insert(q.s);
            });
    }
    out
}

/// `!(p1|…|^q1|…)`: one hop along any predicate NOT in the excluded set, per
/// direction (SPARQL 1.1 §18.3). The plain elements exclude a **forward** hop;
/// the `^`-elements exclude a **reverse** hop; the two contributions are
/// unioned, and a direction with no listed elements is omitted entirely (see
/// [`NegatedSets`]). Reads the sets the program resolved once, so no excluded IRI
/// is looked up on a traversal step.
fn step_negated<D: DatasetView + Sync>(
    sets: &NegatedSets<D::Id>,
    node: D::Id,
    forward: bool,
    ctx: &PathCtx<'_, D>,
) -> PathSet<D::Id> {
    let mut out = PathSet::new(&ctx.storage);
    if let Some(excluded) = &sets.forward {
        out.extend(step_excluding(excluded, node, forward, ctx));
    }
    if let Some(excluded) = &sets.inverse {
        out.extend(step_excluding(excluded, node, !forward, ctx));
    }
    out
}

/// One hop along any predicate NOT in `excluded`, in the given direction —
/// the direction-parameterised primitive `step_negated` composes twice (once
/// per element kind) to get the full negated-set relation.
fn step_excluding<D: DatasetView + Sync>(
    excluded: &[D::Id],
    node: D::Id,
    forward: bool,
    ctx: &PathCtx<'_, D>,
) -> PathSet<D::Id> {
    let mut out = PathSet::new(&ctx.storage);
    let mut stopped = false;
    if forward {
        ctx.scope
            .for_each_quad(ctx.dataset, Some(node), None, None, |q| {
                if stopped {
                    return;
                }
                if !ctx.charge_candidate() {
                    stopped = true;
                    return;
                }
                if !excluded.contains(&q.p) {
                    out.insert(q.o);
                }
            });
    } else {
        ctx.scope
            .for_each_quad(ctx.dataset, None, None, Some(node), |q| {
                if stopped {
                    return;
                }
                if !ctx.charge_candidate() {
                    stopped = true;
                    return;
                }
                if !excluded.contains(&q.p) {
                    out.insert(q.s);
                }
            });
    }
    out
}

/// `<any>` / `<any:ns>`: one hop along any predicate, optionally restricted to
/// predicates whose IRI begins with the namespace prefix.
fn step_wildcard<D: DatasetView + Sync>(
    namespace: Option<&NamedNode>,
    node: D::Id,
    forward: bool,
    ctx: &PathCtx<'_, D>,
) -> PathSet<D::Id> {
    let prefix = namespace.map(NamedNode::as_str);
    let pred_ok = |pid: D::Id| -> bool {
        match prefix {
            None => true,
            Some(pfx) => ctx
                .dataset
                .with_term(
                    pid,
                    |term| matches!(term, TermRef::Iri(iri) if iri.starts_with(pfx)),
                )
                .unwrap_or(false),
        }
    };
    let mut out = PathSet::new(&ctx.storage);
    let mut stopped = false;
    if forward {
        ctx.scope
            .for_each_quad(ctx.dataset, Some(node), None, None, |q| {
                if stopped {
                    return;
                }
                if !ctx.charge_candidate() {
                    stopped = true;
                    return;
                }
                if pred_ok(q.p) {
                    out.insert(q.o);
                }
            });
    } else {
        ctx.scope
            .for_each_quad(ctx.dataset, None, None, Some(node), |q| {
                if stopped {
                    return;
                }
                if !ctx.charge_candidate() {
                    stopped = true;
                    return;
                }
                if pred_ok(q.p) {
                    out.insert(q.s);
                }
            });
    }
    out
}

#[cfg(test)]
thread_local! {
    /// Test-only instrumentation: the number of range-path frontier advances performed on
    /// this thread since [`counting_level_advances`] last reset it.
    static LEVEL_ADVANCES: Cell<u64> = const { Cell::new(0) };
    /// Test-only count of distinct `(power bit, source node)` relation entries computed.
    static POWER_EXPANSIONS: Cell<u64> = const { Cell::new(0) };
}

/// Record one range-path frontier advance.
///
/// Compiled to nothing outside `cfg(test)`. The bound on [`range_reach`]'s level count is
/// a structural property of the algorithm, and pinning a structural property by test needs
/// a counter — a wall-clock timeout would only say "it finished on this machine today",
/// which is exactly the assertion a regression can pass by being slow instead of infinite.
#[inline]
fn note_level_advance() {
    #[cfg(test)]
    LEVEL_ADVANCES.with(|advances| advances.set(advances.get().saturating_add(1)));
}

/// Record one binary-power cache miss. Compiled out of production builds.
#[inline]
fn note_power_expansion() {
    #[cfg(test)]
    POWER_EXPANSIONS.with(|expansions| expansions.set(expansions.get().saturating_add(1)));
}

/// Run `body`, returning its value with the number of range-path frontier advances it
/// performed on this thread.
#[cfg(test)]
fn counting_level_advances<T>(body: impl FnOnce() -> T) -> (T, u64) {
    LEVEL_ADVANCES.with(|advances| advances.set(0));
    let value = body();
    (value, LEVEL_ADVANCES.with(Cell::get))
}

/// Run `body`, returning its value with the number of distinct binary relation entries it
/// computed on this thread.
#[cfg(test)]
fn counting_power_expansions<T>(body: impl FnOnce() -> T) -> (T, u64) {
    POWER_EXPANSIONS.with(|expansions| expansions.set(0));
    let value = body();
    (value, POWER_EXPANSIONS.with(Cell::get))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::governor::{GovernorState, QueryGovernors};
    use purrdf_core::{RdfDataset, RdfDatasetBuilder, ResourceDimension, TrippedGovernor};
    use purrdf_sparql_algebra::{Chain, Child};
    use purrdf_sparql_algebra::{NamedNode, TriplePattern};

    const EX: &str = "http://ex/";

    fn iri(local: &str) -> String {
        format!("{EX}{local}")
    }

    /// Build a directed graph over predicate `local`-named edges. Each edge is a
    /// `(subject_local, predicate_local, object_local)` triple.
    fn graph_of(edges: &[(&str, &str, &str)]) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        for (s, p, o) in edges {
            let s = b.intern_iri(&iri(s));
            let p = b.intern_iri(&iri(p));
            let o = b.intern_iri(&iri(o));
            b.push_quad(s, p, o, None);
        }
        b.freeze().expect("freeze")
    }

    fn nn(local: &str) -> NamedNode {
        NamedNode::new_unchecked(iri(local))
    }

    fn named(local: &str) -> PropertyPathExpression {
        PropertyPathExpression::NamedNode(nn(local))
    }

    /// A negated-property-set element: `npe("p", false)` is the plain `:p`,
    /// `npe("p", true)` is the inverted `^:p`.
    fn npe(local: &str, inverse: bool) -> NegatedPathElement {
        NegatedPathElement {
            predicate: nn(local),
            inverse,
        }
    }

    fn var(name: &str) -> TermPattern {
        TermPattern::Variable(Variable::new(name))
    }

    fn ground(local: &str) -> TermPattern {
        TermPattern::NamedNode(nn(local))
    }

    /// Resolve a dataset id to its IRI local name (tests use only IRIs).
    fn local_of(ds: &RdfDataset, id: TermId) -> String {
        match ds.resolve(id) {
            TermRef::Iri(s) => s.strip_prefix(EX).unwrap_or(s).to_owned(),
            other => format!("{other:?}"),
        }
    }

    /// Evaluate a path and materialise the named columns as local-name rows, sorted
    /// for order-insensitive multiset comparison.
    fn run(
        ds: &RdfDataset,
        subject: &TermPattern,
        path: &PropertyPathExpression,
        object: &TermPattern,
        vars: &[&str],
    ) -> Vec<Vec<Option<String>>> {
        let mut ctx = EvalCtx::new(ds);
        let seq = eval_path(subject, path, object, &mut ctx).expect("path eval");
        let cols: Vec<usize> = vars
            .iter()
            .map(|v| {
                seq.schema
                    .index_of(&Variable::new(*v))
                    .expect("var present")
            })
            .collect();
        let mut out: Vec<Vec<Option<String>>> = seq
            .rows
            .iter()
            .map(|row| {
                cols.iter()
                    .map(|&c| match row[c] {
                        Some(SolutionTerm::Existing(id)) => Some(local_of(ds, id)),
                        // A reflexive zero-length pairing on a ground endpoint absent
                        // from the dataset mints a `Computed` term for that value
                        // (see `eval_path`'s `BoundAbsent` handling) — resolve it back
                        // through the scratch interner the same way the real
                        // evaluator's result egress does.
                        Some(term @ SolutionTerm::Computed(_)) => {
                            let value = ctx.scratch.value_of(ds, term);
                            Some(match value {
                                TermValue::Iri(s) => s.strip_prefix(EX).unwrap_or(&s).to_owned(),
                                other => format!("{other:?}"),
                            })
                        }
                        None => None,
                    })
                    .collect()
            })
            .collect();
        out.sort_by_key(|row| format!("{row:?}"));
        out
    }

    /// The local-name set reachable from `start` along `path` (forward).
    fn reach_locals(
        ds: &RdfDataset,
        path: &PropertyPathExpression,
        start: &str,
        forward: bool,
    ) -> Vec<String> {
        let sid = ds
            .term_id_by_value(&named_node_to_value(&nn(start)))
            .expect("start present");
        let pctx = PathCtx::resident(
            ds,
            GraphScope::One(purrdf_core::GraphMatch::Default),
            PathProgram::compile(path, ds),
            None,
        );
        let mut v: Vec<String> = reach(ROOT_OP, sid, forward, &pctx)
            .expect("resident path reach allocation")
            .iter()
            .copied()
            .map(|id| local_of(ds, id))
            .collect();
        v.sort();
        v
    }

    fn col1(vals: &[&str]) -> Vec<Vec<Option<String>>> {
        let mut rows: Vec<Vec<Option<String>>> =
            vals.iter().map(|v| vec![Some((*v).to_owned())]).collect();
        rows.sort_by_key(|row| format!("{row:?}"));
        rows
    }

    // ---- single predicate, sequence, alternative, reverse ------------------

    #[test]
    fn named_predicate_forward_and_reverse() {
        let ds = graph_of(&[("a", "p", "b"), ("a", "p", "c")]);
        // { :a :p ?o }
        let rows = run(&ds, &ground("a"), &named("p"), &var("o"), &["o"]);
        assert_eq!(rows, col1(&["b", "c"]));
        // { :b ^:p ?s }  → inverse: ?s is anything that points to :b via :p, i.e. :a
        // (`:b ^:p ?s` ⟺ `?s :p :b`).
        let rev = PropertyPathExpression::Reverse(Child::new(named("p")));
        let rows = run(&ds, &ground("b"), &rev, &var("s"), &["s"]);
        assert_eq!(rows, col1(&["a"]));
    }

    #[test]
    fn star_fanout_charges_each_candidate_edge() {
        let ds = graph_of(&[
            ("a", "p", "n0"),
            ("a", "p", "n1"),
            ("a", "p", "n2"),
            ("a", "p", "n3"),
            ("a", "p", "n4"),
            ("a", "p", "n5"),
            ("a", "p", "n6"),
            ("a", "p", "n7"),
        ]);
        let path = named("p");
        let sid = ds
            .as_ref()
            .term_id_by_value(&named_node_to_value(&nn("a")))
            .expect("start present");
        let state = Arc::new(GovernorState::new(&QueryGovernors::UNBOUNDED.with_fuel(3)));
        let pctx = PathCtx::resident(
            &*ds,
            GraphScope::One(purrdf_core::GraphMatch::Default),
            PathProgram::compile(&path, &*ds),
            Some(Arc::clone(&state).into()),
        );

        let reached = reach(ROOT_OP, sid, true, &pctx).expect("resident path reach allocation");
        assert_eq!(reached.len(), 3, "only three candidate edges fit");
        assert_eq!(
            state.tripped(),
            Some(TrippedGovernor::Budget {
                dimension: ResourceDimension::Fuel,
                limit: 3,
                consumed: 4,
            }),
            "the fourth edge, not the single source node, crosses the budget"
        );
    }

    #[test]
    fn sequence_chains_two_predicates() {
        let ds = graph_of(&[("a", "p", "x"), ("x", "q", "b"), ("x", "q", "c")]);
        // :a :p/:q ?o → b, c
        let seq = PropertyPathExpression::Sequence(
            Chain::try_from(vec![named("p"), named("q")]).expect("two or more nodes"),
        );
        let rows = run(&ds, &ground("a"), &seq, &var("o"), &["o"]);
        assert_eq!(rows, col1(&["b", "c"]));
    }

    #[test]
    fn sequence_backward_from_object() {
        let ds = graph_of(&[("a", "p", "x"), ("x", "q", "b")]);
        // ?s :p/:q :b  → a
        let seq = PropertyPathExpression::Sequence(
            Chain::try_from(vec![named("p"), named("q")]).expect("two or more nodes"),
        );
        let rows = run(&ds, &var("s"), &seq, &ground("b"), &["s"]);
        assert_eq!(rows, col1(&["a"]));
    }

    #[test]
    fn alternative_unions_both() {
        let ds = graph_of(&[("a", "p", "b"), ("a", "q", "c")]);
        let alt = PropertyPathExpression::Alternative(
            Chain::try_from(vec![named("p"), named("q")]).expect("two or more nodes"),
        );
        let rows = run(&ds, &ground("a"), &alt, &var("o"), &["o"]);
        assert_eq!(rows, col1(&["b", "c"]));
    }

    // ---- repetition: *, +, ? -----------------------------------------------

    #[test]
    fn zero_or_more_includes_self_and_transitive() {
        // a -> b -> c -> d (chain)
        let ds = graph_of(&[("a", "p", "b"), ("b", "p", "c"), ("c", "p", "d")]);
        let star = PropertyPathExpression::ZeroOrMore(Child::new(named("p")));
        assert_eq!(
            reach_locals(&ds, &star, "a", true),
            vec!["a", "b", "c", "d"]
        );
        let plus = PropertyPathExpression::OneOrMore(Child::new(named("p")));
        assert_eq!(reach_locals(&ds, &plus, "a", true), vec!["b", "c", "d"]);
        let opt = PropertyPathExpression::ZeroOrOne(Child::new(named("p")));
        assert_eq!(reach_locals(&ds, &opt, "a", true), vec!["a", "b"]);
    }

    #[test]
    fn one_or_more_includes_start_only_via_cycle() {
        // Cyclic a -> b -> c -> a: every node is reachable from itself.
        let cyclic = graph_of(&[("a", "p", "b"), ("b", "p", "c"), ("c", "p", "a")]);
        let plus = PropertyPathExpression::OneOrMore(Child::new(named("p")));
        assert_eq!(
            reach_locals(&cyclic, &plus, "a", true),
            vec!["a", "b", "c"],
            "in a cycle, a is reachable from itself via p+"
        );
        // Acyclic chain: a is NOT reachable from itself.
        let acyclic = graph_of(&[("a", "p", "b"), ("b", "p", "c")]);
        assert_eq!(
            reach_locals(&acyclic, &plus, "a", true),
            vec!["b", "c"],
            "acyclic: a is not in a+"
        );
    }

    #[test]
    fn star_terminates_on_a_cycle() {
        let cyclic = graph_of(&[("a", "p", "b"), ("b", "p", "c"), ("c", "p", "a")]);
        let star = PropertyPathExpression::ZeroOrMore(Child::new(named("p")));
        assert_eq!(reach_locals(&cyclic, &star, "a", true), vec!["a", "b", "c"]);
    }

    #[test]
    fn composite_step_cycle_terminates_and_reports() {
        // Cycle closed by a composite step: a -p-> x -q-> a. (p/q)+ from a must
        // terminate and report a (a reaches itself in one (p/q) application).
        let ds = graph_of(&[("a", "p", "x"), ("x", "q", "a")]);
        let seq = PropertyPathExpression::Sequence(
            Chain::try_from(vec![named("p"), named("q")]).expect("two or more nodes"),
        );
        let plus = PropertyPathExpression::OneOrMore(Child::new(seq.clone()));
        assert_eq!(reach_locals(&ds, &plus, "a", true), vec!["a"]);
        let star = PropertyPathExpression::ZeroOrMore(Child::new(seq));
        assert_eq!(reach_locals(&ds, &star, "a", true), vec!["a"]);
    }

    // ---- Range {n,m} (PurRDF extension), including on cycles -----------------

    #[test]
    fn range_exact_and_bounded_on_chain() {
        // a -> b -> c -> d -> e
        let ds = graph_of(&[
            ("a", "p", "b"),
            ("b", "p", "c"),
            ("c", "p", "d"),
            ("d", "p", "e"),
        ]);
        let rng = |min, max| PropertyPathExpression::Range {
            inner: Child::new(named("p")),
            min,
            max,
        };
        // {0,2}: self + up to 2 hops.
        assert_eq!(
            reach_locals(&ds, &rng(0, Some(2)), "a", true),
            vec!["a", "b", "c"]
        );
        // {2}: exactly two hops.
        assert_eq!(reach_locals(&ds, &rng(2, Some(2)), "a", true), vec!["c"]);
        // {2,}: two or more hops (unbounded tail).
        assert_eq!(
            reach_locals(&ds, &rng(2, None), "a", true),
            vec!["c", "d", "e"]
        );
    }

    #[test]
    fn range_on_cycle_reports_nodes_at_multiple_counts() {
        // 2-cycle a <-> b: from a, k applications land on a (even k) or b (odd k).
        // p{2,4} reaches a (at 2, 4) and b (at 3) — the case a single global
        // visited-set BFS would get wrong.
        let ds = graph_of(&[("a", "p", "b"), ("b", "p", "a")]);
        let rng = PropertyPathExpression::Range {
            inner: Child::new(named("p")),
            min: 2,
            max: Some(4),
        };
        assert_eq!(reach_locals(&ds, &rng, "a", true), vec!["a", "b"]);
    }

    #[test]
    fn cyclic_graph_p_n_m_terminates_without_a_budget() {
        // `min`/`max` are `u32`, so the levels a literal reading of `p{n,m}` would walk
        // are bounded by the query text — billions of them over a graph with three nodes.
        // No governor is engaged here on purpose: an ungoverned caller is every caller
        // that has not opted in, and "the budget stops it" is not a fix for a hang.
        //
        // The bound is asserted by COUNTING frontier advances, never by a wall clock: a
        // regression that reintroduces the level-per-`k` walk fails this deterministically
        // on any machine, where a timeout would only report that today's machine was slow
        // enough or fast enough.
        let ds = graph_of(&[("a", "p", "b"), ("b", "p", "c"), ("c", "p", "a")]);
        let rng = |min, max| PropertyPathExpression::Range {
            inner: Child::new(named("p")),
            min,
            max,
        };
        let all = vec!["a".to_owned(), "b".to_owned(), "c".to_owned()];

        // A huge `max`: the accumulating phase stops at the first level that adds no node
        // the output already holds, so it walks at most one level per graph node.
        let (locals, advances) =
            counting_level_advances(|| reach_locals(&ds, &rng(0, Some(u32::MAX)), "a", true));
        assert_eq!(locals, all);
        assert!(
            advances <= 4,
            "a {{0,u32::MAX}} range over a 3-node cycle must stop within one level per \
             node, walked {advances}"
        );

        // A huge `min` with an open tail: the prefix is composed from binary relation
        // powers, not walked.
        let (locals, advances) =
            counting_level_advances(|| reach_locals(&ds, &rng(4_000_000_000, None), "a", true));
        assert_eq!(locals, all);
        assert!(
            advances <= 16,
            "a {{4000000000,}} range over a 3-node cycle must compose the prefix instead \
             of walking to it, walked {advances}"
        );

        // A huge `min` AND a huge `max`: both phases bounded at once.
        let (locals, advances) = counting_level_advances(|| {
            reach_locals(&ds, &rng(4_000_000_000, Some(u32::MAX)), "a", true)
        });
        assert_eq!(locals, all);
        assert!(
            advances <= 16,
            "a {{4000000000,u32::MAX}} range over a 3-node cycle must bound both the \
             prefix and the accumulation, walked {advances}"
        );
    }

    #[test]
    fn large_exact_range_uses_polynomial_powers_on_coprime_cycles() {
        // `a` fans into disjoint 3- and 4-cycles. The combined frontier sequence has
        // period 12; families of pairwise-coprime cycles make that frontier-set period
        // exponential in the node count, so cycle detection is not an acceptable bound.
        const NODES: u64 = 8;
        let ds = graph_of(&[
            ("a", "p", "b0"),
            ("b0", "p", "b1"),
            ("b1", "p", "b2"),
            ("b2", "p", "b0"),
            ("a", "p", "c0"),
            ("c0", "p", "c1"),
            ("c1", "p", "c2"),
            ("c2", "p", "c3"),
            ("c3", "p", "c0"),
        ]);
        let exactly = PropertyPathExpression::Range {
            inner: Child::new(named("p")),
            min: 4_000_000_000,
            max: Some(4_000_000_000),
        };

        // By hand: (4_000_000_000 - 1) mod 3 = 0 and mod 4 = 3.
        let (locals, expansions) =
            counting_power_expansions(|| reach_locals(&ds, &exactly, "a", true));
        assert_eq!(locals, vec!["b0".to_owned(), "c3".to_owned()]);
        assert!(
            expansions <= NODES * u64::from(u32::BITS),
            "one memo entry per reachable node and exponent bit is the polynomial bound; \
             computed {expansions} entries"
        );
    }

    #[test]
    fn range_path_answers_are_unchanged_by_the_early_exit() {
        // A governor may only change an outcome, never an answer — and the level bounds
        // in `range_reach` are not a governor at all: they remove levels that provably
        // recompute a set the answer already holds. The corpus below spans cyclic and
        // acyclic shapes, ranges with and without a reachable fixpoint, `{0,k}`, `{n,}`,
        // `{n,m}`, `n == m`, an empty window (`max < min`), and repetition counts far past
        // any level the walk actually performs.
        //
        // Every expectation is computed BY HAND from the exact-`k` definition
        // (`S_0 = {start}`, `S_{k+1} = step(S_k)`, answer = `⋃_{k ∈ [min,max]} S_k`), not
        // captured from this implementation: a golden captured from the code under test
        // agrees with it by construction, including when both are wrong.
        struct Case {
            edges: &'static [(&'static str, &'static str, &'static str)],
            start: &'static str,
            min: u32,
            max: Option<u32>,
            expected: &'static [&'static str],
            why: &'static str,
        }

        // a → b → c → a.  S_k = {a},{b},{c} cycling with period 3.
        const CYCLE3: &[(&str, &str, &str)] = &[("a", "p", "b"), ("b", "p", "c"), ("c", "p", "a")];
        // a → b → c → d.  S_0..S_3 = {a},{b},{c},{d}; S_4 and beyond empty.
        const CHAIN: &[(&str, &str, &str)] = &[("a", "p", "b"), ("b", "p", "c"), ("c", "p", "d")];
        // x → x, x → y.  S_0 = {x}; S_k = {x,y} for every k ≥ 1 (a level-set fixpoint).
        const SELF_LOOP: &[(&str, &str, &str)] = &[("x", "p", "x"), ("x", "p", "y")];
        // a → x1..x5 and xi → x(i+1).  S_k = {x_k … x5} for 1 ≤ k ≤ 5; S_6 empty. The
        // cumulative reachable set stops growing at level 1, but the LEVEL sets keep
        // shrinking for four more levels — the shape that makes "stop when the cumulative
        // union stops growing" an unsound rule and the accumulated-output rule a sound one.
        const FAN: &[(&str, &str, &str)] = &[
            ("a", "p", "x1"),
            ("a", "p", "x2"),
            ("a", "p", "x3"),
            ("a", "p", "x4"),
            ("a", "p", "x5"),
            ("x1", "p", "x2"),
            ("x2", "p", "x3"),
            ("x3", "p", "x4"),
            ("x4", "p", "x5"),
        ];
        // a ⇄ b, b → c.  S_0 = {a}; S_odd = {b}; S_even≥2 = {a,c} (period 2 from level 1).
        const CYCLE2_TAIL: &[(&str, &str, &str)] =
            &[("a", "p", "b"), ("b", "p", "a"), ("b", "p", "c")];

        let corpus = [
            Case {
                edges: CYCLE3,
                start: "a",
                min: 0,
                max: Some(0),
                expected: &["a"],
                why: "S_0 is the zero-length identity",
            },
            Case {
                edges: CYCLE3,
                start: "a",
                min: 1,
                max: Some(1),
                expected: &["b"],
                why: "S_1",
            },
            Case {
                edges: CYCLE3,
                start: "a",
                min: 7,
                max: Some(7),
                expected: &["b"],
                why: "7 mod 3 == 1, so S_7 == S_1",
            },
            Case {
                edges: CYCLE3,
                start: "a",
                min: 4_000_000_000,
                max: Some(4_000_000_000),
                expected: &["b"],
                why: "4000000000 mod 3 == 1, so S_4000000000 == S_1",
            },
            Case {
                edges: CYCLE3,
                start: "a",
                min: 2,
                max: Some(5),
                expected: &["a", "b", "c"],
                why: "S_2..S_5 covers every phase of the cycle",
            },
            Case {
                edges: CYCLE3,
                start: "a",
                min: 4,
                max: None,
                expected: &["a", "b", "c"],
                why: "the open tail of a cycle covers every phase",
            },
            Case {
                edges: CYCLE3,
                start: "a",
                min: 0,
                max: None,
                expected: &["a", "b", "c"],
                why: "{0,} is the reflexive-transitive closure",
            },
            Case {
                edges: CYCLE3,
                start: "a",
                min: 3,
                max: Some(1),
                expected: &[],
                why: "max < min admits no repetition count",
            },
            Case {
                edges: CHAIN,
                start: "a",
                min: 0,
                max: Some(2),
                expected: &["a", "b", "c"],
                why: "S_0 ∪ S_1 ∪ S_2",
            },
            Case {
                edges: CHAIN,
                start: "a",
                min: 2,
                max: None,
                expected: &["c", "d"],
                why: "S_2 ∪ S_3; S_4 onwards is empty",
            },
            Case {
                edges: CHAIN,
                start: "a",
                min: 5,
                max: None,
                expected: &[],
                why: "the frontier dies before level 5",
            },
            Case {
                edges: CHAIN,
                start: "a",
                min: 2,
                max: Some(10),
                expected: &["c", "d"],
                why: "a max past the end of the chain adds nothing",
            },
            Case {
                edges: CHAIN,
                start: "a",
                min: 3,
                max: Some(3),
                expected: &["d"],
                why: "S_3",
            },
            Case {
                edges: CHAIN,
                start: "a",
                min: 4,
                max: Some(4),
                expected: &[],
                why: "S_4 is empty",
            },
            Case {
                edges: SELF_LOOP,
                start: "x",
                min: 0,
                max: Some(0),
                expected: &["x"],
                why: "S_0",
            },
            Case {
                edges: SELF_LOOP,
                start: "x",
                min: 1,
                max: Some(1),
                expected: &["x", "y"],
                why: "S_1 = step({x})",
            },
            Case {
                edges: SELF_LOOP,
                start: "x",
                min: 2,
                max: Some(3),
                expected: &["x", "y"],
                why: "the level sets have reached a fixpoint by level 1",
            },
            Case {
                edges: SELF_LOOP,
                start: "x",
                min: 5,
                max: None,
                expected: &["x", "y"],
                why: "a fixpoint reached far below min",
            },
            Case {
                edges: FAN,
                start: "a",
                min: 1,
                max: Some(2),
                expected: &["x1", "x2", "x3", "x4", "x5"],
                why: "S_1 ∪ S_2",
            },
            Case {
                edges: FAN,
                start: "a",
                min: 2,
                max: Some(4),
                expected: &["x2", "x3", "x4", "x5"],
                why: "S_2 ∪ S_3 ∪ S_4 = S_2",
            },
            Case {
                edges: FAN,
                start: "a",
                min: 3,
                max: None,
                expected: &["x3", "x4", "x5"],
                why: "x1 and x2 are reachable in fewer than 3 steps only",
            },
            Case {
                edges: FAN,
                start: "a",
                min: 5,
                max: Some(5),
                expected: &["x5"],
                why: "S_5",
            },
            Case {
                edges: FAN,
                start: "a",
                min: 6,
                max: None,
                expected: &[],
                why: "S_6 is empty",
            },
            Case {
                edges: CYCLE2_TAIL,
                start: "a",
                min: 2,
                max: Some(2),
                expected: &["a", "c"],
                why: "S_2 = step({b})",
            },
            Case {
                edges: CYCLE2_TAIL,
                start: "a",
                min: 3,
                max: None,
                expected: &["a", "b", "c"],
                why: "the tail alternates {b} and {a,c}",
            },
            Case {
                edges: CYCLE2_TAIL,
                start: "a",
                min: 0,
                max: Some(3),
                expected: &["a", "b", "c"],
                why: "S_0 ∪ S_1 ∪ S_2 ∪ S_3",
            },
            Case {
                edges: CYCLE2_TAIL,
                start: "a",
                min: 1_000_000_001,
                max: Some(1_000_000_001),
                expected: &["b"],
                why: "an odd level past the pre-period is S_1",
            },
            Case {
                edges: CYCLE2_TAIL,
                start: "a",
                min: 1_000_000_000,
                max: Some(1_000_000_000),
                expected: &["a", "c"],
                why: "an even level past the pre-period is S_2",
            },
            Case {
                edges: CHAIN,
                start: "b",
                min: 0,
                max: None,
                expected: &["b", "c", "d"],
                why: "a start part-way along the chain",
            },
            Case {
                edges: CHAIN,
                start: "d",
                min: 1,
                max: None,
                expected: &[],
                why: "a sink node reaches nothing in one or more steps",
            },
        ];

        for case in &corpus {
            let ds = graph_of(case.edges);
            let rng = PropertyPathExpression::Range {
                inner: Child::new(named("p")),
                min: case.min,
                max: case.max,
            };
            let expected: Vec<String> = case.expected.iter().map(|s| (*s).to_owned()).collect();
            assert_eq!(
                reach_locals(&ds, &rng, case.start, true),
                expected,
                "p{{{},{:?}}} from {}: {}",
                case.min,
                case.max,
                case.start,
                case.why
            );
        }
    }

    // ---- negated property set & wildcard -----------------------------------

    #[test]
    fn negated_property_set_excludes_named() {
        let ds = graph_of(&[("a", "p", "b"), ("a", "q", "c"), ("a", "r", "d")]);
        // !(:p|:q) → only the :r edge.
        let neg =
            PropertyPathExpression::NegatedPropertySet(vec![npe("p", false), npe("q", false)]);
        let rows = run(&ds, &ground("a"), &neg, &var("o"), &["o"]);
        assert_eq!(rows, col1(&["d"]));
    }

    #[test]
    fn negated_property_set_pure_inverse() {
        // W3C nps_inverse: !^:pr — from :od, the only reverse-non-:pr edge is
        // :sd :pd :od (^:pd), i.e. the forward-listed part is EMPTY and must be
        // omitted from the union entirely (not "match every forward edge").
        let ds = graph_of(&[("sd", "pd", "od"), ("sr", "pr", "or")]);
        let neg = PropertyPathExpression::NegatedPropertySet(vec![npe("pr", true)]);
        let rows = run(&ds, &var("s"), &neg, &var("o"), &["s", "o"]);
        assert_eq!(
            rows,
            vec![vec![Some("od".to_owned()), Some("sd".to_owned())]]
        );
    }

    #[test]
    fn negated_property_set_direct_and_inverse() {
        // W3C nps_direct_and_inverse: !(:pd|^:pr) decomposes into
        // Alternative(NegatedPropertySet([:pd]), Reverse(NegatedPropertySet([:pr]))).
        let ds = graph_of(&[("sd", "pd", "od"), ("sr", "pr", "or")]);
        let neg =
            PropertyPathExpression::NegatedPropertySet(vec![npe("pd", false), npe("pr", true)]);
        let rows = run(&ds, &var("s"), &neg, &var("o"), &["s", "o"]);
        let mut expected = vec![
            vec![Some("sr".to_owned()), Some("or".to_owned())],
            vec![Some("od".to_owned()), Some("sd".to_owned())],
        ];
        expected.sort_by_key(|row| format!("{row:?}"));
        assert_eq!(rows, expected);
    }

    #[test]
    fn wildcard_any_and_namespace_scoped() {
        // Two predicate namespaces: http://ex/ and http://other/.
        let mut b = RdfDatasetBuilder::new();
        let a = b.intern_iri(&iri("a"));
        let p = b.intern_iri(&iri("p"));
        let other_p = b.intern_iri("http://other/p");
        let x = b.intern_iri(&iri("x"));
        let y = b.intern_iri(&iri("y"));
        b.push_quad(a, p, x, None);
        b.push_quad(a, other_p, y, None);
        let ds = b.freeze().expect("freeze");

        // <any> → both objects.
        let any = PropertyPathExpression::Wildcard { namespace: None };
        let rows = run(&ds, &ground("a"), &any, &var("o"), &["o"]);
        assert_eq!(rows, col1(&["x", "y"]));

        // <any:http://ex/> → only the ex-namespaced edge.
        let scoped = PropertyPathExpression::Wildcard {
            namespace: Some(NamedNode::new_unchecked(EX)),
        };
        let rows = run(&ds, &ground("a"), &scoped, &var("o"), &["o"]);
        assert_eq!(rows, col1(&["x"]));
    }

    // ---- endpoint binding modes --------------------------------------------

    #[test]
    fn both_ground_is_ask_shaped() {
        let ds = graph_of(&[("a", "p", "b"), ("b", "p", "c")]);
        let plus = PropertyPathExpression::OneOrMore(Child::new(named("p")));
        // :a :p+ :c  → true (one unit solution).
        let mut ctx = EvalCtx::new(&ds);
        let hit = eval_path(&ground("a"), &plus, &ground("c"), &mut ctx).expect("eval");
        assert_eq!(hit.len(), 1);
        assert!(hit.schema.is_empty());
        // :a :p+ :a  → false (no solutions; acyclic).
        let mut ctx = EvalCtx::new(&ds);
        let miss = eval_path(&ground("a"), &plus, &ground("a"), &mut ctx).expect("eval");
        assert!(miss.is_empty());
    }

    #[test]
    fn both_variable_enumerates_pairs_with_zero_length_self_pairs() {
        // a -> b, plus an isolated edge c -> (nothing further). Node universe = {a,b,c}.
        let ds = graph_of(&[("a", "p", "b"), ("c", "q", "a")]);
        let star = PropertyPathExpression::ZeroOrMore(Child::new(named("p")));
        // ?s :p* ?o : every node pairs with itself (zero-length) + a→b transitive.
        let rows = run(&ds, &var("s"), &star, &var("o"), &["s", "o"]);
        let mut expected = vec![
            vec![Some("a".to_owned()), Some("a".to_owned())],
            vec![Some("a".to_owned()), Some("b".to_owned())],
            vec![Some("b".to_owned()), Some("b".to_owned())],
            vec![Some("c".to_owned()), Some("c".to_owned())],
        ];
        expected.sort_by_key(|row| format!("{row:?}"));
        assert_eq!(rows, expected);
    }

    #[test]
    fn same_variable_keeps_only_reflexive_pairs() {
        // Cycle a -> b -> a: with p+, both a and b reach themselves.
        let ds = graph_of(&[("a", "p", "b"), ("b", "p", "a")]);
        let plus = PropertyPathExpression::OneOrMore(Child::new(named("p")));
        // ?x :p+ ?x  → a, b (each reaches itself via the cycle).
        let rows = run(&ds, &var("x"), &plus, &var("x"), &["x"]);
        assert_eq!(rows, col1(&["a", "b"]));
    }

    // ---- same-variable reflexive short-circuit -----------------------------

    #[test]
    fn same_var_reflexive_star() {
        // Graph a -> b -> c. Node universe = {a, b, c}.
        // ?x :p* ?x — p* is reflexive, so every node is a solution via zero-length identity.
        let ds = graph_of(&[("a", "p", "b"), ("b", "p", "c")]);
        let star = PropertyPathExpression::ZeroOrMore(Child::new(named("p")));
        let rows = run(&ds, &var("x"), &star, &var("x"), &["x"]);
        assert_eq!(rows, col1(&["a", "b", "c"]));
    }

    #[test]
    fn same_var_reflexive_optional() {
        // Graph a -> b -> c. Node universe = {a, b, c}.
        // ?x :p? ?x — p? is reflexive, so every node is a solution via zero-length identity.
        let ds = graph_of(&[("a", "p", "b"), ("b", "p", "c")]);
        let opt = PropertyPathExpression::ZeroOrOne(Child::new(named("p")));
        let rows = run(&ds, &var("x"), &opt, &var("x"), &["x"]);
        assert_eq!(rows, col1(&["a", "b", "c"]));
    }

    #[test]
    fn same_var_reflexive_range_zero_min() {
        // ?x :p{0,2} ?x — min=0 makes it reflexive; every node is a solution.
        let ds = graph_of(&[("a", "p", "b"), ("b", "p", "c")]);
        let rng = PropertyPathExpression::Range {
            inner: Child::new(named("p")),
            min: 0,
            max: Some(2),
        };
        let rows = run(&ds, &var("x"), &rng, &var("x"), &["x"]);
        assert_eq!(rows, col1(&["a", "b", "c"]));
    }

    #[test]
    fn same_var_nonreflexive_no_cycle_is_empty() {
        // Acyclic a -> b -> c. ?x :p+ ?x — p+ is non-reflexive; no node cycles back.
        let ds = graph_of(&[("a", "p", "b"), ("b", "p", "c")]);
        let plus = PropertyPathExpression::OneOrMore(Child::new(named("p")));
        let rows = run(&ds, &var("x"), &plus, &var("x"), &["x"]);
        assert_eq!(rows, col1(&[]));
    }

    #[test]
    fn absent_ground_endpoint_is_empty() {
        let ds = graph_of(&[("a", "p", "b")]);
        let plus = PropertyPathExpression::OneOrMore(Child::new(named("p")));
        // :nobody is not in the graph → empty, but the schema still carries ?o.
        let mut ctx = EvalCtx::new(&ds);
        let seq = eval_path(&ground("nobody"), &plus, &var("o"), &mut ctx).expect("eval");
        assert!(seq.is_empty());
        assert_eq!(seq.schema.vars(), &[Variable::new("o")]);
    }

    // ---- nested composition (corpus-shaped) --------------------------------

    #[test]
    fn nested_alternative_inverse_plus() {
        // Temporal-shaped: (:before | ^:after)+ — before-edges and reversed
        // after-edges, transitively. e1 before e2; e3 after e2 (so e2 ^after e3).
        let ds = graph_of(&[("e1", "before", "e2"), ("e3", "after", "e2")]);
        let alt = PropertyPathExpression::Alternative(
            Chain::try_from(vec![
                named("before"),
                PropertyPathExpression::Reverse(Child::new(named("after"))),
            ])
            .expect("two or more nodes"),
        );
        let plus = PropertyPathExpression::OneOrMore(Child::new(alt));
        // From e1: e1 -before-> e2 -^after-> e3.
        assert_eq!(reach_locals(&ds, &plus, "e1", true), vec!["e2", "e3"]);
    }

    #[test]
    fn list_walk_members_rest_star_first() {
        // owl:members/rdf:rest*/rdf:first over a 3-element RDF list.
        let ds = graph_of(&[
            ("axiom", "members", "l0"),
            ("l0", "first", "A"),
            ("l0", "rest", "l1"),
            ("l1", "first", "B"),
            ("l1", "rest", "l2"),
            ("l2", "first", "C"),
            ("l2", "rest", "nil"),
        ]);
        // :axiom :members/:rest*/:first ?x → A, B, C
        let rest_star = PropertyPathExpression::ZeroOrMore(Child::new(named("rest")));
        let path = PropertyPathExpression::Sequence(
            Chain::try_from(vec![
                named("members"),
                PropertyPathExpression::Sequence(
                    Chain::try_from(vec![rest_star, named("first")]).expect("two or more nodes"),
                ),
            ])
            .expect("two or more nodes"),
        );
        let rows = run(&ds, &ground("axiom"), &path, &var("x"), &["x"]);
        assert_eq!(rows, col1(&["A", "B", "C"]));
    }

    #[test]
    fn determinism_rows_are_termid_ordered() {
        let ds = graph_of(&[("a", "p", "b"), ("b", "p", "c"), ("c", "p", "d")]);
        let star = PropertyPathExpression::ZeroOrMore(Child::new(named("p")));
        let mut ctx = EvalCtx::new(&ds);
        let first = eval_path(&ground("a"), &star, &var("o"), &mut ctx).expect("eval");
        let mut ctx = EvalCtx::new(&ds);
        let second = eval_path(&ground("a"), &star, &var("o"), &mut ctx).expect("eval");
        // Identical row order run-to-run (BTreeSet over TermId).
        let ids = |seq: &SolutionSeq| -> Vec<Option<SolutionTerm>> {
            seq.rows.iter().map(|r| r[0]).collect()
        };
        assert_eq!(ids(&first), ids(&second));
    }

    // ---- negated property set under transitive closure ---------------------

    #[test]
    fn negated_under_one_or_more() {
        // Graph: a -r-> b -r-> c, a -p-> x.
        // !(:p)+ from a: the negated step excludes :p so from a it follows :r to b,
        // then from b it follows :r to c. The :p edge is never followed.
        // Expected: {b, c}.
        let ds = graph_of(&[("a", "r", "b"), ("b", "r", "c"), ("a", "p", "x")]);
        let neg = PropertyPathExpression::NegatedPropertySet(vec![npe("p", false)]);
        let plus = PropertyPathExpression::OneOrMore(Child::new(neg));
        assert_eq!(reach_locals(&ds, &plus, "a", true), vec!["b", "c"]);
    }

    // ---- ground endpoint absent from the dataset: zero-length identity ------

    #[test]
    fn zero_or_more_reflexive_ground_endpoint_absent_from_empty_dataset() {
        // W3C zero_or_more_set_start / zero_or_more_set_end: `:p*` on a
        // completely empty dataset still admits the zero-length reflexive
        // pairing for a ground endpoint that never appears in any triple.
        let ds = graph_of(&[]);
        let star = PropertyPathExpression::ZeroOrMore(Child::new(named("p")));
        // ?s :p* :o → s = :o (the object, bound to itself)
        let rows = run(&ds, &var("s"), &star, &ground("o"), &["s"]);
        assert_eq!(rows, col1(&["o"]));
        // :s :p* ?o → o = :s (the subject, bound to itself)
        let rows = run(&ds, &ground("s"), &star, &var("o"), &["o"]);
        assert_eq!(rows, col1(&["s"]));
    }

    #[test]
    fn zero_or_one_reflexive_ground_endpoint_absent_from_empty_dataset() {
        // Same shape as above but for `?` (zero_or_one_set_start/_end).
        let ds = graph_of(&[]);
        let opt = PropertyPathExpression::ZeroOrOne(Child::new(named("p")));
        let rows = run(&ds, &var("s"), &opt, &ground("o"), &["s"]);
        assert_eq!(rows, col1(&["o"]));
        let rows = run(&ds, &ground("s"), &opt, &var("o"), &["o"]);
        assert_eq!(rows, col1(&["s"]));
    }

    #[test]
    fn non_reflexive_ground_endpoint_absent_from_dataset_is_empty() {
        // A non-reflexive path (no *, +, ? at the relevant position) cannot
        // connect an absent node to anything else — it has no edges.
        let ds = graph_of(&[("a", "p", "b")]);
        let rows = run(&ds, &var("s"), &named("p"), &ground("nobody"), &["s"]);
        assert_eq!(rows, col1(&[]));
    }

    #[test]
    fn variable_inside_quoted_triple_endpoint_names_the_path_not_a_bgp() {
        // Regression guard: a variable inside a quoted-triple-term PATH endpoint
        // used to report the shared `convert::ground_term_pattern_to_value`
        // helper's hardcoded BGP wording ("... in a BGP") even though no BGP is
        // anywhere in this query — `resolve_end` is a path-endpoint-only call
        // site. The message must now name what it actually is.
        let ds = graph_of(&[]);
        let mut ctx = EvalCtx::new(&ds);
        let object = TermPattern::Triple(Child::new(TriplePattern {
            subject: ground("s"),
            predicate: purrdf_sparql_algebra::NamedNodePattern::NamedNode(nn("p")),
            object: var("o"),
        }));
        let err = eval_path(&var("r"), &named("reifies"), &object, &mut ctx)
            .expect_err("a variable inside a quoted-triple path endpoint is Unsupported");
        let message = err.to_string();
        assert!(
            message.contains("property-path endpoint"),
            "message must name the path endpoint, not a BGP: {message:?}"
        );
        assert!(
            !message.contains("BGP"),
            "message must not claim this variable was found in a BGP: {message:?}"
        );
    }
}

#[cfg(test)]
mod recursion_free_tests {
    //! The machine against a recursive reference over generated shapes, under no budget
    //! and under every small budget, and a path a hundred thousand levels deep evaluated
    //! on a 128 KiB thread.
    //!
    //! The reference is the recursive formulation of every traversal function, over the
    //! path expression itself and address-keyed caches; the machine must answer the same
    //! set, the same bag, the same reflexivity and repetition verdicts, and — under a
    //! fuel budget — trip at the same charge with the same partial answer, which it can
    //! only do by issuing the same graph reads and composition charges in the same order.

    use super::*;
    use crate::governor::{GovernorState, QueryGovernors};
    use purrdf_core::{GraphMatch, RdfDataset, RdfDatasetBuilder, TrippedGovernor};
    use purrdf_sparql_algebra::{Chain, Child};
    use purrdf_testkit::rng::SplitMix64;

    const EX: &str = "http://example.org/";

    fn iri(local: &str) -> String {
        format!("{EX}{local}")
    }

    fn node(local: &str) -> NamedNode {
        NamedNode::new_unchecked(iri(local))
    }

    /// `p{i}`; `p3` occurs in no generated dataset, so it is the absent-predicate lane.
    fn predicate(i: u64) -> NamedNode {
        node(&format!("p{i}"))
    }

    // ── The recursive reference ──────────────────────────────────────────────────────

    /// The identity of a path node or an element slice: its address.
    fn address<T: ?Sized>(value: &T) -> usize {
        <*const T>::from(value).addr()
    }

    /// The reference's context: the machine's, for the dataset, the scope and the budget,
    /// beside the address-keyed caches the recursive functions read.
    struct ReferenceCtx<'a, D: DatasetView + Sync> {
        base: &'a PathCtx<'a, D>,
        negated: BTreeMap<usize, NegatedSets<D::Id>>,
        reach_cache: ReferenceReachCache<D::Id>,
    }

    fn reference_build_negated_cache<D: DatasetView + Sync>(
        path: &PropertyPathExpression,
        dataset: &D,
    ) -> BTreeMap<usize, NegatedSets<D::Id>> {
        let mut cache = BTreeMap::new();
        reference_collect_negated(path, dataset, &mut cache);
        cache
    }

    fn reference_collect_negated<D: DatasetView + Sync>(
        path: &PropertyPathExpression,
        dataset: &D,
        cache: &mut BTreeMap<usize, NegatedSets<D::Id>>,
    ) {
        use PropertyPathExpression as P;
        match path {
            P::NegatedPropertySet(elems) => {
                let key = address(elems.as_slice());
                cache.entry(key).or_insert_with(|| {
                    let mut forward = None;
                    let mut inverse = None;
                    for e in elems {
                        let target = if e.inverse {
                            inverse.get_or_insert_with(|| {
                                crate::AdmittedVec::new(&crate::WorkspaceCapability::default())
                            })
                        } else {
                            forward.get_or_insert_with(|| {
                                crate::AdmittedVec::new(&crate::WorkspaceCapability::default())
                            })
                        };
                        if let Some(id) = dataset
                            .term_id_by_value(&named_node_to_value(&e.predicate))
                            .unwrap()
                            && let Err(index) = target.binary_search(&id)
                        {
                            target
                                .push(id)
                                .expect("resident reference exclusion allocation");
                            target.as_mut_slice()[index..].rotate_right(1);
                        }
                    }
                    NegatedSets { forward, inverse }
                });
            }
            P::Reverse(i) | P::ZeroOrOne(i) | P::ZeroOrMore(i) | P::OneOrMore(i) => {
                reference_collect_negated(i, dataset, cache);
            }
            P::Range { inner, .. } => reference_collect_negated(inner, dataset, cache),
            P::Sequence(elements) | P::Alternative(elements) => {
                for element in elements {
                    reference_collect_negated(element, dataset, cache);
                }
            }
            P::NamedNode(_) | P::Wildcard { .. } => {}
        }
    }

    fn reference_reach_cached<D: DatasetView + Sync>(
        path: &PropertyPathExpression,
        node: D::Id,
        forward: bool,
        ctx: &ReferenceCtx<'_, D>,
    ) -> Rc<BTreeSet<D::Id>> {
        if ctx.base.stopped() {
            return Rc::new(BTreeSet::new());
        }
        if let PropertyPathExpression::Reverse(inner) = path {
            return reference_reach_cached(inner, node, !forward, ctx);
        }
        let key = (address(path), node, forward);
        if let Some(cached) = ctx.reach_cache.borrow().get(&key) {
            return cached.clone();
        }
        let result = Rc::new(reference_reach_uncached(path, node, forward, ctx));
        if ctx.base.stopped() {
            return result;
        }
        ctx.reach_cache.borrow_mut().insert(key, result.clone());
        result
    }

    fn reference_reach_uncached<D: DatasetView + Sync>(
        path: &PropertyPathExpression,
        node: D::Id,
        forward: bool,
        ctx: &ReferenceCtx<'_, D>,
    ) -> BTreeSet<D::Id> {
        use PropertyPathExpression as P;
        match path {
            P::NamedNode(p) => reference_step_predicate(p, node, forward, ctx),
            P::Reverse(inner) => reference_reach_cached(inner, node, !forward, ctx)
                .as_ref()
                .clone(),
            P::Sequence(elements) => reference_sequence_reach(elements, node, forward, ctx),
            P::Alternative(elements) => {
                let mut out = BTreeSet::new();
                for element in elements {
                    out.extend(
                        reference_reach_cached(element, node, forward, ctx)
                            .iter()
                            .copied(),
                    );
                }
                out
            }
            P::ZeroOrOne(inner) => {
                let mut out = reference_reach_cached(inner, node, forward, ctx)
                    .as_ref()
                    .clone();
                out.insert(node);
                out
            }
            P::ZeroOrMore(inner) => {
                let mut out = reference_closure(inner, node, forward, ctx);
                out.insert(node);
                out
            }
            P::OneOrMore(inner) => reference_closure(inner, node, forward, ctx),
            P::Range { inner, min, max } => {
                reference_range_reach(inner, node, forward, *min, *max, ctx)
            }
            P::NegatedPropertySet(elems) => reference_step_negated(elems, node, forward, ctx),
            P::Wildcard { namespace } => step_wildcard(namespace.as_ref(), node, forward, ctx.base)
                .into_iter()
                .collect(),
        }
    }

    fn reference_sequence_reach<D: DatasetView + Sync>(
        elements: &[PropertyPathExpression],
        node: D::Id,
        forward: bool,
        ctx: &ReferenceCtx<'_, D>,
    ) -> BTreeSet<D::Id> {
        let mut frontier = BTreeSet::from([node]);
        for k in 0..elements.len() {
            if frontier.is_empty() {
                break;
            }
            let element = &elements[if forward { k } else { elements.len() - 1 - k }];
            let mut next = BTreeSet::new();
            for mid in frontier {
                next.extend(
                    reference_reach_cached(element, mid, forward, ctx)
                        .iter()
                        .copied(),
                );
            }
            frontier = next;
        }
        frontier
    }

    fn reference_path_is_reflexive(path: &PropertyPathExpression) -> bool {
        use PropertyPathExpression as P;
        match path {
            P::ZeroOrMore(_) | P::ZeroOrOne(_) => true,
            P::Range { min, .. } => *min == 0,
            P::Reverse(inner) => reference_path_is_reflexive(inner),
            P::Sequence(elements) => elements.iter().all(reference_path_is_reflexive),
            P::Alternative(elements) => elements.iter().any(reference_path_is_reflexive),
            P::NamedNode(_) | P::OneOrMore(_) | P::NegatedPropertySet(_) | P::Wildcard { .. } => {
                false
            }
        }
    }

    fn reference_path_has_repetition(path: &PropertyPathExpression) -> bool {
        use PropertyPathExpression as P;
        match path {
            P::ZeroOrMore(_) | P::OneOrMore(_) | P::ZeroOrOne(_) | P::Range { .. } => true,
            P::NamedNode(_) | P::NegatedPropertySet(_) | P::Wildcard { .. } => false,
            P::Reverse(inner) => reference_path_has_repetition(inner),
            P::Sequence(elements) | P::Alternative(elements) => {
                elements.iter().any(reference_path_has_repetition)
            }
        }
    }

    fn reference_simple_reach_multiset<D: DatasetView + Sync>(
        path: &PropertyPathExpression,
        node: D::Id,
        forward: bool,
        ctx: &ReferenceCtx<'_, D>,
    ) -> Vec<D::Id> {
        use PropertyPathExpression as P;
        match path {
            P::NamedNode(p) => reference_step_predicate(p, node, forward, ctx)
                .into_iter()
                .collect(),
            P::Reverse(inner) => reference_simple_reach_multiset(inner, node, !forward, ctx),
            P::Sequence(elements) => {
                let mut frontier = vec![node];
                for k in 0..elements.len() {
                    if frontier.is_empty() {
                        break;
                    }
                    let element = &elements[if forward { k } else { elements.len() - 1 - k }];
                    let mut next = Vec::new();
                    for mid in frontier {
                        next.extend(reference_simple_reach_multiset(element, mid, forward, ctx));
                    }
                    frontier = next;
                }
                frontier
            }
            P::Alternative(elements) => {
                let mut out = Vec::new();
                for element in elements {
                    out.extend(reference_simple_reach_multiset(element, node, forward, ctx));
                }
                out
            }
            P::NegatedPropertySet(elems) => reference_step_negated(elems, node, forward, ctx)
                .into_iter()
                .collect(),
            P::Wildcard { namespace } => step_wildcard(namespace.as_ref(), node, forward, ctx.base)
                .into_iter()
                .collect(),
            P::ZeroOrMore(_) | P::OneOrMore(_) | P::ZeroOrOne(_) | P::Range { .. } => {
                unreachable!("the reference bag is asked only of a repetition-free path")
            }
        }
    }

    fn reference_step_predicate<D: DatasetView + Sync>(
        p: &NamedNode,
        node: D::Id,
        forward: bool,
        ctx: &ReferenceCtx<'_, D>,
    ) -> BTreeSet<D::Id> {
        let pid = ctx
            .base
            .dataset
            .term_id_by_value(&named_node_to_value(p))
            .unwrap();
        step_predicate(pid, node, forward, ctx.base)
            .into_iter()
            .collect()
    }

    fn reference_step_negated<D: DatasetView + Sync>(
        elems: &[NegatedPathElement],
        node: D::Id,
        forward: bool,
        ctx: &ReferenceCtx<'_, D>,
    ) -> BTreeSet<D::Id> {
        let sets = &ctx.negated[&address(elems)];
        let mut out = BTreeSet::new();
        if let Some(excluded) = &sets.forward {
            out.extend(step_excluding(excluded, node, forward, ctx.base));
        }
        if let Some(excluded) = &sets.inverse {
            out.extend(step_excluding(excluded, node, !forward, ctx.base));
        }
        out
    }

    fn reference_closure<D: DatasetView + Sync>(
        inner: &PropertyPathExpression,
        node: D::Id,
        forward: bool,
        ctx: &ReferenceCtx<'_, D>,
    ) -> BTreeSet<D::Id> {
        let mut result = BTreeSet::new();
        let mut visited: DetHashSet<D::Id> = DetHashSet::default();
        let mut frontier: Vec<D::Id> = reference_reach_cached(inner, node, forward, ctx)
            .iter()
            .copied()
            .collect();
        while let Some(n) = frontier.pop() {
            if !visited.insert(n) {
                continue;
            }
            if ctx.base.stopped() {
                break;
            }
            result.insert(n);
            for next in reference_reach_cached(inner, n, forward, ctx)
                .iter()
                .copied()
            {
                if !visited.contains(&next) {
                    frontier.push(next);
                }
            }
        }
        result
    }

    fn reference_closure_multi<D: DatasetView + Sync>(
        inner: &PropertyPathExpression,
        seeds: &BTreeSet<D::Id>,
        forward: bool,
        ctx: &ReferenceCtx<'_, D>,
    ) -> BTreeSet<D::Id> {
        let mut result = BTreeSet::new();
        let mut visited: DetHashSet<D::Id> = DetHashSet::default();
        let mut frontier: Vec<D::Id> = Vec::new();
        for &s in seeds {
            frontier.extend(
                reference_reach_cached(inner, s, forward, ctx)
                    .iter()
                    .copied(),
            );
        }
        while let Some(n) = frontier.pop() {
            if !visited.insert(n) {
                continue;
            }
            if ctx.base.stopped() {
                break;
            }
            result.insert(n);
            for next in reference_reach_cached(inner, n, forward, ctx)
                .iter()
                .copied()
            {
                if !visited.contains(&next) {
                    frontier.push(next);
                }
            }
        }
        result
    }

    fn reference_range_reach<D: DatasetView + Sync>(
        inner: &PropertyPathExpression,
        node: D::Id,
        forward: bool,
        min: u32,
        max: Option<u32>,
        ctx: &ReferenceCtx<'_, D>,
    ) -> BTreeSet<D::Id> {
        let mut out = BTreeSet::new();
        if max.is_some_and(|m| m < min) {
            return out;
        }
        let mut current: BTreeSet<D::Id> = BTreeSet::from([node]);
        if !reference_advance_to_min(inner, &mut current, forward, min, ctx) {
            return out;
        }
        let mut level = min;
        loop {
            let before = out.len();
            out.extend(current.iter().copied());
            let grew = out.len() != before;
            match max {
                Some(m) if level >= m => break,
                None => {
                    out.extend(reference_closure_multi(inner, &current, forward, ctx));
                    break;
                }
                Some(_) => {}
            }
            if !grew {
                break;
            }
            let Some(next) = reference_step_level(inner, &current, forward, ctx) else {
                return out;
            };
            current = next;
            level += 1;
        }
        out
    }

    fn reference_advance_to_min<D: DatasetView + Sync>(
        inner: &PropertyPathExpression,
        current: &mut BTreeSet<D::Id>,
        forward: bool,
        min: u32,
        ctx: &ReferenceCtx<'_, D>,
    ) -> bool {
        if min <= LINEAR_RANGE_PREFIX {
            for _ in 0..min {
                let Some(next) = reference_step_level(inner, current, forward, ctx) else {
                    return false;
                };
                *current = next;
                if current.is_empty() {
                    return false;
                }
            }
            return true;
        }
        let mut powers = PowerReachCache::new();
        let mut remaining = min;
        let mut bit = 0;
        while remaining != 0 {
            if remaining & 1 == 1 {
                let Some(next) =
                    reference_apply_power(inner, current, forward, bit, ctx, &mut powers)
                else {
                    return false;
                };
                *current = next;
                if current.is_empty() {
                    return false;
                }
            }
            remaining >>= 1;
            bit += 1;
        }
        true
    }

    fn reference_apply_power<D: DatasetView + Sync>(
        inner: &PropertyPathExpression,
        current: &BTreeSet<D::Id>,
        forward: bool,
        bit: u32,
        ctx: &ReferenceCtx<'_, D>,
        powers: &mut PowerReachCache<D::Id>,
    ) -> Option<BTreeSet<D::Id>> {
        let mut out = BTreeSet::new();
        for source in current {
            let reached = reference_power_reach(inner, *source, forward, bit, ctx, powers);
            if ctx.base.stopped() {
                return None;
            }
            for target in reached.iter().copied() {
                if !ctx.base.charge_candidate() {
                    return None;
                }
                out.insert(target);
            }
        }
        Some(out)
    }

    fn reference_power_reach<D: DatasetView + Sync>(
        inner: &PropertyPathExpression,
        node: D::Id,
        forward: bool,
        bit: u32,
        ctx: &ReferenceCtx<'_, D>,
        powers: &mut PowerReachCache<D::Id>,
    ) -> Rc<BTreeSet<D::Id>> {
        let key = (bit, node);
        if let Some(cached) = powers.get(&key) {
            return Rc::clone(cached);
        }
        let reached = if bit == 0 {
            reference_reach_cached(inner, node, forward, ctx)
        } else {
            let first = reference_power_reach(inner, node, forward, bit - 1, ctx, powers);
            if ctx.base.stopped() {
                return Rc::new(BTreeSet::new());
            }
            let mut out = BTreeSet::new();
            for mid in first.iter().copied() {
                let second = reference_power_reach(inner, mid, forward, bit - 1, ctx, powers);
                if ctx.base.stopped() {
                    return Rc::new(out);
                }
                for target in second.iter().copied() {
                    if !ctx.base.charge_candidate() {
                        return Rc::new(out);
                    }
                    out.insert(target);
                }
            }
            Rc::new(out)
        };
        if !ctx.base.stopped() {
            powers.insert(key, Rc::clone(&reached));
        }
        reached
    }

    fn reference_step_level<D: DatasetView + Sync>(
        inner: &PropertyPathExpression,
        current: &BTreeSet<D::Id>,
        forward: bool,
        ctx: &ReferenceCtx<'_, D>,
    ) -> Option<BTreeSet<D::Id>> {
        let mut next = BTreeSet::new();
        for n in current {
            let reached = reference_reach_cached(inner, *n, forward, ctx);
            if ctx.base.stopped() {
                return None;
            }
            for target in reached.iter().copied() {
                if !ctx.base.charge_candidate() {
                    return None;
                }
                next.insert(target);
            }
        }
        Some(next)
    }

    // ── Generated shapes ─────────────────────────────────────────────────────────────

    /// One generated case: a small graph, a path over it, a start node and a direction.
    struct Case {
        dataset: Arc<RdfDataset>,
        path: PropertyPathExpression,
        start: TermId,
        forward: bool,
    }

    /// A graph of two to six nodes over one to three predicates with one to eight edges,
    /// self-loops and cycles included.
    fn generate_dataset(state: &mut SplitMix64) -> Arc<RdfDataset> {
        let nodes = 2 + state.below(5);
        let predicates = 1 + state.below(3);
        let edges = 1 + state.below(8);
        let mut builder = RdfDatasetBuilder::new();
        for _ in 0..edges {
            let s = builder.intern_iri(&iri(&format!("n{}", state.below(nodes))));
            let p = builder.intern_iri(&iri(&format!("p{}", state.below(predicates))));
            let o = builder.intern_iri(&iri(&format!("n{}", state.below(nodes))));
            builder.push_quad(s, p, o, None);
        }
        builder.freeze().expect("the generated dataset freezes")
    }

    fn generate_leaf(state: &mut SplitMix64) -> PropertyPathExpression {
        use PropertyPathExpression as P;
        match state.below(6) {
            0..=2 => P::NamedNode(predicate(state.below(4))),
            3 => P::NegatedPropertySet(
                (0..=state.below(2))
                    .map(|_| NegatedPathElement {
                        predicate: predicate(state.below(4)),
                        inverse: state.below(2) == 1,
                    })
                    .collect(),
            ),
            4 => P::Wildcard { namespace: None },
            _ => P::Wildcard {
                namespace: Some(NamedNode::new_unchecked(if state.below(2) == 0 {
                    EX.to_owned()
                } else {
                    "http://other.example.org/".to_owned()
                })),
            },
        }
    }

    /// A chain of two or three generated elements.
    fn generate_chain(state: &mut SplitMix64, budget: &mut u32) -> Chain<PropertyPathExpression> {
        let first = generate_path(state, budget);
        let second = generate_path(state, budget);
        let rest: Vec<PropertyPathExpression> = (0..state.below(2))
            .map(|_| generate_path(state, budget))
            .collect();
        Chain::new(first, second, rest)
    }

    /// A path over every variant, its operator count bounded by `budget`. `Range` draws
    /// its `min` from both lanes of the prefix walk — at most three levels, and past the
    /// linear prefix where the binary relation powers compose it — and its `max` from an
    /// open tail, an exact count, a window, and (when `min > 0`) an empty window.
    fn generate_path(state: &mut SplitMix64, budget: &mut u32) -> PropertyPathExpression {
        use PropertyPathExpression as P;
        if *budget == 0 {
            return generate_leaf(state);
        }
        *budget -= 1;
        match state.below(12) {
            0..=2 => generate_leaf(state),
            3 => P::Reverse(Child::new(generate_path(state, budget))),
            4 => P::ZeroOrOne(Child::new(generate_path(state, budget))),
            5 => P::ZeroOrMore(Child::new(generate_path(state, budget))),
            6 => P::OneOrMore(Child::new(generate_path(state, budget))),
            7 | 8 => {
                const MINS: [u32; 7] = [0, 1, 2, 3, 65, 66, 70];
                let min = MINS[state.below(MINS.len() as u64) as usize];
                let max = match state.below(4) {
                    0 => None,
                    1 => Some(min),
                    2 => Some(min + 1 + state.below(3) as u32),
                    _ => min.checked_sub(1),
                };
                P::Range {
                    inner: Child::new(generate_path(state, budget)),
                    min,
                    max,
                }
            }
            9 | 10 => P::Sequence(generate_chain(state, budget)),
            _ => P::Alternative(generate_chain(state, budget)),
        }
    }

    /// `count` cases from `seed`.
    fn cases(count: usize, seed: u64) -> Vec<Case> {
        let mut state = SplitMix64::new(seed);
        (0..count)
            .map(|_| {
                let dataset = generate_dataset(&mut state);
                let mut budget = 1 + state.below(5) as u32;
                let path = generate_path(&mut state, &mut budget);
                let universe: Vec<TermId> = node_universe(&PathCtx::resident(
                    &*dataset,
                    GraphScope::One(GraphMatch::Default),
                    PathProgram::compile(&path, &*dataset),
                    None,
                ))
                .expect("resident node universe allocation")
                .into_iter()
                .collect();
                let start = universe[state.below(universe.len() as u64) as usize];
                let forward = state.below(2) == 0;
                Case {
                    dataset,
                    path,
                    start,
                    forward,
                }
            })
            .collect()
    }

    /// A fresh governor with `fuel`.
    fn governor(fuel: u64) -> Arc<GovernorState> {
        Arc::new(GovernorState::new(
            &QueryGovernors::UNBOUNDED.with_fuel(fuel),
        ))
    }

    /// A fresh machine context over `case` under `governors`.
    fn machine_ctx(case: &Case, governors: Option<Arc<GovernorState>>) -> PathCtx<'_, RdfDataset> {
        PathCtx::resident(
            &*case.dataset,
            GraphScope::One(GraphMatch::Default),
            PathProgram::compile(&case.path, &*case.dataset),
            governors.map(Into::into),
        )
    }

    /// A fresh reference context over `base`.
    fn reference_ctx<'a>(
        case: &'a Case,
        base: &'a PathCtx<'a, RdfDataset>,
    ) -> ReferenceCtx<'a, RdfDataset> {
        ReferenceCtx {
            base,
            negated: reference_build_negated_cache(&case.path, &*case.dataset),
            reach_cache: RefCell::new(DetHashMap::default()),
        }
    }

    /// What one governed run reports: the governor that tripped, if one did.
    fn verdict(governors: Option<&Arc<GovernorState>>) -> Option<TrippedGovernor> {
        governors.and_then(|state| state.tripped())
    }

    /// The machine's set answer and the reference's, under `fuel`, must agree — and so
    /// must the governor verdicts.
    fn assert_same_set(case: &Case, fuel: Option<u64>) {
        let machine_state = fuel.map(governor);
        let machine = machine_ctx(case, machine_state.clone());
        let reached = reach(ROOT_OP, case.start, case.forward, &machine)
            .expect("resident path reach allocation");

        let reference_state = fuel.map(governor);
        let base = machine_ctx(case, reference_state.clone());
        let reference = reference_ctx(case, &base);
        let expected = reference_reach_cached(&case.path, case.start, case.forward, &reference);

        assert_eq!(
            reached.iter().copied().collect::<Vec<_>>(),
            expected.iter().copied().collect::<Vec<_>>(),
            "set reach of {:?} from {:?} forward={} under fuel {fuel:?}",
            case.path,
            case.start,
            case.forward
        );
        assert_eq!(
            verdict(machine_state.as_ref()),
            verdict(reference_state.as_ref()),
            "governor verdict for {:?} under fuel {fuel:?}",
            case.path
        );
    }

    /// The machine's bag answer and the reference's, under `fuel`, must agree entry for
    /// entry — multiplicity and order included — and so must the governor verdicts.
    fn assert_same_bag(case: &Case, fuel: Option<u64>) {
        let machine_state = fuel.map(governor);
        let machine = machine_ctx(case, machine_state.clone());
        let reached = bag_reach(ROOT_OP, case.start, case.forward, &machine)
            .expect("resident bag path allocation");

        let reference_state = fuel.map(governor);
        let base = machine_ctx(case, reference_state.clone());
        let reference = reference_ctx(case, &base);
        let expected =
            reference_simple_reach_multiset(&case.path, case.start, case.forward, &reference);

        assert_eq!(
            &*reached,
            expected.as_slice(),
            "bag reach of {:?} from {:?} forward={} under fuel {fuel:?}",
            case.path,
            case.start,
            case.forward
        );
        assert_eq!(
            verdict(machine_state.as_ref()),
            verdict(reference_state.as_ref()),
            "governor verdict for {:?} under fuel {fuel:?}",
            case.path
        );
    }

    #[test]
    fn the_program_folds_reflexivity_and_repetition_as_the_reference_does() {
        for case in cases(150, 0x5eed_0001) {
            let program = PathProgram::compile(&case.path, &*case.dataset);
            assert_eq!(
                program.is_reflexive(),
                reference_path_is_reflexive(&case.path),
                "reflexivity of {:?}",
                case.path
            );
            assert_eq!(
                program.has_repetition(),
                reference_path_has_repetition(&case.path),
                "repetition in {:?}",
                case.path
            );
        }
    }

    #[test]
    fn the_machine_answers_the_reference_set_and_bag_on_generated_shapes() {
        for case in cases(150, 0x5eed_0002) {
            assert_same_set(&case, None);
            if !reference_path_has_repetition(&case.path) {
                assert_same_bag(&case, None);
            }
        }
    }

    /// Under every small budget the machine trips at the same charge as the reference and
    /// answers the same partial set or bag. Two traversals that issue their graph reads
    /// and composition charges in a different order would diverge at the budget that
    /// falls between the two orders' first differing charges, so agreement at every
    /// budget is agreement on the charge sequence.
    #[test]
    fn the_machine_charges_in_the_reference_order_under_every_budget() {
        const FUEL_SWEEP: u64 = 24;
        for case in cases(120, 0x5eed_0003) {
            let repetition = reference_path_has_repetition(&case.path);
            for fuel in 0..=FUEL_SWEEP {
                assert_same_set(&case, Some(fuel));
                if !repetition {
                    assert_same_bag(&case, Some(fuel));
                }
            }
        }
    }

    // ── A hundred thousand levels on a 128 KiB thread ────────────────────────────────

    /// `:a :p :b . :b :p :c`.
    fn two_hop_dataset() -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        let a = builder.intern_iri(&iri("a"));
        let b = builder.intern_iri(&iri("b"));
        let c = builder.intern_iri(&iri("c"));
        let p = builder.intern_iri(&iri("p"));
        builder.push_quad(a, p, b, None);
        builder.push_quad(b, p, c, None);
        builder.freeze().expect("the two-hop dataset freezes")
    }

    /// `:a path ?o` through `eval_path`, the objects spelled by local name and sorted.
    fn objects_from_a(dataset: &RdfDataset, path: &PropertyPathExpression) -> Vec<String> {
        let mut ctx = EvalCtx::new(dataset);
        let subject = TermPattern::NamedNode(node("a"));
        let object = TermPattern::Variable(Variable::new("o"));
        let seq = eval_path(&subject, path, &object, &mut ctx).expect("the deep path evaluates");
        let mut out: Vec<String> = seq
            .rows
            .iter()
            .map(|row| match row[0] {
                Some(SolutionTerm::Existing(id)) => match dataset.resolve(id) {
                    TermRef::Iri(spelled) => spelled.strip_prefix(EX).unwrap_or(spelled).to_owned(),
                    other => format!("{other:?}"),
                },
                other => format!("{other:?}"),
            })
            .collect();
        out.sort();
        out
    }

    /// `path`, wrapped in `levels` applications of `wrap`.
    fn nested(
        mut path: PropertyPathExpression,
        levels: usize,
        wrap: fn(Child<PropertyPathExpression>) -> PropertyPathExpression,
    ) -> PropertyPathExpression {
        for _ in 0..levels {
            path = wrap(Child::new(path));
        }
        path
    }

    /// `^^…^:p` with an even number of reverses is `:p`: the bag lane, the reverses
    /// resolved by the request frame's loop.
    #[test]
    fn a_hundred_thousand_reverses_evaluate_on_a_128_kib_thread() {
        let objects = purrdf_stack::on_stack(128 * 1024, || {
            let dataset = two_hop_dataset();
            let path = nested(
                PropertyPathExpression::NamedNode(node("p")),
                100_000,
                PropertyPathExpression::Reverse,
            );
            objects_from_a(&dataset, &path)
        })
        .expect("spawn");
        assert_eq!(objects, vec!["b".to_owned()]);
    }

    /// `((:p?)?)…?` is `:p?`: the set lane, one request frame per level on the heap.
    #[test]
    fn a_hundred_thousand_zero_or_ones_evaluate_on_a_128_kib_thread() {
        let objects = purrdf_stack::on_stack(128 * 1024, || {
            let dataset = two_hop_dataset();
            let path = nested(
                PropertyPathExpression::NamedNode(node("p")),
                100_000,
                PropertyPathExpression::ZeroOrOne,
            );
            objects_from_a(&dataset, &path)
        })
        .expect("spawn");
        assert_eq!(objects, vec!["a".to_owned(), "b".to_owned()]);
    }
}
