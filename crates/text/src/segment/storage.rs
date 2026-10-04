// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Flat radix lookup and an independently derived minimal acyclic byte automaton.
//!
//! A state's right language determines equivalence: acceptance and ordered
//! (byte, successor-equivalence-class) pairs suffice. Bottom-up hash interning
//! merges those equivalence classes. Counts of accepted suffixes give lexical
//! rank; costs are indexed by rank and never participate in state identity.
//! This follows finite-language equivalence, not an imported implementation.

use purrdf_hash::fixed::FixedState;
use std::collections::HashMap;

/// Semantically interchangeable lookup storage, exposed for measured comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DictionaryRepresentation {
    /// Path-compressed trie; edge labels borrow the word arena.
    Radix,
    /// Minimal acyclic byte automaton; terminal costs are addressed by word rank.
    MinimalAcyclic,
}
/// Retained physical lookup storage, excluding allocator bookkeeping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DictionaryStorageStats {
    /// Actual selected lookup representation.
    pub representation: DictionaryRepresentation,
    /// State count.
    pub states: usize,
    /// Edge count.
    pub edges: usize,
    /// Retained allocations, including the shared word arena, offsets and costs.
    pub retained_bytes: usize,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) enum Storage {
    Radix(Radix),
    Minimal(Dag),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct RadixState {
    first: u32,
    len: u32,
    rank: u32,
}
impl Default for RadixState {
    fn default() -> Self {
        Self {
            first: 0,
            len: 0,
            rank: u32::MAX,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct RadixEdge {
    start: u32,
    len: u32,
    target: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Radix {
    states: Vec<RadixState>,
    edges: Vec<RadixEdge>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct DagState {
    first: u32,
    len: u32,
    count: u32,
    terminal: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct DagEdge {
    target: u32,
    skip: u32,
    byte: u8,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct Dag {
    states: Vec<DagState>,
    edges: Vec<DagEdge>,
    root: u32,
}

impl Storage {
    pub(super) fn build(
        arena: &str,
        offsets: &[u32],
        representation: DictionaryRepresentation,
    ) -> Self {
        let radix = Radix::build(arena.as_bytes(), offsets);
        match representation {
            DictionaryRepresentation::Radix => Self::Radix(radix),
            DictionaryRepresentation::MinimalAcyclic => {
                Self::Minimal(Dag::build(&radix, arena.as_bytes()))
            }
        }
    }
    pub(super) const fn representation(&self) -> DictionaryRepresentation {
        match self {
            Self::Radix(_) => DictionaryRepresentation::Radix,
            Self::Minimal(_) => DictionaryRepresentation::MinimalAcyclic,
        }
    }
    pub(super) fn stats(&self, common_bytes: usize) -> DictionaryStorageStats {
        let (states, edges, bytes) = match self {
            Self::Radix(radix) => (
                radix.states.len(),
                radix.edges.len(),
                radix.states.capacity() * size_of::<RadixState>()
                    + radix.edges.capacity() * size_of::<RadixEdge>(),
            ),
            Self::Minimal(dag) => (
                dag.states.len(),
                dag.edges.len(),
                dag.states.capacity() * size_of::<DagState>()
                    + dag.edges.capacity() * size_of::<DagEdge>(),
            ),
        };
        DictionaryStorageStats {
            representation: self.representation(),
            states,
            edges,
            retained_bytes: common_bytes + bytes,
        }
    }
    pub(super) fn exact(&self, input: &[u8], arena: &str) -> Option<usize> {
        let mut found = None;
        self.prefixes(input, arena, |end, rank| {
            if end == input.len() {
                found = Some(rank);
            }
        });
        found
    }
    pub(super) fn prefixes(&self, input: &[u8], arena: &str, mut sink: impl FnMut(usize, usize)) {
        match self {
            Self::Radix(radix) => {
                let mut node = 0usize;
                let mut position = 0usize;
                while position < input.len() {
                    let state = radix.states[node];
                    let edges =
                        &radix.edges[state.first as usize..(state.first + state.len) as usize];
                    let Ok(at) = edges.binary_search_by_key(&input[position], |edge| {
                        arena.as_bytes()[edge.start as usize]
                    }) else {
                        break;
                    };
                    let edge = edges[at];
                    let label =
                        &arena.as_bytes()[edge.start as usize..(edge.start + edge.len) as usize];
                    if !input[position..].starts_with(label) {
                        break;
                    }
                    position += label.len();
                    node = edge.target as usize;
                    let rank = radix.states[node].rank;
                    if rank != u32::MAX {
                        sink(position, rank as usize);
                    }
                }
            }
            Self::Minimal(dag) => {
                let mut node = dag.root as usize;
                let mut rank = 0usize;
                for (position, &byte) in input.iter().enumerate() {
                    let state = dag.states[node];
                    let edges =
                        &dag.edges[state.first as usize..(state.first + state.len) as usize];
                    let Ok(at) = edges.binary_search_by_key(&byte, |edge| edge.byte) else {
                        break;
                    };
                    let edge = edges[at];
                    rank += edge.skip as usize;
                    node = edge.target as usize;
                    if dag.states[node].terminal {
                        sink(position + 1, rank);
                    }
                }
            }
        }
    }
}
impl Radix {
    fn build(arena: &[u8], offsets: &[u32]) -> Self {
        let mut states = vec![RadixState::default()];
        let mut edges = Vec::new();
        let mut tasks = vec![(0usize, 0usize, offsets.len() - 1, 0usize)];
        while let Some((node, mut begin, end, depth)) = tasks.pop() {
            if begin < end && (offsets[begin + 1] - offsets[begin]) as usize == depth {
                states[node].rank = narrow(begin);
                begin += 1;
            }
            states[node].first = narrow(edges.len());
            while begin < end {
                let start = offsets[begin] as usize + depth;
                let byte = arena[start];
                let mut through = begin + 1;
                while through < end && arena[offsets[through] as usize + depth] == byte {
                    through += 1;
                }
                let first = &arena[start..offsets[begin + 1] as usize];
                let last = &arena[offsets[through - 1] as usize + depth..offsets[through] as usize];
                let shared = first.iter().zip(last).take_while(|(a, b)| a == b).count();
                let child = states.len();
                states.push(RadixState::default());
                edges.push(RadixEdge {
                    start: narrow(start),
                    len: narrow(shared),
                    target: narrow(child),
                });
                states[node].len += 1;
                tasks.push((child, begin, through, depth + shared));
                begin = through;
            }
        }
        states.shrink_to_fit();
        edges.shrink_to_fit();
        Self { states, edges }
    }
}

type Signature = (bool, Vec<(u8, u32)>);
struct Interner {
    classes: HashMap<Signature, u32, FixedState>,
    states: Vec<DagState>,
    edges: Vec<DagEdge>,
}
impl Interner {
    fn intern(&mut self, terminal: bool, transitions: Vec<(u8, u32)>) -> u32 {
        let key = (terminal, transitions);
        if let Some(&state) = self.classes.get(&key) {
            return state;
        }
        let id = narrow(self.states.len());
        let first = narrow(self.edges.len());
        let mut count = u32::from(terminal);
        for &(byte, target) in &key.1 {
            self.edges.push(DagEdge {
                target,
                skip: count,
                byte,
            });
            count += self.states[target as usize].count;
        }
        self.states.push(DagState {
            first,
            len: narrow(key.1.len()),
            count,
            terminal,
        });
        self.classes.insert(key, id);
        id
    }
}
impl Dag {
    fn build(radix: &Radix, arena: &[u8]) -> Self {
        let mut interner = Interner {
            classes: HashMap::with_hasher(FixedState::default()),
            states: Vec::new(),
            edges: Vec::new(),
        };
        let mut classes = vec![0u32; radix.states.len()];
        for at in (0..radix.states.len()).rev() {
            let state = radix.states[at];
            let mut transitions = Vec::with_capacity(state.len as usize);
            for edge in &radix.edges[state.first as usize..(state.first + state.len) as usize] {
                let label = &arena[edge.start as usize..(edge.start + edge.len) as usize];
                let mut target = classes[edge.target as usize];
                for &byte in label[1..].iter().rev() {
                    target = interner.intern(false, vec![(byte, target)]);
                }
                transitions.push((label[0], target));
            }
            classes[at] = interner.intern(state.rank != u32::MAX, transitions);
        }
        interner.states.shrink_to_fit();
        interner.edges.shrink_to_fit();
        Self {
            states: interner.states,
            edges: interner.edges,
            root: classes[0],
        }
    }
}
fn narrow(value: usize) -> u32 {
    u32::try_from(value).expect("states and edges are bounded by the admitted word arena")
}
