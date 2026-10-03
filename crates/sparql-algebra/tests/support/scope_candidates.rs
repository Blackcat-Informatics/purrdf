// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Executable comparison of identity admission, typed binders and frozen contracts.
//!
//! Source predicates carry stable occurrence provenance in these small research
//! inputs. Repeated predicates are retained as repeated atoms, never sets. The
//! certificate proves binding relationships for the supplied provenance, not
//! general SPARQL equivalence. Explicit graph markers model binder regions; their
//! meaning is research metadata rather than a proposed SPARQL vocabulary.

// Each consumer uses a different subset of the shared research API.
#![allow(dead_code, unreachable_pub)]

use std::collections::{BTreeMap, BTreeSet};
use std::mem::size_of;

use purrdf_sparql_algebra::{
    BlankNode, Flow, GraphPattern, GroundTerm, NamedNode, NamedNodePattern, NodeRef, Query,
    QueryDataset, SparqlParser, SparqlVersion, TermPattern, TriplePattern, Variable, Visit,
    fold_post_order, pattern_to_select_query, walk_pre_post,
};

const SITE: &str = "http://example.org/site/";
const REGION: &str = "http://example.org/scope/";
const MAX_CLAUSES: usize = 4096;

/// Semantic categories whose identity rules differ.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Category {
    User,
    Existential,
    Witness,
    TemplateAllocation,
    DatasetBlank,
}

/// Where a binding acquired its identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Origin {
    SourceVariable,
    SourceBlank,
    PathTranslation,
    Template,
    DatasetParameter,
}

/// Declaration carried by the explicit representation candidate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Binder {
    pub id: u32,
    pub category: Category,
    pub origin: Origin,
    pub owner: u32,
}

/// An encoded identity; raw blank spellings belong to an admitted source scope.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Identity {
    pub name: String,
    pub blank_scope: Option<u32>,
    pub blank_category: Option<Category>,
    pub hidden: bool,
}

impl Identity {
    fn variable(variable: &Variable) -> Self {
        Self {
            name: variable.as_str().to_owned(),
            blank_scope: None,
            blank_category: None,
            hidden: variable.is_hidden(),
        }
    }

    fn blank(name: &str, scope: u32) -> Self {
        Self {
            name: name.to_owned(),
            blank_scope: Some(scope),
            blank_category: Some(Category::Existential),
            hidden: false,
        }
    }

    fn allocation(name: &str) -> Self {
        Self {
            blank_category: Some(Category::TemplateAllocation),
            ..Self::blank(name, 0)
        }
    }

    fn dataset(name: &str) -> Self {
        Self {
            blank_category: Some(Category::DatasetBlank),
            ..Self::blank(name, 0)
        }
    }
}

/// One actual source-site binding incidence.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Atom {
    pub site: u32,
    pub object: bool,
    pub identity: Identity,
    pub region: u32,
    pub binder: Option<Binder>,
    pub template: bool,
    pub ground: bool,
}

/// Typed experiment diagnostics; a site and identity identify repair locations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Diagnostic {
    Admission(String),
    MissingDeclaration {
        site: u32,
        identity: Identity,
    },
    DuplicateBinder {
        id: u32,
        first: Identity,
        second: Identity,
    },
    Category {
        site: u32,
        identity: Identity,
        declared: Category,
    },
    Origin {
        site: u32,
        identity: Identity,
    },
    Ownership {
        site: u32,
        owner: u32,
        region: u32,
    },
    PrematureProjection(Identity),
    NonDistinguishedProjection(Identity),
    AliasCollision(String),
    Correspondence,
    BindingPartition,
    BranchMultiplicity,
    VisibleSchema,
    ClauseBudget,
    UnsupportedOperator,
    UnsupportedTerm(CompoundTerm),
    UnsupportedProjection(Identity),
    UnsupportedGraphName(Identity),
    UnsupportedTemplateGraphName(Identity),
    UnsupportedQueryForm(QueryForm),
    ContradictorySourceSite(u32),
    AmbiguousSourceOwner(u32),
    MissingProvenance,
}

/// Recursive term families outside the certificate prototype's flat-term model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompoundTerm {
    PatternTriple,
    GroundTriple,
}

/// Query heads whose observation rules are outside this prototype.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryForm {
    Ask,
    Describe,
}

/// Actual algebra plus explicit declarations and carrier correspondence.
#[derive(Clone)]
pub struct Evidence {
    pub query: Query,
    pub declarations: BTreeMap<Identity, Binder>,
    pub aliases: BTreeMap<Identity, String>,
}

#[derive(Clone, Default)]
struct Summary {
    clauses: Vec<Vec<Atom>>,
    live: BTreeSet<Identity>,
    seen: BTreeSet<Identity>,
    projected: BTreeSet<Identity>,
    diagnostics: Vec<Diagnostic>,
}

/// A reusable, precondition-frozen binding-incidence contract.
#[derive(Clone)]
pub struct Contract {
    clauses: Vec<Vec<Atom>>,
    visible: Vec<Identity>,
    declarations: BTreeMap<Identity, Binder>,
}

fn user(name: &str) -> TermPattern {
    TermPattern::Variable(Variable::new(name))
}

fn witness(name: &str) -> TermPattern {
    TermPattern::Variable(Variable::hidden(name))
}

fn edge(site: u32, subject: TermPattern, object: TermPattern) -> GraphPattern {
    GraphPattern::Bgp {
        patterns: vec![TriplePattern {
            subject,
            predicate: NamedNodePattern::NamedNode(
                NamedNode::new(format!("{SITE}{site}")).expect("valid research source IRI"),
            ),
            object,
        }],
    }
}

fn join(left: GraphPattern, right: GraphPattern) -> GraphPattern {
    GraphPattern::Join {
        left: left.into(),
        right: right.into(),
    }
}

fn select(pattern: GraphPattern) -> Query {
    Query::Select {
        pattern: GraphPattern::Project {
            inner: pattern.into(),
            variables: vec![Variable::new("s"), Variable::new("o")],
        },
        dataset: QueryDataset::default(),
        base_iri: None,
        version: Some(SparqlVersion::V12),
    }
}

fn body(query: &Query) -> &GraphPattern {
    match query.pattern() {
        GraphPattern::Project { inner, .. } => inner,
        pattern => pattern,
    }
}

/// Prepare current encoded variable declarations; source scope provenance is supplied separately.
pub fn declare_variables(query: &Query) -> BTreeMap<Identity, Binder> {
    let mut variables = BTreeSet::new();
    walk_pre_post(NodeRef::Pattern(query.pattern()), |visit, node| {
        if visit == Visit::Enter {
            node.for_each_variable(|variable| {
                variables.insert(variable.clone());
            });
        }
        Flow::Descend
    });
    variables
        .into_iter()
        .enumerate()
        .map(|(id, variable)| {
            let (category, origin) = if variable.is_hidden() {
                (Category::Witness, Origin::PathTranslation)
            } else {
                (Category::User, Origin::SourceVariable)
            };
            (
                Identity::variable(&variable),
                Binder {
                    id: u32::try_from(id).expect("bounded evidence identities"),
                    category,
                    origin,
                    owner: 0,
                },
            )
        })
        .collect()
}

/// A duplicate-arm path/UNION fixture with two independent generated witnesses.
pub fn fixture() -> Evidence {
    let a = edge(1, user("s"), witness("h"));
    let alternative = GraphPattern::union(
        GraphPattern::union(a.clone(), edge(2, user("s"), witness("h"))),
        a,
    );
    let query = select(join(
        alternative,
        join(
            edge(3, witness("h"), witness("k")),
            edge(4, witness("k"), user("o")),
        ),
    ));
    Evidence {
        declarations: declare_variables(&query),
        query,
        aliases: BTreeMap::new(),
    }
}

/// Generate genuine positive-pattern shapes with independent width, branch-bag
/// multiplicity and witness-junction dimensions. Source-site labels are assigned
/// before any transformation and retained by all copies of a repeated arm.
pub fn generated_fixture(
    branches: u32,
    width: u32,
    junctions: u32,
    duplicate_arms: u32,
) -> Evidence {
    assert!(branches >= 2 && width > 0 && junctions > 0);
    let mut alternatives = Vec::new();
    for branch in 0..branches {
        let mut patterns = Vec::new();
        for part in 0..width {
            let GraphPattern::Bgp { patterns: mut next } =
                edge(branch * width + part + 1, user("s"), witness("junction0"))
            else {
                unreachable!();
            };
            patterns.append(&mut next);
        }
        alternatives.push(GraphPattern::Bgp { patterns });
    }
    for _ in 0..duplicate_arms {
        alternatives.push(alternatives[0].clone());
    }
    let alternatives = alternatives
        .into_iter()
        .reduce(GraphPattern::union)
        .expect("nonempty branches");
    let tail = (0..junctions)
        .rev()
        .map(|junction| {
            edge(
                branches * width + junction + 1,
                witness(&format!("junction{junction}")),
                if junction + 1 == junctions {
                    user("o")
                } else {
                    witness(&format!("junction{}", junction + 1))
                },
            )
        })
        .reduce(|tail, head| join(head, tail))
        .expect("nonempty junction chain");
    let query = select(join(alternatives, tail));
    Evidence {
        declarations: declare_variables(&query),
        query,
        aliases: BTreeMap::new(),
    }
}

/// Whether the complete actual query is admitted by the identity-reencoding
/// transform, independently of a fixture or mutation's name.
pub fn supports_reencoding(evidence: &Evidence) -> bool {
    if !matches!(evidence.query, Query::Select { .. }) {
        return false;
    }
    walk_pre_post(NodeRef::Pattern(evidence.query.pattern()), |visit, node| {
        if visit == Visit::Enter
            && (matches!(node, NodeRef::Pattern(pattern) if !matches!(pattern,
                GraphPattern::Bgp { .. } | GraphPattern::Join { .. }
                | GraphPattern::Union { .. } | GraphPattern::Project { .. }
                | GraphPattern::Graph { .. }))
                || matches!(
                    node,
                    NodeRef::Term(TermPattern::Triple(_)) | NodeRef::Ground(GroundTerm::Triple(_))
                ))
        {
            return Flow::Stop;
        }
        Flow::Descend
    })
}

/// Configure independent repeated solution branches before freezing a contract.
/// This also varies the number of concrete VALUES rows or template input rows.
pub fn repeated_input(evidence: &Evidence, copies: usize) -> Evidence {
    assert!(copies > 0);
    let repeated = (0..copies)
        .map(|_| body(&evidence.query).clone())
        .reduce(GraphPattern::union)
        .expect("nonempty repeated input");
    let mut out = evidence.clone();
    match &mut out.query {
        Query::Select { pattern, .. } => {
            let GraphPattern::Project { inner, .. } = pattern else {
                panic!("configured SELECT projection");
            };
            *inner = repeated.into();
        }
        Query::Construct { pattern, .. } => *pattern = repeated,
        Query::Ask { .. } | Query::Describe { .. } => {
            panic!("repeated evidence requires SELECT or CONSTRUCT")
        }
    }
    out
}

/// A bijective representation-only renumbering, applicable to typed categories
/// that have no hidden variable to alpha-rename, including template and VALUES blanks.
pub fn renumber_binders(evidence: &Evidence, offset: u32) -> Evidence {
    let mut out = evidence.clone();
    for binder in out.declarations.values_mut() {
        binder.id = binder
            .id
            .checked_add(offset)
            .expect("bounded generated binder numbering");
    }
    out
}

fn site(triple: &TriplePattern) -> Result<u32, Diagnostic> {
    let NamedNodePattern::NamedNode(predicate) = &triple.predicate else {
        return Err(Diagnostic::MissingProvenance);
    };
    predicate
        .as_str()
        .strip_prefix(SITE)
        .and_then(|value| value.parse().ok())
        .ok_or(Diagnostic::MissingProvenance)
}

fn atom(
    term: &TermPattern,
    site: u32,
    object: bool,
    region: u32,
    blank_scope: u32,
    template: bool,
    declarations: &BTreeMap<Identity, Binder>,
) -> Option<Atom> {
    let identity = match term {
        TermPattern::Variable(variable) => Identity::variable(variable),
        TermPattern::BlankNode(blank) => {
            if template {
                Identity::allocation(blank.as_str())
            } else {
                Identity::blank(blank.as_str(), blank_scope)
            }
        }
        TermPattern::NamedNode(_) | TermPattern::Literal(_) | TermPattern::Triple(_) => {
            return None;
        }
    };
    let binder = declarations.get(&identity).copied();
    Some(Atom {
        site,
        object,
        identity,
        region,
        binder,
        template,
        ground: false,
    })
}

fn triple_summary(
    triple: &TriplePattern,
    region: u32,
    blank_scope: u32,
    template: bool,
    evidence: &Evidence,
) -> Summary {
    let mut out = Summary::default();
    if matches!(triple.subject, TermPattern::Triple(_))
        || matches!(triple.object, TermPattern::Triple(_))
    {
        out.diagnostics
            .push(Diagnostic::UnsupportedTerm(CompoundTerm::PatternTriple));
    }
    let source = match site(triple) {
        Ok(source) => source,
        Err(error) => {
            out.diagnostics.push(error);
            return out;
        }
    };
    let atoms: Vec<_> = [(&triple.subject, false), (&triple.object, true)]
        .into_iter()
        .filter_map(|(term, object)| {
            atom(
                term,
                source,
                object,
                region,
                blank_scope,
                template,
                &evidence.declarations,
            )
        })
        .collect();
    out.live
        .extend(atoms.iter().map(|atom| atom.identity.clone()));
    out.seen.clone_from(&out.live);
    out.clauses.push(atoms);
    out
}

fn conjunction(mut left: Summary, right: Summary) -> Summary {
    for identity in left
        .projected
        .intersection(&right.seen)
        .chain(right.projected.intersection(&left.seen))
    {
        left.diagnostics
            .push(Diagnostic::PrematureProjection(identity.clone()));
    }
    let product = left.clauses.len().saturating_mul(right.clauses.len());
    if product > MAX_CLAUSES {
        left.diagnostics.push(Diagnostic::ClauseBudget);
        left.clauses.clear();
    } else if right.clauses.len() == 1 {
        for clause in &mut left.clauses {
            clause.extend(right.clauses[0].iter().cloned());
        }
    } else {
        left.clauses = left
            .clauses
            .into_iter()
            .flat_map(|clause| {
                right.clauses.iter().map(move |other| {
                    let mut combined = clause.clone();
                    combined.extend(other.iter().cloned());
                    combined
                })
            })
            .collect();
    }
    left.live.extend(right.live);
    left.seen.extend(right.seen);
    left.projected.extend(right.projected);
    left.diagnostics.extend(right.diagnostics);
    left
}

fn spine_owner(pattern: &GraphPattern) -> Option<u32> {
    let mut owner = None;
    purrdf_sparql_algebra::scope::visit_spine_leaves(pattern, &mut |leaf| {
        if let GraphPattern::Bgp { patterns } = leaf {
            for source in patterns.iter().filter_map(|triple| site(triple).ok()) {
                owner = Some(owner.map_or(source, |previous: u32| previous.min(source)));
            }
        }
        false
    });
    owner
}

fn summary(evidence: &Evidence) -> Summary {
    let unsupported_form = match &evidence.query {
        Query::Ask { .. } => Some(QueryForm::Ask),
        Query::Describe { .. } => Some(QueryForm::Describe),
        Query::Select { .. } | Query::Construct { .. } => None,
    };
    if let Some(form) = unsupported_form {
        return Summary {
            diagnostics: vec![Diagnostic::UnsupportedQueryForm(form)],
            ..Summary::default()
        };
    }
    // Addresses are private lookup keys only. Diagnostics and outputs use source
    // site numbers; address order never escapes the borrowed extraction.
    let mut regions = BTreeMap::<*const TriplePattern, (u32, u32)>::new();
    let mut sources = BTreeMap::<u32, &TriplePattern>::new();
    let mut raw_owners = BTreeMap::<u32, *const GraphPattern>::new();
    let mut provenance_diagnostics = Vec::new();
    let mut pending = vec![(NodeRef::Pattern(evidence.query.pattern()), 0, None)];
    while let Some((node, inherited, inherited_bgp)) = pending.pop() {
        let region = if let NodeRef::Pattern(GraphPattern::Graph {
            name: NamedNodePattern::NamedNode(name),
            ..
        }) = node
        {
            name.as_str()
                .strip_prefix(REGION)
                .and_then(|value| value.parse().ok())
                .unwrap_or(inherited)
        } else {
            inherited
        };
        // The production law determines source ownership: positive Join pieces
        // share their BGP scope; UNION, GRAPH and projection are boundaries.
        // The minimum frozen source site names that owner in this research IR.
        // Its identity must be injective over actual owners containing raw
        // blanks; coherent copies across boundaries need richer provenance.
        let bgp = match node {
            NodeRef::Pattern(pattern)
                if purrdf_sparql_algebra::scope::joins_blank_scope(pattern) =>
            {
                inherited_bgp.or_else(|| {
                    spine_owner(pattern).map(|owner| (owner, std::ptr::from_ref(pattern)))
                })
            }
            NodeRef::Pattern(pattern @ GraphPattern::Bgp { .. }) => inherited_bgp
                .or_else(|| spine_owner(pattern).map(|owner| (owner, std::ptr::from_ref(pattern)))),
            NodeRef::Pattern(_) => None,
            _ => inherited_bgp,
        };
        if let NodeRef::Triple(triple) = node {
            regions.insert(triple, (region, bgp.map_or(0, |(owner, _)| owner)));
            if let Ok(source) = site(triple) {
                // All supported fields are flat, so this borrowed comparison
                // is stack safe. Recursive terms get their separate refusal.
                let flat = |triple: &TriplePattern| {
                    !matches!(triple.subject, TermPattern::Triple(_))
                        && !matches!(triple.object, TermPattern::Triple(_))
                };
                if let Some(previous) = sources.insert(source, triple)
                    && flat(previous)
                    && flat(triple)
                    && previous != triple
                {
                    provenance_diagnostics.push(Diagnostic::ContradictorySourceSite(source));
                }
            }
            if let Some((owner, actual_owner)) = bgp
                && (matches!(triple.subject, TermPattern::BlankNode(_))
                    || matches!(triple.object, TermPattern::BlankNode(_)))
                && let Some(previous) = raw_owners.insert(owner, actual_owner)
                && previous != actual_owner
            {
                provenance_diagnostics.push(Diagnostic::AmbiguousSourceOwner(owner));
            }
        }
        node.for_each_child(|child| pending.push((child, region, bgp)));
    }
    let mut out = fold_post_order(
        NodeRef::Pattern(evidence.query.pattern()),
        |node, children| match node {
            NodeRef::Triple(triple) => {
                let (region, bgp) = regions[&std::ptr::from_ref(triple)];
                triple_summary(triple, region, bgp, false, evidence)
            }
            NodeRef::Pattern(GraphPattern::Bgp { .. }) => {
                children.reduce(conjunction).unwrap_or_else(|| Summary {
                    clauses: vec![vec![]],
                    ..Summary::default()
                })
            }
            NodeRef::Pattern(GraphPattern::Join { .. }) => conjunction(
                children.next().expect("left"),
                children.next().expect("right"),
            ),
            NodeRef::Pattern(GraphPattern::Union { .. }) => {
                let mut result = Summary::default();
                for child in children {
                    result.clauses.extend(child.clauses);
                    result.live.extend(child.live);
                    result.seen.extend(child.seen);
                    result.projected.extend(child.projected);
                    result.diagnostics.extend(child.diagnostics);
                }
                if result.clauses.len() > MAX_CLAUSES {
                    result.diagnostics.push(Diagnostic::ClauseBudget);
                }
                result
            }
            NodeRef::Pattern(pattern @ GraphPattern::Project { variables, .. }) => {
                let mut result = children.next().expect("projected input");
                let retained: BTreeSet<_> = variables.iter().map(Identity::variable).collect();
                // Carrier aliases are ordinary SPARQL names, but their supplied
                // category still forbids observing a non-distinguished binding.
                for identity in &retained {
                    if evidence.declarations.get(identity).is_some_and(|binder| {
                        matches!(binder.category, Category::Witness | Category::Existential)
                    }) {
                        result
                            .diagnostics
                            .push(Diagnostic::NonDistinguishedProjection(identity.clone()));
                    }
                }
                if !std::ptr::eq(pattern, evidence.query.pattern()) {
                    for identity in result.live.difference(&retained) {
                        if evidence
                            .declarations
                            .get(identity)
                            .is_some_and(|binder| binder.category == Category::User)
                        {
                            result
                                .diagnostics
                                .push(Diagnostic::UnsupportedProjection(identity.clone()));
                        }
                    }
                }
                result.projected.extend(
                    result
                        .live
                        .difference(&retained)
                        .filter(|identity| {
                            identity.hidden || evidence.aliases.contains_key(*identity)
                        })
                        .cloned(),
                );
                result.live.retain(|identity| retained.contains(identity));
                result
            }
            NodeRef::Pattern(GraphPattern::Graph { name, .. }) => {
                let mut result = children.next().expect("graph input");
                if let NamedNodePattern::Variable(variable) = name {
                    result
                        .diagnostics
                        .push(Diagnostic::UnsupportedGraphName(Identity::variable(
                            variable,
                        )));
                }
                result
            }
            NodeRef::Pattern(GraphPattern::Values {
                variables,
                bindings,
            }) => {
                let mut result = Summary::default();
                for (row, values) in bindings.iter().enumerate() {
                    let mut clause = Vec::new();
                    for (column, (variable, value)) in variables.iter().zip(values).enumerate() {
                        let source = 100_000_000_u32
                            + u32::try_from(row * variables.len() + column)
                                .expect("bounded VALUES provenance");
                        let identity = Identity::variable(variable);
                        result.live.insert(identity.clone());
                        clause.push(Atom {
                            site: source,
                            object: false,
                            binder: evidence.declarations.get(&identity).copied(),
                            identity,
                            region: 0,
                            template: false,
                            ground: false,
                        });
                        if let Some(GroundTerm::BlankNode(blank)) = value {
                            let identity = Identity::dataset(blank.as_str());
                            clause.push(Atom {
                                site: source,
                                object: true,
                                binder: evidence.declarations.get(&identity).copied(),
                                identity,
                                region: 0,
                                template: false,
                                ground: true,
                            });
                        }
                        if matches!(value, Some(GroundTerm::Triple(_))) {
                            result
                                .diagnostics
                                .push(Diagnostic::UnsupportedTerm(CompoundTerm::GroundTriple));
                        }
                    }
                    result.clauses.push(clause);
                }
                result.seen = result.live.clone();
                result
            }
            NodeRef::Pattern(_) => Summary {
                diagnostics: vec![Diagnostic::UnsupportedOperator],
                ..Summary::default()
            },
            _ => Summary::default(),
        },
    );
    if let Query::Construct { template, .. } = &evidence.query {
        for quad in template {
            out = conjunction(out, triple_summary(&quad.triple, 0, 0, true, evidence));
            if let Some(NamedNodePattern::Variable(variable)) = &quad.graph {
                out.diagnostics
                    .push(Diagnostic::UnsupportedTemplateGraphName(
                        Identity::variable(variable),
                    ));
            }
        }
    }
    out.diagnostics.extend(provenance_diagnostics);
    out
}

fn visible(query: &Query) -> Vec<Identity> {
    if let GraphPattern::Project { variables, .. } = query.pattern() {
        variables.iter().map(Identity::variable).collect()
    } else {
        vec![]
    }
}

/// What actual current admission proves, without any candidate metadata.
pub fn existing(evidence: &Evidence) -> Result<(), Diagnostic> {
    evidence
        .query
        .validate_hidden_variables()
        .map_err(|error| Diagnostic::Admission(error.to_string()))
}

fn check_atoms(summary: &Summary, evidence: &Evidence) -> Result<(), Diagnostic> {
    if let Some(error) = summary.diagnostics.first() {
        return Err(error.clone());
    }
    let mut binder_ids = BTreeMap::new();
    for atom in summary.clauses.iter().flatten() {
        let Some(binder) = atom.binder else {
            return Err(Diagnostic::MissingDeclaration {
                site: atom.site,
                identity: atom.identity.clone(),
            });
        };
        if let Some(first) = binder_ids.insert(binder.id, &atom.identity)
            && first != &atom.identity
        {
            return Err(Diagnostic::DuplicateBinder {
                id: binder.id,
                first: first.clone(),
                second: atom.identity.clone(),
            });
        }
        let category_valid = if atom.identity.blank_scope.is_some() {
            binder.category
                == if atom.ground {
                    Category::DatasetBlank
                } else if atom.template {
                    Category::TemplateAllocation
                } else {
                    Category::Existential
                }
        } else if atom.identity.hidden || evidence.aliases.contains_key(&atom.identity) {
            matches!(binder.category, Category::Witness | Category::Existential) && !atom.template
        } else {
            binder.category == Category::User
        };
        if !category_valid {
            return Err(Diagnostic::Category {
                site: atom.site,
                identity: atom.identity.clone(),
                declared: binder.category,
            });
        }
        let origin_valid = matches!(
            (binder.category, binder.origin),
            (Category::User, Origin::SourceVariable)
                | (Category::Existential, Origin::SourceBlank)
                | (Category::Witness, Origin::PathTranslation)
                | (Category::TemplateAllocation, Origin::Template)
                | (Category::DatasetBlank, Origin::DatasetParameter)
        );
        if !origin_valid {
            return Err(Diagnostic::Origin {
                site: atom.site,
                identity: atom.identity.clone(),
            });
        }
        let expected_owner = if binder.category == Category::User {
            0
        } else {
            atom.identity.blank_scope.unwrap_or(atom.region)
        };
        if binder.owner != expected_owner {
            return Err(Diagnostic::Ownership {
                site: atom.site,
                owner: binder.owner,
                region: expected_owner,
            });
        }
    }
    let mut names: BTreeSet<String> = evidence
        .declarations
        .iter()
        .filter(|(_, binder)| binder.category == Category::User)
        .map(|(identity, _)| identity.name.clone())
        .collect();
    for alias in evidence.aliases.values() {
        if !names.insert(alias.clone()) {
            return Err(Diagnostic::AliasCollision(alias.clone()));
        }
    }
    Ok(())
}

/// Structural checking of the altered graph against its typed declarations.
pub fn explicit(evidence: &Evidence) -> Result<(), Diagnostic> {
    existing(evidence)?;
    check_atoms(&summary(evidence), evidence)
}

impl Contract {
    /// Freeze original declarations, source-site partitions, branch bags and schema.
    pub fn prepare(evidence: &Evidence) -> Result<Self, Diagnostic> {
        existing(evidence)?;
        let mut summary = summary(evidence);
        check_atoms(&summary, evidence)?;
        canonicalize(&mut summary.clauses);
        Ok(Self {
            clauses: summary.clauses,
            visible: visible(&evidence.query),
            declarations: evidence.declarations.clone(),
        })
    }

    /// Check actual output with an explicit one-to-one identity correspondence.
    pub fn check(
        &self,
        evidence: &Evidence,
        correspondence: &BTreeMap<Identity, Identity>,
    ) -> Result<(), Diagnostic> {
        existing(evidence)?;
        let mut after = summary(evidence);
        check_atoms(&after, evidence)?;
        for before in correspondence.keys() {
            if !self.declarations.contains_key(before) {
                return Err(Diagnostic::Correspondence);
            }
        }
        let mut target = BTreeSet::new();
        for before in self.declarations.keys() {
            let destination = correspondence.get(before).unwrap_or(before);
            if !evidence.declarations.contains_key(destination) || !target.insert(destination) {
                return Err(Diagnostic::Correspondence);
            }
        }
        let inverse: BTreeMap<_, _> = correspondence.iter().map(|(a, b)| (b, a)).collect();
        for atom in after.clauses.iter_mut().flatten() {
            if let Some(original) = inverse.get(&atom.identity) {
                atom.identity = (*original).clone();
            }
            // Binder numbers may change in a representation migration, but the
            // category, origin and ownership obligations must survive it.
            if let Some(original) = self.declarations.get(&atom.identity)
                && let Some(current) = atom.binder.as_mut()
            {
                current.id = original.id;
            }
        }
        let schema: Vec<_> = visible(&evidence.query)
            .into_iter()
            .map(|identity| {
                inverse
                    .get(&identity)
                    .map_or_else(|| identity.clone(), |original| (*original).clone())
            })
            .collect();
        if schema != self.visible {
            return Err(Diagnostic::VisibleSchema);
        }
        canonicalize(&mut after.clauses);
        if after.clauses.len() != self.clauses.len() {
            return Err(Diagnostic::BranchMultiplicity);
        }
        if after.clauses != self.clauses {
            return Err(Diagnostic::BindingPartition);
        }
        Ok(())
    }

    /// Retained payload accounting; BTreeMap allocator/node overhead is separate.
    pub fn retained_payload_bytes(&self) -> usize {
        size_of::<Self>()
            + self.clauses.capacity() * size_of::<Vec<Atom>>()
            + self
                .clauses
                .iter()
                .map(|clause| {
                    clause.capacity() * size_of::<Atom>()
                        + clause
                            .iter()
                            .map(|atom| atom.identity.name.capacity())
                            .sum::<usize>()
                })
                .sum::<usize>()
            + self.visible.capacity() * size_of::<Identity>()
            + self
                .visible
                .iter()
                .map(|identity| identity.name.capacity())
                .sum::<usize>()
            + self
                .declarations
                .keys()
                .map(|identity| {
                    size_of::<Identity>() + size_of::<Binder>() + identity.name.capacity()
                })
                .sum::<usize>()
    }
}

fn canonicalize(clauses: &mut [Vec<Atom>]) {
    for clause in clauses.iter_mut() {
        clause.sort();
    }
    clauses.sort();
}

/// Identity substitution using the authoritative serializer/reparser boundary.
pub fn carrier(evidence: &Evidence) -> (Evidence, BTreeMap<Identity, Identity>) {
    let rendered = pattern_to_select_query(evidence.query.pattern());
    let query = SparqlParser::new()
        .parse_query(&rendered)
        .expect("valid carrier");
    let original = summary(evidence);
    let bare = Evidence {
        declarations: BTreeMap::new(),
        query,
        aliases: BTreeMap::new(),
    };
    let actual = summary(&bare);
    let source: BTreeMap<_, _> = original
        .clauses
        .iter()
        .flatten()
        .map(|atom| ((atom.site, atom.object), &atom.identity))
        .collect();
    let mut correspondence = BTreeMap::new();
    for atom in actual.clauses.iter().flatten() {
        let from = source[&(atom.site, atom.object)];
        if from != &atom.identity
            && let Some(previous) = correspondence.insert(from.clone(), atom.identity.clone())
        {
            assert_eq!(
                previous, atom.identity,
                "one source identity has one carrier name"
            );
        }
    }
    let mut out = bare;
    for (identity, binder) in &evidence.declarations {
        let destination = correspondence.get(identity).unwrap_or(identity).clone();
        out.declarations.insert(destination.clone(), *binder);
        if binder.category == Category::Witness {
            out.aliases.insert(destination.clone(), destination.name);
        }
    }
    (out, correspondence)
}

/// Legal reassociation of the same generated binding incidences.
pub fn reassociate(evidence: &Evidence) -> Evidence {
    let GraphPattern::Join { left, right } = body(&evidence.query) else {
        panic!("fixture join");
    };
    let GraphPattern::Join {
        left: middle,
        right: tail,
    } = &**right
    else {
        panic!("fixture tail");
    };
    let mut out = evidence.clone();
    out.query = select(join(
        join((**left).clone(), (**middle).clone()),
        (**tail).clone(),
    ));
    out
}

/// Distribute a join across all UNION arms, retaining repeated arms.
pub fn distribute(evidence: &Evidence) -> Evidence {
    let GraphPattern::Join { left, right } = body(&evidence.query) else {
        panic!("fixture join");
    };
    let GraphPattern::Union { arms } = &**left else {
        panic!("fixture union");
    };
    let distributed = arms
        .iter()
        .map(|arm| join(arm.clone(), (**right).clone()))
        .reduce(GraphPattern::union)
        .expect("nonempty alternative");
    let mut out = evidence.clone();
    out.query = select(distributed);
    out
}

fn rewrite(evidence: &Evidence, edit: impl Fn(&TriplePattern) -> TriplePattern) -> Evidence {
    let query = fold_post_order(
        NodeRef::Pattern(evidence.query.pattern()),
        |node, children| {
            match node {
                NodeRef::Pattern(GraphPattern::Bgp { patterns }) => GraphPattern::Bgp {
                    patterns: patterns.iter().map(&edit).collect(),
                },
                NodeRef::Pattern(GraphPattern::Join { .. }) => join(
                    children.next().expect("left"),
                    children.next().expect("right"),
                ),
                NodeRef::Pattern(GraphPattern::Union { .. }) => children
                    .reduce(GraphPattern::union)
                    .expect("nonempty union"),
                NodeRef::Pattern(GraphPattern::Project { variables, .. }) => {
                    GraphPattern::Project {
                        variables: variables.clone(),
                        inner: children.next().expect("projection").into(),
                    }
                }
                NodeRef::Pattern(GraphPattern::Graph { name, .. }) => GraphPattern::Graph {
                    name: name.clone(),
                    inner: children.next().expect("graph").into(),
                },
                NodeRef::Pattern(_) => panic!(
                    "identity rewrite accepts only the documented positive-pattern evidence operators"
                ),
                // Non-pattern children contribute no graph value; their parent reads
                // its original leaves. This is not an unimplemented graph operator.
                _ => GraphPattern::Bgp { patterns: vec![] },
            }
        },
    );
    let mut out = evidence.clone();
    let Query::Select { pattern, .. } = &mut out.query else {
        panic!("select fixture");
    };
    *pattern = query;
    out
}

/// A consistent bijective alpha-renaming, leaving caller-visible names intact.
pub fn rename(evidence: &Evidence, suffix: u32) -> (Evidence, BTreeMap<Identity, Identity>) {
    let mut correspondence = BTreeMap::new();
    for identity in evidence
        .declarations
        .keys()
        .filter(|identity| identity.hidden)
    {
        correspondence.insert(
            identity.clone(),
            Identity::variable(&Variable::hidden(format!(
                "renamed{suffix}:{}",
                identity.name
            ))),
        );
    }
    let mut out = rewrite(evidence, |triple| {
        let mut triple = triple.clone();
        for term in [&mut triple.subject, &mut triple.object] {
            if let TermPattern::Variable(variable) = term
                && let Some(new) = correspondence.get(&Identity::variable(variable))
            {
                *variable = Variable::new(new.name.clone());
            }
        }
        triple
    });
    out.declarations = evidence
        .declarations
        .iter()
        .map(|(identity, binder)| {
            (
                correspondence.get(identity).unwrap_or(identity).clone(),
                *binder,
            )
        })
        .collect();
    (out, correspondence)
}

/// One comparison row: fixture construction names never steer detectors.
pub struct Case {
    pub name: &'static str,
    pub before: Evidence,
    pub after: Evidence,
    pub correspondence: BTreeMap<Identity, Identity>,
    pub legal: bool,
    pub claim: &'static str,
}

fn case(
    name: &'static str,
    before: &Evidence,
    after: Evidence,
    legal: bool,
    claim: &'static str,
) -> Case {
    Case {
        name,
        before: before.clone(),
        after,
        correspondence: BTreeMap::new(),
        legal,
        claim,
    }
}

/// Complete named mutation/control corpus over the compared input contracts.
pub fn cases() -> Vec<Case> {
    let base = fixture();
    let mut cases = vec![case(
        "generated_union_witness",
        &base,
        base.clone(),
        true,
        "generated witnesses connect every alternative and tail",
    )];
    cases.push(case(
        "join_reassociation",
        &base,
        reassociate(&base),
        true,
        "source binding partitions survive reassociation",
    ));
    let GraphPattern::Join {
        left: alternatives,
        right: tail,
    } = body(&base.query)
    else {
        panic!("fixture join");
    };
    let GraphPattern::Join {
        left: first,
        right: last,
    } = &**tail
    else {
        panic!("fixture tail");
    };
    let (GraphPattern::Bgp { patterns: first }, GraphPattern::Bgp { patterns: last }) =
        (&**first, &**last)
    else {
        panic!("BGP pieces");
    };
    let mut merged = base.clone();
    merged.query = select(join(
        (**alternatives).clone(),
        GraphPattern::Bgp {
            patterns: first.iter().chain(last).cloned().collect(),
        },
    ));
    cases.push(case(
        "bgp_merge",
        &base,
        merged.clone(),
        true,
        "generated witnesses preserve connections when BGP fragments merge",
    ));
    cases.push(case(
        "bgp_split",
        &merged,
        base.clone(),
        true,
        "generated witnesses preserve connections when a BGP splits",
    ));
    cases.push(case(
        "distribution",
        &base,
        distribute(&base),
        true,
        "branch bags survive distribution including a repeated arm",
    ));
    let (renamed, correspondence) = rename(&base, 7);
    let mut alpha = case(
        "bijective_alpha_renaming",
        &base,
        renamed,
        true,
        "correspondence is a consistent bijection",
    );
    alpha.correspondence = correspondence;
    cases.push(alpha);
    cases.push(case(
        "repeated_normalization",
        &base,
        reassociate(&reassociate_inverse(&reassociate(&base))),
        true,
        "normalization preserves original binding relationships repeatedly",
    ));
    let (rendered, correspondence) = carrier(&base);
    let mut serialization = case(
        "serialization_reparse",
        &base,
        rendered.clone(),
        true,
        "legal carrier names retain source-site binding identity and visible schema",
    );
    serialization.correspondence = correspondence;
    cases.push(serialization);
    let mut alias_projection = rendered.clone();
    let alias = rendered
        .aliases
        .keys()
        .next()
        .expect("generated carrier alias");
    let Query::Select {
        pattern: GraphPattern::Project { variables, .. },
        ..
    } = &mut alias_projection.query
    else {
        panic!("carrier projection");
    };
    variables.push(Variable::new(alias.name.clone()));
    cases.push(case(
        "carrier_alias_explicit_output",
        &rendered,
        alias_projection,
        false,
        "a reencoded non-distinguished witness was explicitly projected by its legal alias",
    ));
    let mut caller = base.clone();
    caller.query = select(join(
        body(&base.query).clone(),
        edge(5, user("s"), user("__purrdf_hidden_0")),
    ));
    let Query::Select {
        pattern: GraphPattern::Project { variables, .. },
        ..
    } = &mut caller.query
    else {
        panic!("caller projection");
    };
    variables.push(Variable::new("__purrdf_hidden_0"));
    caller.declarations = declare_variables(&caller.query);
    let (caller_carrier, correspondence) = carrier(&caller);
    let mut lookalike = case(
        "caller_generated_name_lookalike",
        &caller,
        caller_carrier,
        true,
        "ordinary caller projection remains visible while generated aliases avoid its spelling",
    );
    lookalike.correspondence = correspondence;
    cases.push(lookalike);
    let h = Identity::variable(&Variable::hidden("h"));
    let k = Identity::variable(&Variable::hidden("k"));
    for (name, from, to, only_site, claim) in [
        (
            "capture_user",
            h.clone(),
            Identity::variable(&Variable::new("s")),
            None,
            "hidden and caller-visible identities became one",
        ),
        (
            "merged_witnesses",
            k.clone(),
            h.clone(),
            None,
            "independent existential witnesses became one",
        ),
        (
            "lost_connection",
            h.clone(),
            Identity::variable(&Variable::hidden("fresh")),
            Some(3),
            "tail uses a fresh declared witness instead of the branch witness",
        ),
        (
            "inconsistent_alpha",
            h.clone(),
            Identity::variable(&Variable::hidden("fresh")),
            Some(1),
            "only one source site was renamed",
        ),
    ] {
        let mut altered = rewrite(&base, |triple| {
            let mut triple = triple.clone();
            if only_site.is_none_or(|wanted| site(&triple).expect("provenance") == wanted) {
                for term in [&mut triple.subject, &mut triple.object] {
                    if matches!(term, TermPattern::Variable(variable) if Identity::variable(variable) == from)
                    {
                        *term = TermPattern::Variable(Variable::new(to.name.clone()));
                    }
                }
            }
            triple
        });
        if let std::collections::btree_map::Entry::Vacant(entry) = altered.declarations.entry(to) {
            let mut declaration = base.declarations[&from];
            declaration.id += 100;
            entry.insert(declaration);
        }
        cases.push(case(name, &base, altered, false, claim));
    }
    let GraphPattern::Join { left, right } = body(&base.query) else {
        panic!("fixture join");
    };
    let mut escape = base.clone();
    escape.query = select(join(
        GraphPattern::Graph {
            name: NamedNodePattern::NamedNode(
                NamedNode::new(format!("{REGION}99")).expect("valid research region IRI"),
            ),
            inner: (**left).clone().into(),
        },
        (**right).clone(),
    ));
    cases.push(case(
        "scope_escape",
        &base,
        escape,
        false,
        "path-origin witness moved into an undeclared binder region",
    ));
    let mut projected = base.clone();
    projected.query = select(join(
        GraphPattern::Project {
            inner: (**left).clone().into(),
            variables: vec![Variable::new("s")],
        },
        (**right).clone(),
    ));
    cases.push(case(
        "premature_projection",
        &base,
        projected,
        false,
        "branch witness is discarded before the tail consumes it",
    ));
    let GraphPattern::Union { arms } = &**left else {
        panic!("fixture union");
    };
    let mut lost_arm = base.clone();
    lost_arm.query = select(join(
        arms.iter()
            .take(2)
            .cloned()
            .reduce(GraphPattern::union)
            .expect("two arms"),
        (**right).clone(),
    ));
    cases.push(case(
        "repeated_arm_removal",
        &base,
        lost_arm,
        false,
        "identical UNION arms contribute independent bag multiplicities",
    ));
    let mut leak = base.clone();
    let Query::Select {
        pattern: GraphPattern::Project { variables, .. },
        ..
    } = &mut leak.query
    else {
        panic!("projection");
    };
    variables.push(Variable::hidden("h"));
    cases.push(case(
        "explicit_hidden_output",
        &base,
        leak,
        false,
        "hidden witness becomes a visible column",
    ));
    let mut schema = base.clone();
    let Query::Select {
        pattern: GraphPattern::Project { variables, .. },
        ..
    } = &mut schema.query
    else {
        panic!("projection");
    };
    variables.reverse();
    cases.push(case(
        "visible_schema_reordered",
        &base,
        schema,
        false,
        "carrier changed caller-visible column order",
    ));
    let mut aliases = base.clone();
    aliases.aliases.insert(h, "s".to_owned());
    cases.push(case(
        "carrier_alias_collision",
        &base,
        aliases,
        false,
        "generated carrier alias captures caller variable s",
    ));
    let raw = raw_blank_fixture();
    cases.push(case(
        "independent_same_spelling_blanks",
        &raw,
        raw.clone(),
        true,
        "the same opaque blank spelling in independent raw BGP arms is legal",
    ));
    let shared = shared_blank_fixture();
    cases.push(case(
        "shared_blank_two_triples_one_bgp",
        &shared,
        shared.clone(),
        true,
        "two different source sites in one BGP refer to one existential blank binder",
    ));
    let mut raw_split = shared.clone();
    raw_split.query = select(join(
        edge(1, user("s"), TermPattern::BlankNode(BlankNode::new("same"))),
        edge(2, TermPattern::BlankNode(BlankNode::new("same")), user("o")),
    ));
    cases.push(case(
        "raw_blank_positive_split",
        &shared,
        raw_split.clone(),
        true,
        "production positive Join ownership preserves a raw blank across compiler BGP fragments",
    ));
    raw_split.query = select(join(
        GraphPattern::Graph {
            name: NamedNodePattern::NamedNode(
                NamedNode::new("http://example.org/left_graph").expect("valid research graph IRI"),
            ),
            inner: edge(1, user("s"), TermPattern::BlankNode(BlankNode::new("same"))).into(),
        },
        GraphPattern::Graph {
            name: NamedNodePattern::NamedNode(
                NamedNode::new("http://example.org/right_graph").expect("valid research graph IRI"),
            ),
            inner: edge(2, TermPattern::BlankNode(BlankNode::new("same")), user("o")).into(),
        },
    ));
    cases.push(case(
        "raw_blank_source_owner_divergence",
        &shared,
        raw_split.clone(),
        false,
        "a shared source existential was moved across actual GRAPH scope boundaries",
    ));
    raw_split.declarations.insert(
        Identity::blank("same", 2),
        Binder {
            id: 12,
            category: Category::Existential,
            origin: Origin::SourceBlank,
            owner: 2,
        },
    );
    cases.push(case(
        "raw_blank_distinct_source_owners_fully_declared",
        &shared,
        raw_split.clone(),
        false,
        "valid declarations for independent source scopes cannot recover the lost source connection",
    ));
    let mut captured = shared;
    captured.declarations = raw_split.declarations.clone();
    cases.push(case(
        "raw_blank_merge_captures_independent",
        &raw_split,
        captured,
        false,
        "merging independently owned GRAPH existentials into one BGP accidentally creates a connection",
    ));
    let template = template_fixture();
    cases.push(case(
        "template_allocation_category",
        &template,
        template.clone(),
        true,
        "template blank identity has template allocation origin",
    ));
    let mut confused = template.clone();
    let identity = Identity::allocation("allocation");
    confused
        .declarations
        .get_mut(&identity)
        .expect("allocation")
        .category = Category::DatasetBlank;
    confused
        .declarations
        .get_mut(&identity)
        .expect("allocation")
        .origin = Origin::DatasetParameter;
    cases.push(case(
        "template_dataset_confusion",
        &template,
        confused,
        false,
        "a template allocation is misdeclared as a concrete dataset blank",
    ));
    let mut duplicated_template = template.clone();
    let Query::Construct {
        template: allocations,
        ..
    } = &mut duplicated_template.query
    else {
        panic!("template fixture");
    };
    allocations[1].triple.subject = TermPattern::BlankNode(BlankNode::new("allocation"));
    cases.push(case(
        "merged_template_allocations",
        &template,
        duplicated_template,
        false,
        "two independent allocation sites became one template identity",
    ));
    let concrete = dataset_fixture();
    cases.push(case(
        "concrete_prebound_dataset_blank",
        &concrete,
        concrete.clone(),
        true,
        "a VALUES blank is a concrete identity rather than a query existential",
    ));
    let mut existential = concrete.clone();
    let binder = existential
        .declarations
        .get_mut(&Identity::dataset("allocation"))
        .expect("concrete identity");
    binder.category = Category::Existential;
    binder.origin = Origin::SourceBlank;
    cases.push(case(
        "dataset_existential_confusion",
        &concrete,
        existential,
        false,
        "a concrete prebound blank was misdeclared as an existential",
    ));
    let mut origins = base.clone();
    origins.declarations.get_mut(&k).expect("witness").origin = Origin::Template;
    cases.push(case(
        "witness_template_origin",
        &base,
        origins,
        false,
        "a generated match witness cannot originate in a template allocation",
    ));
    let mut duplicated_binder = base.clone();
    duplicated_binder
        .declarations
        .get_mut(&k)
        .expect("second witness")
        .id = base.declarations[&Identity::variable(&Variable::hidden("h"))].id;
    cases.push(case(
        "duplicated_binder_id",
        &base,
        duplicated_binder,
        false,
        "distinct encoded identities were assigned one explicit binder id",
    ));
    cases
}

fn reassociate_inverse(evidence: &Evidence) -> Evidence {
    let GraphPattern::Join { left, right } = body(&evidence.query) else {
        panic!("join");
    };
    let GraphPattern::Join {
        left: first,
        right: middle,
    } = &**left
    else {
        panic!("left join");
    };
    let mut out = evidence.clone();
    out.query = select(join(
        (**first).clone(),
        join((**middle).clone(), (**right).clone()),
    ));
    out
}

fn raw_blank_fixture() -> Evidence {
    let query = select(GraphPattern::union(
        edge(1, user("s"), TermPattern::BlankNode(BlankNode::new("same"))),
        edge(2, user("s"), TermPattern::BlankNode(BlankNode::new("same"))),
    ));
    let mut declarations = declare_variables(&query);
    for owner in [1, 2] {
        declarations.insert(
            Identity::blank("same", owner),
            Binder {
                id: owner + 10,
                category: Category::Existential,
                origin: Origin::SourceBlank,
                owner,
            },
        );
    }
    Evidence {
        query,
        declarations,
        aliases: BTreeMap::new(),
    }
}

fn shared_blank_fixture() -> Evidence {
    let mut patterns = Vec::new();
    for pattern in [
        edge(1, user("s"), TermPattern::BlankNode(BlankNode::new("same"))),
        edge(2, TermPattern::BlankNode(BlankNode::new("same")), user("o")),
    ] {
        let GraphPattern::Bgp { patterns: mut next } = pattern else {
            unreachable!();
        };
        patterns.append(&mut next);
    }
    let query = select(GraphPattern::Bgp { patterns });
    let mut declarations = declare_variables(&query);
    declarations.insert(
        Identity::blank("same", 1),
        Binder {
            id: 11,
            category: Category::Existential,
            origin: Origin::SourceBlank,
            owner: 1,
        },
    );
    Evidence {
        query,
        declarations,
        aliases: BTreeMap::new(),
    }
}

fn template_fixture() -> Evidence {
    let mut evidence = fixture();
    evidence.query = Query::Construct {
        template: ["allocation", "allocation_b"]
            .into_iter()
            .enumerate()
            .map(|(index, allocation)| purrdf_sparql_algebra::QuadPattern {
                triple: TriplePattern {
                    subject: TermPattern::BlankNode(BlankNode::new(allocation)),
                    predicate: NamedNodePattern::NamedNode(
                        NamedNode::new(format!("{SITE}{}", index + 5))
                            .expect("valid research source IRI"),
                    ),
                    object: user("o"),
                },
                graph: None,
            })
            .collect(),
        pattern: body(&evidence.query).clone(),
        dataset: QueryDataset::default(),
        base_iri: None,
        version: Some(SparqlVersion::V12),
    };
    evidence.declarations.insert(
        Identity::allocation("allocation"),
        Binder {
            id: 100,
            category: Category::TemplateAllocation,
            origin: Origin::Template,
            owner: 0,
        },
    );
    evidence.declarations.insert(
        Identity::allocation("allocation_b"),
        Binder {
            id: 101,
            category: Category::TemplateAllocation,
            origin: Origin::Template,
            owner: 0,
        },
    );
    evidence
}

fn dataset_fixture() -> Evidence {
    let query = select(GraphPattern::Values {
        variables: vec![Variable::new("s"), Variable::new("o")],
        bindings: vec![vec![
            Some(GroundTerm::BlankNode(BlankNode::new("allocation"))),
            None,
        ]],
    });
    let mut declarations = declare_variables(&query);
    declarations.insert(
        Identity::dataset("allocation"),
        Binder {
            id: 100,
            category: Category::DatasetBlank,
            origin: Origin::DatasetParameter,
            owner: 0,
        },
    );
    Evidence {
        query,
        declarations,
        aliases: BTreeMap::new(),
    }
}

/// Legal raw RDF 1.2 inputs requiring recursive term-position evidence absent
/// from this bounded certificate prototype. They are refused explicitly rather
/// than silently losing the incidences contained inside a triple term.
pub fn unsupported_terms() -> Vec<(&'static str, Evidence)> {
    let query = select(edge(
        1,
        TermPattern::Triple(
            TriplePattern {
                subject: user("s"),
                predicate: NamedNodePattern::NamedNode(
                    NamedNode::new("http://example.org/nested")
                        .expect("valid research predicate IRI"),
                ),
                object: witness("inside_quote"),
            }
            .into(),
        ),
        user("o"),
    ));
    let pattern = Evidence {
        declarations: declare_variables(&query),
        query,
        aliases: BTreeMap::new(),
    };
    let mut ground = dataset_fixture();
    let Query::Select {
        pattern: GraphPattern::Project { inner, .. },
        ..
    } = &mut ground.query
    else {
        panic!("VALUES projection");
    };
    let GraphPattern::Values { bindings, .. } = &mut **inner else {
        panic!("VALUES input");
    };
    bindings[0][0] = Some(GroundTerm::Triple(
        purrdf_sparql_algebra::GroundTriple {
            subject: GroundTerm::NamedNode(
                NamedNode::new("http://example.org/s").expect("valid research subject IRI"),
            ),
            predicate: NamedNode::new("http://example.org/nested")
                .expect("valid research predicate IRI"),
            object: GroundTerm::BlankNode(BlankNode::new("allocation")),
        }
        .into(),
    ));
    vec![
        ("nested_hidden_pattern_identity", pattern),
        ("nested_blank_values_identity", ground),
    ]
}

/// Before/after deletion of an inner projection that separates an ordinary user
/// binding from a same-spelling sibling. The predecessor is outside the bounded
/// contract's proof domain, so it cannot supply a certificate for the deletion.
pub fn projection_boundary() -> (Evidence, Evidence) {
    let inner = edge(1, user("s"), user("x"));
    let sibling = edge(2, user("x"), user("o"));
    let query = select(join(
        GraphPattern::Project {
            inner: inner.clone().into(),
            variables: vec![Variable::new("s")],
        },
        sibling.clone(),
    ));
    let before = Evidence {
        declarations: declare_variables(&query),
        query,
        aliases: BTreeMap::new(),
    };
    let mut after = before.clone();
    after.query = select(join(inner, sibling));
    (before, after)
}

/// Variable graph names add incidences absent from the flat source-triple model.
/// Both the original and altered graph binding explicitly require richer evidence.
pub fn graph_name_boundary() -> (Evidence, Evidence) {
    let make = |name| {
        let query = select(join(
            GraphPattern::Graph {
                name: NamedNodePattern::Variable(Variable::new(name)),
                inner: edge(1, user("s"), user("o")).into(),
            },
            edge(2, user("g"), user("o")),
        ));
        Evidence {
            declarations: declare_variables(&query),
            query,
            aliases: BTreeMap::new(),
        }
    };
    (make("g"), make("x"))
}

/// Exact source copies across independent GRAPH owners need more than the
/// minimum source-site owner key. Their merger changes the raw binding partition
/// even when every copied source field is coherent, so preparation is refused.
pub fn raw_owner_boundary() -> (Evidence, Evidence) {
    let triple = edge(1, user("s"), TermPattern::BlankNode(BlankNode::new("same")));
    let graph = |inner: GraphPattern| GraphPattern::Graph {
        name: NamedNodePattern::NamedNode(
            NamedNode::new("http://example.org/g").expect("valid research graph IRI"),
        ),
        inner: inner.into(),
    };
    let query = select(join(graph(triple.clone()), graph(triple.clone())));
    let mut declarations = declare_variables(&query);
    declarations.insert(
        Identity::blank("same", 1),
        Binder {
            id: 11,
            category: Category::Existential,
            origin: Origin::SourceBlank,
            owner: 1,
        },
    );
    let before = Evidence {
        query,
        declarations,
        aliases: BTreeMap::new(),
    };
    let mut after = before.clone();
    after.query = select(graph(join(triple.clone(), triple)));
    (before, after)
}

/// A repeated source identifier cannot describe different original fields.
/// This is provenance admission, not evidence that an optimizer was incorrect.
pub fn contradictory_source_boundary() -> Evidence {
    let query = select(join(
        edge(1, user("s"), user("x")),
        edge(1, user("x"), user("o")),
    ));
    Evidence {
        declarations: declare_variables(&query),
        query,
        aliases: BTreeMap::new(),
    }
}

/// Template graph positions require their own output-binding incidence, absent
/// from this category-only CONSTRUCT experiment. Both names are bound inputs.
pub fn template_graph_boundary() -> (Evidence, Evidence) {
    let mut before = template_fixture();
    let Query::Construct { template, .. } = &mut before.query else {
        unreachable!();
    };
    template[0].graph = Some(NamedNodePattern::Variable(Variable::new("s")));
    let mut after = before.clone();
    let Query::Construct { template, .. } = &mut after.query else {
        unreachable!();
    };
    template[0].graph = Some(NamedNodePattern::Variable(Variable::new("o")));
    (before, after)
}

/// A variable predicate cannot provide the experiment's stable named source
/// site, so preparation already refuses it rather than erasing that incidence.
pub fn template_predicate_boundary() -> (Evidence, Evidence) {
    let mut before = template_fixture();
    let Query::Construct { template, .. } = &mut before.query else {
        unreachable!();
    };
    template[0].triple.predicate = NamedNodePattern::Variable(Variable::new("s"));
    let mut after = before.clone();
    let Query::Construct { template, .. } = &mut after.query else {
        unreachable!();
    };
    template[0].triple.predicate = NamedNodePattern::Variable(Variable::new("o"));
    (before, after)
}

/// DESCRIBE heads observe binding targets outside the SELECT/CONSTRUCT model.
pub fn describe_boundary() -> (Evidence, Evidence) {
    let query = Query::Describe {
        pattern: edge(1, user("s"), user("o")),
        targets: vec![NamedNodePattern::Variable(Variable::new("s"))],
        dataset: QueryDataset::default(),
        base_iri: None,
        version: Some(SparqlVersion::V12),
    };
    let before = Evidence {
        declarations: declare_variables(&query),
        query,
        aliases: BTreeMap::new(),
    };
    let mut after = before.clone();
    let Query::Describe { targets, .. } = &mut after.query else {
        unreachable!();
    };
    targets[0] = NamedNodePattern::Variable(Variable::new("o"));
    (before, after)
}

/// ASK observer semantics are outside the SELECT/CONSTRUCT certificate.
pub fn ask_boundary() -> Evidence {
    let query = Query::Ask {
        pattern: edge(1, user("s"), user("o")),
        dataset: QueryDataset::default(),
        base_iri: None,
        version: Some(SparqlVersion::V12),
    };
    Evidence {
        declarations: declare_variables(&query),
        query,
        aliases: BTreeMap::new(),
    }
}

/// Unavailable-proof inputs, separated from supported intentional hazard counts.
pub fn proof_boundaries() -> Vec<(&'static str, Evidence)> {
    let mut inputs = unsupported_terms();
    inputs.push(("ordinary_inner_projection_scope", projection_boundary().0));
    let (before, after) = graph_name_boundary();
    inputs.push(("variable_graph_name_connection", before));
    inputs.push(("variable_graph_name_changed_connection", after));
    inputs.push(("ambiguous_copied_raw_source_owner", raw_owner_boundary().0));
    inputs.push((
        "contradictory_repeated_source_fields",
        contradictory_source_boundary(),
    ));
    inputs.push((
        "template_variable_graph_before",
        template_graph_boundary().0,
    ));
    inputs.push(("template_variable_graph_after", template_graph_boundary().1));
    inputs.push((
        "template_variable_predicate",
        template_predicate_boundary().0,
    ));
    inputs.push(("describe_observer_target_before", describe_boundary().0));
    inputs.push(("describe_observer_target_after", describe_boundary().1));
    inputs.push(("ask_observer_form", ask_boundary()));
    inputs
}

/// Measured shapes: a small query, rich branch/width cases and deep borrowed walks.
pub fn measured_inputs() -> Vec<(String, Evidence)> {
    let mut inputs = vec![("representative".to_owned(), fixture())];
    let wide = (1..=256)
        .map(|site| edge(site, user("s"), witness("h")))
        .reduce(GraphPattern::union)
        .expect("wide union");
    let query = select(join(wide, edge(257, witness("h"), user("o"))));
    inputs.push((
        "branches_256".to_owned(),
        Evidence {
            declarations: declare_variables(&query),
            query,
            aliases: BTreeMap::new(),
        },
    ));
    let mut patterns = Vec::new();
    for source in 1..=1024 {
        let GraphPattern::Bgp {
            patterns: mut single,
        } = edge(source, user("s"), user("o"))
        else {
            unreachable!();
        };
        patterns.append(&mut single);
    }
    let query = select(GraphPattern::Bgp { patterns });
    inputs.push((
        "width_1024".to_owned(),
        Evidence {
            declarations: declare_variables(&query),
            query,
            aliases: BTreeMap::new(),
        },
    ));
    for depth in [64, 512, 100_000] {
        let mut pattern = edge(1, user("s"), user("o"));
        for _ in 0..depth {
            pattern = GraphPattern::Graph {
                name: NamedNodePattern::NamedNode(
                    NamedNode::new("http://example.org/g").expect("valid research graph IRI"),
                ),
                inner: pattern.into(),
            };
        }
        let query = select(pattern);
        inputs.push((
            format!("deep_{depth}"),
            Evidence {
                declarations: declare_variables(&query),
                query,
                aliases: BTreeMap::new(),
            },
        ));
    }
    inputs
}

/// Measured layouts, rather than claims about hypothetical representation sizes.
pub fn layouts() -> [(&'static str, usize); 6] {
    [
        ("Variable", size_of::<Variable>()),
        ("Binder", size_of::<Binder>()),
        ("Identity", size_of::<Identity>()),
        ("Atom", size_of::<Atom>()),
        ("Contract", size_of::<Contract>()),
        ("Evidence", size_of::<Evidence>()),
    ]
}
