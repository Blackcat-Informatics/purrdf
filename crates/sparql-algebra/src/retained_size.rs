// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Conservative retained-storage accounting without serializing query algebra.

use crate::algebra::{
    AggregateExpression, AggregateFunction, CdtCall, Expression, Function, GraphPattern,
    NegatedPathElement, OrderExpression, PropertyFunctionCall, PropertyPathExpression, PurrdfCall,
    Query, QueryDataset, SparqlVersion,
};
use crate::ast::{
    BlankNode, GroundTerm, GroundTriple, Literal, NamedNode, NamedNodePattern, QuadPattern,
    TermPattern, TriplePattern, Variable,
};

impl Query {
    /// Conservative bytes retained by this algebra, including vector capacities.
    ///
    /// Shared strings are charged per occurrence, including Arc counters; allocator
    /// metadata and rounding are excluded. This is a cache accounting unit, not a
    /// canonical identity or a process-memory measurement. No text is serialized.
    /// Arithmetic saturates; nesting beyond 256 allocated containers returns `usize::MAX`,
    /// refusing finite-cache retention without overflowing the accounting stack.
    #[must_use]
    pub fn retained_size_bytes(&self) -> usize {
        size_of::<Self>().saturating_add(self.heap_bytes(0))
    }
}

trait HeapBytes {
    fn heap_bytes(&self, depth: usize) -> usize;
}
fn sum(parts: impl IntoIterator<Item = usize>) -> usize {
    parts.into_iter().fold(0, usize::saturating_add)
}
impl<T: HeapBytes> HeapBytes for Vec<T> {
    fn heap_bytes(&self, depth: usize) -> usize {
        if depth >= 256 {
            return usize::MAX;
        }
        self.capacity()
            .saturating_mul(size_of::<T>())
            .saturating_add(sum(self.iter().map(|item| item.heap_bytes(depth + 1))))
    }
}
impl<T: HeapBytes> HeapBytes for Option<T> {
    fn heap_bytes(&self, depth: usize) -> usize {
        self.as_ref().map_or(0, |item| item.heap_bytes(depth))
    }
}
impl<T: HeapBytes> HeapBytes for Box<T> {
    fn heap_bytes(&self, depth: usize) -> usize {
        if depth >= 256 {
            return usize::MAX;
        }
        size_of::<T>().saturating_add((**self).heap_bytes(depth + 1))
    }
}
impl<A: HeapBytes, B: HeapBytes> HeapBytes for (A, B) {
    fn heap_bytes(&self, depth: usize) -> usize {
        self.0
            .heap_bytes(depth)
            .saturating_add(self.1.heap_bytes(depth))
    }
}
impl HeapBytes for String {
    fn heap_bytes(&self, _: usize) -> usize {
        self.capacity()
    }
}
fn shared_string(value: &str) -> usize {
    value.len().saturating_add(2 * size_of::<usize>())
}
macro_rules! lexical {
    ($($ty:ty),+ $(,)?) => { $(impl HeapBytes for $ty {
        fn heap_bytes(&self, _: usize) -> usize { shared_string(self.as_str()) }
    })+ };
}
lexical!(NamedNode, BlankNode, Variable);
impl HeapBytes for Literal {
    fn heap_bytes(&self, depth: usize) -> usize {
        sum([
            shared_string(self.value()),
            self.datatype().heap_bytes(depth),
            self.language().map_or(0, shared_string),
        ])
    }
}
macro_rules! fields {
    ($ty:ty { $($field:ident),+ $(,)? }) => { impl HeapBytes for $ty {
        fn heap_bytes(&self, depth: usize) -> usize {
            let Self { $($field),+ } = self;
            sum([$($field.heap_bytes(depth)),+])
        }
    } };
}
macro_rules! inline {
    ($($ty:ty),+ $(,)?) => { $(impl HeapBytes for $ty {
        fn heap_bytes(&self, _: usize) -> usize { 0 }
    })+ };
}
inline!(
    bool,
    usize,
    u32,
    crate::algebra::PurrdfFn,
    crate::algebra::CdtFn
);
fields!(TriplePattern {
    subject,
    predicate,
    object
});
fields!(GroundTriple {
    subject,
    predicate,
    object
});
fields!(QuadPattern { triple, graph });
fields!(QueryDataset { default, named });
fields!(PropertyFunctionCall {
    iri,
    subject_args,
    object_args
});
fields!(NegatedPathElement { predicate, inverse });
fields!(PurrdfCall { fn_kind, iri });
fields!(CdtCall { fn_kind, iri });
fields!(AggregateExpression {
    function,
    args,
    scalarvals,
    order_by,
    distinct
});
impl HeapBytes for NamedNodePattern {
    fn heap_bytes(&self, depth: usize) -> usize {
        match self {
            Self::NamedNode(x) => x.heap_bytes(depth),
            Self::Variable(x) => x.heap_bytes(depth),
        }
    }
}
impl HeapBytes for TermPattern {
    fn heap_bytes(&self, depth: usize) -> usize {
        match self {
            Self::NamedNode(x) => x.heap_bytes(depth),
            Self::BlankNode(x) => x.heap_bytes(depth),
            Self::Literal(x) => x.heap_bytes(depth),
            Self::Variable(x) => x.heap_bytes(depth),
            Self::Triple(x) => x.heap_bytes(depth),
        }
    }
}
impl HeapBytes for GroundTerm {
    fn heap_bytes(&self, depth: usize) -> usize {
        match self {
            Self::NamedNode(x) => x.heap_bytes(depth),
            Self::BlankNode(x) => x.heap_bytes(depth),
            Self::Literal(x) => x.heap_bytes(depth),
            Self::Triple(x) => x.heap_bytes(depth),
        }
    }
}
impl HeapBytes for SparqlVersion {
    fn heap_bytes(&self, depth: usize) -> usize {
        match self {
            Self::Other(x) => x.heap_bytes(depth),
            Self::V12 | Self::V12Basic => 0,
        }
    }
}
impl HeapBytes for Query {
    fn heap_bytes(&self, depth: usize) -> usize {
        match self {
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
            } => sum([
                pattern.heap_bytes(depth),
                dataset.heap_bytes(depth),
                base_iri.heap_bytes(depth),
                version.heap_bytes(depth),
            ]),
            Self::Construct {
                template,
                pattern,
                dataset,
                base_iri,
                version,
            } => sum([
                template.heap_bytes(depth),
                pattern.heap_bytes(depth),
                dataset.heap_bytes(depth),
                base_iri.heap_bytes(depth),
                version.heap_bytes(depth),
            ]),
            Self::Describe {
                targets,
                pattern,
                dataset,
                base_iri,
                version,
            } => sum([
                targets.heap_bytes(depth),
                pattern.heap_bytes(depth),
                dataset.heap_bytes(depth),
                base_iri.heap_bytes(depth),
                version.heap_bytes(depth),
            ]),
        }
    }
}
impl HeapBytes for GraphPattern {
    fn heap_bytes(&self, depth: usize) -> usize {
        match self {
            Self::Bgp { patterns } => patterns.heap_bytes(depth),
            Self::Path {
                subject,
                path,
                object,
            } => sum([
                subject.heap_bytes(depth),
                path.heap_bytes(depth),
                object.heap_bytes(depth),
            ]),
            Self::Join { left, right }
            | Self::Lateral { left, right }
            | Self::Union { left, right }
            | Self::Minus { left, right } => sum([left.heap_bytes(depth), right.heap_bytes(depth)]),
            Self::LeftJoin {
                left,
                right,
                expression,
            } => sum([
                left.heap_bytes(depth),
                right.heap_bytes(depth),
                expression.heap_bytes(depth),
            ]),
            Self::Filter { expr, inner } => sum([expr.heap_bytes(depth), inner.heap_bytes(depth)]),
            Self::Graph { name, inner }
            | Self::Service {
                name,
                inner,
                silent: _,
            } => sum([name.heap_bytes(depth), inner.heap_bytes(depth)]),
            Self::Extend {
                inner,
                variable,
                expression,
            } => sum([
                inner.heap_bytes(depth),
                variable.heap_bytes(depth),
                expression.heap_bytes(depth),
            ]),
            Self::Values {
                variables,
                bindings,
            } => sum([variables.heap_bytes(depth), bindings.heap_bytes(depth)]),
            Self::OrderBy { inner, expression } => {
                sum([inner.heap_bytes(depth), expression.heap_bytes(depth)])
            }
            Self::Project { inner, variables } => {
                sum([inner.heap_bytes(depth), variables.heap_bytes(depth)])
            }
            Self::Distinct { inner }
            | Self::Reduced { inner }
            | Self::Slice {
                inner,
                start: _,
                length: _,
            } => inner.heap_bytes(depth),
            Self::Group {
                inner,
                variables,
                aggregates,
            } => sum([
                inner.heap_bytes(depth),
                variables.heap_bytes(depth),
                aggregates.heap_bytes(depth),
            ]),
            Self::PropertyFunction(x) => x.heap_bytes(depth),
            Self::Unfold {
                inner,
                expression,
                element,
                companion,
            } => sum([
                inner.heap_bytes(depth),
                expression.heap_bytes(depth),
                element.heap_bytes(depth),
                companion.heap_bytes(depth),
            ]),
        }
    }
}
impl HeapBytes for PropertyPathExpression {
    fn heap_bytes(&self, depth: usize) -> usize {
        match self {
            Self::NamedNode(x) => x.heap_bytes(depth),
            Self::Reverse(x)
            | Self::ZeroOrMore(x)
            | Self::OneOrMore(x)
            | Self::ZeroOrOne(x)
            | Self::Range {
                inner: x,
                min: _,
                max: _,
            } => x.heap_bytes(depth),
            Self::Sequence(a, b) | Self::Alternative(a, b) => {
                sum([a.heap_bytes(depth), b.heap_bytes(depth)])
            }
            Self::NegatedPropertySet(x) => x.heap_bytes(depth),
            Self::Wildcard { namespace } => namespace.heap_bytes(depth),
        }
    }
}
impl HeapBytes for Expression {
    fn heap_bytes(&self, depth: usize) -> usize {
        match self {
            Self::NamedNode(x) => x.heap_bytes(depth),
            Self::Literal(x) => x.heap_bytes(depth),
            Self::Variable(x) | Self::Bound(x) => x.heap_bytes(depth),
            Self::Or(a, b)
            | Self::And(a, b)
            | Self::Equal(a, b)
            | Self::SameTerm(a, b)
            | Self::Greater(a, b)
            | Self::GreaterOrEqual(a, b)
            | Self::Less(a, b)
            | Self::LessOrEqual(a, b)
            | Self::Add(a, b)
            | Self::Subtract(a, b)
            | Self::Multiply(a, b)
            | Self::Divide(a, b) => sum([a.heap_bytes(depth), b.heap_bytes(depth)]),
            Self::UnaryPlus(x) | Self::UnaryMinus(x) | Self::Not(x) => x.heap_bytes(depth),
            Self::In(x, values) => sum([x.heap_bytes(depth), values.heap_bytes(depth)]),
            Self::If(a, b, c) => sum([
                a.heap_bytes(depth),
                b.heap_bytes(depth),
                c.heap_bytes(depth),
            ]),
            Self::Coalesce(values) => values.heap_bytes(depth),
            Self::FunctionCall(f, args) => sum([f.heap_bytes(depth), args.heap_bytes(depth)]),
            Self::Exists(x) => x.heap_bytes(depth),
        }
    }
}
impl HeapBytes for OrderExpression {
    fn heap_bytes(&self, depth: usize) -> usize {
        match self {
            Self::Asc(x) | Self::Desc(x) => x.heap_bytes(depth),
        }
    }
}
impl HeapBytes for AggregateFunction {
    fn heap_bytes(&self, depth: usize) -> usize {
        match self {
            Self::Custom(x) => x.heap_bytes(depth),
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
    fn heap_bytes(&self, depth: usize) -> usize {
        match self {
            Self::Purrdf(x) => x.heap_bytes(depth),
            Self::Cdt(x) => x.heap_bytes(depth),
            Self::Custom(x) => x.heap_bytes(depth),
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
    fn oversized_nesting_cannot_be_undercharged_to_a_finite_cache() {
        let mut pattern = GraphPattern::Bgp { patterns: vec![] };
        for _ in 0..300 {
            pattern = GraphPattern::Distinct {
                inner: Box::new(pattern),
            };
        }
        let query = Query::Ask {
            pattern,
            dataset: QueryDataset::default(),
            base_iri: None,
            version: None,
        };
        assert_eq!(query.retained_size_bytes(), usize::MAX);
    }
}
