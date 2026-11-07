// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Opaque proofs and original-run-bound checking evidence.

use super::{
    BTreeSet, Engine, FolAnswerContract, FolBudget, FolLit, FolProgram, FolProofError, FolScope,
    FolStatus, FolTerm, FolVerdict, NodeId, SortContext, Subst, Truth, Unified, canon,
    evaluated_scope, freshen_clause, ground, unify,
};
use crate::term::{RetainedGraph, RetainedTerm, TermDag, TermSource};
use purrdf_lex::allocation::{Memory, Resident, Shared, StorageError};

/// Existing SHA-1 rule/premise Merkle and flat identity laws, explicitly versioned.
/// No earlier public proof record is silently decoded as this opaque owner.
pub const FOL_PROOF_IDENTITY_VERSION: u32 = 1;

/// Immutable original-run evidence. Only the resolver can mint it. Cloning
/// retains the original source capability, program, sorts, bounded admission,
/// completed-demand inventory and one structural graph; it creates no new arena.
#[derive(Debug, Clone)]
pub struct FolCheckEvidence(Shared<CheckContext>);

#[derive(Debug)]
#[cfg_attr(test, derive(Clone))]
struct CheckContext {
    source: TermSource,
    program: FolProgram,
    sorts: SortContext,
    budget: FolBudget,
    scope: FolScope,
    calls: BTreeSet<String>,
    rules: BTreeSet<String>,
    negatives: BTreeSet<String>,
    steps: u64,
    graph: RetainedGraph,
}

/// An opaque owned ground proof. All descendants share the original checking
/// evidence and retained graph. Future proof forms can extend the private step
/// representation without changing a public variant payload.
#[derive(Debug, Clone)]
pub struct FolProof {
    evidence: FolCheckEvidence,
    step: Shared<ProofStep>,
}

#[derive(Debug, Clone)]
pub(super) enum ProofStep {
    Assert {
        rule: usize,
        rule_identity: String,
        goal: NodeId,
    },
    ByRule {
        rule: usize,
        rule_identity: String,
        goal: NodeId,
        premises: Vec<FolProof>,
    },
}

impl FolCheckEvidence {
    pub(super) fn capture(
        dag: &mut TermDag,
        program: &FolProgram,
        sorts: &SortContext,
        budget: FolBudget,
        scope: FolScope,
        engine: &Engine,
    ) -> Result<Self, StorageError> {
        let source = dag.source()?;
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut roots = Vec::new();
        memory.push(&mut roots, program.goal())?;
        for (node, _) in program.goal_vars() {
            memory.push(&mut roots, *node)?;
        }
        for clause in program.clauses() {
            memory.push(&mut roots, clause.head())?;
            for literal in clause.body() {
                memory.push(&mut roots, literal.atom())?;
            }
        }
        for &sort in program.meta_sorts().values() {
            memory.push(&mut roots, sort)?;
        }
        for node in sorts
            .nodes()
            .chain(engine.calls.values().copied())
            .chain(engine.atoms.values().copied())
        {
            memory.push(&mut roots, node)?;
        }
        let negatives = engine
            .ground_rules
            .values()
            .flat_map(|rule| rule.neg.iter().map(|&atom| canon(dag, atom)))
            .collect();
        let context = CheckContext {
            source,
            program: program.clone(),
            sorts: sorts.clone(),
            budget,
            scope,
            calls: engine.calls.keys().cloned().collect(),
            rules: engine.ground_rules.keys().cloned().collect(),
            negatives,
            steps: engine.steps,
            graph: RetainedGraph::capture(dag, &roots)?,
        };
        Ok(Self(
            Shared::try_new(context).map_err(|_| StorageError::AllocationFailed)?,
        ))
    }

    /// Original ordered program, including projection and sort declarations.
    #[must_use]
    pub fn program(&self) -> &FolProgram {
        &self.0.program
    }
    /// Original immutable order-sorted context.
    #[must_use]
    pub fn sort_context(&self) -> &SortContext {
        &self.0.sorts
    }
    /// Original explicitly selected answer contract.
    #[must_use]
    pub fn answer_contract(&self) -> FolAnswerContract {
        self.0.scope.contract
    }
    /// Original run completion status; this alone never licenses absence.
    #[must_use]
    pub fn status(&self) -> FolStatus {
        self.0.scope.status
    }
    /// The admitted grounding work ceiling reused by negative checking replay.
    #[must_use]
    pub fn budget(&self) -> FolBudget {
        self.0.budget
    }

    pub(super) fn graph(&self) -> &RetainedGraph {
        &self.0.graph
    }
    pub(super) fn scope(&self) -> &FolScope {
        &self.0.scope
    }
    pub(super) fn same(&self, other: &Self) -> bool {
        Shared::ptr_eq(&self.0, &other.0)
    }

    fn validate_negative_completion(&self, dag: &mut TermDag) -> Result<FolScope, FolProofError> {
        let original = &*self.0;
        if original.scope.status != FolStatus::Complete {
            return Err(FolProofError::CompletionIncomplete {
                steps: original.steps,
                max_steps: original.budget.max_steps,
            });
        }
        // Independently check this coverage edge, rather than trusting a model
        // set or completion flag produced by the evaluator.
        if original
            .negatives
            .iter()
            .any(|key| !original.calls.contains(key) || !original.scope.ground.contains(key))
        {
            return Err(FolProofError::InvalidCompletion);
        }
        let mut replay = Engine::default();
        if ground(
            dag,
            &mut replay,
            &original.program,
            &original.sorts,
            original.budget,
        )? {
            return Err(FolProofError::InvalidCompletion);
        }
        if replay.exhausted {
            return Err(FolProofError::CompletionIncomplete {
                steps: replay.steps,
                max_steps: original.budget.max_steps,
            });
        }
        let scope = evaluated_scope(dag, &replay, original.scope.contract);
        if replay.calls.keys().ne(original.calls.iter())
            || replay.ground_rules.keys().ne(original.rules.iter())
            || scope != original.scope
        {
            return Err(FolProofError::InvalidCompletion);
        }
        Ok(scope)
    }
}

impl FolProof {
    pub(super) fn new(evidence: &FolCheckEvidence, step: ProofStep) -> Result<Self, StorageError> {
        Ok(Self {
            evidence: evidence.clone(),
            step: Shared::try_new(step).map_err(|_| StorageError::AllocationFailed)?,
        })
    }

    fn header(&self) -> (usize, &str, NodeId) {
        match &*self.step {
            ProofStep::Assert {
                rule,
                rule_identity,
                goal,
            }
            | ProofStep::ByRule {
                rule,
                rule_identity,
                goal,
                ..
            } => (*rule, rule_identity, *goal),
        }
    }
    /// Whether this node is an unconditional assertion.
    #[must_use]
    pub fn is_assert(&self) -> bool {
        matches!(&*self.step, ProofStep::Assert { .. })
    }
    /// Original authored clause index.
    #[must_use]
    pub fn rule(&self) -> usize {
        self.header().0
    }
    /// Original content-addressed clause identity, re-derived by the checker.
    #[must_use]
    pub fn rule_identity(&self) -> &str {
        self.header().1
    }
    /// Original local conclusion handle; use [`Self::conclusion`] after source death.
    #[must_use]
    pub fn goal(&self) -> NodeId {
        self.header().2
    }
    /// Positive premises, each retaining this same original checking owner.
    #[must_use]
    pub fn premises(&self) -> &[Self] {
        match &*self.step {
            ProofStep::Assert { .. } => &[],
            ProofStep::ByRule { premises, .. } => premises,
        }
    }
    /// Structural conclusion independent of producer or answer-row lifetime.
    #[must_use]
    pub fn conclusion(&self) -> FolTerm<'_> {
        FolTerm::retained(self.evidence.graph(), self.goal())
    }
    /// Original source/program/sort/contract-bound checking evidence.
    #[must_use]
    pub fn checking_evidence(&self) -> &FolCheckEvidence {
        &self.evidence
    }
    /// Version of the unchanged existing derivation-identity laws.
    #[must_use]
    pub fn identity_version(&self) -> u32 {
        FOL_PROOF_IDENTITY_VERSION
    }
    /// Original derivation identity, reproducible after source destruction.
    #[must_use]
    pub fn derivation_id(&self) -> String {
        super::derivation_id(self.evidence.graph(), self)
    }
}

impl PartialEq for FolProof {
    fn eq(&self, other: &Self) -> bool {
        self.evidence.same(&other.evidence)
            && core::ptr::eq(&raw const *self.step, &raw const *other.step)
    }
}
impl Eq for FolProof {}

/// Re-derive a ground proof under its original typed evidence, never an evaluator
/// Boolean. Foreign sources/programs/sorts/runs are refused. Negative checks
/// validate demanded coverage and replay the one ground implementation within
/// the original admitted budget, then compare the complete closure and WFS model.
/// This is bounded replay and structural proof checking, not algorithm diversity.
/// Structural checking separately admits each distinct proof node and premise
/// edge under that same retained ceiling; shared steps are checked once.
///
/// Checking nodes, symbols and metavariable mints are rolled back, including on
/// failure or unwinding. Allocation capacities may remain warm. The live
/// original arena remains required for checking; owned inspection does not.
///
/// # Errors
/// Returns typed source/context, structural, completion or native storage refusals.
pub fn check_fol_proof(
    dag: &mut TermDag,
    proof: &FolProof,
    evidence: &FolCheckEvidence,
) -> Result<NodeId, FolProofError> {
    if !dag.has_source(&evidence.0.source) || !evidence.0.source.same(&proof.evidence.0.source) {
        return Err(FolProofError::ForeignSource);
    }
    if proof.evidence.0.program != evidence.0.program {
        return Err(FolProofError::ForeignProgram);
    }
    if proof.evidence.0.sorts != evidence.0.sorts {
        return Err(FolProofError::ForeignSorts);
    }
    if proof.evidence.answer_contract() != evidence.answer_contract() {
        return Err(FolProofError::ForeignContract);
    }
    if !proof.evidence.same(evidence) {
        return Err(FolProofError::ForeignRun);
    }
    let mut transaction = dag.transaction();
    let dag = transaction.dag();
    let mut negative_scope = None;
    let mut checked = std::collections::BTreeMap::new();
    let mut pending =
        purrdf_lex::walk::WorkList::<(&FolProof, Option<&super::FolClause>), 16>::with((
            proof, None,
        ));
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let mut steps = 0_u64;
    while let Some((current, expanded)) = pending.pop() {
        if !current.evidence.same(evidence) {
            return Err(FolProofError::ForeignRun);
        }
        let key = core::ptr::from_ref(&*current.step);
        if checked.contains_key(&key) {
            continue;
        }
        if let Some(clause) = expanded {
            let conclusion = check_node(
                dag,
                current,
                evidence,
                clause,
                &mut negative_scope,
                &checked,
            )?;
            checked.insert(key, conclusion);
        } else {
            let clause = checked_clause(dag, current, evidence)?;
            let charge = 1 + current.premises().len() as u64;
            if charge > evidence.budget().max_steps - steps {
                return Err(FolProofError::CheckingWorkExhausted {
                    steps,
                    max_steps: evidence.budget().max_steps,
                });
            }
            steps += charge;
            pending.try_push_admitted((current, Some(clause)), &mut memory)?;
            for premise in current.premises().iter().rev() {
                pending.try_push_admitted((premise, None), &mut memory)?;
            }
        }
    }
    Ok(*checked
        .get(&core::ptr::from_ref(&*proof.step))
        .expect("the original proof was checked"))
}

fn checked_clause<'a>(
    dag: &TermDag,
    proof: &FolProof,
    evidence: &'a FolCheckEvidence,
) -> Result<&'a super::FolClause, FolProofError> {
    let rule = proof.rule();
    let goal = proof.goal();
    if evidence.graph().get(goal).is_none() || goal.index() >= dag.len() as u64 {
        return Err(FolProofError::UnknownConclusion { goal });
    }
    if !dag.free_meta(goal).is_empty() {
        return Err(FolProofError::NonGroundConclusion { goal });
    }
    let clause = evidence
        .program()
        .clauses()
        .get(rule)
        .ok_or(FolProofError::UnknownRule { rule })?;
    if proof.is_assert() {
        if !clause.body().is_empty() {
            return Err(FolProofError::AssertHasBody { rule });
        }
    } else {
        let positive = clause
            .body()
            .iter()
            .filter(|literal| matches!(literal, FolLit::Pos(_)))
            .count();
        if proof.premises().len() != positive {
            return Err(FolProofError::PremiseCountMismatch {
                rule,
                body: positive,
                premises: proof.premises().len(),
            });
        }
    }
    Ok(clause)
}

fn check_node(
    dag: &mut TermDag,
    proof: &FolProof,
    evidence: &FolCheckEvidence,
    clause: &super::FolClause,
    negative_scope: &mut Option<FolScope>,
    checked: &std::collections::BTreeMap<*const ProofStep, NodeId>,
) -> Result<NodeId, FolProofError> {
    let original = &*evidence.0;
    let rule = proof.rule();
    let goal = proof.goal();
    let (head, body, declarations) = freshen_clause(dag, clause, original.program.meta_sorts())?;
    let mut subst = Subst::default();
    for (meta, sort) in declarations {
        subst.declare_meta_sort(meta, sort)?;
    }
    let mut premises = proof.premises().iter().map(|premise| {
        let key = core::ptr::from_ref(&*premise.step);
        *checked
            .get(&key)
            .expect("every positive premise was independently checked")
    });
    for (position, literal) in body.iter().enumerate() {
        if let FolLit::Pos(atom) = literal {
            let derived = premises.next().expect("the positive premise count matched");
            if !matches!(
                unify::unify_sorted(dag, *atom, derived, &mut subst, &original.sorts)?,
                Unified::Ok
            ) {
                return Err(FolProofError::PremiseMismatch { rule, position });
            }
        }
    }
    if !matches!(
        unify::unify_sorted(dag, head, goal, &mut subst, &original.sorts)?,
        Unified::Ok
    ) {
        let derived = unify::apply(dag, &subst, head)?;
        return Err(FolProofError::HeadMismatch {
            rule,
            derived: RetainedTerm::capture(dag, derived)?,
            stated: RetainedTerm::capture(dag, goal)?,
        });
    }
    for (position, literal) in body.iter().enumerate() {
        if let FolLit::Neg(atom) = literal {
            let ground_atom = unify::apply(dag, &subst, *atom)?;
            if negative_scope.is_none() {
                *negative_scope = Some(evidence.validate_negative_completion(dag)?);
            }
            let scope = negative_scope
                .as_ref()
                .expect("completion was just verified");
            if !dag.free_meta(ground_atom).is_empty()
                || scope.verdict(&canon(dag, ground_atom)) != FolVerdict::Established(Truth::False)
            {
                return Err(FolProofError::NegatedPremiseNotExcluded { rule, position });
            }
        }
    }
    if proof.rule_identity() != super::clause_identity(dag, clause, original.program.meta_sorts()) {
        return Err(FolProofError::RuleIdentityMismatch { rule });
    }
    Ok(goal)
}

#[cfg(test)]
mod tests {
    use super::super::{FolClause, FolControl, resolve_fol};
    use super::*;
    use std::collections::BTreeMap;

    fn run(
        dag: &mut TermDag,
        program: &FolProgram,
        sorts: &SortContext,
        steps: u64,
    ) -> super::super::FolOutcome {
        let FolControl::Decided(outcome) = resolve_fol(
            dag,
            program,
            sorts,
            FolAnswerContract::GroundOnly,
            &FolBudget { max_steps: steps },
        )
        .unwrap() else {
            panic!("the ground fixture must decide");
        };
        *outcome
    }

    fn changed(
        evidence: &FolCheckEvidence,
        change: impl FnOnce(&mut CheckContext),
    ) -> FolCheckEvidence {
        let mut context = (*evidence.0).clone();
        change(&mut context);
        FolCheckEvidence(Shared::try_new(context).unwrap())
    }

    fn claim(original: &FolProof, evidence: &FolCheckEvidence) -> FolProof {
        FolProof::new(evidence, (*original.step).clone()).unwrap()
    }

    #[test]
    fn negative_completion_requires_bounded_replay_and_complete_demanded_coverage() {
        let mut dag = TermDag::default();
        let p = dag.intern_leaf("p").unwrap();
        let q = dag.intern_leaf("q").unwrap();
        let a = dag.intern_leaf("a").unwrap();
        let (_, x) = dag.fresh_meta().unwrap();
        let px = dag.intern_app(p, vec![x]).unwrap();
        let qx = dag.intern_app(q, vec![x]).unwrap();
        let pa = dag.intern_app(p, vec![a]).unwrap();
        let program = FolProgram::new(
            &dag,
            vec![FolClause::new(&dag, px, vec![FolLit::Neg(qx)]).unwrap()],
            pa,
            vec![],
            BTreeMap::new(),
        )
        .unwrap();
        let sorts = SortContext::default();
        let full = run(&mut dag, &program, &sorts, 100);
        let proof = full.answers[0].proof().clone();
        let (marker, _) = dag.fresh_meta().unwrap();
        let nodes = dag.len();
        assert_eq!(
            check_fol_proof(&mut dag, &proof, full.checking_evidence()).unwrap(),
            pa
        );
        assert_eq!(dag.len(), nodes);
        let (after, _) = dag.fresh_meta().unwrap();
        assert_eq!(
            after.index(),
            marker.index() + 1,
            "checking must roll back fresh metavariable identities"
        );

        let partial = run(&mut dag, &program, &sorts, 1);
        let incomplete = claim(&proof, partial.checking_evidence());
        let nodes = dag.len();
        assert!(matches!(
            check_fol_proof(&mut dag, &incomplete, partial.checking_evidence()),
            Err(FolProofError::CompletionIncomplete { max_steps: 1, .. })
        ));
        assert_eq!(dag.len(), nodes);

        // A completion flag or false model entry cannot replace demand coverage.
        let missing = changed(full.checking_evidence(), |context| {
            context.calls.clear();
        });
        let forged = claim(&proof, &missing);
        assert_eq!(
            check_fol_proof(&mut dag, &forged, &missing).unwrap_err(),
            FolProofError::InvalidCompletion
        );
        // Coverage alone cannot replace replay of the full rule inventory/model.
        let omitted = changed(full.checking_evidence(), |context| {
            context.rules.clear();
        });
        let forged = claim(&proof, &omitted);
        assert_eq!(
            check_fol_proof(&mut dag, &forged, &omitted).unwrap_err(),
            FolProofError::InvalidCompletion
        );
        let model = changed(full.checking_evidence(), |context| {
            context.scope.true_set.clear();
        });
        let forged = claim(&proof, &model);
        assert_eq!(
            check_fol_proof(&mut dag, &forged, &model).unwrap_err(),
            FolProofError::InvalidCompletion
        );
        let bounded = changed(full.checking_evidence(), |context| {
            context.budget.max_steps = 1;
        });
        let forged = claim(&proof, &bounded);
        assert!(matches!(
            check_fol_proof(&mut dag, &forged, &bounded),
            Err(FolProofError::CompletionIncomplete { max_steps: 1, .. })
        ));
        assert_eq!(
            check_fol_proof(&mut dag, &proof, full.checking_evidence()).unwrap(),
            pa
        );
        assert_eq!(dag.len(), nodes);
    }

    #[test]
    fn checked_conclusions_reject_foreign_contexts_and_forged_positive_premises() {
        let mut dag = TermDag::default();
        let p = dag.intern_leaf("p").unwrap();
        let q = dag.intern_leaf("q").unwrap();
        let a = dag.intern_leaf("a").unwrap();
        let (_, x) = dag.fresh_meta().unwrap();
        let px = dag.intern_app(p, vec![x]).unwrap();
        let qx = dag.intern_app(q, vec![x]).unwrap();
        let pa = dag.intern_app(p, vec![a]).unwrap();
        let qa = dag.intern_app(q, vec![a]).unwrap();
        let program = FolProgram::new(
            &dag,
            vec![
                FolClause::new(&dag, px, vec![FolLit::Pos(qx)]).unwrap(),
                FolClause::new(&dag, qa, vec![]).unwrap(),
            ],
            pa,
            vec![],
            BTreeMap::new(),
        )
        .unwrap();
        let sorts = SortContext::default();
        let original = run(&mut dag, &program, &sorts, 100);
        let proof = original.answers[0].proof().clone();
        let nodes = dag.len();
        assert_eq!(
            check_fol_proof(&mut dag, &proof, original.checking_evidence()).unwrap(),
            pa
        );
        assert_eq!(dag.len(), nodes);
        let mut foreign = dag.clone();
        assert_eq!(
            check_fol_proof(&mut foreign, &proof, original.checking_evidence()).unwrap_err(),
            FolProofError::ForeignSource
        );

        let forged = FolProof::new(
            original.checking_evidence(),
            ProofStep::ByRule {
                rule: proof.rule(),
                rule_identity: proof.rule_identity().into(),
                goal: pa,
                premises: vec![proof.clone()],
            },
        )
        .unwrap();
        assert_eq!(
            check_fol_proof(&mut dag, &forged, original.checking_evidence()).unwrap_err(),
            FolProofError::PremiseMismatch {
                rule: 0,
                position: 0
            }
        );
        let symbolic = FolProof::new(
            original.checking_evidence(),
            ProofStep::Assert {
                rule: 1,
                rule_identity: String::new(),
                goal: x,
            },
        )
        .unwrap();
        assert_eq!(
            check_fol_proof(&mut dag, &symbolic, original.checking_evidence()).unwrap_err(),
            FolProofError::NonGroundConclusion { goal: x }
        );
        let limited = changed(original.checking_evidence(), |context| {
            context.budget.max_steps = 1;
        });
        let limited_claim = claim(&proof, &limited);
        assert!(matches!(
            check_fol_proof(&mut dag, &limited_claim, &limited),
            Err(FolProofError::CheckingWorkExhausted {
                steps: 0,
                max_steps: 1
            })
        ));
        let extra = dag.intern_leaf("extra").unwrap();
        let unknown = FolProof::new(
            original.checking_evidence(),
            ProofStep::Assert {
                rule: 1,
                rule_identity: String::new(),
                goal: extra,
            },
        )
        .unwrap();
        assert_eq!(
            check_fol_proof(&mut dag, &unknown, original.checking_evidence()).unwrap_err(),
            FolProofError::UnknownConclusion { goal: extra }
        );
        let mut clauses = program.clauses().to_vec();
        clauses.push(FolClause::new(&dag, extra, vec![]).unwrap());
        let other_program = FolProgram::new(&dag, clauses, pa, vec![], BTreeMap::new()).unwrap();
        let other = run(&mut dag, &other_program, &sorts, 100);
        assert_eq!(
            check_fol_proof(&mut dag, &proof, other.checking_evidence()).unwrap_err(),
            FolProofError::ForeignProgram
        );
        let sort = dag.intern_leaf("sort").unwrap();
        let other_sorts = SortContext::new(
            unify::SortOrder::from_subclass_edges(&[(sort, sort)]),
            BTreeMap::new(),
            BTreeMap::new(),
        );
        let other = run(&mut dag, &program, &other_sorts, 100);
        assert_eq!(
            check_fol_proof(&mut dag, &proof, other.checking_evidence()).unwrap_err(),
            FolProofError::ForeignSorts
        );
        let other = run(&mut dag, &program, &sorts, 100);
        assert_eq!(
            check_fol_proof(&mut dag, &proof, other.checking_evidence()).unwrap_err(),
            FolProofError::ForeignRun
        );
        assert_eq!(
            check_fol_proof(
                &mut dag,
                other.answers[0].proof(),
                other.checking_evidence()
            )
            .unwrap(),
            pa
        );
        assert_eq!(
            check_fol_proof(&mut dag, &proof, original.checking_evidence()).unwrap(),
            pa
        );
    }
}
