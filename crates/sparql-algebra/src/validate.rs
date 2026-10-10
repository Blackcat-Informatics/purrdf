// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed structural admission for compiler-built query algebra.

use crate::parser::table::ScopeTable;
use purrdf_lex::allocation::{Admission, Memory, Resident, StorageError};
use std::collections::BTreeSet;

use crate::walk::NodeRef;
use crate::{
    AggregateExpression, Expression, Function, GraphPattern, GroundTerm, Literal, NamedNodePattern,
    ParseError, PropertyPathExpression, Query, Result, TermPattern, TriplePattern, Variable,
};

/// A term position that may hold a triple term: the one shape the nesting count
/// walks, shared by pattern terms and `VALUES` cells.
trait TripleTermSlot: Sized {
    /// The subject and object of the triple term this is, or `None` for a
    /// non-triple term.
    fn subject_and_object(&self) -> Option<(&Self, &Self)>;
}

/// The slot implementation for each term kind whose `Triple` variant holds a
/// triple with `subject` and `object` of that same kind.
macro_rules! triple_term_slot {
    ($($term:ty),+) => {$(
        impl TripleTermSlot for $term {
            fn subject_and_object(&self) -> Option<(&Self, &Self)> {
                match self {
                    Self::Triple(triple) => Some((&triple.subject, &triple.object)),
                    _ => None,
                }
            }
        }
    )+};
}

triple_term_slot!(TermPattern, GroundTerm);

/// How many triple terms `root`'s longest chain holds, the outermost included.
///
/// The single nesting count for every term kind. Counted iteratively, following
/// the object position in a loop and parking a subject only when it is itself a
/// triple term, so it needs no more stack however deep the term nests and does
/// not allocate unless a subject nests.
fn triple_term_nesting<T: TripleTermSlot>(root: &T) -> usize {
    let mut admission = Resident;
    let mut memory = Memory::new(&mut admission);
    triple_term_nesting_with_memory(root, &mut memory).expect("resident triple nesting census")
}

fn triple_term_nesting_with_memory<T: TripleTermSlot, S: Admission + ?Sized>(
    root: &T,
    memory: &mut Memory<'_, S>,
) -> std::result::Result<usize, StorageError> {
    let mut deepest = 0_usize;
    let mut pending = purrdf_lex::walk::WorkList::<(&T, usize), 8>::new();
    let mut next = Some((root, 0_usize));
    while let Some((term, above)) = next.take().or_else(|| pending.pop()) {
        if let Some((subject, object)) = term.subject_and_object() {
            let depth = above.checked_add(1).ok_or(StorageError::SizeOverflow)?;
            deepest = deepest.max(depth);
            if subject.subject_and_object().is_some() {
                pending.try_push_admitted((subject, depth), memory)?;
            }
            next = Some((object, depth));
        }
    }
    pending.release_admitted(memory)?;
    Ok(deepest)
}

impl TermPattern {
    /// How many triple terms this term's longest chain holds, the outermost included:
    /// `0` for an IRI, blank node, literal or variable, `1` for `<<( ?s ?p ?o )>>`, `2`
    /// for `<<( ?s ?p <<( ?s ?p ?o )>> )>>`.
    ///
    /// Counted iteratively, so it needs no more stack however deep the term nests, and
    /// without allocating unless a triple term's subject is itself one.
    #[must_use]
    pub fn triple_term_nesting(&self) -> usize {
        triple_term_nesting(self)
    }

    /// Count nesting through the same native body under concrete work-list admission.
    /// # Errors
    /// Returns checked depth overflow, admission refusal or native allocator failure.
    pub fn triple_term_nesting_with_memory<S: Admission + ?Sized>(
        &self,
        memory: &mut Memory<'_, S>,
    ) -> std::result::Result<usize, StorageError> {
        triple_term_nesting_with_memory(self, memory)
    }

    /// Call `visit` on every variable this term mentions, a quoted triple term's own
    /// positions included — subject, predicate and object, at every depth. A
    /// variable mentioned twice is visited twice.
    ///
    /// The one variable walk over a term pattern every analysis shares. It keeps its
    /// own work list, so a term nested to any depth needs no more machine stack, and
    /// a term that is not a triple term is answered without allocating.
    pub fn for_each_variable<'a>(&'a self, mut visit: impl FnMut(&'a Variable)) {
        let mut admission = Resident;
        let mut memory = Memory::new(&mut admission);
        self.for_each_variable_with_memory(&mut memory, |variable, _| {
            visit(variable);
            Ok::<(), core::convert::Infallible>(())
        })
        .expect("resident term-variable walk allocation failed");
    }

    /// Visit the same variable occurrences under native fallible work-list admission.
    /// Repeated occurrences remain repeated; each quoted predicate is visited before
    /// its subject and object, exactly as in the resident variable walk.
    ///
    /// # Errors
    /// Returns the original visitor error or physical work-list failure.
    pub fn for_each_variable_with_memory<'a, S, E>(
        &'a self,
        memory: &mut Memory<'_, S>,
        mut visit: impl FnMut(&'a Variable, &mut Memory<'_, S>) -> std::result::Result<(), E>,
    ) -> std::result::Result<(), crate::walk::MutationError<E>>
    where
        S: Admission + ?Sized,
    {
        use crate::walk::MutationError;
        let mut pending: purrdf_lex::walk::WorkList<&'a Self, 8> =
            purrdf_lex::walk::WorkList::new();
        pending
            .try_push_admitted(self, memory)
            .map_err(MutationError::Storage)?;
        while let Some(term) = pending.pop() {
            match term {
                Self::Variable(variable) => {
                    visit(variable, memory).map_err(MutationError::Visitor)?;
                }
                Self::Triple(triple) => {
                    if let NamedNodePattern::Variable(variable) = &triple.predicate {
                        visit(variable, memory).map_err(MutationError::Visitor)?;
                    }
                    pending
                        .try_push_admitted(&triple.object, memory)
                        .map_err(MutationError::Storage)?;
                    pending
                        .try_push_admitted(&triple.subject, memory)
                        .map_err(MutationError::Storage)?;
                }
                Self::NamedNode(_) | Self::BlankNode(_) | Self::Literal(_) => {}
            }
        }
        pending
            .release_admitted(memory)
            .map_err(MutationError::Storage)
    }

    /// Add every variable this term mentions to `out`, a quoted triple term's own
    /// positions included (see [`Self::for_each_variable`]).
    pub fn collect_variables(&self, out: &mut impl Extend<Variable>) {
        self.for_each_variable(|variable| out.extend([variable.clone()]));
    }

    /// Add the name of every variable this term mentions to `out`, a quoted triple
    /// term's own positions included (see [`Self::for_each_variable`]).
    pub fn collect_variable_names(&self, out: &mut BTreeSet<String>) {
        self.for_each_variable(|variable| {
            out.insert(variable.as_str().to_owned());
        });
    }
}

impl TriplePattern {
    /// Add the name of every variable this triple pattern mentions to `out`: its
    /// subject's and object's (quoted triple terms included) and a variable
    /// predicate.
    pub fn collect_variable_names(&self, out: &mut BTreeSet<String>) {
        self.subject.collect_variable_names(out);
        if let NamedNodePattern::Variable(variable) = &self.predicate {
            out.insert(variable.as_str().to_owned());
        }
        self.object.collect_variable_names(out);
    }
}

impl GroundTerm {
    /// How many triple terms this `VALUES` cell's longest chain holds, the outermost
    /// included; see [`TermPattern::triple_term_nesting`].
    #[must_use]
    pub fn triple_term_nesting(&self) -> usize {
        triple_term_nesting(self)
    }

    /// Count nesting through the same native body under concrete work-list admission.
    /// # Errors
    /// Returns checked depth overflow, admission refusal or native allocator failure.
    pub fn triple_term_nesting_with_memory<S: Admission + ?Sized>(
        &self,
        memory: &mut Memory<'_, S>,
    ) -> std::result::Result<usize, StorageError> {
        triple_term_nesting_with_memory(self, memory)
    }
}

impl GraphPattern {
    /// Check the reserved non-distinguished identity contract without changing
    /// the admission rules for ordinary raw-algebra names or terms.
    ///
    /// # Errors
    /// Refuses explicit hidden projection, grouping, output or expression references.
    pub fn validate_hidden_variables(&self) -> Result<()> {
        crate::scope::validate_pattern(self).map_err(ParseError::from)
    }
}

/// Structural validation refusal, preserving the original semantic cause.
#[derive(Debug)]
pub enum ValidationError {
    /// The original syntax message, constructed through original native storage.
    Invalid(String),
    /// The original observer violation.
    Scope(crate::scope::ScopeError),
    /// Checked native storage refusal.
    Storage(StorageError),
}
purrdf_lex::variant_from!(ValidationError { Storage(StorageError) });
impl From<crate::scope::ScopeValidationError> for ValidationError {
    fn from(error: crate::scope::ScopeValidationError) -> Self {
        match error {
            crate::scope::ScopeValidationError::Scope(error) => Self::Scope(error),
            crate::scope::ScopeValidationError::Storage(error) => Self::Storage(error),
        }
    }
}
purrdf_lex::variant_from!(ValidationError { Scope(crate::scope::ScopeError) });
impl core::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Invalid(message) => f.write_str(message),
            Self::Scope(error) => error.fmt(f),
            Self::Storage(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for ValidationError {}

fn resident_validation<T>(
    body: impl FnOnce(&mut Memory<'_, Resident>) -> std::result::Result<T, ValidationError>,
) -> Result<T> {
    match body(&mut Memory::new(&mut Resident)) {
        Ok(value) => Ok(value),
        Err(ValidationError::Invalid(message)) => Err(ParseError::syntax(message, 0)),
        Err(ValidationError::Scope(error)) => Err(ParseError::from(error)),
        Err(ValidationError::Storage(error)) => panic!("resident structural validation: {error}"),
    }
}

impl Query {
    /// Check hidden observer identities without re-admitting ordinary terms.
    /// # Errors
    /// Refuses explicit observation of a hidden match witness.
    pub fn validate_hidden_variables(&self) -> Result<()> {
        crate::scope::validate_query(self).map_err(ParseError::from)
    }

    /// Check the original structural query invariants.
    /// # Errors
    /// Returns the original syntax/observer refusal.
    pub fn validate(&self) -> Result<()> {
        resident_validation(|memory| self.validate_with_memory(memory))
    }

    /// Check ordinary preparation's original structural invariants.
    /// # Errors
    /// Returns structural or contextual-application refusal.
    pub fn validate_ordinary(&self) -> Result<()> {
        resident_validation(|memory| self.validate_ordinary_with_memory(memory))
    }

    /// Validate through the original native account, including temporary tables.
    /// # Errors
    /// Returns original structural/observer errors or physical storage refusal.
    pub fn validate_with_memory<S: Admission + ?Sized>(
        &self,
        memory: &mut Memory<'_, S>,
    ) -> std::result::Result<(), ValidationError> {
        let mut calls = ScopeTable::default();
        self.walk_with_memory::<true, S>(&mut calls, memory)?;
        calls.release(memory)?;
        Ok(())
    }

    /// The ordinary-entry variant of the same native validation body.
    /// # Errors
    /// Returns original semantic refusal or native working-storage failure.
    pub fn validate_ordinary_with_memory<S: Admission + ?Sized>(
        &self,
        memory: &mut Memory<'_, S>,
    ) -> std::result::Result<(), ValidationError> {
        let mut calls = ScopeTable::default();
        self.walk_with_memory::<false, S>(&mut calls, memory)?;
        calls.release(memory)?;
        Ok(())
    }

    /// Sorted, de-duplicated extension-function IRIs; custom aggregates are separate.
    /// # Errors
    /// Returns the same structural diagnostics as validation.
    pub fn custom_function_calls(&self) -> Result<BTreeSet<String>> {
        resident_validation(|memory| self.custom_function_calls_with_memory(memory))
            .map(|calls| calls.into_iter().collect())
    }

    /// The same extension census with native output and working-storage admission.
    /// Returned strings remain in the caller's original account.
    /// # Errors
    /// Returns original validation or physical storage refusal.
    pub fn custom_function_calls_with_memory<S: Admission + ?Sized>(
        &self,
        memory: &mut Memory<'_, S>,
    ) -> std::result::Result<Vec<String>, ValidationError> {
        let mut calls = ScopeTable::default();
        self.walk_with_memory::<true, S>(&mut calls, memory)?;
        let mut output = Vec::new();
        for (name, ()) in calls.iter() {
            let text = memory.string(name)?;
            memory.push(&mut output, text)?;
        }
        calls.release(memory)?;
        output.sort_unstable();
        Ok(output)
    }

    fn walk_with_memory<'a, const APPLICATION: bool, S: Admission + ?Sized>(
        &'a self,
        calls: &mut ScopeTable<&'a str, ()>,
        memory: &mut Memory<'_, S>,
    ) -> std::result::Result<(), ValidationError> {
        let (pattern, dataset, base) = match self {
            Self::Select {
                pattern,
                dataset,
                base_iri,
                ..
            }
            | Self::Ask {
                pattern,
                dataset,
                base_iri,
                ..
            }
            | Self::Construct {
                pattern,
                dataset,
                base_iri,
                ..
            }
            | Self::Describe {
                pattern,
                dataset,
                base_iri,
                ..
            } => (pattern, dataset, base_iri),
        };
        for name in dataset.default.iter().chain(&dataset.named) {
            iri(name.as_str(), memory)?;
        }
        if let Some(base) = base {
            let before = memory.admitted_bytes();
            match purrdf_iri::BaseIri::parse_with_memory(base.as_str(), memory) {
                Ok(parsed) => {
                    drop(parsed);
                    memory.release_bytes(memory.admitted_bytes() - before)?;
                }
                Err(purrdf_iri::IriReadError::Storage(error)) => return Err(error.into()),
                Err(purrdf_iri::IriReadError::Lexical(error)) => {
                    let temporary_bytes = memory.admitted_bytes() - before;
                    let failure = invalid(&error, memory);
                    drop(error);
                    memory.release_bytes(temporary_bytes)?;
                    return Err(failure);
                }
            }
        }
        crate::scope::validate_query_head_with_memory(self, memory)
            .map_err(ValidationError::from)?;
        let mut stack = Vec::new();
        memory.push(&mut stack, NodeRef::Pattern(pattern))?;
        match self {
            Self::Construct { template, .. } => {
                for quad in template {
                    if let Some(graph) = &quad.graph {
                        named(graph, memory)?;
                    }
                    memory.push(&mut stack, NodeRef::Triple(&quad.triple))?;
                }
            }
            Self::Describe { targets, .. } => {
                for target in targets {
                    named(target, memory)?;
                }
            }
            Self::Select { .. } | Self::Ask { .. } => {}
        }
        visit_nodes_with_memory(
            stack,
            |node, memory| check::<APPLICATION, S>(node, calls, memory),
            memory,
        )
    }
}

/// The original root order and numbered observer traversal.
fn visit_nodes_with_memory<'a, S: Admission + ?Sized>(
    stack: Vec<NodeRef<'a>>,
    mut check: impl FnMut(NodeRef<'a>, &mut Memory<'_, S>) -> std::result::Result<(), ValidationError>,
    memory: &mut Memory<'_, S>,
) -> std::result::Result<(), ValidationError> {
    for root in stack.iter().rev().copied() {
        crate::scope::walk_nodes_with_memory(
            root,
            |node, index, memory| {
                crate::scope::check_node(node, crate::scope::ScopeSite::pattern(index))?;
                check(node, memory)
            },
            memory,
        )?;
    }
    memory.release_vec(stack)?;
    Ok(())
}

pub(crate) use crate::scope::for_each_quad_variable;

/// The one output rule shared by query admission and update carriers.
pub(crate) fn check_quad_output(quad: &crate::QuadPattern) -> Result<()> {
    crate::scope::validate_quad(quad).map_err(ParseError::from)
}

fn invalid<S: Admission + ?Sized>(
    message: impl core::fmt::Display,
    memory: &mut Memory<'_, S>,
) -> ValidationError {
    match memory.format(&message) {
        Ok(message) => ValidationError::Invalid(message),
        Err(error) => ValidationError::Storage(error),
    }
}

/// Accept `value` iff it is a well-formed **absolute** IRI.
///
/// `purrdf_iri::is_absolute` rather than `parse(..)?.has_scheme()`: the two run the
/// same grammar and return the same errors, but `parse` owns a copy of `value` so
/// its component accessors can hand back slices, and this reads one bit and drops
/// it. That copy is a heap `String` per IRI in the query, charged on every
/// `Query::validate` — which a governed SHACL change path used to reach once per
/// focus node. The swap is an ALLOCATION change and not a validation one: what is
/// accepted and what is rejected here is unchanged, which
/// `tests::absolute_iris_are_still_accepted_and_relative_ones_still_refused` pins
/// from both sides.
fn iri<S: Admission + ?Sized>(
    value: &str,
    memory: &mut Memory<'_, S>,
) -> std::result::Result<(), ValidationError> {
    let absolute = match purrdf_iri::is_absolute_with_memory(value, memory) {
        Ok(value) => value,
        Err(purrdf_iri::IriReadError::Storage(error)) => return Err(error.into()),
        Err(purrdf_iri::IriReadError::Lexical(error)) => {
            let bytes = error.owned_text_bytes();
            let failure = invalid(&error, memory);
            drop(error);
            memory.release_bytes(bytes)?;
            return Err(failure);
        }
    };
    if absolute {
        Ok(())
    } else {
        Err(invalid("relative IRI in query algebra", memory))
    }
}

fn variable<S: Admission + ?Sized>(
    value: &Variable,
    memory: &mut Memory<'_, S>,
) -> std::result::Result<(), ValidationError> {
    // `VARNAME` is position-dependent, so this is a whole-string test and not a
    // per-character one: a name may CONTINUE with a combining mark but may not
    // BEGIN with one, and no position admits `'-'`.
    if !value.is_hidden() && !crate::lexer::is_varname(value.as_str()) {
        return Err(invalid("invalid query variable name", memory));
    }
    Ok(())
}

fn distinct_variables<'a, S: Admission + ?Sized>(
    values: impl IntoIterator<Item = &'a Variable>,
    memory: &mut Memory<'_, S>,
) -> std::result::Result<(), ValidationError> {
    let mut seen = ScopeTable::default();
    for value in values {
        variable(value, memory)?;
        if seen.insert(value, (), memory)?.is_some() {
            return Err(invalid(
                "duplicate output variable in query algebra",
                memory,
            ));
        }
    }
    seen.release(memory)?;
    Ok(())
}

fn named<S: Admission + ?Sized>(
    value: &NamedNodePattern,
    memory: &mut Memory<'_, S>,
) -> std::result::Result<(), ValidationError> {
    match value {
        NamedNodePattern::NamedNode(n) => iri(n.as_str(), memory),
        NamedNodePattern::Variable(v) => variable(v, memory),
    }
}

fn literal<S: Admission + ?Sized>(
    value: &Literal,
    memory: &mut Memory<'_, S>,
) -> std::result::Result<(), ValidationError> {
    iri(value.datatype().as_str(), memory)?;
    match value.language() {
        Some(_) => {
            let expected = purrdf_iri::vocab::language_datatype_iri(value.direction().is_some());
            if value.datatype().as_str() != expected {
                return Err(invalid(
                    "literal datatype disagrees with its language and direction",
                    memory,
                ));
            }
        }
        None if value.direction().is_some()
            || matches!(
                value.datatype().as_str(),
                crate::ast::RDF_LANG_STRING | crate::ast::RDF_DIR_LANG_STRING
            ) =>
        {
            return Err(invalid(
                "a language-string datatype or direction requires a language tag",
                memory,
            ));
        }
        None => {}
    }
    if value
        .language()
        .is_some_and(|tag| !crate::parser::is_langtag(tag))
    {
        return Err(invalid("invalid language tag in query algebra", memory));
    }
    Ok(())
}

/// The checks of `node` itself; its children are checked when they are reached.
/// Every [`Function::Custom`] IRI `node` calls is recorded in `calls`.
fn check<'a, const APPLICATION: bool, S: Admission + ?Sized>(
    node: NodeRef<'a>,
    calls: &mut ScopeTable<&'a str, ()>,
    memory: &mut Memory<'_, S>,
) -> std::result::Result<(), ValidationError> {
    match node {
        NodeRef::Pattern(pattern) => check_pattern::<APPLICATION, S>(pattern, memory),
        NodeRef::Expr(expr) => check_expression(expr, calls, memory),
        NodeRef::Path(path) => check_path(path, memory),
        NodeRef::Triple(triple) => named(&triple.predicate, memory),
        NodeRef::Term(term) => match term {
            TermPattern::NamedNode(n) => iri(n.as_str(), memory),
            TermPattern::Variable(v) => variable(v, memory),
            TermPattern::Literal(l) => literal(l, memory),
            TermPattern::Triple(_) | TermPattern::BlankNode(_) => Ok(()),
        },
        NodeRef::Ground(term) => match term {
            GroundTerm::NamedNode(n) => iri(n.as_str(), memory),
            GroundTerm::Literal(l) => literal(l, memory),
            GroundTerm::BlankNode(_) => Ok(()),
            GroundTerm::Triple(t) => {
                if matches!(t.subject, GroundTerm::Literal(_) | GroundTerm::Triple(_)) {
                    return Err(invalid(
                        "a ground triple term requires an IRI or blank subject",
                        memory,
                    ));
                }
                iri(t.predicate.as_str(), memory)
            }
        },
        NodeRef::Order(_) => Ok(()),
        NodeRef::Aggregate(aggregate) => check_aggregate(aggregate, memory),
    }
}

fn check_aggregate<S: Admission + ?Sized>(
    value: &AggregateExpression,
    memory: &mut Memory<'_, S>,
) -> std::result::Result<(), ValidationError> {
    if let crate::AggregateFunction::Custom(name) = value.function() {
        iri(name.as_str(), memory)?;
    }
    for (_, value) in value.scalarvals() {
        literal(value, memory)?;
    }
    Ok(())
}

fn check_pattern<const APPLICATION: bool, S: Admission + ?Sized>(
    pattern: &GraphPattern,
    memory: &mut Memory<'_, S>,
) -> std::result::Result<(), ValidationError> {
    use GraphPattern as G;
    match pattern {
        G::Apply {
            left,
            right,
            policy,
        } => {
            if !APPLICATION {
                return Err(invalid(
                    "contextual application requires typed contextual preparation",
                    memory,
                ));
            }
            if policy.dataset_required
                && (policy.row_pipeline
                    || policy.reduced_adjacent
                    || policy.group_domain.is_some()
                    || !policy.inputs.is_empty()
                    || policy.optional.is_some()
                    || !matches!(&**left, G::Bgp { patterns } if patterns.is_empty())
                    || !matches!(&**right, G::Graph { .. }))
            {
                return Err(invalid(
                    "dataset-required application requires an uncorrelated Graph operand and empty driver",
                    memory,
                ));
            }
            if policy.reduced_adjacent
                && (policy.group_domain.is_some()
                    || policy.row_pipeline
                    || !policy.inputs.is_empty()
                    || policy.optional.is_some()
                    || !matches!(&**left, G::Bgp {patterns} if patterns.is_empty())
                    || !matches!(&**right, G::Reduced { .. }))
            {
                return Err(invalid(
                    "adjacent reduction requires an uncorrelated Reduced operand and empty driver",
                    memory,
                ));
            }
            if policy.row_pipeline {
                if policy.group_domain.is_some()
                    || policy.reduced_adjacent
                    || policy.optional.is_some()
                {
                    return Err(invalid(
                        "scalar continuation cannot carry group, reduction or retry policies",
                        memory,
                    ));
                }
                let mut row = &**right;
                loop {
                    match row {
                        G::Extend { inner, .. }
                        | G::Filter { inner, .. }
                        | G::Project { inner, .. } => row = inner,
                        G::Bgp { patterns } if patterns.is_empty() => break,
                        _ => {
                            return Err(invalid(
                                "scalar continuation requires a zero-or-one-row scalar operand",
                                memory,
                            ));
                        }
                    }
                }
            }
            if let Some(domain) = &policy.group_domain {
                if !matches!(&**left, G::Bgp { patterns } if patterns.is_empty())
                    || !matches!(&**right, G::Group { .. })
                    || policy.row_pipeline
                    || policy.reduced_adjacent
                    || !policy.inputs.is_empty()
                    || policy.optional.is_some()
                {
                    return Err(invalid(
                        "group mapping domain requires an uncorrelated Group operand and empty driver",
                        memory,
                    ));
                }
                let mut seen = ScopeTable::default();
                for name in domain {
                    variable(name, memory)?;
                    if seen.insert(name, (), memory)?.is_some() {
                        return Err(invalid("group mapping domain repeats a column", memory));
                    }
                }
                seen.release(memory)?;
            }
            let mut inputs = ScopeTable::default();
            for (input, driver) in &policy.inputs {
                variable(input, memory)?;
                variable(driver, memory)?;
                if inputs.insert(input, (), memory)?.is_some() {
                    return Err(invalid("application input is declared twice", memory));
                }
            }
            if let Some(optional) = &policy.optional {
                variable(&optional.forget_marker, memory)?;
                if inputs.get(&&optional.forget_marker).is_some() {
                    return Err(invalid(
                        "optional visibility marker collides with an application input",
                        memory,
                    ));
                }
                let mut retry = ScopeTable::default();
                for (input, driver) in &optional.retry_inputs {
                    variable(input, memory)?;
                    variable(driver, memory)?;
                    if inputs.get(&input).is_none() {
                        return Err(invalid(
                            "retry refers to an undeclared application input",
                            memory,
                        ));
                    }
                    if retry.insert(input, (), memory)?.is_some() {
                        return Err(invalid("retry application input is declared twice", memory));
                    }
                }
                retry.release(memory)?;
            }
            inputs.release(memory)?;
        }
        G::Bgp { .. }
        | G::Path { .. }
        | G::Join { .. }
        | G::Lateral { .. }
        | G::Minus { .. }
        | G::Union { .. }
        | G::LeftJoin { .. }
        | G::Filter { .. }
        | G::OrderBy { .. }
        | G::Distinct { .. }
        | G::Reduced { .. }
        | G::Slice { .. } => {}
        G::Graph { name, .. } | G::Service { name, .. } => named(name, memory)?,
        G::Extend {
            variable: target, ..
        } => variable(target, memory)?,
        G::Values {
            variables,
            bindings,
        } => {
            distinct_variables(variables, memory)?;
            if bindings.iter().any(|row| row.len() != variables.len()) {
                return Err(invalid(
                    "VALUES row width differs from its variables",
                    memory,
                ));
            }
        }
        G::Project { variables, .. } => {
            // Repeated projection variables are normalized by the result schema.
            for value in variables {
                variable(value, memory)?;
            }
        }
        G::Group {
            variables,
            aggregates,
            ..
        } => {
            let mut outputs = ScopeTable::default();
            for value in variables {
                variable(value, memory)?;
                outputs.insert(value, (), memory)?;
            }
            for (target, _) in aggregates {
                variable(target, memory)?;
                if outputs.insert(target, (), memory)?.is_some() {
                    return Err(invalid(
                        "aggregate output collides with another group output",
                        memory,
                    ));
                }
            }
            outputs.release(memory)?;
        }
        G::PropertyFunction(call) => iri(&call.iri, memory)?,
        G::Unfold {
            element, companion, ..
        } => distinct_variables(std::iter::once(element).chain(companion), memory)?,
    }
    Ok(())
}

fn check_expression<'a, S: Admission + ?Sized>(
    expr: &'a Expression,
    calls: &mut ScopeTable<&'a str, ()>,
    memory: &mut Memory<'_, S>,
) -> std::result::Result<(), ValidationError> {
    use Expression as E;
    match expr {
        E::NamedNode(n) => iri(n.as_str(), memory)?,
        E::Literal(l) => literal(l, memory)?,
        E::Variable(v) | E::Bound(v) => variable(v, memory)?,
        E::FunctionCall(function, args) => match function {
            Function::Custom(n) => {
                iri(n.as_str(), memory)?;
                calls.insert(n.as_str(), (), memory)?;
            }
            Function::Cdt(call) => {
                if crate::CdtFn::from_iri(&call.iri) != Some(call.fn_kind)
                    || !call.fn_kind.arity().admits(args.len())
                {
                    return Err(invalid(
                        "invalid composite datatype function identity or arity",
                        memory,
                    ));
                }
            }
            Function::Purrdf(call) => {
                iri(&call.iri, memory)?;
                if !call.iri.ends_with(call.local_name()) {
                    return Err(invalid(
                        "extension function IRI disagrees with its kind",
                        memory,
                    ));
                }
            }
            Function::Adjust if args.len() != 2 => {
                return Err(invalid("ADJUST requires two arguments", memory));
            }
            _ => {}
        },
        E::Or(_)
        | E::And(_)
        | E::Arithmetic(..)
        | E::Equal(..)
        | E::SameTerm(..)
        | E::Greater(..)
        | E::GreaterOrEqual(..)
        | E::Less(..)
        | E::LessOrEqual(..)
        | E::UnaryPlus(_)
        | E::UnaryMinus(_)
        | E::Not(_)
        | E::In(..)
        | E::If(..)
        | E::Coalesce(_)
        | E::Exists(_) => {}
    }
    Ok(())
}

fn check_path<S: Admission + ?Sized>(
    path: &PropertyPathExpression,
    memory: &mut Memory<'_, S>,
) -> std::result::Result<(), ValidationError> {
    use PropertyPathExpression as P;
    match path {
        P::NamedNode(n) => iri(n.as_str(), memory)?,
        P::Reverse(_)
        | P::ZeroOrMore(_)
        | P::OneOrMore(_)
        | P::ZeroOrOne(_)
        | P::Sequence(_)
        | P::Alternative(_) => {}
        P::Range { min, max, .. } => {
            if max.is_some_and(|max| *min > max) {
                return Err(invalid(
                    "path range lower bound exceeds upper bound",
                    memory,
                ));
            }
        }
        P::NegatedPropertySet(values) => {
            for v in values {
                iri(v.predicate.as_str(), memory)?;
            }
        }
        P::Wildcard { namespace } => {
            if let Some(n) = namespace {
                iri(n.as_str(), memory)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::SparqlParser;
    use crate::tree::Child;
    use crate::{
        GroundTerm, GroundTriple, NamedNode, NamedNodePattern, TermPattern, TriplePattern, Variable,
    };

    fn pattern_triple(subject: TermPattern, object: TermPattern) -> TermPattern {
        TermPattern::Triple(Child::new(TriplePattern {
            subject,
            predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked(
                "http://example.org/p",
            )),
            object,
        }))
    }

    fn ground_triple(subject: GroundTerm, object: GroundTerm) -> GroundTerm {
        GroundTerm::Triple(Child::new(GroundTriple {
            subject,
            predicate: NamedNode::new_unchecked("http://example.org/p"),
            object,
        }))
    }

    #[test]
    fn pattern_nesting_counts_the_longest_chain_through_subject_or_object() {
        let var = || TermPattern::Variable(Variable::new("x"));
        assert_eq!(var().triple_term_nesting(), 0);
        assert_eq!(pattern_triple(var(), var()).triple_term_nesting(), 1);
        let object_deep = pattern_triple(var(), pattern_triple(var(), var()));
        assert_eq!(object_deep.triple_term_nesting(), 2);
        let subject_deep = pattern_triple(
            pattern_triple(pattern_triple(var(), var()), var()),
            pattern_triple(var(), var()),
        );
        assert_eq!(subject_deep.triple_term_nesting(), 3);
    }

    #[test]
    fn ground_nesting_counts_the_longest_chain_through_subject_or_object() {
        let iri = || GroundTerm::NamedNode(NamedNode::new_unchecked("http://example.org/a"));
        assert_eq!(iri().triple_term_nesting(), 0);
        assert_eq!(ground_triple(iri(), iri()).triple_term_nesting(), 1);
        let subject_deep = ground_triple(ground_triple(ground_triple(iri(), iri()), iri()), iri());
        assert_eq!(subject_deep.triple_term_nesting(), 3);
        let mut chain = iri();
        for _ in 0..10_000 {
            chain = ground_triple(iri(), chain);
        }
        assert_eq!(chain.triple_term_nesting(), 10_000);
    }

    /// Every extension-function call is listed, wherever the expression sits — a
    /// `FILTER`, a `BIND`, a projection, an aggregate's argument, and a `FILTER` inside
    /// `NOT EXISTS` — and nothing else is: the same IRIs in predicate or object position
    /// and a built-in call are not calls.
    #[test]
    fn every_extension_function_call_is_listed_and_a_predicate_iri_is_not() {
        let query = SparqlParser::new()
            .parse_query(
                "PREFIX ex: <http://example.org/ns#>
                 SELECT ?s (ex:project(?s) AS ?p) (SUM(ex:aggregated(?o)) AS ?sum)
                 WHERE {
                   ?s ex:inFilter ?o .
                   ?s ex:object ex:inFilter .
                   BIND (ex:bound(?o) AS ?b)
                   FILTER (ex:inFilter(?o) && STRLEN(STR(?o)) > 0)
                   FILTER NOT EXISTS { ?s ?q ?r FILTER (ex:inExists(?r)) }
                 }
                 GROUP BY ?s",
            )
            .expect("the query parses");
        let calls: Vec<String> = query
            .custom_function_calls()
            .expect("the query is well-formed")
            .into_iter()
            .collect();
        assert_eq!(
            calls,
            [
                "http://example.org/ns#aggregated",
                "http://example.org/ns#bound",
                "http://example.org/ns#inExists",
                "http://example.org/ns#inFilter",
                "http://example.org/ns#project",
            ]
        );
        let none = SparqlParser::new()
            .parse_query(
                "PREFIX ex: <http://example.org/ns#>
                 SELECT ?s WHERE { ?s ex:inFilter ex:bound FILTER (STRLEN(STR(?s)) > 0) }",
            )
            .expect("the query parses");
        assert!(
            none.custom_function_calls()
                .expect("the query is well-formed")
                .is_empty()
        );
    }
}
