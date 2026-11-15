// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Revision 3.1 of the canonical blank-node labelling, in two modes.
//!
//! An evaluation module, hidden from the documentation. The literal oracle
//! (every child of every branching call, eager copies, a full search for
//! components) lives in the `canon_converge` example. [`optimised`]
//! computes the same result with a structure-of-arrays partition, an undo
//! trail, lazy restriction, incremental component discovery and the pruning the
//! definition permits. Both read [`Input`], the tokenised counted lines.

pub mod compare;
pub mod optimised;
pub mod view;
pub mod wl2;

use std::cmp::Ordering;
use std::fmt;
use std::sync::Arc;

use crate::FastMap;

/// One counted line holding at least one blank node.
#[derive(Clone, Debug)]
pub struct Line {
    /// The counted text as written (an annotation row of count above one
    /// begins `@a*k`).
    pub text: String,
    /// Byte ranges of `text` between blank-node tokens: one more than `occ`.
    pub segs: Vec<(u32, u32)>,
    /// The node of each blank occurrence, in order of appearance.
    pub occ: Vec<u32>,
    pub count: u64,
}

impl Line {
    /// The text with each blank token written by `token`.
    pub fn spell(&self, out: &mut Vec<u8>, mut token: impl FnMut(u32, &mut Vec<u8>)) {
        let bytes = self.text.as_bytes();
        for (i, &(a, b)) in self.segs.iter().enumerate() {
            out.extend_from_slice(&bytes[a as usize..b as usize]);
            if let Some(&node) = self.occ.get(i) {
                token(node, out);
            }
        }
    }

    /// The distinct nodes of the line, in order of first appearance.
    pub fn nodes(&self) -> Vec<u32> {
        let mut seen: Vec<u32> = Vec::with_capacity(self.occ.len());
        for &u in &self.occ {
            if !seen.contains(&u) {
                seen.push(u);
            }
        }
        seen
    }
}

/// The lines of a fold.
#[derive(Clone, Debug, Default)]
pub struct Input {
    /// Distinct lines with a blank node.
    pub lines: Vec<Line>,
    /// Distinct lines with no blank node (written into the document as they are).
    pub ground: Vec<String>,
    /// The input's own label of each node.
    pub labels: Vec<String>,
}

/// A line text the tokeniser cannot read.
#[derive(Debug)]
pub struct BadLine(pub String);

/// The segments, the blank occurrences and the count of one line.
type Tokens = (Vec<(u32, u32)>, Vec<u32>, u64);

/// Read one counted line text into its marker, terms and blank occurrences.
fn tokenise(
    text: &str,
    ids: &mut FastMap<String, u32>,
    labels: &mut Vec<String>,
) -> Result<Tokens, BadLine> {
    let b = text.as_bytes();
    let bad = |why: &str| BadLine(format!("{why}: {text}"));
    let mut count = 1u64;
    // The marker and the one space after it.
    let mut i = 0usize;
    if b.first() == Some(&b'@') {
        let word_end = text.find(' ').ok_or_else(|| bad("marker without a term"))?;
        let word = &text[..word_end];
        i = word_end + 1;
        match word {
            "@r" | "@a" | "@g" => {}
            "@s" => {
                // The kind letter, then one space.
                let kind_end = text[i..]
                    .find(' ')
                    .map(|k| i + k)
                    .ok_or_else(|| bad("suppression entry without a body"))?;
                i = kind_end + 1;
            }
            other => {
                let digits = other
                    .strip_prefix("@a*")
                    .ok_or_else(|| bad("unknown marker"))?;
                count = digits
                    .parse::<u64>()
                    .map_err(|_| bad("annotation count is not an unsigned 64-bit integer"))?;
            }
        }
    }
    let mut segs = Vec::new();
    let mut occ = Vec::new();
    let mut seg_start = 0usize;
    // `i` is a term position.
    while i < b.len() {
        let rest = &text[i..];
        let end = if rest.starts_with("<<()>>") {
            i + 6
        } else if rest.starts_with("<<(") || rest.starts_with(")>>") {
            i + 3
        } else if b[i] == b'<' {
            i + rest.find('>').ok_or_else(|| bad("unclosed IRI"))? + 1
        } else if b[i] == b'"' {
            let mut j = i + 1;
            while j < b.len() && b[j] != b'"' {
                j += if b[j] == b'\\' { 2 } else { 1 };
            }
            if j >= b.len() {
                return Err(bad("unclosed literal"));
            }
            j += 1;
            while j < b.len() && b[j] != b' ' {
                j += 1;
            }
            j
        } else if rest.starts_with("_:") {
            let j = rest.find(' ').map_or(b.len(), |k| i + k);
            let label = &text[i + 2..j];
            let next = u32::try_from(labels.len()).map_err(|_| bad("width"))?;
            let id = *ids.entry(label.to_owned()).or_insert_with(|| {
                labels.push(label.to_owned());
                next
            });
            segs.push((seg_start as u32, i as u32));
            occ.push(id);
            seg_start = j;
            j
        } else if b[i] == b'.' {
            i + 1
        } else {
            return Err(bad("no term at a term position"));
        };
        if end < b.len() && b[end] != b' ' {
            return Err(bad("a term not followed by one space"));
        }
        i = end + 1;
    }
    segs.push((seg_start as u32, b.len() as u32));
    Ok((segs, occ, count))
}

impl Input {
    /// Read a document of counted line texts, one per line.
    pub fn read(text: &str) -> Result<Self, BadLine> {
        let mut ids: FastMap<String, u32> = FastMap::default();
        let mut input = Self::default();
        let mut seen: crate::FastSet<&str> = crate::FastSet::default();
        for raw in text.lines() {
            let line = raw.trim_end_matches('\r');
            if line.is_empty() || line.starts_with('#') || !seen.insert(line) {
                continue;
            }
            let (segs, occ, count) = tokenise(line, &mut ids, &mut input.labels)?;
            if occ.is_empty() {
                input.ground.push(line.to_owned());
            } else {
                input.lines.push(Line {
                    text: line.to_owned(),
                    segs,
                    occ,
                    count,
                });
            }
        }
        Ok(input)
    }

    pub fn nodes(&self) -> usize {
        self.labels.len()
    }

    /// The masked text of a line with no fixed node (5.6): every token `_:`.
    pub fn plain_masked(&self, l: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.lines[l].text.len());
        self.lines[l].spell(&mut out, |_, o| o.extend_from_slice(b"_:"));
        out
    }

    /// Per node, the lines holding it (each once), ascending.
    pub fn node_lines(&self) -> Vec<Vec<u32>> {
        let mut out = vec![Vec::new(); self.nodes()];
        for (l, line) in self.lines.iter().enumerate() {
            for u in line.nodes() {
                out[u as usize].push(l as u32);
            }
        }
        out
    }
}

/// A certificate entry `[m, text]` (6.3).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Entry {
    pub m: u64,
    pub text: Box<[u8]>,
}

/// A certificate `(E, C)` (6.3). The derived order is the order of the text:
/// `E` as a list (a proper prefix first), then `C` the same way.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Cert {
    pub e: Vec<Entry>,
    pub c: Vec<Arc<Self>>,
}

/// The cost of a byte string in units: one per 8 bytes, rounded up.
pub const fn bytes_cost(len: usize) -> u64 {
    len.div_ceil(8) as u64
}

/// Compare two entries, charging (8.1(e)) one unit for `m` plus one per 8
/// bytes of text read: none when `m` differs; else up to and including the
/// first differing byte, the shorter text when one is a prefix, the whole text
/// when equal.
pub fn cmp_entry(a: &Entry, b: &Entry, compared: &mut u64) -> Ordering {
    *compared += 1;
    if a.m != b.m {
        return a.m.cmp(&b.m);
    }
    let read = a
        .text
        .iter()
        .zip(b.text.iter())
        .position(|(x, y)| x != y)
        .map_or_else(|| a.text.len().min(b.text.len()), |i| i + 1);
    *compared += bytes_cost(read);
    a.text.cmp(&b.text)
}

/// Compare two lists of entries, a proper prefix first, charging each entry
/// comparison; stops at the first difference.
pub fn cmp_entries(a: &[Entry], b: &[Entry], compared: &mut u64) -> Ordering {
    for (x, y) in a.iter().zip(b) {
        match cmp_entry(x, y, compared) {
            Ordering::Equal => {}
            o => return o,
        }
    }
    a.len().cmp(&b.len())
}

/// Compare two certificates, charging each entry compared (8.1(e)); stops at
/// the first difference.
/// Compare two invariants (lists of cell sizes), charging one unit per
/// element compared.
pub fn cmp_invariant(a: &[u32], b: &[u32], units: &mut u64) -> Ordering {
    for (x, y) in a.iter().zip(b) {
        *units += 1;
        match x.cmp(y) {
            Ordering::Equal => {}
            o => return o,
        }
    }
    a.len().cmp(&b.len())
}

pub fn cmp_cert(a: &Cert, b: &Cert, compared: &mut u64) -> Ordering {
    match cmp_entries(&a.e, &b.e, compared) {
        Ordering::Equal => {}
        o => return o,
    }
    for (x, y) in a.c.iter().zip(&b.c) {
        match cmp_cert(x, y, compared) {
            Ordering::Equal => {}
            o => return o,
        }
    }
    a.c.len().cmp(&b.c.len())
}

/// The units of forming entries (8.1(d)): one plus one per 8 bytes each.
pub fn formed_cost(e: &[Entry]) -> u64 {
    e.iter().map(|x| 1 + bytes_cost(x.text.len())).sum()
}

/// The start charge of a top component (8.1(a)): per line one unit plus one
/// per 8 bytes of its masked text.
pub fn start_cost(input: &Input, lines: &[u32]) -> u64 {
    lines
        .iter()
        .map(|&l| 1 + bytes_cost(input.plain_masked(l as usize).len()))
        .sum()
}

/// The chosen structure of one call: the nodes it fixed, in path-position
/// order, and its nested calls in certificate order.
#[derive(Clone, Debug)]
pub struct Chosen {
    pub cert: Arc<Cert>,
    pub fixed: Vec<u32>,
    pub nested: Vec<Self>,
    pub size: u64,
    /// The structural statistics of this call's result path.
    pub path: PathStats,
}

/// Structural statistics of a result path: the call, the chosen child at
/// every branching call and every nested call under them (never a discarded
/// child). Lists are in pre-order, nested calls in certificate order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PathStats {
    pub discrete: u64,
    pub nested: u64,
    pub branched: u64,
    /// Target-cell sizes at the branching calls.
    pub targets: Vec<u64>,
    /// At each branching call, the children attaining the smallest certificate.
    pub attaining: Vec<u64>,
    /// At each nesting, the size of every class of two or more siblings with
    /// equal certificates.
    pub multiplicities: Vec<u64>,
    pub nest_depth: u64,
    pub branch_depth: u64,
    /// Units by part: refinement label elements of the chosen children,
    /// removed vertices, removed incidences, split charges, entries formed.
    pub refine: u64,
    pub removed_vertices: u64,
    pub removed_incidences: u64,
    pub split: u64,
    pub formed: u64,
}

impl PathStats {
    fn add(&mut self, o: &Self) {
        self.discrete += o.discrete;
        self.nested += o.nested;
        self.branched += o.branched;
        self.targets.extend_from_slice(&o.targets);
        self.attaining.extend_from_slice(&o.attaining);
        self.multiplicities.extend_from_slice(&o.multiplicities);
        self.refine += o.refine;
        self.removed_vertices += o.removed_vertices;
        self.removed_incidences += o.removed_incidences;
        self.split += o.split;
        self.formed += o.formed;
    }

    /// A fixing with its own charges `own`, then its nested calls (sorted by
    /// certificate).
    pub fn of_nest(own: Self, nested: &[Chosen]) -> Self {
        let mut p = own;
        p.multiplicities
            .extend(classes(nested.iter().map(|n| &n.cert)));
        for n in nested {
            p.add(&n.path);
            p.nest_depth = p.nest_depth.max(n.path.nest_depth + 1);
            p.branch_depth = p.branch_depth.max(n.path.branch_depth);
        }
        p
    }

    /// A branching call: its chosen child's path, this call first.
    pub fn of_branch(child: &Self, target: u64, attaining: u64) -> Self {
        let mut p = child.clone();
        p.branched += 1;
        p.targets.insert(0, target);
        p.attaining.insert(0, attaining);
        p.branch_depth += 1;
        p
    }
}

/// Sizes of the classes of two or more equal values in a sorted sequence.
pub fn classes<'a>(sorted: impl Iterator<Item = &'a Arc<Cert>>) -> Vec<u64> {
    let mut out = Vec::new();
    let mut run: Option<(&Arc<Cert>, u64)> = None;
    for c in sorted {
        run = match run {
            Some((prev, k)) if prev == c => Some((prev, k + 1)),
            Some((_, k)) => {
                if k >= 2 {
                    out.push(k);
                }
                Some((c, 1))
            }
            None => Some((c, 1)),
        };
    }
    if let Some((_, k)) = run
        && k >= 2
    {
        out.push(k);
    }
    out
}

/// The structural record of one top component (a function of the input).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TopRecord {
    pub nodes: u64,
    pub lines: u64,
    /// Blank occurrences over its lines.
    pub elements: u64,
    pub root_singletons: u64,
    pub root_nonsingleton: u64,
    pub root_largest: u64,
    pub forest: bool,
    pub start: u64,
    pub root_refine: u64,
    pub path: PathStats,
}

impl TopRecord {
    /// The record of a top component after its root REFINE (`node_cells` are
    /// the sizes of its node cells).
    pub fn root(input: &Input, nodes: &[u32], lines: &[u32], node_cells: &[usize]) -> Self {
        let incidences: usize = lines
            .iter()
            .map(|&l| input.lines[l as usize].nodes().len())
            .sum();
        Self {
            nodes: nodes.len() as u64,
            lines: lines.len() as u64,
            elements: lines
                .iter()
                .map(|&l| input.lines[l as usize].occ.len() as u64)
                .sum(),
            root_singletons: node_cells.iter().filter(|&&s| s == 1).count() as u64,
            root_nonsingleton: node_cells.iter().filter(|&&s| s > 1).sum::<usize>() as u64,
            root_largest: node_cells
                .iter()
                .copied()
                .filter(|&s| s > 1)
                .max()
                .unwrap_or(0) as u64,
            forest: incidences + 1 == nodes.len() + lines.len(),
            start: start_cost(input, lines),
            root_refine: 0,
            path: PathStats::default(),
        }
    }
}

fn json_list(v: &[u64]) -> String {
    let items: Vec<String> = v.iter().map(u64::to_string).collect();
    format!("[{}]", items.join(", "))
}

/// The structural part of a statistics record, as JSON.
pub fn structural_json(done: &Labelled) -> String {
    let one = |r: &TopRecord| {
        let p = &r.path;
        format!(
            "{{\"nodes\": {}, \"lines\": {}, \"label_elements\": {}, \"root_singletons\": {}, \"root_nonsingleton_nodes\": {}, \"root_largest_cell\": {}, \"forest\": {}, \"calls_discrete\": {}, \"calls_nested\": {}, \"calls_branched\": {}, \"target_sizes\": {}, \"attaining\": {}, \"multiplicities\": {}, \"max_nest_depth\": {}, \"max_branch_depth\": {}, \"units_start\": {}, \"units_refine\": {}, \"units_removed_vertices\": {}, \"units_removed_incidences\": {}, \"units_split\": {}, \"units_formed\": {}}}",
            r.nodes,
            r.lines,
            r.elements,
            r.root_singletons,
            r.root_nonsingleton,
            r.root_largest,
            r.forest,
            p.discrete,
            p.nested,
            p.branched,
            json_list(&p.targets),
            json_list(&p.attaining),
            json_list(&p.multiplicities),
            p.nest_depth,
            p.branch_depth,
            r.start,
            r.root_refine + p.refine,
            p.removed_vertices,
            p.removed_incidences,
            p.split,
            p.formed,
        )
    };
    let comps: Vec<String> = done.records.iter().map(one).collect();
    let (factors, order) = group_order(done);
    let total = |f: &dyn Fn(&TopRecord) -> u64| done.records.iter().map(f).sum::<u64>();
    format!(
        "{{\"components\": [{}], \"top_multiplicities\": {}, \"top_split\": {}, \"nodes\": {}, \"lines\": {}, \"label_elements\": {}, \"structural_units\": {}, \"group_order_factors\": [{}], \"group_order\": {}}}",
        comps.join(", "),
        json_list(&done.top_multiplicities),
        done.top_split,
        total(&|r| r.nodes),
        total(&|r| r.lines),
        total(&|r| r.elements),
        done.top_split
            + total(&|r| r.start
                + r.root_refine
                + r.path.refine
                + r.path.removed_vertices
                + r.path.removed_incidences
                + r.path.split
                + r.path.formed),
        factors.join(", "),
        order.map_or_else(|| "null".to_owned(), |o| format!("\"{o}\"")),
    )
}

/// The order of the automorphism group: the attaining counts (above one) and
/// the factorials of the sibling multiplicities, and their product when it
/// fits 128 bits.
pub fn group_order(done: &Labelled) -> (Vec<String>, Option<u128>) {
    let mut factors = Vec::new();
    let mut product: Option<u128> = Some(1);
    let mul = |x: u64, product: &mut Option<u128>| {
        *product = product.and_then(|p| p.checked_mul(u128::from(x)));
    };
    let multiplicities = done
        .records
        .iter()
        .flat_map(|r| r.path.multiplicities.iter())
        .chain(done.top_multiplicities.iter());
    for r in &done.records {
        for &a in &r.path.attaining {
            if a > 1 {
                factors.push(format!("\"{a}\""));
                mul(a, &mut product);
            }
        }
    }
    for &k in multiplicities {
        factors.push(format!("\"{k}!\""));
        for x in 2..=k {
            mul(x, &mut product);
        }
    }
    (factors, product)
}

/// The engine part of a statistics record (informative), as JSON.
pub fn engine_json(s: &Stats) -> String {
    format!(
        "{{\"leaves\": {}, \"children\": {}, \"pruned_orbit\": {}, \"pruned_entries\": {}, \"pruned_nested_entries\": {}, \"pruned_twin\": {}, \"comparison_units\": {}, \"total_units\": {}, \"generators\": {}}}",
        s.leaves,
        s.children,
        s.pruned_orbit,
        s.pruned_e,
        s.pruned_nested_e,
        s.pruned_twin,
        s.entries_compared,
        s.units(),
        s.generators
    )
}

impl Chosen {
    /// Final positions (6.5) of every node of this call, from offset `o`, by an
    /// explicit pre-order stack.
    pub fn assign(&self, o: u64, out: &mut [u64]) {
        let mut stack: Vec<(&Self, u64)> = vec![(self, o)];
        while let Some((call, o)) = stack.pop() {
            for (i, &node) in call.fixed.iter().enumerate() {
                out[node as usize] = o + i as u64;
            }
            let mut next = o + call.fixed.len() as u64;
            for nested in &call.nested {
                stack.push((nested, next));
                next += nested.size;
            }
        }
    }
}

/// Which non-singleton node cell a branching call targets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selector {
    /// The text's rule: the first non-singleton cell.
    First,
    /// Experiment: the first of the smallest non-singleton cells.
    Smallest,
    /// Experiment: the first of the largest non-singleton cells.
    Largest,
    /// Experiment: the first largest non-singleton cell contained in the
    /// target cell of the nearest branching ancestor (then the next ancestor),
    /// falling back to `Largest`.
    Traces,
}

/// The index of the target among `cells` (members of each node cell, in
/// colour order); `ancestors` are the target cells of the enclosing branching
/// calls, innermost last.
pub fn pick_target(sel: Selector, cells: &[&[u32]], ancestors: &[Vec<u32>]) -> usize {
    if sel == Selector::Traces {
        for anc in ancestors.iter().rev() {
            let set: crate::FastSet<u32> = anc.iter().copied().collect();
            let inside = cells.iter().map(|c| {
                if c.iter().all(|v| set.contains(v)) {
                    c.len()
                } else {
                    0
                }
            });
            if let Some(i) = Selector::Largest.pick(inside) {
                return i;
            }
        }
        return Selector::Largest
            .pick(cells.iter().map(|c| c.len()))
            .expect("a non-singleton cell");
    }
    sel.pick(cells.iter().map(|c| c.len()))
        .expect("a non-singleton cell")
}

impl Selector {
    /// The index of the target among cells of the given sizes.
    pub fn pick(self, sizes: impl Iterator<Item = usize>) -> Option<usize> {
        let mut best: Option<(usize, usize)> = None;
        for (i, s) in sizes.enumerate() {
            if s < 2 {
                continue;
            }
            best = match (self, best) {
                (_, None) => Some((i, s)),
                (Self::First | Self::Traces, b) => b,
                (Self::Smallest, Some((_, bs))) if s < bs => Some((i, s)),
                (Self::Largest, Some((_, bs))) if s > bs => Some((i, s)),
                (_, b) => b,
            };
        }
        best.map(|(i, _)| i)
    }
}

/// A switch of a run. `Invariant`, `WholeFold`, `Wl2` and `Trace` are
/// experiments (off by default); the rest are OPTIMISED shortcuts (on by
/// default, ignored by LITERAL).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flag {
    /// Compare the ordered cell sizes of a branching call's children first.
    Invariant = 1,
    /// Revision 3: one refinement over the whole fold (LITERAL only).
    WholeFold = 2,
    /// Two-dimensional refinement in calls that reach rule 3.
    Wl2 = 4,
    /// Write every REFINE step to standard error (LITERAL only).
    Trace = 8,
    Orbits = 16,
    Twins = 32,
    PruneE = 64,
    PruneNestedE = 128,
    /// Keep the unfinished component in place (lazy restriction).
    Lazy = 256,
    /// Stop component discovery when all but one search has finished.
    Incremental = 512,
}

/// How to run.
#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Work units after which the run is refused.
    pub limit: u64,
    pub selector: Selector,
    pub flags: u16,
}

impl Options {
    pub const DEFAULT: Self = Self {
        limit: u64::MAX,
        selector: Selector::First,
        flags: Flag::Orbits as u16
            | Flag::Twins as u16
            | Flag::PruneE as u16
            | Flag::PruneNestedE as u16
            | Flag::Lazy as u16
            | Flag::Incremental as u16,
    };

    pub const fn has(self, flag: Flag) -> bool {
        self.flags & flag as u16 != 0
    }

    #[must_use]
    pub const fn with(mut self, flag: Flag, on: bool) -> Self {
        if on {
            self.flags |= flag as u16;
        } else {
            self.flags &= !(flag as u16);
        }
        self
    }
}

/// What a run did. Units (8.1 of revision 3.2) are `start + visit_elements +
/// restriction + entries_formed + entries_compared`; everything else is
/// bookkeeping.
#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub nodes: u64,
    pub lines: u64,
    pub top_components: u64,
    /// Experiment: the units of the node invariant (one per element formed,
    /// one per element compared).
    pub invariant: u64,
    /// 8.1(a): the start charge of the masked texts of top components.
    pub start: u64,
    /// Incidences visited by REFINE (each incidence of a member of a splitter).
    pub visits: u64,
    /// The same visits counted by label element (a node repeated `r` times in
    /// a line counts `r`).
    pub visit_elements: u64,
    pub entries_formed: u64,
    pub entries_compared: u64,
    /// 8.1(c): removed vertices, removed incidences and the split charge of
    /// every FIX and of the split into top components.
    pub restriction: u64,
    /// Comparisons of two keys when ordering fragments (not units).
    pub key_compares: u64,
    /// Pairs popped from the queue.
    pub pops: u64,
    pub calls: u64,
    pub branch_calls: u64,
    /// Nodes in calls that reached rule 3, and the largest target cell met.
    pub branch_nodes: u64,
    pub largest_target: u64,
    /// Children of branching calls whose refinement was run.
    pub children: u64,
    /// Calls that ended as leaves (rule 1).
    pub leaves: u64,
    pub pruned_orbit: u64,
    pub pruned_twin: u64,
    pub pruned_e: u64,
    pub pruned_nested_e: u64,
    pub pruned_invariant: u64,
    /// Two children, or two sibling calls, with equal certificates were met.
    pub equal_certificates: u64,
    pub generators: u64,
    /// Vertices visited by component discovery (not units).
    pub discovery_steps: u64,
    /// Vertices copied by restriction (not units).
    pub copied: u64,
    /// Undo-trail records written (not units).
    pub trail: u64,
    pub max_depth: u64,
    /// Two-dimensional refinement: pair-colour updates (bookkeeping).
    pub wl2_steps: u64,
    /// Two-dimensional refinement: one unit per tuple element formed (14.3).
    pub wl2_units: u64,
}

impl Stats {
    pub fn units(&self) -> u64 {
        self.start
            + self.invariant
            + self.wl2_units
            + self.visit_elements
            + self.restriction
            + self.entries_formed
            + self.entries_compared
    }
}

/// Why nothing is labelled (8.3).
#[derive(Debug)]
pub enum Refusal {
    /// The engine's own work limit was reached.
    Limit(Box<Stats>),
    /// A count would not fit an unsigned 64-bit integer.
    Width,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Limit(s) => write!(
                f,
                "not labelled: limit (units {}, nodes in branching calls {}, largest target cell {})",
                s.units(),
                s.branch_nodes,
                s.largest_target
            ),
            Self::Width => f.write_str("not labelled: width"),
        }
    }
}

/// A finished labelling.
#[derive(Debug)]
pub struct Labelled {
    /// The final position of every node.
    pub positions: Vec<u64>,
    /// The result (7.6): the spelled lines (counted form) holding a blank
    /// node, sorted bytewise, one per row, each ending in a line feed. Lines
    /// with no blank node are not in it.
    pub document: String,
    /// The top certificates in order (for tests and the dependence search).
    pub tops: Vec<Arc<Cert>>,
    /// The structural record of each top component, in certificate order.
    pub records: Vec<TopRecord>,
    pub top_multiplicities: Vec<u64>,
    /// The split charge of the whole fold into top components.
    pub top_split: u64,
    pub stats: Stats,
}

impl Labelled {
    pub fn has_automorphisms(&self) -> bool {
        self.stats.equal_certificates > 0
            || self.stats.pruned_orbit > 0
            || self.stats.pruned_twin > 0
    }
}

/// The certificate entry of a fully fixed line (6.3), positions read from `pos`.
pub fn entry(line: &Line, pos: &[u64]) -> Entry {
    let mut text = Vec::with_capacity(line.text.len() + 4 * line.occ.len());
    let mut m = 0;
    line.spell(&mut text, |u, o| {
        let q = pos[u as usize];
        m = m.max(q);
        o.extend_from_slice(b"_:");
        o.extend_from_slice(q.to_string().as_bytes());
    });
    Entry {
        m,
        text: text.into_boxed_slice(),
    }
}

/// The document of a finished labelling: identifiers `c14n<final position>`,
/// written as blank-node tokens `_:c14n<n>`.
pub fn document(input: &Input, positions: &[u64]) -> String {
    let mut rows: Vec<Vec<u8>> = input
        .lines
        .iter()
        .map(|line| {
            let mut out = Vec::with_capacity(line.text.len() + 8);
            line.spell(&mut out, |u, o| {
                o.extend_from_slice(b"_:c14n");
                o.extend_from_slice(positions[u as usize].to_string().as_bytes());
            });
            out
        })
        .collect();
    rows.sort_unstable();
    let mut doc = String::new();
    for row in rows {
        doc.push_str(std::str::from_utf8(&row).expect("spelled from UTF-8"));
        doc.push('\n');
    }
    doc
}

/// The connected components of the incidence graph over `nodes` (free nodes,
/// `free[u]` true) through `lines`, each as (nodes ascending, lines ascending),
/// in order of their smallest node.
pub fn components(
    input: &Input,
    node_lines: &[Vec<u32>],
    nodes: &[u32],
    free: &[bool],
    steps: &mut u64,
) -> Vec<(Vec<u32>, Vec<u32>)> {
    let mut comp_of: FastMap<u32, usize> = FastMap::default();
    let mut line_seen: crate::FastSet<u32> = crate::FastSet::default();
    let mut out: Vec<(Vec<u32>, Vec<u32>)> = Vec::new();
    let mut sorted = nodes.to_vec();
    sorted.sort_unstable();
    for &start in &sorted {
        if comp_of.contains_key(&start) {
            continue;
        }
        let id = out.len();
        let mut cn = vec![start];
        let mut cl = Vec::new();
        comp_of.insert(start, id);
        let mut k = 0;
        while k < cn.len() {
            let u = cn[k];
            k += 1;
            *steps += 1;
            for &l in &node_lines[u as usize] {
                if !line_seen.insert(l) {
                    continue;
                }
                cl.push(l);
                *steps += 1;
                for &w in &input.lines[l as usize].occ {
                    if free[w as usize] && !comp_of.contains_key(&w) {
                        comp_of.insert(w, id);
                        cn.push(w);
                    }
                }
            }
        }
        cn.sort_unstable();
        cl.sort_unstable();
        out.push((cn, cl));
    }
    out
}

/// The label of the incidence of `u` in `line` (4.1): the ascending free
/// occurrence numbers of `u`, a node being free when `free[node]`.
pub fn label(line: &Line, u: u32, free: &[bool]) -> Vec<u64> {
    let mut out = Vec::with_capacity(1);
    let mut k = 0u64;
    for &w in &line.occ {
        if free[w as usize] {
            if w == u {
                out.push(k);
            }
            k += 1;
        }
    }
    out
}
