// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The canonical labelling of the blank nodes of a set of typed lines by
//! nested colour refinement, identifier `(nested-refinement-labelling, 1)`.
//!
//! The input is a set of [`Line`]s. The result is typed throughout: for every
//! blank node its position and its stable identity (component digest, local
//! position, copy index); the canonical line sequence in ascending order of
//! the line encoding with its digest; and the structural record, which states
//! whether the input is `guaranteed` (labelled with no search) and, when it is
//! not, which calls were the first to need one. A refusal is a typed value.
//!
//! The definition, in outline. Each connected component of blank nodes starts
//! alone: one cell of nodes, its lines in cells by their encoding with every
//! blank leaf free, then colour refinement of the incidence graph by a queue
//! of `(side, colour)` pairs. A call whose node partition is discrete fixes
//! every node; one with a singleton cell fixes the singletons and nests on the
//! free components that remain, each inheriting the partition; one with no
//! singleton branches on the cell of colour zero, individualising each member
//! in turn. A call's certificate is `(s, E, C)`: the number of nodes it fixes,
//! the sorted entries of the lines it completes, and the certificates of its
//! nested calls in order; a branching call takes the smallest certificate of
//! its children. Components are ordered by the BLAKE3 digest of their
//! certificate encoding, positions follow from the structure of the calls, and
//! no hash enters the search itself.
//!
//! [`reference`] is written for clarity and explores every child of every
//! branching call it searches; it is the oracle the optimised engine is held to.

mod certificate;
mod lines;
pub mod reference;
#[cfg(test)]
mod tests;
mod view;
#[cfg(test)]
mod view_tests;

pub use certificate::{Certificate, Entry, EntryToken, RebuildError, rebuild};
pub use lines::{Direction, GraphTerm, InputReason, Kind, Line, LineSet, Quantity, Term};
pub use view::{Comparison, Record, ViewError, compare_view, lines_of_view};

use purrdf_hash::Domain;
use purrdf_lex::term_syntax::write_blank;
use purrdf_lex::text_out::TextOut;

/// The name of the labelling: an opaque constant.
pub const NAME: &[u8] = b"nested-refinement-labelling";
/// The version of the labelling.
pub const VERSION: u64 = 1;

/// The derive-key context of the component identity.
pub const COMPONENT_CONTEXT: Domain = Domain::new(b"nested-refinement-labelling 1 component");
/// The derive-key context of the digest of the canonical line sequence.
pub const LINES_CONTEXT: Domain = Domain::new(b"nested-refinement-labelling 1 lines");

/// `H(context, message)`: BLAKE3 in key-derivation mode, 32 bytes.
pub(crate) fn digest_of(context: Domain, message: &[u8]) -> [u8; 32] {
    purrdf_hash::blake3::derive_key(context.as_str(), message)
}

/// The stable identity of a blank node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Identity {
    /// The identity of its top component.
    pub component: [u8; 32],
    /// Its position minus the offset of its top component.
    pub local_position: u64,
    /// The number of earlier top components of equal identity.
    pub copy_index: u64,
}

/// One blank node of the canonical map.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Placement {
    /// Its final position.
    pub position: u64,
    /// Its stable identity.
    pub identity: Identity,
}

/// A factor of the order of an automorphism group.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FactorKind {
    /// An attaining count (code 0).
    Attain = 0,
    /// A symmetric group over equal nested calls (code 1).
    Sym = 1,
}

/// A factor record `(kind, value, exponent)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Factor {
    /// The kind.
    pub kind: FactorKind,
    /// The base.
    pub value: u64,
    /// The exponent.
    pub exponent: u64,
}

/// A first failure of the search-free run: the argument `p` of the call, the
/// size of its target cell, and the nodes and lines of its top component.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FailureRecord {
    /// The argument `p` of the failing call.
    pub path_position: u64,
    /// The size of its target cell.
    pub tied_cell: u64,
    /// The nodes of its top component.
    pub component_nodes: u64,
    /// The lines of its top component.
    pub component_lines: u64,
}

/// A failure with the source identifiers that locate it (not structural).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FailureDiagnostic {
    /// The structural record.
    pub record: FailureRecord,
    /// The smallest source identifier of its top component.
    pub component_source: Box<[u8]>,
    /// The smallest source identifier of its target cell.
    pub cell_source: Box<[u8]>,
}

/// Units by part.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Units {
    /// START: the start charge of every line of a top component.
    pub start: u64,
    /// REFINE: label elements read by popped cells.
    pub refine: u64,
    /// FIX: removed vertices, removed incidences and split components.
    pub fix: u64,
    /// ENTRIES: one plus one per eight bytes of each entry formed.
    pub entries: u64,
}

impl Units {
    /// The sum part by part, or `None` when a part exceeds 64 bits (the
    /// refusal `width` with the quantity `units`, 7.11).
    const fn checked_add(self, other: Self) -> Option<Self> {
        let (Some(start), Some(refine), Some(fix), Some(entries)) = (
            self.start.checked_add(other.start),
            self.refine.checked_add(other.refine),
            self.fix.checked_add(other.fix),
            self.entries.checked_add(other.entries),
        ) else {
            return None;
        };
        Some(Self {
            start,
            refine,
            fix,
            entries,
        })
    }

    /// The sum part by part for an engine figure, which is not a result
    /// value and stays at `2^64 - 1` rather than refuse.
    const fn saturating_add(self, other: Self) -> Self {
        Self {
            start: self.start.saturating_add(other.start),
            refine: self.refine.saturating_add(other.refine),
            fix: self.fix.saturating_add(other.fix),
            entries: self.entries.saturating_add(other.entries),
        }
    }

    /// The sum of the four parts.
    #[must_use]
    pub const fn total(&self) -> u64 {
        self.start + self.refine + self.fix + self.entries
    }
}

/// Calls of the result path by the rule they take.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct RuleCounts {
    /// Discrete: a leaf.
    pub discrete: u64,
    /// A singleton cell: fix and nest.
    pub nest: u64,
    /// No singleton: branch.
    pub branch: u64,
}

/// Chosen children by the rule they take.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ChildCounts {
    /// Discrete.
    pub discrete: u64,
    /// Fix and nest.
    pub nest: u64,
}

/// The structural record of one top component.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ComponentRecord {
    /// `n_K`.
    pub blank_nodes: u64,
    /// `m_K`.
    pub lines: u64,
    /// `O_K`.
    pub label_elements: u64,
    /// `M_K`.
    pub incidences: u64,
    /// The encoded bytes of its lines with every leaf free.
    pub line_bytes: u64,
    /// Nodes in singleton cells after the fresh refinement.
    pub singled_out: u64,
    /// Nodes in larger cells after it.
    pub unsettled: u64,
    /// The largest cell after it, absent when every cell is a singleton.
    pub largest_cell: Option<u64>,
    /// Whether the incidence graph is acyclic.
    pub forest: bool,
    /// Result-path calls by rule.
    pub calls: RuleCounts,
    /// Chosen children by rule.
    pub chosen_children: ChildCounts,
    /// The target cell size of each branching call, in path order.
    pub target_sizes: Vec<u64>,
    /// The attaining children of each branching call, in path order.
    pub attaining: Vec<u64>,
    /// The class multiplicities of the nested calls of each call or chosen
    /// child that has nested calls, in path order.
    pub nestings: Vec<Vec<u64>>,
    /// The largest nesting depth of a result-path call.
    pub nesting_depth: u64,
    /// The largest number of branching calls on one chain from the top.
    pub branching_depth: u64,
    /// Units on the result path.
    pub units: Units,
    /// `A(K)`.
    pub allowance: u64,
    /// The factorisation of the order of its automorphism group.
    pub group_order: Vec<Factor>,
}

/// The structural record of the whole input: the statistics record of 7.7.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct StatisticsRecord {
    /// The records of the top components, in their order.
    pub components: Vec<ComponentRecord>,
    /// The number of top components.
    pub top_components: u64,
    /// `(bits(n_K) - 1, count)` ascending.
    pub size_histogram: Vec<(u64, u64)>,
    /// Top components of one node.
    pub single_node_components: u64,
    /// Sum over components.
    pub blank_nodes: u64,
    /// Sum over components.
    pub label_elements: u64,
    /// Sum over components.
    pub incidences: u64,
    /// Sum over components.
    pub singled_out: u64,
    /// Sum over components.
    pub unsettled: u64,
    /// The sum of `lines`.
    pub blank_bearing_lines: u64,
    /// The sum of `line_bytes`.
    pub blank_bearing_bytes: u64,
    /// Sums by rule.
    pub calls: RuleCounts,
    /// Sums by rule.
    pub chosen_children: ChildCounts,
    /// Sums by part, FIX including the split into top components.
    pub units: Units,
    /// `A`.
    pub allowance: u64,
    /// Class multiplicities of equal identity, in component order.
    pub top_multiplicities: Vec<u64>,
    /// The factorisation of the order of the input's automorphism group.
    pub group_order: Vec<Factor>,
    /// The first failures, ascending; empty exactly when guaranteed.
    pub broken_by: Vec<FailureRecord>,
    /// Whether every branching call of the result path has a target cell
    /// that is one twin class.
    pub guaranteed: bool,
}

/// The engine's own limit on the limited units of a search (8.1 and 8.3 of the
/// definition). The definition has no caller budget; an engine states its own.
/// The engine trips only past `max(limit, A)` for an input that is not
/// guaranteed, with the typed refusal [`Refusal::Limit`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Limit(u64);

impl Limit {
    /// The limit of the reference engine.
    pub const REFERENCE: Self = Self(reference::ENGINE_LIMIT);

    /// A limit of `units` limited units.
    #[must_use]
    pub const fn new(units: u64) -> Self {
        Self(units)
    }

    /// The limit in limited units.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Why an engine run did not complete.
#[derive(Debug)]
pub(crate) enum Halt {
    /// The definition's typed refusal.
    Refused(Refusal),
    /// The caller's stop signal was set. Not a refusal of the definition.
    Stopped,
}

/// A refusal: a closed list of typed variants.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Refusal {
    /// The input is not well formed (code 0).
    Input {
        /// The smallest reason that applies.
        reason: InputReason,
    },
    /// An empty-graph line names a graph another line carries (code 1).
    Graph {
        /// The smallest term variant that applies.
        term: GraphTerm,
        /// For a blank node, the smallest such source identifier.
        source: Option<Box<[u8]>>,
    },
    /// A quantity does not fit in 64 bits (code 2).
    Width {
        /// The quantity.
        quantity: Quantity,
    },
    /// The engine's own limit tripped on an input that is not guaranteed
    /// (code 3).
    Limit {
        /// The first failures of the search-free run, never empty.
        broken_by: Vec<FailureRecord>,
        /// Engine-specific: the smallest source of the component searched.
        source: Box<[u8]>,
        /// Engine-specific: its node count.
        nodes: u64,
        /// Engine-specific: the limited units spent; determined by the engine
        /// and the input as given, source identifiers included, and NOT a
        /// property of the dataset.
        units: u64,
    },
}

impl Refusal {
    /// The variant's code.
    #[must_use]
    pub const fn code(&self) -> u8 {
        match self {
            Self::Input { .. } => 0,
            Self::Graph { .. } => 1,
            Self::Width { .. } => 2,
            Self::Limit { .. } => 3,
        }
    }
}

/// One top component of the result.
#[derive(Clone, Debug)]
pub struct Component {
    /// Its identity.
    pub identity: [u8; 32],
    /// Its offset.
    pub offset: u64,
    /// Its node count.
    pub nodes: u64,
    /// Its copy index.
    pub copy_index: u64,
    /// The node index of its smallest source identifier.
    pub smallest_node: usize,
    /// The certificate of its top call.
    pub certificate: Certificate,
    /// Engine figure: the twin units the search-free run spent on it.
    /// NOT a property of the dataset and NOT part of the structural record: an
    /// engine figure determined by the input as given, source identifiers
    /// included, which differs between relabellings of one dataset.
    pub twin_units: u64,
    /// `W` of its allowance: over its lines, distinct nodes times start charge.
    pub w: u64,
}

/// The labelled result.
#[derive(Clone, Debug)]
pub struct Labelled {
    /// The canonical map, by node index of the admitted [`LineSet`].
    pub placements: Vec<Placement>,
    /// The canonical line sequence: encodings, ascending.
    pub lines: Vec<Box<[u8]>>,
    /// Its digest.
    pub digest: [u8; 32],
    /// The top components in their order.
    pub components: Vec<Component>,
    /// The structural record.
    pub record: StatisticsRecord,
    /// The diagnostic part: failures with their sources, ascending.
    pub diagnostic: Vec<FailureDiagnostic>,
    /// Engine figure: units over the calls the engine explored.
    /// NOT a property of the dataset and NOT part of the structural record: an
    /// engine figure determined by the input as given, source identifiers
    /// included, which differs between relabellings of one dataset.
    pub explored: Units,
    /// Engine figure: twin units of the search-free run (8.1(f)).
    /// NOT a property of the dataset and NOT part of the structural record: an
    /// engine figure determined by the input as given, source identifiers
    /// included, which differs between relabellings of one dataset.
    pub twin_units: u64,
    /// Engine figure: comparison units (8.1(e)) of the explored calls under
    /// the reference method of the worked examples.
    /// NOT a property of the dataset and NOT part of the structural record: an
    /// engine figure determined by the input as given, source identifiers
    /// included, which differs between relabellings of one dataset.
    pub comparison_units: u64,
}

/// `bits(v)`: the number of binary digits of `v`.
pub(crate) const fn bits(v: u64) -> u64 {
    v.bit_width() as u64
}

/// Write the label of `position` into `out` in the blank-node form of the
/// target syntax: the position in decimal with no leading zero and no affix,
/// printed after `_:` (7.14 of the definition), so position `7` is `_:7` in
/// N-Quads. This is the one place a position becomes text. A label is output
/// only and is never parsed to recover a position or an identity. Position
/// order differs from the byte order of labels (`10` precedes `2`), so a
/// dataset read back from its decimal labels as names is not promised the
/// identity map of the definition's Lemma I.
pub fn write_label<W: TextOut + ?Sized>(position: u64, out: &mut W) {
    write_blank(&position.to_string(), out);
}

/// Admit `lines` and label them with the reference engine.
///
/// # Errors
/// The typed refusal of the definition when the input is ill formed, a
/// quantity overflows, or the engine's own limit trips on an input that is
/// not guaranteed.
pub fn label(lines: &[Line]) -> Result<(LineSet, Labelled), Refusal> {
    let set = admit(lines)?;
    let labelled = reference::label_set(&set, reference::ENGINE_LIMIT, false)?;
    Ok((set, labelled))
}

/// Admit `lines`: intern them and refuse an ill-formed input.
///
/// # Errors
/// `input` or `graph` with the precedence of the definition.
pub fn admit(lines: &[Line]) -> Result<LineSet, Refusal> {
    lines::admit(lines).into_result()
}
