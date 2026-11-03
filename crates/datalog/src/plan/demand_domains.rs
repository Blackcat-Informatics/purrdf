// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Structural proofs for the two predicate domains of a demand transformation.

use super::{BTreeMap, ClauseAtom, DlClause};
use crate::demand::projection::Plan;
use crate::seminaive::EvalError;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Family {
    All,
    Original,
    Shadow,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Key {
    pub(crate) family: Family,
    pub(crate) surface: Option<String>,
}

impl Key {
    pub(crate) fn label(&self) -> &str {
        self.surface
            .as_deref()
            .unwrap_or_else(|| match self.family {
                Family::All => super::ANY_PREDICATE,
                Family::Original => "*(original)",
                Family::Shadow => "*(shadow)",
                Family::Internal => unreachable!("internal wildcard is never certified"),
            })
    }
}

impl Ord for Key {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.family
            .cmp(&other.family)
            .then_with(|| self.label().cmp(other.label()))
            .then_with(|| self.surface.cmp(&other.surface))
    }
}
impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Fields are sealed; checking actual relational binders is the only entrance.
#[derive(Debug)]
pub(crate) struct Checked<const N: usize> {
    pub(crate) projection: Arc<Plan>,
    heads: Vec<Vec<Key>>,
    bodies: Vec<Vec<Key>>,
    negatives: Vec<Vec<Vec<Key>>>,
}

impl<const N: usize> Checked<N> {
    pub(crate) fn new(rules: &[DlClause<N>], projection: Arc<Plan>) -> Result<Self, EvalError> {
        let mut result = Self {
            projection,
            heads: Vec::new(),
            bodies: Vec::new(),
            negatives: Vec::new(),
        };
        if !result.projection.variables_are_fresh() {
            return Err(Self::refuse(
                0,
                "generated variables overlap the authored variable inventory",
            ));
        }
        if !result.projection.domains_are_disjoint() {
            return Err(Self::refuse(
                0,
                "original, shadow and internal predicate names overlap",
            ));
        }
        for (rule_index, rule) in rules.iter().enumerate() {
            for atom in rule.body().iter().chain(
                rule.negations()
                    .iter()
                    .flat_map(crate::guard::Negation::atoms),
            ) {
                if atom.predicate_iri() == result.projection.map()
                    && result.projection.map().is_some()
                {
                    let terms = atom.terms();
                    let original = Plan::original_column::<N>();
                    if atom.is_negated()
                        || (N == 4 && terms[0].iri_value() != Some(result.projection.unit()))
                        || !terms[2]
                            .variable()
                            .is_some_and(|name| result.projection.generated_variable(name))
                        || terms[original]
                            .variable()
                            .is_some_and(|name| result.projection.generated_variable(name))
                        || terms[original] == terms[2]
                    {
                        return Err(Self::refuse(
                            rule_index,
                            "mapping binder has invalid layout or fresh-variable provenance",
                        ));
                    }
                }
            }
            for variable in rule
                .head_atoms()
                .chain(rule.body())
                .chain(
                    rule.negations()
                        .iter()
                        .flat_map(crate::guard::Negation::atoms),
                )
                .flat_map(ClauseAtom::terms)
                .filter_map(|term| term.variable())
            {
                if !result.projection.knows_variable(variable) {
                    return Err(Self::refuse(
                        rule_index,
                        "transformed variable has no authored or fresh provenance",
                    ));
                }
            }
            let positive: Vec<_> = rule
                .body()
                .iter()
                .filter(|atom| !atom.is_negated())
                .collect();
            let mut heads = Vec::new();
            for atom in rule.head_atoms() {
                if atom
                    .predicate_iri()
                    .is_some_and(|iri| Some(iri) == result.projection.map())
                {
                    return Err(Self::refuse(
                        rule_index,
                        "a rule may not derive the private mapping relation",
                    ));
                }
                heads.push(result.key(atom, &positive, rule_index)?);
            }
            let mut bodies = Vec::new();
            for atom in rule.body() {
                if atom.is_negated() && atom.predicate().is_var() {
                    return Err(Self::refuse(
                        rule_index,
                        "a negative variable predicate requires its own mapped conjunction",
                    ));
                }
                bodies.push(result.key(atom, &positive, rule_index)?);
            }
            let mut negatives = Vec::new();
            for group in rule.negations() {
                let scope: Vec<_> = group.atoms().iter().collect();
                let mut keys = Vec::new();
                for atom in group.atoms() {
                    keys.push(result.key(atom, &scope, rule_index)?);
                }
                negatives.push(keys);
            }
            result.heads.push(heads);
            result.bodies.push(bodies);
            result.negatives.push(negatives);
        }
        Ok(result)
    }

    fn refuse(rule: usize, reason: &str) -> EvalError {
        EvalError::DemandScope {
            rule,
            reason: reason.to_owned(),
        }
    }

    fn key(
        &self,
        atom: &ClauseAtom<N>,
        scope: &[&ClauseAtom<N>],
        rule: usize,
    ) -> Result<Key, EvalError> {
        if let Some(surface) = atom.predicate().surface() {
            let family = if atom
                .predicate_iri()
                .is_some_and(|iri| self.projection.is_shadow(iri))
            {
                Family::Shadow
            } else if atom
                .predicate_iri()
                .is_some_and(|iri| self.projection.is_internal(iri))
            {
                Family::Internal
            } else {
                Family::Original
            };
            return Ok(Key {
                family,
                surface: Some(surface),
            });
        }
        let mut families = BTreeMap::new();
        for binder in scope {
            if binder.is_negated()
                || binder.predicate_iri() != self.projection.map()
                || self.projection.map().is_none()
            {
                continue;
            }
            let terms = binder.terms();
            if N == 4 && terms[0].iri_value() != Some(self.projection.unit()) {
                return Err(Self::refuse(
                    rule,
                    "quad mapping binder has an invalid unit coordinate",
                ));
            }
            let original = Plan::original_column::<N>();
            if terms[original] == terms[2] {
                return Err(Self::refuse(
                    rule,
                    "mapping source and target variables must be distinct",
                ));
            }
            if atom.predicate() == terms[original] {
                families.insert(Family::Original, ());
            }
            if atom.predicate() == terms[2] {
                families.insert(Family::Shadow, ());
            }
        }
        if families.len() != 1 {
            return Err(Self::refuse(
                rule,
                "variable predicate lacks one same-conjunction mapping provenance",
            ));
        }
        Ok(Key {
            family: *families.first_key_value().expect("one checked family").0,
            surface: None,
        })
    }

    pub(crate) fn head(&self, rule: usize, head: usize) -> &Key {
        &self.heads[rule][head]
    }
    pub(crate) fn body(&self, rule: usize, body: usize) -> &Key {
        &self.bodies[rule][body]
    }
    pub(crate) fn negative(&self, rule: usize, group: usize, member: usize) -> &Key {
        &self.negatives[rule][group][member]
    }
}
