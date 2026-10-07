// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Conservative retained-storage accounting without serializing query algebra.

use crate::algebra::{
    AggregateFunction, CdtCall, Expression, Function, GraphPattern, NegatedPathElement,
    OrderExpression, PropertyPathExpression, PurrdfCall, Query, QueryDataset, SparqlVersion,
};
use crate::ast::{
    BlankNode, GroundTerm, Literal, NamedNode, NamedNodePattern, QuadPattern, TermPattern,
    TriplePattern, Variable,
};
use crate::walk::{Flow, NodeRef, walk_pre_post};

impl Query {
    /// Conservative bytes retained by this algebra, including vector capacities.
    ///
    /// Shared strings are charged per occurrence, including Arc counters; allocator
    /// metadata and rounding are excluded. This is a cache accounting unit, not a
    /// canonical identity or a process-memory measurement. No text is serialized.
    /// Arithmetic saturates. The algebra is walked over a work list, so a tree of any
    /// height is charged in full.
    #[must_use]
    pub fn retained_size_bytes(&self) -> usize {
        let (pattern, head) = match self {
            Self::Select {
                pattern,
                dataset,
                base_iri,
                version,
            }
            | Self::Ask {
                pattern,
                dataset,
                base_iri,
                version,
            } => (
                pattern,
                sum([
                    dataset.heap_bytes(),
                    base_iri.heap_bytes(),
                    version.heap_bytes(),
                ]),
            ),
            Self::Construct {
                template,
                pattern,
                dataset,
                base_iri,
                version,
            } => (
                pattern,
                sum([
                    vec_bytes(template),
                    template
                        .iter()
                        .map(templated)
                        .fold(0, usize::saturating_add),
                    dataset.heap_bytes(),
                    base_iri.heap_bytes(),
                    version.heap_bytes(),
                ]),
            ),
            Self::Describe {
                targets,
                pattern,
                dataset,
                base_iri,
                version,
            } => (
                pattern,
                sum([
                    targets.heap_bytes(),
                    dataset.heap_bytes(),
                    base_iri.heap_bytes(),
                    version.heap_bytes(),
                ]),
            ),
        };
        sum([
            size_of::<Self>(),
            head,
            tree_bytes(NodeRef::Pattern(pattern)),
        ])
    }
}

/// The heap a `CONSTRUCT` template quad retains, its triple's terms included.
fn templated(quad: &QuadPattern) -> usize {
    sum([
        quad.graph.heap_bytes(),
        tree_bytes(NodeRef::Triple(&quad.triple)),
    ])
}

/// The heap the tree under `root` retains: every node's own share, summed over a
/// work list.
fn tree_bytes(root: NodeRef<'_>) -> usize {
    let mut total = 0usize;
    walk_pre_post(root, |visit, node| {
        if visit == crate::walk::Visit::Enter {
            total = total.saturating_add(own_bytes(node));
        }
        Flow::Descend
    });
    total
}

fn sum(parts: impl IntoIterator<Item = usize>) -> usize {
    parts.into_iter().fold(0, usize::saturating_add)
}

/// The allocation a vector holds, by capacity, not counting what its elements own.
fn vec_bytes<T>(values: &[T]) -> usize {
    values.len().saturating_mul(size_of::<T>())
}

/// A list edge's allocation, by capacity.
fn edge_bytes<T>(capacity: usize) -> usize {
    capacity.saturating_mul(size_of::<T>())
}

/// The heap a node owns directly: its leaves, the boxes and vectors its children
/// live in — but not what those children own, which is charged when they are
/// visited.
fn own_bytes(node: NodeRef<'_>) -> usize {
    match node {
        NodeRef::Pattern(pattern) => pattern_bytes(pattern),
        NodeRef::Expr(expr) => expr_bytes(expr),
        NodeRef::Path(path) => path_bytes(path),
        NodeRef::Triple(triple) => triple.predicate.heap_bytes(),
        NodeRef::Term(term) => match term {
            TermPattern::NamedNode(x) => x.heap_bytes(),
            TermPattern::BlankNode(x) => x.heap_bytes(),
            TermPattern::Literal(x) => x.heap_bytes(),
            TermPattern::Variable(x) => x.heap_bytes(),
            TermPattern::Triple(_) => size_of::<TriplePattern>(),
        },
        NodeRef::Ground(term) => match term {
            GroundTerm::NamedNode(x) => x.heap_bytes(),
            GroundTerm::BlankNode(x) => x.heap_bytes(),
            GroundTerm::Literal(x) => x.heap_bytes(),
            GroundTerm::Triple(triple) => sum([
                size_of::<crate::ast::GroundTriple>(),
                triple.predicate.heap_bytes(),
            ]),
        },
        NodeRef::Order(_) => 0,
        NodeRef::Aggregate(aggregate) => sum([
            aggregate.function.heap_bytes(),
            edge_bytes::<Expression>(aggregate.args.capacity()),
            edge_bytes::<(String, Literal)>(aggregate.scalarvals.capacity()),
            aggregate
                .scalarvals
                .iter()
                .map(|(key, value)| sum([key.capacity(), value.heap_bytes()]))
                .fold(0, usize::saturating_add),
            edge_bytes::<OrderExpression>(aggregate.order_by.capacity()),
        ]),
    }
}

fn pattern_bytes(pattern: &GraphPattern) -> usize {
    use GraphPattern as G;
    const INNER: usize = size_of::<GraphPattern>();
    match pattern {
        G::Bgp { patterns } => edge_bytes::<TriplePattern>(patterns.capacity()),
        G::Path { .. } => 0,
        G::Apply { policy, .. } => {
            let pairs = |values: &Vec<(Variable, Variable)>| {
                sum([
                    edge_bytes::<(Variable, Variable)>(values.capacity()),
                    values
                        .iter()
                        .map(|(a, b)| sum([a.heap_bytes(), b.heap_bytes()]))
                        .fold(0, usize::saturating_add),
                ])
            };
            sum([
                2 * INNER,
                size_of::<crate::algebra::ApplicationPolicy>(),
                pairs(&policy.inputs),
                policy.group_domain.as_ref().map_or(0, |domain| {
                    sum([
                        domain.len().saturating_mul(size_of::<Variable>()),
                        domain
                            .iter()
                            .map(Variable::heap_bytes)
                            .fold(0, usize::saturating_add),
                    ])
                }),
                policy.optional.as_ref().map_or(0, |optional| {
                    sum([
                        pairs(&optional.retry_inputs),
                        optional.forget_marker.heap_bytes(),
                    ])
                }),
            ])
        }
        G::Join { .. } | G::Lateral { .. } | G::Minus { .. } | G::LeftJoin { .. } => 2 * INNER,
        G::Filter { .. }
        | G::OrderBy { .. }
        | G::Distinct { .. }
        | G::Reduced { .. }
        | G::Slice { .. } => {
            let keys = match pattern {
                G::OrderBy { expression, .. } => {
                    edge_bytes::<OrderExpression>(expression.capacity())
                }
                _ => 0,
            };
            sum([INNER, keys])
        }
        G::Union { arms } => edge_bytes::<GraphPattern>(arms.capacity()),
        G::Graph { name, .. } | G::Service { name, .. } => sum([INNER, name.heap_bytes()]),
        G::Extend { variable, .. } => sum([INNER, variable.heap_bytes()]),
        G::Values {
            variables,
            bindings,
        } => sum([
            variables.heap_bytes(),
            edge_bytes::<Vec<Option<GroundTerm>>>(bindings.capacity()),
            bindings
                .iter()
                .map(|row| edge_bytes::<Option<GroundTerm>>(row.capacity()))
                .fold(0, usize::saturating_add),
        ]),
        G::Project { variables, .. } => sum([INNER, variables.heap_bytes()]),
        G::Group {
            variables,
            aggregates,
            ..
        } => sum([
            INNER,
            variables.heap_bytes(),
            edge_bytes::<(Variable, crate::AggregateExpression)>(aggregates.capacity()),
            aggregates
                .iter()
                .map(|(output, _)| output.heap_bytes())
                .fold(0, usize::saturating_add),
        ]),
        G::PropertyFunction(call) => sum([
            call.iri.capacity(),
            edge_bytes::<TermPattern>(call.subject_args.capacity()),
            edge_bytes::<TermPattern>(call.object_args.capacity()),
        ]),
        G::Unfold {
            element, companion, ..
        } => sum([INNER, element.heap_bytes(), companion.heap_bytes()]),
    }
}

fn expr_bytes(expr: &Expression) -> usize {
    use Expression as E;
    const OPERAND: usize = size_of::<Expression>();
    match expr {
        E::NamedNode(x) => x.heap_bytes(),
        E::Literal(x) => x.heap_bytes(),
        E::Variable(x) | E::Bound(x) => x.heap_bytes(),
        E::Or(operands) | E::And(operands) => edge_bytes::<Expression>(operands.capacity()),
        E::Arithmetic(_, steps) => sum([
            OPERAND,
            edge_bytes::<(crate::ArithmeticOperator, Expression)>(steps.capacity()),
        ]),
        E::Equal(..)
        | E::SameTerm(..)
        | E::Greater(..)
        | E::GreaterOrEqual(..)
        | E::Less(..)
        | E::LessOrEqual(..) => 2 * OPERAND,
        E::UnaryPlus(_) | E::UnaryMinus(_) | E::Not(_) => OPERAND,
        E::In(_, list) => sum([OPERAND, edge_bytes::<Expression>(list.capacity())]),
        E::If(..) => 3 * OPERAND,
        E::Coalesce(list) => edge_bytes::<Expression>(list.capacity()),
        E::FunctionCall(function, args) => sum([
            function.heap_bytes(),
            edge_bytes::<Expression>(args.capacity()),
        ]),
        E::Exists(_) => size_of::<GraphPattern>(),
    }
}

fn path_bytes(path: &PropertyPathExpression) -> usize {
    use PropertyPathExpression as P;
    match path {
        P::NamedNode(x) => x.heap_bytes(),
        P::Reverse(_) | P::ZeroOrMore(_) | P::OneOrMore(_) | P::ZeroOrOne(_) | P::Range { .. } => {
            size_of::<PropertyPathExpression>()
        }
        P::Sequence(elements) | P::Alternative(elements) => {
            edge_bytes::<PropertyPathExpression>(elements.capacity())
        }
        P::NegatedPropertySet(elements) => elements.heap_bytes(),
        P::Wildcard { namespace } => namespace.heap_bytes(),
    }
}

/// The heap a leaf value owns; leaves own no node, so this never recurses.
trait HeapBytes {
    fn heap_bytes(&self) -> usize;
}
impl<T: HeapBytes> HeapBytes for Vec<T> {
    fn heap_bytes(&self) -> usize {
        edge_bytes::<T>(self.capacity()).saturating_add(sum(self.iter().map(HeapBytes::heap_bytes)))
    }
}
impl<T: HeapBytes> HeapBytes for Option<T> {
    fn heap_bytes(&self) -> usize {
        self.as_ref().map_or(0, HeapBytes::heap_bytes)
    }
}
impl HeapBytes for String {
    fn heap_bytes(&self) -> usize {
        self.capacity()
    }
}
fn shared_string(value: &str) -> usize {
    value.len().saturating_add(2 * size_of::<usize>())
}
macro_rules! lexical {
    ($($ty:ty),+ $(,)?) => { $(impl HeapBytes for $ty {
        fn heap_bytes(&self) -> usize { shared_string(self.as_str()) }
    })+ };
}
lexical!(NamedNode, BlankNode, Variable);
impl HeapBytes for Literal {
    fn heap_bytes(&self) -> usize {
        sum([
            shared_string(self.value()),
            self.datatype().heap_bytes(),
            self.language().map_or(0, shared_string),
        ])
    }
}
impl HeapBytes for NamedNodePattern {
    fn heap_bytes(&self) -> usize {
        match self {
            Self::NamedNode(x) => x.heap_bytes(),
            Self::Variable(x) => x.heap_bytes(),
        }
    }
}
impl HeapBytes for NegatedPathElement {
    fn heap_bytes(&self) -> usize {
        self.predicate.heap_bytes()
    }
}
impl HeapBytes for QueryDataset {
    fn heap_bytes(&self) -> usize {
        sum([self.default.heap_bytes(), self.named.heap_bytes()])
    }
}
impl HeapBytes for SparqlVersion {
    fn heap_bytes(&self) -> usize {
        match self {
            Self::Other(x) => x.heap_bytes(),
            Self::V12 | Self::V12Basic => 0,
        }
    }
}
impl HeapBytes for PurrdfCall {
    fn heap_bytes(&self) -> usize {
        self.iri.heap_bytes()
    }
}
impl HeapBytes for CdtCall {
    fn heap_bytes(&self) -> usize {
        self.iri.heap_bytes()
    }
}
impl HeapBytes for AggregateFunction {
    fn heap_bytes(&self) -> usize {
        match self {
            Self::Custom(x) => x.heap_bytes(),
            Self::Count
            | Self::Sum
            | Self::Avg
            | Self::Min
            | Self::Max
            | Self::Sample
            | Self::GroupConcat
            | Self::Fold => 0,
        }
    }
}
impl HeapBytes for Function {
    fn heap_bytes(&self) -> usize {
        match self {
            Self::Purrdf(x) => x.heap_bytes(),
            Self::Cdt(x) => x.heap_bytes(),
            Self::Custom(x) => x.heap_bytes(),
            Self::Str
            | Self::Lang
            | Self::LangMatches
            | Self::Datatype
            | Self::Iri
            | Self::Uri
            | Self::BNode
            | Self::Rand
            | Self::Abs
            | Self::Ceil
            | Self::Floor
            | Self::Round
            | Self::Concat
            | Self::SubStr
            | Self::StrLen
            | Self::Replace
            | Self::UCase
            | Self::LCase
            | Self::EncodeForUri
            | Self::Contains
            | Self::StrStarts
            | Self::StrEnds
            | Self::StrBefore
            | Self::StrAfter
            | Self::Year
            | Self::Month
            | Self::Day
            | Self::Hours
            | Self::Minutes
            | Self::Seconds
            | Self::Timezone
            | Self::Tz
            | Self::Adjust
            | Self::Now
            | Self::Uuid
            | Self::StrUuid
            | Self::Md5
            | Self::Sha1
            | Self::Sha256
            | Self::Sha384
            | Self::Sha512
            | Self::Sha3_224
            | Self::Sha3_256
            | Self::Sha3_384
            | Self::Sha3_512
            | Self::StrLang
            | Self::StrDt
            | Self::IsIri
            | Self::IsUri
            | Self::IsBlank
            | Self::IsLiteral
            | Self::IsNumeric
            | Self::Regex
            | Self::Triple
            | Self::Subject
            | Self::Predicate
            | Self::Object
            | Self::IsTriple
            | Self::LangDir
            | Self::StrLangDir
            | Self::HasLang
            | Self::HasLangDir => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_capacity_is_charged_even_when_the_vector_is_empty() {
        let query = |patterns| Query::Ask {
            pattern: GraphPattern::Bgp { patterns },
            dataset: QueryDataset::default(),
            base_iri: None,
            version: None,
        };
        let empty = query(Vec::new()).retained_size_bytes();
        let reserved = Vec::with_capacity(128);
        let expected = reserved.capacity() * size_of::<TriplePattern>();
        assert_eq!(query(reserved).retained_size_bytes() - empty, expected);
    }

    #[test]
    fn deep_nesting_is_charged_in_full() {
        const LEVELS: usize = 100_000;
        let mut pattern = GraphPattern::Bgp { patterns: vec![] };
        for _ in 0..LEVELS {
            pattern = GraphPattern::Distinct {
                inner: crate::Child::new(pattern),
            };
        }
        let query = Query::Ask {
            pattern,
            dataset: QueryDataset::default(),
            base_iri: None,
            version: None,
        };
        assert_eq!(
            query.retained_size_bytes(),
            size_of::<Query>() + LEVELS * size_of::<GraphPattern>()
        );
    }
}
