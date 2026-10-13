// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Assessor-only predicates: a monotone, sourced rule-admission contract.
//! No vocabulary or process-wide policy is supplied by this crate.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::clause::{ClauseTerm, DlClause};
use crate::plan::{ANY_PREDICATE, dependency_edges_with, predicate_symbol};

// Both executable program carriers expose the same consuming admission method.
// Keep its certificate transition in one source body without requiring callers
// to import a trait or exposing the private certificate fields.
macro_rules! program_declarations {
    () => {
        /// Re-admit a reused program without withdrawing its earlier declarations.
        /// # Errors
        /// Returns the full-IR refusal before executing under the extended policy.
        pub fn admit_declarations(
            mut self,
            additions: &$crate::admission::NeverDeriveDeclarations,
        ) -> Result<Self, $crate::admission::AdmissionRefusal> {
            let certificate = match &self.admission {
                Some(existing) => existing.extend(&self.rules, additions)?,
                None => additions.admit(&self.rules)?,
            };
            self.admission = Some(std::sync::Arc::new(certificate));
            Ok(self)
        }
    };
}
pub(crate) use program_declarations;

/// The authority layer contributing a declaration. Layers cannot withdraw one
/// another's declarations; identical predicate/source pairs coalesce.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeclarationLayer {
    /// Ontology-authored obligations.
    Ontology,
    /// Store or application profile obligations.
    StoreProfile,
    /// Obligations authored with the rule bundle.
    RuleSet,
}

/// A caller's stable identifier for the source of a declaration.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeclarationSource {
    /// The authority that contributed the obligation.
    pub layer: DeclarationLayer,
    /// Caller-owned source identity, such as a document or profile IRI.
    pub identity: String,
}

/// The effective union of assessor-only declarations, with every source kept.
/// There is deliberately no remove, replace or layer-override operation.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct NeverDeriveDeclarations {
    predicates: BTreeMap<String, BTreeSet<DeclarationSource>>,
}

impl NeverDeriveDeclarations {
    /// Add a declaration without changing any existing authority's contribution.
    pub fn declare(&mut self, predicate: String, source: DeclarationSource) {
        self.predicates.entry(predicate).or_default().insert(source);
    }

    /// Add all declarations and sources from another authority's set.
    pub fn extend(&mut self, declarations: &Self) {
        for (predicate, sources) in &declarations.predicates {
            self.predicates
                .entry(predicate.clone())
                .or_default()
                .extend(sources.iter().cloned());
        }
    }

    /// The effective predicate and its sorted, deduplicated provenance.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &BTreeSet<DeclarationSource>)> {
        self.predicates
            .iter()
            .map(|(predicate, sources)| (predicate.as_str(), sources))
    }

    /// Whether no authority contributed an obligation.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.predicates.is_empty()
    }

    /// Content identity includes every predicate, authority layer and source.
    #[must_use]
    pub fn digest(&self) -> [u8; 32] {
        use purrdf_hash::frame::frame_le_into;
        const DOMAIN: purrdf_hash::Domain =
            purrdf_hash::Domain::new(b"purrdf-datalog/never-derive-declarations/v1");
        let mut hasher = purrdf_hash::blake3::Hasher::new();
        frame_le_into(&mut hasher, DOMAIN.as_bytes());
        hasher.update(&(self.predicates.len() as u64).to_le_bytes());
        for (predicate, sources) in &self.predicates {
            frame_le_into(&mut hasher, predicate.as_bytes());
            hasher.update(&(sources.len() as u64).to_le_bytes());
            for source in sources {
                let layer = match source.layer {
                    DeclarationLayer::Ontology => 0,
                    DeclarationLayer::StoreProfile => 1,
                    DeclarationLayer::RuleSet => 2,
                };
                hasher.update(&[layer]);
                frame_le_into(&mut hasher, source.identity.as_bytes());
            }
        }
        *hasher.finalize().as_bytes()
    }

    /// Check the full DL-clause IR, before any executor narrows its head forms.
    /// # Errors
    /// Refuses a reachable protected head, or a head whose predicate cannot be
    /// established statically. A guard is never invoked as a safety proof.
    pub fn admit(&self, rules: &[DlClause]) -> Result<NeverDeriveCertificate, AdmissionRefusal> {
        if !self.is_empty() {
            check_heads(rules, self)?;
        }
        Ok(NeverDeriveCertificate {
            program: crate::cache::canonical_rule_hash(rules),
            declarations: self.clone(),
        })
    }
}

/// One authored rule in a refusal's deterministic source-to-head chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivationStep {
    /// Zero-based authored rule position.
    pub rule: usize,
    /// The predicate derived by this step; unbracketed for constant IRI heads.
    pub predicate: String,
}

/// Exact static violation, or a static decision the analysis cannot establish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionRefusal {
    /// A rule chain could derive an assessor-only predicate.
    ProtectedHead {
        /// The protected predicate, supplied by the caller.
        predicate: String,
        /// Every authority contributing this declaration.
        sources: Vec<DeclarationSource>,
        /// Stable authored rule/predicate chain.
        chain: Vec<DerivationStep>,
    },
    /// A variable or otherwise unresolved head may name a protected predicate.
    UndecidableHead {
        /// The first authored rule with an unresolved predicate.
        rule: usize,
        /// The declarations whose protection could not be proved.
        declarations: NeverDeriveDeclarations,
    },
    /// A cached certificate was supplied for another rule bundle.
    ProgramChanged,
}

impl fmt::Display for AdmissionRefusal {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProtectedHead {
                predicate, chain, ..
            } => {
                write!(
                    output,
                    "rule admission refuses derivation of assessor-only predicate <{predicate}>:"
                )?;
                for step in chain {
                    write!(output, " rule {} -> <{}>", step.rule, step.predicate)?;
                }
                Ok(())
            }
            Self::UndecidableHead { rule, .. } => write!(
                output,
                "rule {rule}'s head predicate cannot be proved disjoint from the never-derive declarations"
            ),
            Self::ProgramChanged => output.write_str(
                "never-derive admission certificate belongs to a different rule program",
            ),
        }
    }
}
impl std::error::Error for AdmissionRefusal {}

/// Immutable admission bound to the complete rule bundle and declaration union.
/// The private fields prevent a caller constructing an unchecked certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeverDeriveCertificate {
    program: [u8; 32],
    declarations: NeverDeriveDeclarations,
}

impl NeverDeriveCertificate {
    /// The union actually admitted, including all declaration sources.
    #[must_use]
    pub fn declarations(&self) -> &NeverDeriveDeclarations {
        &self.declarations
    }

    /// Re-admit under an additional profile without withdrawing earlier layers.
    /// # Errors
    /// A changed bundle, protected derivation, or unresolved head is refused.
    pub fn extend(
        &self,
        rules: &[DlClause],
        additions: &NeverDeriveDeclarations,
    ) -> Result<Self, AdmissionRefusal> {
        if self.program != crate::cache::canonical_rule_hash(rules) {
            return Err(AdmissionRefusal::ProgramChanged);
        }
        let mut effective = self.declarations.clone();
        effective.extend(additions);
        effective.admit(rules)
    }
}

fn check_heads(
    rules: &[DlClause],
    declarations: &NeverDeriveDeclarations,
) -> Result<(), AdmissionRefusal> {
    let mut protected = Vec::new();
    let mut unresolved = None;
    for (rule, clause) in rules.iter().enumerate() {
        for head in clause.head_atoms() {
            match head.predicate() {
                ClauseTerm::Iri(predicate) => {
                    if let Some(sources) = declarations.predicates.get(predicate) {
                        protected.push((rule, predicate.clone(), sources));
                    }
                }
                ClauseTerm::Var(_) | ClauseTerm::Literal(_) | ClauseTerm::DefaultGraph => {
                    unresolved.get_or_insert(rule);
                }
            }
        }
    }
    if let Some(rule) = unresolved {
        return Err(AdmissionRefusal::UndecidableHead {
            rule,
            declarations: declarations.clone(),
        });
    }
    if protected.is_empty() {
        return Ok(());
    }
    let (graph, heads) = rule_dependencies(rules);
    let destinations: BTreeSet<_> = graph.values().flatten().copied().collect();
    let roots: Vec<_> = (0..rules.len())
        .filter(|rule| !destinations.contains(rule))
        .collect();
    let mut chosen: Option<(Vec<usize>, String, &BTreeSet<DeclarationSource>)> = None;
    for (last, predicate, sources) in protected {
        let path = roots
            .iter()
            .filter_map(|root| crate::paths::shortest_path(&graph, root, &last))
            .min_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)))
            .unwrap_or_else(|| vec![last]);
        let better = chosen
            .as_ref()
            .is_none_or(|(previous, previous_predicate, _)| {
                (path.len(), &path, &predicate) < (previous.len(), previous, previous_predicate)
            });
        if better {
            chosen = Some((path, predicate, sources));
        }
    }
    let (path, predicate, sources) = chosen.expect("a protected head was collected");
    let chain = path
        .iter()
        .enumerate()
        .map(|(position, rule)| {
            let selected = if position + 1 == path.len() {
                predicate.clone()
            } else {
                let next = path[position + 1];
                let symbol = heads[rule]
                    .iter()
                    .find(|symbol| reads_predicate(&rules[next], symbol))
                    .expect("dependency edge names a head read by the next rule");
                rules[*rule]
                    .head_atoms()
                    .find(|head| predicate_symbol(head) == *symbol)
                    .and_then(|head| head.predicate_iri())
                    .expect("protected analysis admitted only constant IRI heads")
                    .to_owned()
            };
            DerivationStep {
                rule: *rule,
                predicate: selected,
            }
        })
        .collect();
    Err(AdmissionRefusal::ProtectedHead {
        predicate,
        sources: sources.iter().cloned().collect(),
        chain,
    })
}

fn reads_predicate(rule: &DlClause, symbol: &str) -> bool {
    rule.body()
        .iter()
        .chain(
            rule.negations()
                .iter()
                .flat_map(crate::guard::Negation::atoms),
        )
        .any(|atom| predicate_symbol(atom) == symbol || predicate_symbol(atom) == ANY_PREDICATE)
}

fn rule_dependencies(
    rules: &[DlClause],
) -> (
    BTreeMap<usize, BTreeSet<usize>>,
    BTreeMap<usize, BTreeSet<String>>,
) {
    let mut producers: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
    let mut heads: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
    for (index, clause) in rules.iter().enumerate() {
        for head in clause.head_atoms() {
            let symbol = predicate_symbol(head);
            producers.entry(symbol.clone()).or_default().insert(index);
            heads.entry(index).or_default().insert(symbol);
        }
    }
    let (_, edges) = dependency_edges_with(rules, |rule, _, body, _| (rule, body));
    let mut graph: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for (rule, body) in edges {
        let Some(rule) = rule else {
            continue;
        };
        if body == ANY_PREDICATE {
            for producer in producers.values().flatten() {
                graph.entry(*producer).or_default().insert(rule);
            }
        } else {
            for producer in producers.get(&body).into_iter().flatten() {
                graph.entry(*producer).or_default().insert(rule);
            }
        }
    }
    (graph, heads)
}
