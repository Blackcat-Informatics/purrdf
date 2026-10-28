// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Independently checked, program-bound constant-refined weak acyclicity.
//! This is the frontier-dependent WA law used by the restricted witness recipe,
//! not joint acyclicity. New analysis and evidence are governed; execution's
//! existing allocation and interruption contract is separate.

use super::{ChaseError, ChaseOutcome, ChaseTermination};
use crate::clause::{ClauseAtom, ClauseTerm, DlClause};
use crate::seminaive::EvalOptions;
use crate::stop::{StopSignal, is_stopped};
use crate::store::{Arity, RelationStore, StoreShape, TermInterner};
use core::cmp::Ordering;
use purrdf_core::graph::GraphError;
use purrdf_lex::allocation::{Admission, Memory, Resident, StorageError};

/// Version of the refined, frontier-dependent WA proof law.
pub const WA_LAW_VERSION: u32 = 1;

/// Logical work ceiling for preparation or independent checking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnalysisLimits {
    /// Source visits, byte comparisons/hash input, graph visits and buffer moves.
    pub work: u64,
}
impl Default for AnalysisLimits {
    fn default() -> Self {
        Self { work: u64::MAX }
    }
}

/// Exact logical work consumed and currently retained admitted layouts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnalysisReport {
    /// Work admitted before each operation; a refused operation is not performed.
    pub work: u64,
    /// Original evidence buffer layouts, excluding released construction scratch.
    pub retained_bytes: usize,
}

/// An allocation-free authored position identifying one original atom slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourcePosition {
    /// Authored clause ordinal.
    pub clause: usize,
    /// Whether this is a head rather than body atom.
    pub head: bool,
    /// Head disjunct ordinal; zero for body positions.
    pub disjunct: usize,
    /// Atom ordinal within its body or head conjunction.
    pub atom: usize,
    /// Subject, predicate, object or graph slot, starting at zero.
    pub slot: usize,
}

impl purrdf_hash::error::StructuredError for SourcePosition {
    purrdf_hash::error_visit! { @record self { clause, head, disjunct, atom, slot } => "chase-source-position" {
        "clause" => Unsigned(self.clause as u128), "head" => Boolean(self.head),
        "disjunct" => Unsigned(self.disjunct as u128), "atom" => Unsigned(self.atom as u128), "slot" => Unsigned(self.slot as u128),
    }}
}

/// The first existential edge in canonical position order whose endpoints share an SCC.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerminationViolation {
    /// Original source coordinate representing the refined origin position.
    pub from: SourcePosition,
    /// Original source coordinate representing the refined target position.
    pub to: SourcePosition,
}
impl purrdf_hash::error::StructuredError for TerminationViolation {
    purrdf_hash::error_visit! { @record self { from, to } => "chase-wa-violation" {
        "from" => Record(&self.from), "to" => Record(&self.to),
    }}
}
impl core::fmt::Display for TerminationViolation {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            out,
            "existential edge {:?} -> {:?} is cyclic; weak acyclicity is uncertified",
            self.from, self.to
        )
    }
}

/// No error variant is a termination certificate or a proof of nontermination.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminationError {
    /// The sufficient WA criterion did not hold.
    Uncertified(TerminationViolation),
    /// Caller cancellation, with actual analysis work retained.
    Stopped {
        /// Work already admitted.
        work: u64,
    },
    /// The next operation would exceed the selected ceiling or counter width.
    WorkExhausted {
        /// Work already admitted.
        work: u64,
        /// Selected work ceiling.
        limit: u64,
    },
    /// Original allocation/accounting refusal.
    Storage(StorageError),
    /// A malformed internal graph or its original storage error.
    Graph(GraphError),
    /// Evidence belongs to a different complete authored program.
    ProgramMismatch,
    /// Unsupported law version or arity.
    LawMismatch,
    /// Missing, extra, duplicate, noncanonical positions or invalid rank inequalities.
    InvalidEvidence,
}
purrdf_lex::variant_from!(TerminationError { Storage(StorageError), Graph(GraphError) });
impl purrdf_hash::error::StructuredError for TerminationError {
    purrdf_hash::error_visit! {
        Uncertified(value) => "chase-wa-uncertified" { "edge" => Record(value) },
        Stopped { work } => "chase-analysis-stopped" { "work" => Unsigned(u128::from(*work)) },
        WorkExhausted { work, limit } => "chase-analysis-work" { "work" => Unsigned(u128::from(*work)), "limit" => Unsigned(u128::from(*limit)) },
        Storage(cause) => "chase-analysis-storage" { "cause" => Cause(cause) },
        Graph(cause) => "chase-analysis-graph" { "cause" => Cause(cause) },
        ProgramMismatch => "chase-analysis-program" {},
        LawMismatch => "chase-analysis-law" {},
        InvalidEvidence => "chase-analysis-evidence" {},
    }
}
impl core::fmt::Display for TerminationError {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Uncertified(edge) => edge.fmt(out),
            Self::Stopped { work } => {
                write!(out, "termination analysis stopped after {work} work units")
            }
            Self::WorkExhausted { work, limit } => write!(
                out,
                "termination analysis exhausted {limit} work units after {work}"
            ),
            Self::Storage(error) => error.fmt(out),
            Self::Graph(error) => error.fmt(out),
            Self::ProgramMismatch => {
                out.write_str("termination evidence belongs to a different program")
            }
            Self::LawMismatch => {
                out.write_str("termination evidence has an unsupported law or arity")
            }
            Self::InvalidEvidence => {
                out.write_str("termination evidence violates source position or rank obligations")
            }
        }
    }
}
purrdf_lex::error_source!(TerminationError { Storage, Graph });

struct Meter<'a> {
    work: u64,
    limits: AnalysisLimits,
    stop: Option<&'a dyn StopSignal>,
}
impl Meter<'_> {
    fn charge(&mut self, units: usize) -> Result<(), TerminationError> {
        if is_stopped(self.stop) {
            return Err(TerminationError::Stopped { work: self.work });
        }
        let next = u64::try_from(units)
            .ok()
            .and_then(|units| self.work.checked_add(units));
        match next {
            Some(next) if next <= self.limits.work => {
                self.work = next;
                Ok(())
            }
            _ => Err(TerminationError::WorkExhausted {
                work: self.work,
                limit: self.limits.work,
            }),
        }
    }
}

/// Owned rank evidence with its original native allocation account.
/// No clone can duplicate buffers outside that account. The account starts empty.
/// Use [`Self::close`] to destroy evidence and reclaim its grant with typed errors.
/// Dropping without closing destroys buffers but conservatively leaves a borrowed
/// account charged; Drop never swallows a failed refund. Resident accounts hold no
/// external grant. These rules also apply while the certificate is prepared.
pub struct WaCertificate<A: Admission = Resident> {
    program: [u8; 32],
    law: u32,
    arity: usize,
    positions: Vec<SourcePosition>,
    ranks: Vec<u64>,
    existential_edges: usize,
    report: AnalysisReport,
    // Rust drops fields in declaration order: the original account outlives both buffers.
    account: A,
}
purrdf_hash::debug_non_exhaustive!([A: Admission] WaCertificate<A> { program, law, arity, report });

impl<A: Admission> WaCertificate<A> {
    /// Original production analysis work and retained admitted evidence layouts.
    #[must_use]
    pub const fn report(&self) -> AnalysisReport {
        self.report
    }
    /// The complete program digest, using the canonical authored IR encoding.
    #[must_use]
    pub const fn program_identity(&self) -> &[u8; 32] {
        &self.program
    }
    /// The inspectable exact source positions and their corresponding ranks.
    #[must_use]
    pub fn positions(&self) -> &[SourcePosition] {
        &self.positions
    }
    /// Ranks correspond one-for-one to [`Self::positions`].
    #[must_use]
    pub fn ranks(&self) -> &[u64] {
        &self.ranks
    }
    /// The WA summary carried by the existing chase outcome.
    #[must_use]
    pub fn summary(&self) -> ChaseTermination {
        ChaseTermination::WeaklyAcyclic {
            positions: self.positions.len(),
            existential_edges: self.existential_edges,
        }
    }
    /// Destroy all evidence before releasing its original grant.
    /// # Errors
    /// Returns the account's original refund refusal; no evidence remains alive.
    pub fn close(mut self) -> Result<(), TerminationError> {
        drop(core::mem::take(&mut self.positions));
        drop(core::mem::take(&mut self.ranks));
        self.account.resize(0)?;
        Ok(())
    }
    /// Check every obligation independently and bind this evidence to immutable source.
    /// The checker rebuilds source obligations, but performs no SCC or termination search.
    /// # Errors
    /// Returns altered evidence/source, original work/stop or storage refusal.
    pub fn prepare<'p, const N: usize>(
        mut self,
        program: &'p [DlClause<N>],
        limits: AnalysisLimits,
        stop: Option<&dyn StopSignal>,
    ) -> Result<PreparedChase<'p, A, N>, TerminationError> {
        let checked = self.verify(program, limits, stop);
        match checked {
            Ok(verification) => Ok(PreparedChase {
                program,
                certificate: self,
                verification,
            }),
            Err(original) => {
                self.close()?;
                Err(original)
            }
        }
    }
    /// Independently check source identity, exact position coverage and all rank inequalities.
    /// # Errors
    /// Returns malformed evidence, original accounting failure, cancellation or work exhaustion.
    pub fn verify<const N: usize>(
        &mut self,
        program: &[DlClause<N>],
        limits: AnalysisLimits,
        stop: Option<&dyn StopSignal>,
    ) -> Result<AnalysisReport, TerminationError> {
        let mut meter = Meter {
            work: 0,
            limits,
            stop,
        };
        if self.law != WA_LAW_VERSION || self.arity != N {
            return Err(TerminationError::LawMismatch);
        }
        let identity =
            crate::cache::canonical_rule_hash_with_control(program, |n| meter.charge(n))?;
        if identity != self.program {
            return Err(TerminationError::ProgramMismatch);
        }
        let mut memory = Memory::resume(&mut self.account, self.report.retained_bytes);
        memory.try_scope(|memory| {
            let positions = source_positions(program, memory, &mut meter)?;
            if positions.len() != self.positions.len() || self.ranks.len() != positions.len() {
                return Err(TerminationError::InvalidEvidence);
            }
            for (original, supplied) in positions.iter().zip(&self.positions) {
                meter.charge(1)?;
                if original != supplied {
                    return Err(TerminationError::InvalidEvidence);
                }
            }
            let mut specials = Vec::new();
            walk_edges(program, &mut meter, |from, to, special, meter| {
                let from = locate(program, &positions, from, meter)?;
                let to = locate(program, &positions, to, meter)?;
                check_rank(&self.ranks, from, to, special)?;
                if special {
                    insert_sorted(&mut specials, (from, to), memory, meter)?;
                }
                Ok(())
            })?;
            if specials.len() != self.existential_edges {
                return Err(TerminationError::InvalidEvidence);
            }
            compatibility(program, &positions, &mut meter, |from, to, _| {
                check_rank(&self.ranks, from, to, false)?;
                check_rank(&self.ranks, to, from, false)
            })?;
            memory.release_vec(specials)?;
            memory.release_vec(positions)?;
            meter.charge(0)?;
            Ok(AnalysisReport {
                work: meter.work,
                retained_bytes: memory.admitted_bytes(),
            })
        })
    }
}

/// Immutable program and independently checked admission, reusable across source stores.
/// Execution has the existing chase allocation/interrupt contract. Call [`Self::close`]
/// when done to reclaim the original evidence grant with typed failure.
pub struct PreparedChase<'p, A: Admission, const N: usize = 4> {
    program: &'p [DlClause<N>],
    certificate: WaCertificate<A>,
    verification: AnalysisReport,
}
purrdf_hash::debug_non_exhaustive!(['p, A: Admission, const N: usize] PreparedChase<'p, A, N> { certificate, verification });
impl<A: Admission, const N: usize> PreparedChase<'_, A, N> {
    /// The original verified admission evidence.
    #[must_use]
    pub const fn certificate(&self) -> &WaCertificate<A> {
        &self.certificate
    }
    /// Work spent checking, separate from production and execution work.
    #[must_use]
    pub const fn verification(&self) -> AnalysisReport {
        self.verification
    }
    /// Execute the existing chase with this already-checked admission.
    /// # Errors
    /// Returns the existing executor's fragment, storage, stop or budget failure.
    pub fn chase_with(
        &self,
        edb: RelationStore<TermInterner, N>,
        options: &EvalOptions,
        stop: Option<&dyn StopSignal>,
    ) -> Result<ChaseOutcome<N>, ChaseError>
    where
        Arity<N>: StoreShape<N>,
    {
        super::execute(self.program, edb, options, stop, self.certificate.summary())
    }
    /// Destroy evidence before releasing its original account.
    /// # Errors
    /// Returns the original refund refusal.
    pub fn close(self) -> Result<(), TerminationError> {
        self.certificate.close()
    }
}

/// Compute native WA rank evidence with resident physical admission.
/// # Errors
/// Returns uncertified WA, malformed graph or native allocation failure.
pub fn certify<const N: usize>(program: &[DlClause<N>]) -> Result<WaCertificate, TerminationError> {
    certify_with(program, Resident, AnalysisLimits::default(), None)
}

/// Compute WA rank evidence through an initially empty original account.
/// Every source visit, comparison, buffer move, hash chunk and SCC visit is governed.
/// # Errors
/// Returns uncertified WA, stop, work exhaustion or original storage/accounting failure.
pub fn certify_with<const N: usize, A: Admission>(
    program: &[DlClause<N>],
    mut account: A,
    limits: AnalysisLimits,
    stop: Option<&dyn StopSignal>,
) -> Result<WaCertificate<A>, TerminationError> {
    let mut meter = Meter {
        work: 0,
        limits,
        stop,
    };
    let identity = crate::cache::canonical_rule_hash_with_control(program, |n| meter.charge(n))?;
    let mut memory = Memory::new(&mut account);
    let (positions, ranks, existential_edges) = memory.try_scope(|memory| {
        let positions = source_positions(program, memory, &mut meter)?;
        let mut adjacency = Vec::new();
        for _ in &positions {
            meter.charge(1)?;
            memory.push(&mut adjacency, Vec::new())?;
        }
        let mut specials = Vec::new();
        walk_edges(program, &mut meter, |from, to, special, meter| {
            let from = locate(program, &positions, from, meter)?;
            let to = locate(program, &positions, to, meter)?;
            insert_sorted(&mut adjacency[from], to, memory, meter)?;
            if special {
                insert_sorted(&mut specials, (from, to), memory, meter)?;
            }
            Ok(())
        })?;
        compatibility(program, &positions, &mut meter, |from, to, meter| {
            insert_sorted(&mut adjacency[from], to, memory, meter)?;
            insert_sorted(&mut adjacency[to], from, memory, meter)
        })?;
        let components =
            purrdf_core::graph::scc_component_index_with_control(&adjacency, memory, || {
                meter.charge(1)
            })?;
        for &(from, to) in &specials {
            meter.charge(1)?;
            if components[from] == components[to] {
                return Err(TerminationError::Uncertified(TerminationViolation {
                    from: positions[from],
                    to: positions[to],
                }));
            }
        }
        let mut ranks = Vec::new();
        for &component in &components {
            meter.charge(1)?;
            memory.push(
                &mut ranks,
                u64::try_from(positions.len() - component)
                    .map_err(|_| StorageError::SizeOverflow)?,
            )?;
        }
        let existential_edges = specials.len();
        while let Some(edges) = adjacency.pop() {
            memory.release_vec(edges)?;
        }
        memory.release_vec(adjacency)?;
        memory.release_vec(specials)?;
        memory.release_vec(components)?;
        meter.charge(0)?;
        Ok((positions, ranks, existential_edges))
    })?;
    let report = AnalysisReport {
        work: meter.work,
        retained_bytes: memory.admitted_bytes(),
    };
    Ok(WaCertificate {
        account,
        program: identity,
        law: WA_LAW_VERSION,
        arity: N,
        positions,
        ranks,
        existential_edges,
        report,
    })
}

fn check_rank(
    ranks: &[u64],
    from: usize,
    to: usize,
    special: bool,
) -> Result<(), TerminationError> {
    if if special {
        ranks[from] < ranks[to]
    } else {
        ranks[from] <= ranks[to]
    } {
        Ok(())
    } else {
        Err(TerminationError::InvalidEvidence)
    }
}

fn atom<const N: usize>(
    program: &[DlClause<N>],
    position: SourcePosition,
) -> Result<&ClauseAtom<N>, TerminationError> {
    let clause = program
        .get(position.clause)
        .ok_or(TerminationError::InvalidEvidence)?;
    if position.slot >= N {
        return Err(TerminationError::InvalidEvidence);
    }
    if position.head {
        clause
            .head_disjuncts()
            .get(position.disjunct)
            .and_then(|head| head.atoms().get(position.atom))
            .ok_or(TerminationError::InvalidEvidence)
    } else if position.disjunct == 0 {
        clause
            .body()
            .get(position.atom)
            .ok_or(TerminationError::InvalidEvidence)
    } else {
        Err(TerminationError::InvalidEvidence)
    }
}

fn surface(term: &ClauseTerm) -> (&str, &str, &str) {
    match term {
        ClauseTerm::Iri(value) => ("<", value, ">"),
        ClauseTerm::Literal(value) => ("", value, ""),
        ClauseTerm::DefaultGraph | ClauseTerm::Var(_) => ("", "", ""),
    }
}
fn compare_term(
    left: &ClauseTerm,
    right: &ClauseTerm,
    meter: &mut Meter<'_>,
) -> Result<Ordering, TerminationError> {
    meter.charge(1)?;
    match (left.is_var(), right.is_var()) {
        (true, true) => return Ok(Ordering::Equal),
        (true, false) => return Ok(Ordering::Greater),
        (false, true) => return Ok(Ordering::Less),
        (false, false) => {}
    }
    let (a, b, c) = surface(left);
    let (d, e, f) = surface(right);
    let mut left = a.bytes().chain(b.bytes()).chain(c.bytes());
    let mut right = d.bytes().chain(e.bytes()).chain(f.bytes());
    loop {
        meter.charge(1)?;
        match (left.next(), right.next()) {
            (None, None) => return Ok(Ordering::Equal),
            (a, b) if a != b => return Ok(a.cmp(&b)),
            _ => {}
        }
    }
}
fn compare_position<const N: usize>(
    program: &[DlClause<N>],
    left: SourcePosition,
    right: SourcePosition,
    meter: &mut Meter<'_>,
) -> Result<Ordering, TerminationError> {
    meter.charge(1)?;
    let slot = left.slot.cmp(&right.slot);
    if slot != Ordering::Equal {
        return Ok(slot);
    }
    let left_terms = atom(program, left)?.terms();
    let right_terms = atom(program, right)?.terms();
    for index in 0..N {
        if index == left.slot {
            continue;
        }
        let order = compare_term(left_terms[index], right_terms[index], meter)?;
        if order != Ordering::Equal {
            return Ok(order);
        }
    }
    Ok(Ordering::Equal)
}
fn search<const N: usize>(
    program: &[DlClause<N>],
    positions: &[SourcePosition],
    position: SourcePosition,
    meter: &mut Meter<'_>,
) -> Result<Result<usize, usize>, TerminationError> {
    let (mut lo, mut hi) = (0, positions.len());
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        match compare_position(program, positions[mid], position, meter)? {
            Ordering::Less => lo = mid + 1,
            Ordering::Greater => hi = mid,
            Ordering::Equal => return Ok(Ok(mid)),
        }
    }
    Ok(Err(lo))
}
fn locate<const N: usize>(
    program: &[DlClause<N>],
    positions: &[SourcePosition],
    position: SourcePosition,
    meter: &mut Meter<'_>,
) -> Result<usize, TerminationError> {
    search(program, positions, position, meter)?.map_err(|_| TerminationError::InvalidEvidence)
}
fn source_positions<const N: usize, A: Admission + ?Sized>(
    program: &[DlClause<N>],
    memory: &mut Memory<'_, A>,
    meter: &mut Meter<'_>,
) -> Result<Vec<SourcePosition>, TerminationError> {
    let mut positions = Vec::new();
    walk_edges(program, meter, |from, to, _, meter| {
        for position in [from, to] {
            if let Err(at) = search(program, &positions, position, meter)? {
                meter.charge(positions.len() - at)?;
                memory.reserve_for_push(&mut positions)?;
                positions.insert(at, position);
            }
        }
        Ok(())
    })?;
    Ok(positions)
}
fn insert_sorted<T: Ord, A: Admission + ?Sized>(
    rows: &mut Vec<T>,
    value: T,
    memory: &mut Memory<'_, A>,
    meter: &mut Meter<'_>,
) -> Result<(), TerminationError> {
    let (mut lo, mut hi) = (0, rows.len());
    while lo < hi {
        meter.charge(1)?;
        let mid = lo + (hi - lo) / 2;
        match rows[mid].cmp(&value) {
            Ordering::Less => lo = mid + 1,
            Ordering::Greater => hi = mid,
            Ordering::Equal => return Ok(()),
        }
    }
    meter.charge(rows.len() - lo)?;
    memory.reserve_for_push(rows)?;
    rows.insert(lo, value);
    Ok(())
}
fn compatibility<const N: usize>(
    program: &[DlClause<N>],
    positions: &[SourcePosition],
    meter: &mut Meter<'_>,
    mut emit: impl FnMut(usize, usize, &mut Meter<'_>) -> Result<(), TerminationError>,
) -> Result<(), TerminationError> {
    for (i, &left) in positions.iter().enumerate() {
        for (j, &right) in positions.iter().enumerate().skip(i + 1) {
            meter.charge(1)?;
            if left.slot != right.slot {
                continue;
            }
            let a = atom(program, left)?.terms();
            let b = atom(program, right)?.terms();
            let mut compatible = true;
            for slot in 0..N {
                meter.charge(1)?;
                if slot != left.slot
                    && !a[slot].is_var()
                    && !b[slot].is_var()
                    && compare_term(a[slot], b[slot], meter)? != Ordering::Equal
                {
                    compatible = false;
                    break;
                }
            }
            if compatible {
                emit(i, j, meter)?;
            }
        }
    }
    Ok(())
}
fn same_name(left: &str, right: &str, meter: &mut Meter<'_>) -> Result<bool, TerminationError> {
    if left.len() != right.len() {
        meter.charge(1)?;
        return Ok(false);
    }
    for (a, b) in left.bytes().zip(right.bytes()) {
        meter.charge(1)?;
        if a != b {
            return Ok(false);
        }
    }
    Ok(true)
}
fn walk_edges<const N: usize>(
    program: &[DlClause<N>],
    meter: &mut Meter<'_>,
    mut emit: impl FnMut(
        SourcePosition,
        SourcePosition,
        bool,
        &mut Meter<'_>,
    ) -> Result<(), TerminationError>,
) -> Result<(), TerminationError> {
    for (ci, clause) in program.iter().enumerate() {
        meter.charge(1)?;
        for (bi, body) in clause.body().iter().enumerate() {
            for (slot, term) in body.terms().into_iter().enumerate() {
                meter.charge(1)?;
                let Some(variable) = term.variable() else {
                    continue;
                };
                let from = SourcePosition {
                    clause: ci,
                    head: false,
                    disjunct: 0,
                    atom: bi,
                    slot,
                };
                let mut frontier = false;
                for (di, head) in clause.head_disjuncts().iter().enumerate() {
                    for (ai, head_atom) in head.atoms().iter().enumerate() {
                        for (hs, ht) in head_atom.terms().into_iter().enumerate() {
                            meter.charge(1)?;
                            if let Some(name) = ht.variable()
                                && same_name(variable, name, meter)?
                            {
                                frontier = true;
                                emit(
                                    from,
                                    SourcePosition {
                                        clause: ci,
                                        head: true,
                                        disjunct: di,
                                        atom: ai,
                                        slot: hs,
                                    },
                                    false,
                                    meter,
                                )?;
                            }
                        }
                    }
                }
                if !frontier {
                    continue;
                }
                for existential in clause.existentials() {
                    meter.charge(1)?;
                    for (di, head) in clause.head_disjuncts().iter().enumerate() {
                        for (ai, head_atom) in head.atoms().iter().enumerate() {
                            for (hs, ht) in head_atom.terms().into_iter().enumerate() {
                                meter.charge(1)?;
                                if let Some(name) = ht.variable()
                                    && same_name(existential, name, meter)?
                                {
                                    emit(
                                        from,
                                        SourcePosition {
                                            clause: ci,
                                            head: true,
                                            disjunct: di,
                                            atom: ai,
                                            slot: hs,
                                        },
                                        true,
                                        meter,
                                    )?;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clause::HeadDisjunct;
    use std::cell::Cell;
    use std::rc::Rc;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

    #[global_allocator]
    static ALLOCATOR: purrdf_alloc_probe::CountingAllocator = purrdf_alloc_probe::CountingAllocator;

    fn rule() -> DlClause {
        let body = ClauseAtom::quad(
            ClauseTerm::var("x"),
            ClauseTerm::iri("https://example.org/p"),
            ClauseTerm::iri("https://example.org/C"),
            ClauseTerm::DefaultGraph,
        );
        let head = ClauseAtom::quad(
            ClauseTerm::var("x"),
            ClauseTerm::iri("https://example.org/q"),
            ClauseTerm::var("y"),
            ClauseTerm::DefaultGraph,
        );
        DlClause::new(vec![HeadDisjunct::atom(head)], vec!["y".into()], vec![body])
    }
    fn edb() -> RelationStore {
        let mut store = RelationStore::new();
        store
            .insert(
                "<https://example.org/a>",
                "<https://example.org/p>",
                "<https://example.org/C>",
                RelationStore::DEFAULT_GRAPH,
            )
            .unwrap();
        store
    }

    /// Prepared admission reuses exact source proof while retaining native model and witness attribution.
    #[test]
    fn prepared_admission_reuses_evidence_without_changing_chase_results() {
        let program = [rule()];
        let prepared = certify(&program)
            .unwrap()
            .prepare(&program, AnalysisLimits::default(), None)
            .unwrap();
        let original_work = prepared.certificate().report();
        let check_work = prepared.verification();
        for _ in 0..2 {
            let reference = super::super::chase(&program, edb()).unwrap();
            let reused = prepared
                .chase_with(edb(), &EvalOptions::default(), None)
                .unwrap();
            assert_eq!(
                reference.facts().facts_sorted(),
                reused.facts().facts_sorted()
            );
            assert_eq!(reference.derivations(), reused.derivations());
            assert_eq!(
                reference.witnesses().witnesses().collect::<Vec<_>>(),
                reused.witnesses().witnesses().collect::<Vec<_>>()
            );
            assert_eq!(reference.budget(), reused.budget());
            assert_eq!(prepared.certificate().report(), original_work);
            assert_eq!(prepared.verification(), check_work);
        }
        prepared.close().unwrap();
    }

    /// Independent checking rejects altered programs, missing/duplicate coordinates and false rank evidence.
    #[test]
    fn checker_rejects_forged_source_positions_ranks_and_identity() {
        let program = [rule()];
        for mutation in 0..6 {
            let mut evidence = certify(&program).unwrap();
            match mutation {
                0 => evidence.ranks.fill(0),
                1 => {
                    evidence.positions.pop();
                }
                2 => {
                    evidence.positions.push(evidence.positions[0]);
                }
                3 => {
                    evidence.positions[0].clause = usize::MAX;
                }
                4 => evidence.law += 1,
                _ => evidence.arity = 3,
            }
            assert!(
                evidence
                    .verify(&program, AnalysisLimits::default(), None)
                    .is_err()
            );
            evidence.close().unwrap();
        }
        let mut evidence = certify(&program).unwrap();
        let changed = [DlClause::datalog(
            ClauseAtom::quad(
                ClauseTerm::var("x"),
                ClauseTerm::iri("https://example.org/changed"),
                ClauseTerm::iri("https://example.org/C"),
                ClauseTerm::DefaultGraph,
            ),
            program[0].body().to_vec(),
        )];
        assert_eq!(
            evidence.verify(&changed, AnalysisLimits::default(), None),
            Err(TerminationError::ProgramMismatch)
        );
        evidence
            .verify(&program, AnalysisLimits::default(), None)
            .unwrap();
        evidence.close().unwrap();
    }

    #[derive(Debug)]
    struct StopAfter {
        polls: AtomicUsize,
        cut: usize,
    }
    impl StopSignal for StopAfter {
        fn stopped(&self) -> bool {
            self.polls.fetch_add(1, AtomicOrdering::Relaxed) >= self.cut
        }
    }

    /// Exact work ceilings admit the whole proof; one unit less and cancellation never certify a prefix.
    #[test]
    fn analysis_and_checker_work_boundaries_and_stops_are_typed() {
        let program = [rule()];
        let signal = StopAfter {
            polls: AtomicUsize::new(0),
            cut: usize::MAX,
        };
        let mut evidence =
            certify_with(&program, Resident, AnalysisLimits::default(), Some(&signal)).unwrap();
        let production = evidence.report().work;
        let polls = signal.polls.load(AtomicOrdering::Relaxed);
        let checked = evidence
            .verify(&program, AnalysisLimits::default(), None)
            .unwrap()
            .work;
        evidence
            .verify(&program, AnalysisLimits { work: checked }, None)
            .unwrap();
        assert!(matches!(
            evidence.verify(&program, AnalysisLimits { work: checked - 1 }, None),
            Err(TerminationError::WorkExhausted { .. })
        ));
        evidence.close().unwrap();
        certify_with(
            &program,
            Resident,
            AnalysisLimits { work: production },
            None,
        )
        .unwrap()
        .close()
        .unwrap();
        assert!(matches!(
            certify_with(
                &program,
                Resident,
                AnalysisLimits {
                    work: production - 1
                },
                None
            ),
            Err(TerminationError::WorkExhausted { .. })
        ));
        for cut in [0, polls / 2, polls - 1] {
            let stop = StopAfter {
                polls: AtomicUsize::new(0),
                cut,
            };
            assert!(matches!(
                certify_with(&program, Resident, AnalysisLimits::default(), Some(&stop)),
                Err(TerminationError::Stopped { .. })
            ));
        }
    }

    struct Account {
        live: Rc<Cell<usize>>,
        peak: Rc<Cell<usize>>,
        cap: usize,
        refuse_refund: Rc<Cell<bool>>,
    }
    impl Admission for Account {
        fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
            if bytes > self.cap || (bytes < self.live.get() && self.refuse_refund.get()) {
                return Err(StorageError::AdmissionFailed);
            }
            self.live.set(bytes);
            self.peak.set(self.peak.get().max(bytes));
            Ok(())
        }
    }
    fn account(cap: usize) -> Account {
        Account {
            live: Rc::new(Cell::new(0)),
            peak: Rc::new(Cell::new(0)),
            cap,
            refuse_refund: Rc::new(Cell::new(false)),
        }
    }

    /// Original grants cover preparation, verification and retained proof; explicit close owns refunds.
    #[test]
    fn evidence_keeps_original_grants_and_reports_refund_failure() {
        let program = [rule()];
        let mut storage = account(usize::MAX);
        let live = storage.live.clone();
        let peak = storage.peak.clone();
        let evidence =
            certify_with(&program, &mut storage, AnalysisLimits::default(), None).unwrap();
        assert_eq!(live.get(), evidence.report().retained_bytes);
        let prepared = evidence
            .prepare(&program, AnalysisLimits::default(), None)
            .unwrap();
        assert_eq!(live.get(), prepared.certificate().report().retained_bytes);
        let needed = peak.get();
        prepared.close().unwrap();
        assert_eq!(live.get(), 0);
        let mut exact = account(needed);
        certify_with(&program, &mut exact, AnalysisLimits::default(), None)
            .unwrap()
            .prepare(&program, AnalysisLimits::default(), None)
            .unwrap()
            .close()
            .unwrap();
        let mut short = account(needed - 1);
        let result = certify_with(&program, &mut short, AnalysisLimits::default(), None)
            .and_then(|proof| proof.prepare(&program, AnalysisLimits::default(), None));
        assert!(matches!(
            result,
            Err(TerminationError::Storage(StorageError::AdmissionFailed))
        ));
        let mut failed_refund = account(usize::MAX);
        let retained = failed_refund.live.clone();
        let refuse = failed_refund.refuse_refund.clone();
        let evidence = certify_with(
            &program,
            &mut failed_refund,
            AnalysisLimits::default(),
            None,
        )
        .unwrap();
        refuse.set(true);
        assert_eq!(
            evidence.close(),
            Err(TerminationError::Storage(StorageError::AdmissionFailed))
        );
        assert!(retained.get() > 0);
        let mut conservative = account(usize::MAX);
        let held = conservative.live.clone();
        let evidence =
            certify_with(&program, &mut conservative, AnalysisLimits::default(), None).unwrap();
        let before = held.get();
        drop(evidence);
        assert_eq!(
            held.get(),
            before,
            "Drop does not silently attempt a fallible refund"
        );
    }

    /// Graph slots and multiple existential names obey the same WA law; no frontier invents only one family.
    #[test]
    fn graph_cycles_and_frontier_empty_multiple_witnesses_keep_their_law() {
        let cycle = [DlClause::new(
            vec![HeadDisjunct::atom(ClauseAtom::quad(
                ClauseTerm::var("g"),
                ClauseTerm::iri("https://example.org/p"),
                ClauseTerm::iri("https://example.org/C"),
                ClauseTerm::var("new"),
            ))],
            vec!["new".into()],
            vec![ClauseAtom::quad(
                ClauseTerm::var("x"),
                ClauseTerm::iri("https://example.org/p"),
                ClauseTerm::iri("https://example.org/C"),
                ClauseTerm::var("g"),
            )],
        )];
        assert!(matches!(
            certify(&cycle),
            Err(TerminationError::Uncertified(_))
        ));
        let independent = [DlClause::new(
            vec![HeadDisjunct::atom(ClauseAtom::quad(
                ClauseTerm::var("first"),
                ClauseTerm::iri("https://example.org/q"),
                ClauseTerm::var("second"),
                ClauseTerm::DefaultGraph,
            ))],
            vec!["first".into(), "second".into()],
            rule().body().to_vec(),
        )];
        let prepared = certify(&independent)
            .unwrap()
            .prepare(&independent, AnalysisLimits::default(), None)
            .unwrap();
        let result = prepared
            .chase_with(edb(), &EvalOptions::default(), None)
            .unwrap();
        assert_eq!(result.witnesses().len(), 2);
        prepared.close().unwrap();
    }

    /// Preparation and checker costs are measured separately; reuse performs neither analysis again.
    #[test]
    fn preparation_work_and_owned_allocation_measurements_are_reproducible() {
        for count in [1, 2, 4] {
            let program = vec![rule(); count];
            let run = || {
                let window = purrdf_alloc_probe::CurrentThreadWindow::open();
                let evidence = certify(&program).unwrap();
                let production = evidence.report();
                let prepared = evidence
                    .prepare(&program, AnalysisLimits::default(), None)
                    .unwrap();
                let checked = prepared.verification();
                let measured = window.close();
                prepared.close().unwrap();
                (production, checked, measured)
            };
            let (production, checked, memory) = run();
            assert_eq!(run(), (production, checked, memory));
            println!(
                "WA clauses {count}: production work {}, checker work {}, retained bytes {}, allocation calls {}, requested bytes {}, peak bytes {}",
                production.work,
                checked.work,
                production.retained_bytes,
                memory.allocations,
                memory.requested_bytes,
                memory.peak_working_bytes
            );
        }
    }
}
