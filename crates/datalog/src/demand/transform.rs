// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One adornment closure and relationally isolated transformation for every demand program.

use super::{
    Arity, BTreeMap, BTreeSet, BindingPattern, ClauseAtom, ClauseTerm, DemandError, DlClause,
    PreparedDemand, StoreShape, TermDictionary,
};
use crate::guard::Negation;
use purrdf_hash::{Domain, frame::frame_le_into};
use std::sync::Arc;

const NAMESPACE: Domain = Domain::new(b"purrdf-datalog/demand-predicates/v1");

/// One digest, one common collision nonce and unique sorted ordinals avoid
/// both existing dictionary terms and collisions between internal descriptors.
fn names<D: TermDictionary, const N: usize>(
    descriptors: &[Descriptor],
    rules: &[DlClause<N>],
    goal: &ClauseAtom<N>,
    dictionary: &D,
    additional: usize,
) -> Result<Vec<String>, DemandError<D::Error>> {
    let authored: BTreeSet<_> = rules
        .iter()
        .flat_map(|rule| {
            rule.body()
                .iter()
                .chain(rule.head_atoms())
                .chain(rule.negations().iter().flat_map(Negation::atoms))
        })
        .chain(std::iter::once(goal))
        .flat_map(ClauseAtom::terms)
        .filter_map(ClauseTerm::surface)
        .collect();
    let mut digest = purrdf_hash::blake3::Hasher::new();
    digest.update(NAMESPACE.as_bytes());
    frame_le_into(&mut digest, &crate::cache::canonical_rule_hash(rules));
    let goal_clause = DlClause::datalog(goal.clone(), Vec::new());
    frame_le_into(
        &mut digest,
        &crate::cache::canonical_rule_hash(&[goal_clause]),
    );
    for descriptor in descriptors {
        frame_le_into(&mut digest, descriptor.predicate.label().as_bytes());
        frame_le_into(&mut digest, descriptor.pattern.code().as_bytes());
    }
    let digest = purrdf_hash::hex::encode(digest.finalize().as_bytes());
    dictionary.check_read().map_err(DemandError::Source)?;
    let mut nonce = 0_u64;
    loop {
        let candidates: Vec<_> = (0..=descriptors.len() + additional)
            .map(|ordinal| format!("urn:purrdf:demand:{digest}:{nonce}:{ordinal}"))
            .collect();
        let mut collision = false;
        for name in &candidates {
            let surface = format!("<{name}>");
            // Check the entire original dictionary, even terms absent from rows.
            let existing = dictionary
                .lookup_term(&surface)
                .map_err(DemandError::Source)?;
            collision |= existing.is_some() || authored.contains(&surface);
        }
        dictionary.check_read().map_err(DemandError::Source)?;
        if !collision {
            return Ok(candidates);
        }
        nonce = nonce
            .checked_add(1)
            .ok_or(DemandError::NamespaceExhausted)?;
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Predicate {
    Fixed(String),
    Any,
}

impl Predicate {
    fn of<const N: usize>(atom: &ClauseAtom<N>) -> Self {
        atom.predicate_iri()
            .map_or(Self::Any, |iri| Self::Fixed(iri.to_owned()))
    }
    fn label(&self) -> &str {
        match self {
            Self::Fixed(name) => name,
            Self::Any => "*",
        }
    }
    fn accepts<const N: usize>(&self, atom: &ClauseAtom<N>) -> bool {
        match (self, atom.predicate_iri()) {
            (Self::Fixed(a), Some(b)) => a == b,
            _ => true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Descriptor {
    predicate: Predicate,
    pattern: BindingPattern,
}

fn positions<const N: usize>(predicate: &Predicate) -> impl Iterator<Item = usize> {
    let any = matches!(predicate, Predicate::Any);
    (0..N).filter(move |at| any || *at != 1)
}

fn pattern<const N: usize>(atom: &ClauseAtom<N>, bound: &BTreeSet<String>) -> BindingPattern {
    let predicate = Predicate::of(atom);
    let mut flags: Vec<_> = positions::<N>(&predicate)
        .map(|at| {
            atom.terms()[at]
                .variable()
                .is_none_or(|name| bound.contains(name))
        })
        .collect();
    if matches!(predicate, Predicate::Any) && flags.iter().all(|flag| *flag) {
        // A marker occupies the predicate cell. Dropping only this demand-key
        // binding is conservative; every original atom/goal keeps its residual.
        flags[2] = false;
    }
    BindingPattern::from_bools(flags)
}

fn lifted<const N: usize>(descriptor: &Descriptor) -> BindingPattern {
    match descriptor.predicate {
        Predicate::Any => descriptor.pattern,
        Predicate::Fixed(_) => BindingPattern::from_bools((0..N).map(|at| {
            at == 1
                || descriptor
                    .pattern
                    .is_bound(if at == 0 { 0 } else { at - 1 })
        })),
    }
}

fn covers<const N: usize>(a: &Descriptor, b: &Descriptor) -> bool {
    match (&a.predicate, &b.predicate) {
        (Predicate::Any, _) => a.pattern.subsumes(lifted::<N>(b)),
        (Predicate::Fixed(x), Predicate::Fixed(y)) => x == y && a.pattern.subsumes(b.pattern),
        _ => false,
    }
}

fn bind<const N: usize>(atom: &ClauseAtom<N>, bound: &mut BTreeSet<String>) {
    bound.extend(
        atom.terms()
            .into_iter()
            .filter_map(ClauseTerm::variable)
            .map(str::to_owned),
    );
}

fn head_bindings<const N: usize>(
    head: &ClauseAtom<N>,
    descriptor: &Descriptor,
) -> BTreeSet<String> {
    positions::<N>(&descriptor.predicate)
        .enumerate()
        .filter(|(payload, _)| descriptor.pattern.is_bound(*payload))
        .filter_map(|(_, at)| head.terms()[at].variable().map(str::to_owned))
        .collect()
}

fn replace_atom<const N: usize>(
    atom: &ClauseAtom<N>,
    variable: &str,
    value: &ClauseTerm,
) -> ClauseAtom<N> {
    let result = ClauseAtom::from_positions(atom.terms().map(|term| {
        if term.variable() == Some(variable) {
            value.clone()
        } else {
            term.clone()
        }
    }));
    if atom.is_negated() {
        result.into_negated()
    } else {
        result
    }
}

fn specialize<const N: usize>(rule: &DlClause<N>, predicate: &Predicate) -> DlClause<N> {
    let head = rule.datalog_head().expect("validated atomic head");
    let (Predicate::Fixed(iri), Some(variable)) = (predicate, head.predicate().variable()) else {
        return rule.clone();
    };
    let value = ClauseTerm::iri(iri);
    DlClause::datalog(
        replace_atom(head, variable, &value),
        rule.body()
            .iter()
            .map(|atom| replace_atom(atom, variable, &value))
            .collect(),
    )
    .with_negations(
        rule.negations()
            .iter()
            .map(|group| {
                Negation::new(
                    group
                        .atoms()
                        .iter()
                        .map(|atom| replace_atom(atom, variable, &value))
                        .collect(),
                    Vec::new(),
                )
            })
            .collect(),
    )
}

fn discovery<const N: usize>(rule: &DlClause<N>) -> Vec<Vec<ClauseAtom<N>>> {
    let positive: Vec<_> = rule
        .body()
        .iter()
        .filter(|atom| !atom.is_negated())
        .cloned()
        .collect();
    let mut result = vec![positive.clone()];
    for atom in rule.body().iter().filter(|atom| atom.is_negated()) {
        let mut body = positive.clone();
        body.push(ClauseAtom::from_positions(atom.terms().map(Clone::clone)));
        result.push(body);
    }
    for group in rule.negations() {
        let mut body = positive.clone();
        body.extend_from_slice(group.atoms());
        result.push(body);
    }
    result
}

fn descriptors<const N: usize>(rules: &[DlClause<N>], goal: &ClauseAtom<N>) -> Vec<Descriptor> {
    let mut found = BTreeSet::from([Descriptor {
        predicate: Predicate::of(goal),
        pattern: pattern(goal, &BTreeSet::new()),
    }]);
    loop {
        let mut next = found.clone();
        for demand in &found {
            for rule in rules.iter().filter(|rule| {
                demand
                    .predicate
                    .accepts(rule.datalog_head().expect("validated head"))
            }) {
                let rule = specialize(rule, &demand.predicate);
                for body in discovery(&rule) {
                    let mut bound =
                        head_bindings(rule.datalog_head().expect("validated head"), demand);
                    for atom in body {
                        if rules.iter().any(|definition| {
                            Predicate::of(&atom)
                                .accepts(definition.datalog_head().expect("validated head"))
                        }) {
                            next.insert(Descriptor {
                                predicate: Predicate::of(&atom),
                                pattern: pattern(&atom, &bound),
                            });
                        }
                        bind(&atom, &mut bound);
                    }
                }
            }
        }
        if next == found {
            break;
        }
        found = next;
    }
    found
        .iter()
        .filter(|entry| {
            !found
                .iter()
                .any(|other| *entry != other && covers::<N>(other, entry))
        })
        .cloned()
        .collect()
}

fn select<'a, const N: usize>(
    descriptors: &'a [Descriptor],
    atom: &ClauseAtom<N>,
    bound: &BTreeSet<String>,
) -> Option<(usize, &'a Descriptor)> {
    let request = Descriptor {
        predicate: Predicate::of(atom),
        pattern: pattern(atom, bound),
    };
    descriptors
        .iter()
        .enumerate()
        .find(|(_, descriptor)| covers::<N>(descriptor, &request))
}

fn demand_atom<const N: usize>(
    atom: &ClauseAtom<N>,
    descriptor: &Descriptor,
    name: &str,
    unit: &str,
) -> ClauseAtom<N> {
    let mut row = std::array::from_fn(|_| ClauseTerm::iri(unit));
    row[1] = ClauseTerm::iri(name);
    if matches!(descriptor.predicate, Predicate::Fixed(_)) {
        for (payload, at) in positions::<N>(&descriptor.predicate).enumerate() {
            if descriptor.pattern.is_bound(payload) {
                row[at] = atom.terms()[at].clone();
            }
        }
    } else {
        let graph = N == 4 && descriptor.pattern.is_bound(3);
        if graph {
            row[3] = atom.terms()[3].clone();
        }
        let mut slots = (0..N).filter(|at| *at != 1 && !(graph && *at == 3));
        for at in 0..N {
            if descriptor.pattern.is_bound(at) && !(graph && at == 3) {
                row[slots.next().expect("generalized demand fits native cells")] =
                    atom.terms()[at].clone();
            }
        }
    }
    ClauseAtom::from_positions(row)
}

struct Fresh {
    authored: BTreeSet<String>,
    generated: BTreeSet<String>,
    next: usize,
}
impl Fresh {
    fn variable(&mut self) -> ClauseTerm {
        loop {
            let name = format!("?__demand{}", self.next);
            self.next += 1;
            if !self.authored.contains(&name) && self.generated.insert(name.clone()) {
                return ClauseTerm::var(name);
            }
        }
    }
}

fn map_atom<const N: usize>(
    original: ClauseTerm,
    target: ClauseTerm,
    map: &str,
    unit: &str,
) -> ClauseAtom<N> {
    let mut row = std::array::from_fn(|_| ClauseTerm::iri(unit));
    row[1] = ClauseTerm::iri(map);
    row[2] = target;
    row[super::projection::Plan::original_column::<N>()] = original;
    ClauseAtom::from_positions(row)
}

fn original<const N: usize>(
    atom: &ClauseAtom<N>,
    map: Option<&str>,
    unit: &str,
    fresh: &mut Fresh,
) -> Vec<ClauseAtom<N>> {
    let mut result = Vec::new();
    if atom.predicate().is_var() {
        result.push(map_atom(
            atom.predicate().clone(),
            fresh.variable(),
            map.expect("variable family has a map"),
            unit,
        ));
    }
    result.push(ClauseAtom::from_positions(atom.terms().map(Clone::clone)));
    result
}

fn shadow<const N: usize>(
    atom: &ClauseAtom<N>,
    shadows: &BTreeMap<String, String>,
    map: Option<&str>,
    unit: &str,
    fresh: &mut Fresh,
) -> (Vec<ClauseAtom<N>>, ClauseAtom<N>) {
    let mut row = atom.terms().map(Clone::clone);
    let mut binders = Vec::new();
    if let Some(iri) = atom.predicate_iri() {
        row[1] = ClauseTerm::iri(&shadows[iri]);
    } else {
        let target = fresh.variable();
        binders.push(map_atom(
            atom.predicate().clone(),
            target.clone(),
            map.expect("variable family has a map"),
            unit,
        ));
        row[1] = target;
    }
    (binders, ClauseAtom::from_positions(row))
}

pub(super) fn prepare<D: TermDictionary, const N: usize>(
    rules: &[DlClause<N>],
    goal: &ClauseAtom<N>,
    dictionary: &D,
) -> Result<PreparedDemand<N>, DemandError<D::Error>>
where
    Arity<N>: StoreShape<N>,
{
    crate::seminaive::compile(rules.to_vec()).map_err(DemandError::Compile)?;
    let descriptors = descriptors(rules, goal);
    let selected: Vec<_> = descriptors
        .iter()
        .enumerate()
        .flat_map(|(ordinal, descriptor)| {
            rules
                .iter()
                .filter(move |rule| {
                    descriptor
                        .predicate
                        .accepts(rule.datalog_head().expect("validated head"))
                })
                .map(move |rule| (ordinal, specialize(rule, &descriptor.predicate)))
        })
        .collect();
    let relevant = || {
        selected
            .iter()
            .flat_map(|(_, rule)| {
                rule.body()
                    .iter()
                    .chain(rule.head_atoms())
                    .chain(rule.negations().iter().flat_map(Negation::atoms))
            })
            .chain(std::iter::once(goal))
    };
    let negative = selected.iter().any(|(_, rule)| {
        !rule.negations().is_empty() || rule.body().iter().any(ClauseAtom::is_negated)
    });
    let variable = relevant().any(|atom| atom.predicate().is_var());
    let predicates: BTreeSet<_> = relevant()
        .filter_map(ClauseAtom::predicate_iri)
        .map(str::to_owned)
        .collect();
    let additional = if negative { predicates.len() } else { 0 } + usize::from(variable);
    let mut names = names(&descriptors, rules, goal, dictionary, additional)?;
    let shadows: BTreeMap<_, _> = if negative {
        predicates
            .into_iter()
            .zip(
                names[descriptors.len()..descriptors.len() + additional - usize::from(variable)]
                    .iter()
                    .cloned(),
            )
            .collect()
    } else {
        BTreeMap::new()
    };
    let map = variable.then(|| names[names.len() - 2].clone());
    let unit = names.last().expect("unit ordinal").clone();
    let atoms = || {
        rules
            .iter()
            .flat_map(|rule| {
                rule.body()
                    .iter()
                    .chain(rule.head_atoms())
                    .chain(rule.negations().iter().flat_map(Negation::atoms))
            })
            .chain(std::iter::once(goal))
    };
    let authored_variables = atoms()
        .flat_map(ClauseAtom::terms)
        .filter_map(ClauseTerm::variable)
        .map(str::to_owned)
        .collect();
    let mut constants: BTreeSet<_> = goal
        .terms()
        .into_iter()
        .filter(|term| !term.is_var())
        .cloned()
        .collect();
    let mut fresh = Fresh {
        authored: authored_variables,
        generated: BTreeSet::new(),
        next: 0,
    };
    let (seed, descriptor) =
        select(&descriptors, goal, &BTreeSet::new()).expect("goal antichain retained");
    let mut transformed = vec![DlClause::datalog(
        demand_atom(goal, descriptor, &names[seed], &unit),
        Vec::new(),
    )];
    for (ordinal, rule) in &selected {
        let descriptor = &descriptors[*ordinal];
        constants.extend(
            rule.head_atoms()
                .chain(rule.body())
                .chain(rule.negations().iter().flat_map(Negation::atoms))
                .flat_map(ClauseAtom::terms)
                .filter(|term| !term.is_var())
                .cloned(),
        );
        let head = rule.datalog_head().expect("validated head");
        let magic = demand_atom(head, descriptor, &names[*ordinal], &unit);
        let mut exact = vec![magic.clone()];
        let mut negatives = Vec::new();
        for atom in rule.body() {
            let body = original(atom, map.as_deref(), &unit, &mut fresh);
            if atom.is_negated() {
                negatives.push(Negation::new(body, Vec::new()));
            } else {
                exact.extend(body);
            }
        }
        for group in rule.negations() {
            let body = group
                .atoms()
                .iter()
                .flat_map(|atom| original(atom, map.as_deref(), &unit, &mut fresh))
                .collect();
            negatives.push(Negation::new(body, Vec::new()));
        }
        // A variable head must carry its own proven Original map provenance.
        if head.predicate().is_var() {
            exact.push(map_atom(
                head.predicate().clone(),
                fresh.variable(),
                map.as_deref().expect("map"),
                &unit,
            ));
        }
        transformed.push(DlClause::datalog(head.clone(), exact).with_negations(negatives));
        let mut prefix = vec![magic];
        let mut bound = head_bindings(head, descriptor);
        let mut propagate =
            |atom: &ClauseAtom<N>, prefix: &[ClauseAtom<N>], bound: &BTreeSet<String>| {
                if let Some((at, needed)) = select(&descriptors, atom, bound) {
                    transformed.push(DlClause::datalog(
                        demand_atom(atom, needed, &names[at], &unit),
                        prefix.to_vec(),
                    ));
                }
            };
        for atom in rule.body().iter().filter(|atom| !atom.is_negated()) {
            propagate(atom, &prefix, &bound);
            if negative {
                let (binders, row) = shadow(atom, &shadows, map.as_deref(), &unit, &mut fresh);
                prefix.extend(binders);
                prefix.push(row);
            } else {
                prefix.extend(original(atom, map.as_deref(), &unit, &mut fresh));
            }
            bind(atom, &mut bound);
        }
        for atom in rule.body().iter().filter(|atom| atom.is_negated()) {
            propagate(atom, &prefix, &bound);
        }
        for group in rule.negations() {
            let mut local = prefix.clone();
            let mut local_bound = bound.clone();
            for atom in group.atoms() {
                propagate(atom, &local, &local_bound);
                let (binders, row) = shadow(atom, &shadows, map.as_deref(), &unit, &mut fresh);
                local.extend(binders);
                local.push(row);
                bind(atom, &mut local_bound);
            }
        }
        if negative {
            let (binders, row) = shadow(head, &shadows, map.as_deref(), &unit, &mut fresh);
            prefix.extend(binders);
            transformed.push(DlClause::datalog(row, prefix));
        }
    }
    let shadow_names: BTreeSet<_> = shadows.values().cloned().collect();
    let internal = names
        .iter()
        .filter(|name| !shadow_names.contains(*name))
        .cloned()
        .collect();
    let projection = Arc::new(super::projection::Plan::new(
        map,
        shadows,
        negative,
        internal,
        constants.into_iter().collect(),
        unit,
        fresh.authored,
        fresh.generated,
    ));
    let executable =
        crate::seminaive::compile_demand(transformed, projection).map_err(DemandError::Compile)?;
    names.sort_unstable();
    Ok(PreparedDemand {
        executable,
        goal: goal.clone(),
        names,
    })
}
