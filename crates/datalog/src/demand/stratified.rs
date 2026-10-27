// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Demand discovery reads only a positive shadow of the immutable original input.
//! Exact rules retain their negative dependencies; no exact derived fact flows
//! back into the shadow, so the authored stratification remains sound.

use super::{
    Arity, BTreeMap, BTreeSet, ClauseAtom, ClauseTerm, DemandAnswer, DemandError, Descriptor,
    DlClause, EvalError, EvalOptions, NoGuards, Preparation, RelationStore, StopSignal, StoreShape,
    TermDictionary, bind, compile, demand_atom, descriptors, evaluate_guarded, head_bindings,
    names, pattern, select,
};
use crate::dictionary::{DictionaryError, DictionaryTerm};
use purrdf_lex::allocation::{Memory, Resident};

pub(super) fn evaluate<D: TermDictionary, const N: usize>(
    rules: &[DlClause<N>],
    goal: &ClauseAtom<N>,
    mut edb: RelationStore<D, N>,
    options: &EvalOptions,
    stop: Option<&dyn StopSignal>,
) -> Result<DemandAnswer<D, N>, DemandError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    // Validate the entire original program, including unreachable negative cycles,
    // before its dictionary can change. The native compiler owns this decision.
    compile(rules.to_vec()).map_err(DemandError::Compile)?;
    let discovery = discovery(rules);
    let descriptors = descriptors(&discovery, goal);
    let predicates: BTreeSet<_> = rules
        .iter()
        .flat_map(|rule| {
            rule.body().iter().chain(rule.head_atoms()).chain(
                rule.negations()
                    .iter()
                    .flat_map(crate::guard::Negation::atoms),
            )
        })
        .chain(std::iter::once(goal))
        .map(|atom| {
            atom.predicate_iri()
                .expect("validation admitted every predicate as an IRI")
        })
        .collect();
    let names = names(&descriptors, rules, goal, edb.interner(), predicates.len())?;
    let shadows: BTreeMap<_, _> = predicates
        .into_iter()
        .zip(names[descriptors.len()..].iter().map(String::as_str))
        .collect();
    let transformed = transform_exact(rules, goal, &descriptors, &names, &shadows);
    // Also compile the transformed program before seeding; neither source mutation
    // nor an executable is a substitute for native stratification.
    let executable = compile(transformed).map_err(DemandError::Compile)?;
    let preparation = seed(&mut edb, &shadows)?;
    let evaluation = evaluate_guarded(&executable, edb, &NoGuards, options, stop)
        .map_err(DemandError::Evaluation)?;
    Ok(DemandAnswer {
        evaluation,
        goal: goal.clone(),
        preparation,
    })
}

/// Descriptor discovery uses outer positive binders first. Every negative group
/// has its own local prefix; its existential variables never bind another group.
fn discovery<const N: usize>(rules: &[DlClause<N>]) -> Vec<DlClause<N>> {
    let mut result = Vec::new();
    for rule in rules {
        let head = rule
            .datalog_head()
            .expect("validation admitted an atomic head");
        let positive: Vec<_> = rule
            .body()
            .iter()
            .filter(|atom| !atom.is_negated())
            .cloned()
            .collect();
        result.push(DlClause::datalog(head.clone(), positive.clone()));
        for atom in rule.body().iter().filter(|atom| atom.is_negated()) {
            let mut body = positive.clone();
            body.push(ClauseAtom::from_positions(atom.terms().map(Clone::clone)));
            result.push(DlClause::datalog(head.clone(), body));
        }
        for group in rule.negations() {
            let mut body = positive.clone();
            body.extend_from_slice(group.atoms());
            result.push(DlClause::datalog(head.clone(), body));
        }
    }
    result
}

fn shadow<const N: usize>(atom: &ClauseAtom<N>, shadows: &BTreeMap<&str, &str>) -> ClauseAtom<N> {
    let mut positions = atom.terms().map(Clone::clone);
    // Every authored atom was inventoried after validation, including negative groups.
    positions[1] = ClauseTerm::iri(shadows[atom.predicate_iri().expect("validated IRI predicate")]);
    ClauseAtom::from_positions(positions)
}

fn transform_exact<const N: usize>(
    rules: &[DlClause<N>],
    goal: &ClauseAtom<N>,
    descriptors: &[Descriptor],
    names: &[String],
    shadows: &BTreeMap<&str, &str>,
) -> Vec<DlClause<N>> {
    // Namespace allocation always reserves the final ordinal for the unit.
    let unit = names.last().expect("allocated unit ordinal");
    let (at, descriptor) = select(
        descriptors,
        goal.predicate_iri().expect("validated goal"),
        pattern(goal, &BTreeSet::new()),
    )
    .expect("descriptor closure retains the goal or its subsumer");
    let mut result = vec![DlClause::datalog(
        demand_atom(goal, descriptor, &names[at], unit),
        Vec::new(),
    )];
    for (ordinal, descriptor) in descriptors.iter().enumerate() {
        for rule in rules.iter().filter(|rule| {
            rule.datalog_head()
                .is_some_and(|head| head.predicate_iri() == Some(descriptor.predicate.as_str()))
        }) {
            let head = rule
                .datalog_head()
                .expect("validated atomic rule selected by its head");
            let magic = demand_atom(head, descriptor, &names[ordinal], unit);
            let mut exact = vec![magic.clone()];
            exact.extend_from_slice(rule.body());
            result.push(
                DlClause::datalog(head.clone(), exact).with_negations(rule.negations().to_vec()),
            );
            let mut prefix = vec![magic];
            let mut bound = head_bindings(head, descriptor.pattern);
            let mut propagate =
                |atom: &ClauseAtom<N>, prefix: &[ClauseAtom<N>], bound: &BTreeSet<String>| {
                    if let Some((index, needed)) = select(
                        descriptors,
                        atom.predicate_iri().expect("validated body predicate"),
                        pattern(atom, bound),
                    ) {
                        result.push(DlClause::datalog(
                            demand_atom(atom, needed, &names[index], unit),
                            prefix.to_vec(),
                        ));
                    }
                };
            for atom in rule.body().iter().filter(|atom| !atom.is_negated()) {
                propagate(atom, &prefix, &bound);
                prefix.push(shadow(atom, shadows));
                bind(atom, &mut bound);
            }
            for atom in rule.body().iter().filter(|atom| atom.is_negated()) {
                propagate(atom, &prefix, &bound);
            }
            for group in rule.negations() {
                let mut local_prefix = prefix.clone();
                let mut local_bound = bound.clone();
                for atom in group.atoms() {
                    propagate(atom, &local_prefix, &local_bound);
                    local_prefix.push(shadow(atom, shadows));
                    bind(atom, &mut local_bound);
                }
            }
            result.push(DlClause::datalog(shadow(head, shadows), prefix));
        }
    }
    result
}

fn seed<D: TermDictionary, const N: usize>(
    edb: &mut RelationStore<D, N>,
    shadows: &BTreeMap<&str, &str>,
) -> Result<Preparation, DemandError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    let mut report = Preparation {
        original_facts: edb.row_count(),
        original_terms: edb.interner().term_count(),
        shadow_facts: 0,
        shadow_terms: 0,
    };
    let mut predicates = BTreeMap::new();
    for (&predicate, &name) in shadows {
        if let Some(id) = edb
            .interner()
            .lookup_term(&format!("<{predicate}>"))
            .map_err(DemandError::Source)?
        {
            predicates.insert(id, format!("<{name}>"));
        }
    }
    edb.interner().check_read().map_err(DemandError::Source)?;
    // Finish the immutable original-row walk before inserting any shadow row.
    // Only predicate identity changes: no source term is rendered or re-interned.
    let rows = Memory::new(&mut Resident)
        .collect(
            edb.rows_from(0)
                .filter_map(|(_, row)| predicates.get(&row[1]).map(|name| (row, name))),
        )
        .map_err(|error| DemandError::Evaluation(EvalError::Storage(error)))?;
    for (row, name) in rows {
        let mut terms = row.map(DictionaryTerm::Id);
        terms[1] = DictionaryTerm::Surface(name);
        // Public owned stores are resident: supplemental stores remain borrowed
        // by Maintained, and their explicit Clone constructs resident metadata.
        edb.insert_terms(terms).map_err(|error| match error {
            DictionaryError::Source(cause) => DemandError::Source(cause),
            DictionaryError::Storage(cause) => DemandError::Evaluation(EvalError::Storage(cause)),
        })?;
    }
    report.shadow_facts = edb.row_count() - report.original_facts;
    report.shadow_terms = edb.interner().term_count() - report.original_terms;
    Ok(report)
}
