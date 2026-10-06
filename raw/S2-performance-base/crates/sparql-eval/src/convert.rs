// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Conversions from the lexical [`purrdf_sparql_algebra`] term types to the
//! dataset-independent [`TermValue`] lookup/build key.
//!
//! The algebra carries terms lexically (an IRI string, a literal's lexical form +
//! datatype IRI); the IR keys term identity on [`TermValue`]. These helpers bridge
//! the two and apply the one normalization the IR's C0.1 literal-identity contract
//! requires at the lookup boundary: a language tag is lowercased so a query literal
//! matches the dataset's interned (already-lowercased) form.

use purrdf_core::TermBox;
use purrdf_core::TermValue;
use purrdf_sparql_algebra::{
    Child, GroundTerm, Literal, NamedNode, NamedNodePattern, TermPattern, TriplePattern,
};

use crate::error::EvalError;

/// An IRI term value.
#[inline]
pub fn named_node_to_value(node: &NamedNode) -> TermValue {
    TermValue::Iri(node.as_str().to_owned())
}

/// The blank-node scope every `BLANK_NODE_LABEL` written inside a `cdt:List` /
/// `cdt:Map` literal **in query text** is bound to.
///
/// A composite literal's lexical form is not opaque text: its blank labels denote
/// blank nodes of the *document that wrote them* (see [`purrdf_core::cdt_blank`]).
/// A SPARQL query is such a document, and it is a DIFFERENT one from the dataset
/// it queries — so `BIND("[_:b, 42]"^^cdt:List AS ?l)` names a node distinct from
/// the `_:b` of a Turtle file the query is evaluated against, exactly as two
/// Turtle files that both write `_:b` name two nodes. Without a scope of its own
/// the query-authored label lands at
/// [`BlankScope::DEFAULT`](purrdf_core::ir::BlankScope::DEFAULT) — which is precisely
/// where a directly-parsed document's blanks live — and the two collapse onto one
/// node.
///
/// Two occurrences of one label in the SAME query still denote the SAME node:
/// they share this one scope, so they intern to a single value. That is the
/// property `bnodes-sparql-01`/`-21`/`-26` of the vendored SEP-0009 corpus pin.
///
/// # Why the top of the space, and what it reserves
///
/// `BlankScope(0)` is a parsed document's own scope and `1, 2, 3, …` are what
/// [`push_dataset`](purrdf_core::RdfDatasetBuilder::push_dataset) hands out to the
/// sources of a merge, counting up. The query therefore takes the far end, and
/// `BlankScope(u32::MAX)` is **reserved**: no dataset may intern a blank node
/// under it. [`ScratchInterner::intern`](crate::scratch::ScratchInterner::intern)
/// enforces the query half of that reservation directly — a value at this scope is
/// never promoted to a dataset term — so the separation does not rest on the
/// dataset merely happening not to hold the label.
pub const QUERY_BLANK_SCOPE: purrdf_core::BlankScope = purrdf_core::BlankScope(u32::MAX);

/// A literal term value, with the language tag lowercased to match the IR's C0.1
/// interned identity (so a query literal resolves to the dataset's stored form).
///
/// This is the QUERY-TEXT ingress for literals — every [`Literal`] the algebra
/// carries (a constant expression, a `CONSTRUCT`/`INSERT` template cell, a
/// `VALUES` cell) becomes a [`TermValue`] here, and nothing else does. So it is
/// also where a composite literal's embedded blank labels are bound into the
/// query's own [`QUERY_BLANK_SCOPE`], the exact counterpart of what
/// [`intern_literal_bound`](purrdf_core::RdfDatasetBuilder::intern_literal_bound)
/// does for a literal read from a document.
///
/// The binding is guarded by the datatype IRI, so an ordinary literal pays two
/// string comparisons and nothing else; it is the total
/// (`_unchecked`) form because an ill-formed composite lexical form in a query is
/// diagnosed by the evaluator's own CDT parse, which reports it against the query
/// rather than refusing a document.
pub fn literal_to_value(lit: &Literal) -> TermValue {
    let datatype = lit.datatype().as_str();
    // C0.1: a language tag determines a string datatype, so the literal is never
    // composite — so the binding is only ever reached for an untagged literal.
    let lexical_form = match lit.language() {
        None => purrdf_core::cdt_blank::bind_cdt_blank_labels_unchecked(
            lit.value(),
            datatype,
            purrdf_core::cdt_blank::BlankBinding::Ambient(QUERY_BLANK_SCOPE),
        )
        .into_owned(),
        Some(_) => lit.value().to_owned(),
    };
    TermValue::Literal {
        lexical_form,
        datatype: datatype.to_owned(),
        language: lit.language().map(purrdf_iri::langtag::identity_fold),
        direction: lit.direction(),
    }
}

/// Convert a **ground** quoted-triple pattern to a [`TermValue::Triple`].
///
/// `site` names the construct the pattern was found in (`"a BGP"`, `"a
/// property-path endpoint"`, …) so the [`EvalError::Unsupported`] a variable
/// component produces names where it actually was, rather than a fixed site
/// baked into this shared helper — see [`ground_term_pattern_to_value`]'s docs
/// for why one caller's wording must not leak into another's diagnostic.
///
/// Returns [`EvalError::Unsupported`] if any component is a variable: matching a
/// quoted triple term whose components *bind* variables (structural triple-term
/// matching) is not supported for any caller of this helper, not only BGPs; only fully-ground quoted triples resolve to a single interned id.
pub fn ground_triple_pattern_to_value(
    pattern: &TriplePattern,
    site: &str,
) -> Result<TermValue, EvalError> {
    ground_term_pattern_to_value(&TermPattern::Triple(Child::new(pattern.clone())), site)
}

/// Convert a **ground** term pattern (no variables) to a [`TermValue`].
///
/// `site` is named in the [`EvalError::Unsupported`] a variable component produces
/// (see below). This helper is shared across every caller that needs a fully-ground
/// term — a BGP triple position, a property-path endpoint, a property-function
/// argument — and each names its own site so the message a caller sees always
/// describes the construct it actually wrote, not whichever caller happened to be
/// first to need this conversion.
///
/// A quoted triple is assembled bottom-up over a work list — its subject, predicate
/// and object each converted fully in that order, and the first variable met ends the
/// conversion with the error — so a term nested to any depth costs no machine stack.
///
/// # Errors
///
/// [`EvalError::Unsupported`] naming `site` when a variable stands anywhere in the
/// pattern: in a quoted triple's subject, predicate or object, or as the whole term.
pub fn ground_term_pattern_to_value(
    pattern: &TermPattern,
    site: &str,
) -> Result<TermValue, EvalError> {
    /// One pending step: convert a term, or assemble the triple whose three
    /// components were converted last.
    enum Step<'a> {
        Term(&'a TermPattern),
        Predicate(&'a NamedNodePattern),
        Assemble,
    }
    let variable = |what: &str| {
        EvalError::unsupported_deferred(
            crate::error::UnsupportedKind::QuotedTripleTermVariable,
            format!("{what} in {site}"),
        )
    };
    // Inline until a term nests deeper than a quoted triple of quoted triples, so a
    // plain term costs only its own value.
    let mut steps: purrdf_core::SmallVec<[Step<'_>; 8]> =
        purrdf_core::smallvec![Step::Term(pattern)];
    let mut values: purrdf_core::SmallVec<[TermValue; 3]> = purrdf_core::SmallVec::new();
    while let Some(step) = steps.pop() {
        match step {
            Step::Term(TermPattern::NamedNode(n)) => values.push(named_node_to_value(n)),
            Step::Term(TermPattern::BlankNode(b)) => values.push(TermValue::Blank {
                label: b.as_str().to_owned(),
                scope: purrdf_core::BlankScope::DEFAULT,
            }),
            Step::Term(TermPattern::Literal(l)) => values.push(literal_to_value(l)),
            Step::Term(TermPattern::Triple(t)) => steps.extend([
                Step::Assemble,
                Step::Term(&t.object),
                Step::Predicate(&t.predicate),
                Step::Term(&t.subject),
            ]),
            Step::Term(TermPattern::Variable(_)) => {
                return Err(variable("variable inside a quoted triple term"));
            }
            Step::Predicate(NamedNodePattern::NamedNode(n)) => values.push(named_node_to_value(n)),
            Step::Predicate(NamedNodePattern::Variable(_)) => {
                return Err(variable("variable predicate inside a quoted triple term"));
            }
            Step::Assemble => {
                let o = values.pop().expect("a quoted triple's object is converted");
                let p = values
                    .pop()
                    .expect("a quoted triple's predicate is converted");
                let s = values
                    .pop()
                    .expect("a quoted triple's subject is converted");
                values.push(TermValue::Triple {
                    s: TermBox::new(s),
                    p: TermBox::new(p),
                    o: TermBox::new(o),
                });
            }
        }
    }
    Ok(values
        .pop()
        .expect("the root's value is the last one assembled"))
}

/// Convert a [`GroundTerm`] (a `VALUES` cell or quoted-triple component) to a
/// [`TermValue`]. Always succeeds — a `GroundTerm` carries no variables.
///
/// A quoted triple is assembled bottom-up over a work list — its subject, predicate and
/// object each converted fully in that order — so a term nested to any depth costs no
/// machine stack.
pub fn ground_term_to_value(term: &GroundTerm) -> TermValue {
    /// One pending step: convert a term, or assemble the triple whose three
    /// components were converted last.
    enum Step<'a> {
        Term(&'a GroundTerm),
        Predicate(&'a NamedNode),
        Assemble,
    }
    // Inline until a term nests deeper than a quoted triple of quoted triples, so a
    // plain term costs only its own value.
    let mut steps: purrdf_core::SmallVec<[Step<'_>; 8]> = purrdf_core::smallvec![Step::Term(term)];
    let mut values: purrdf_core::SmallVec<[TermValue; 3]> = purrdf_core::SmallVec::new();
    while let Some(step) = steps.pop() {
        match step {
            Step::Term(GroundTerm::NamedNode(n)) | Step::Predicate(n) => {
                values.push(named_node_to_value(n));
            }
            Step::Term(GroundTerm::Literal(l)) => values.push(literal_to_value(l)),
            Step::Term(GroundTerm::Triple(t)) => steps.extend([
                Step::Assemble,
                Step::Term(&t.object),
                Step::Predicate(&t.predicate),
                Step::Term(&t.subject),
            ]),
            // Injection-only: a substituted blank-node focus node. The label carries
            // the scope-qualified rendering the injector wrote into the algebra's
            // single string slot, so decoding it is the exact inverse and restores
            // the `(label, scope)` pair `term_id_by_value` resolves against. A label
            // that was never qualified decodes to itself at the default scope.
            //
            // This is the documented contract on `GroundTerm::BlankNode` and
            // `Query::substitute_variable`: the string in this slot is read as a
            // scope-qualified spelling. A default-scope label is its own spelling
            // and passes through byte for byte (the literal label `a.s1` is spelled
            // `a.s1`); only a scoped pair has a distinct spelling — its envelope,
            // `("a", scope 1)` being `purrdfesc1_a`.
            Step::Term(GroundTerm::BlankNode(b)) => {
                let (label, scope) = purrdf_core::BlankScope::unqualify_label(b.as_str());
                values.push(TermValue::Blank {
                    label: label.into_owned(),
                    scope,
                });
            }
            Step::Assemble => {
                let o = values.pop().expect("a quoted triple's object is converted");
                let p = values
                    .pop()
                    .expect("a quoted triple's predicate is converted");
                let s = values
                    .pop()
                    .expect("a quoted triple's subject is converted");
                values.push(TermValue::Triple {
                    s: TermBox::new(s),
                    p: TermBox::new(p),
                    o: TermBox::new(o),
                });
            }
        }
    }
    values
        .pop()
        .expect("the root's value is the last one assembled")
}

#[cfg(test)]
mod term_walk_tests {
    //! The ground-term conversions against their recursive references, and at a hundred
    //! thousand levels on a 128 KiB thread.

    use purrdf_core::{TermBox, TermValue};
    use purrdf_sparql_algebra::{
        BlankNode, Child, GroundTerm, GroundTriple, Literal, NamedNode, NamedNodePattern,
        TermPattern, TriplePattern, Variable,
    };

    use super::{
        ground_term_pattern_to_value, ground_term_to_value, literal_to_value, named_node_to_value,
    };
    use crate::error::EvalError;

    const EX: &str = "http://example.org/";
    const DEPTH: usize = 100_000;
    const SMALL_STACK: usize = 128 * 1024;
    const SITE: &str = "a generated position";

    // ── The recursive references ───────────────────────────────────────────────────

    fn reference_pattern(pattern: &TermPattern) -> Result<TermValue, EvalError> {
        match pattern {
            TermPattern::NamedNode(n) => Ok(named_node_to_value(n)),
            TermPattern::BlankNode(b) => Ok(TermValue::Blank {
                label: b.as_str().to_owned(),
                scope: purrdf_core::BlankScope::DEFAULT,
            }),
            TermPattern::Literal(l) => Ok(literal_to_value(l)),
            TermPattern::Triple(t) => {
                let s = reference_pattern(&t.subject)?;
                let p = match &t.predicate {
                    NamedNodePattern::NamedNode(n) => named_node_to_value(n),
                    NamedNodePattern::Variable(_) => {
                        return Err(EvalError::unsupported_deferred(
                            crate::error::UnsupportedKind::QuotedTripleTermVariable,
                            format!("variable predicate inside a quoted triple term in {SITE}"),
                        ));
                    }
                };
                let o = reference_pattern(&t.object)?;
                Ok(TermValue::Triple {
                    s: TermBox::new(s),
                    p: TermBox::new(p),
                    o: TermBox::new(o),
                })
            }
            TermPattern::Variable(_) => Err(EvalError::unsupported_deferred(
                crate::error::UnsupportedKind::QuotedTripleTermVariable,
                format!("variable inside a quoted triple term in {SITE}"),
            )),
        }
    }

    fn reference_ground(term: &GroundTerm) -> TermValue {
        match term {
            GroundTerm::NamedNode(n) => named_node_to_value(n),
            GroundTerm::Literal(l) => literal_to_value(l),
            GroundTerm::Triple(t) => TermValue::Triple {
                s: TermBox::new(reference_ground(&t.subject)),
                p: TermBox::new(named_node_to_value(&t.predicate)),
                o: TermBox::new(reference_ground(&t.object)),
            },
            GroundTerm::BlankNode(b) => {
                let (label, scope) = purrdf_core::BlankScope::unqualify_label(b.as_str());
                TermValue::Blank {
                    label: label.into_owned(),
                    scope,
                }
            }
        }
    }

    // ── Generated shapes ───────────────────────────────────────────────────────────

    /// A deterministic choice sequence.
    struct Choices {
        state: purrdf_testkit::rng::SplitMix64,
        budget: usize,
    }

    impl Choices {
        const fn new(seed: u64) -> Self {
            Self {
                state: purrdf_testkit::rng::SplitMix64::new(seed),
                budget: 12,
            }
        }

        fn choose(&mut self, n: usize) -> usize {
            self.state.below_usize(n)
        }

        fn spend(&mut self) -> bool {
            if self.budget == 0 {
                return false;
            }
            self.budget -= 1;
            true
        }
    }

    fn iri(local: &str) -> NamedNode {
        NamedNode::new_unchecked(format!("{EX}{local}"))
    }

    /// A term pattern: one leaf in twelve is a variable, so most generated shapes convert
    /// and some err at a nested position.
    fn pattern(choices: &mut Choices) -> TermPattern {
        if !choices.spend() || choices.choose(3) == 0 {
            return match choices.choose(12) {
                0..=4 => TermPattern::NamedNode(iri("a")),
                5..=7 => TermPattern::Literal(Literal::new_simple("x")),
                8..=10 => TermPattern::BlankNode(BlankNode::new("b1")),
                _ => TermPattern::Variable(Variable::new("v")),
            };
        }
        TermPattern::Triple(Child::new(TriplePattern {
            subject: pattern(choices),
            predicate: if choices.choose(12) == 0 {
                NamedNodePattern::Variable(Variable::new("p"))
            } else {
                NamedNodePattern::NamedNode(iri("p"))
            },
            object: pattern(choices),
        }))
    }

    fn ground(choices: &mut Choices) -> GroundTerm {
        if !choices.spend() || choices.choose(3) == 0 {
            return match choices.choose(4) {
                0 => GroundTerm::NamedNode(iri("a")),
                1 => GroundTerm::Literal(Literal::new_simple("x")),
                2 => GroundTerm::BlankNode(BlankNode::new("b1")),
                _ => GroundTerm::BlankNode(BlankNode::new("purrdfesc1_a")),
            };
        }
        GroundTerm::Triple(Child::new(GroundTriple {
            subject: ground(choices),
            predicate: iri("p"),
            object: ground(choices),
        }))
    }

    /// How many triple terms `value`'s object chain nests, read with a loop.
    fn nesting(value: &TermValue) -> usize {
        let mut levels = 0;
        let mut term = value;
        while let TermValue::Triple { o, .. } = term {
            levels += 1;
            term = o;
        }
        levels
    }

    // ── The tests ──────────────────────────────────────────────────────────────────

    /// Both conversions answer exactly what their recursive references answer — the
    /// value, or the error naming the site — for every generated shape, and the shapes
    /// reach nested triples, variable leaves and variable predicates.
    #[test]
    fn the_conversions_agree_with_their_recursive_references_on_generated_shapes() {
        let mut converted = 0;
        let mut refused = 0;
        let mut nested = 0;
        for seed in 0..400_u64 {
            let mut choices = Choices::new(seed);
            let shape = pattern(&mut choices);
            let ours = ground_term_pattern_to_value(&shape, SITE);
            let theirs = reference_pattern(&shape);
            assert_eq!(
                ours.as_ref().map_err(ToString::to_string),
                theirs.as_ref().map_err(ToString::to_string),
                "seed {seed}: {shape:?}"
            );
            match ours {
                Ok(value) => {
                    converted += 1;
                    nested += usize::from(nesting(&value) >= 2);
                }
                Err(_) => refused += 1,
            }

            let mut choices = Choices::new(seed.wrapping_mul(7919));
            let shape = ground(&mut choices);
            assert_eq!(
                ground_term_to_value(&shape),
                reference_ground(&shape),
                "seed {seed}: {shape:?}"
            );
        }
        assert!(
            converted > 0 && refused > 0,
            "{converted} converted, {refused} refused"
        );
        assert!(
            nested > 0,
            "some converted term nests a triple term in a triple term"
        );
    }

    /// A pattern and a `VALUES` cell a hundred thousand triple terms deep convert whole
    /// on a 128 KiB thread, and the same pattern with a variable at its innermost object
    /// is refused, naming the site, without walking off the stack either.
    #[test]
    fn a_hundred_thousand_level_term_converts_on_a_128_kib_thread() {
        purrdf_stack::on_stack(SMALL_STACK, || {
            let mut deep = TermPattern::NamedNode(iri("o"));
            let mut variable = TermPattern::Variable(Variable::new("o"));
            let mut cell = GroundTerm::NamedNode(iri("o"));
            for _ in 0..DEPTH {
                deep = TermPattern::Triple(Child::new(TriplePattern {
                    subject: TermPattern::NamedNode(iri("s")),
                    predicate: NamedNodePattern::NamedNode(iri("p")),
                    object: deep,
                }));
                variable = TermPattern::Triple(Child::new(TriplePattern {
                    subject: TermPattern::NamedNode(iri("s")),
                    predicate: NamedNodePattern::NamedNode(iri("p")),
                    object: variable,
                }));
                cell = GroundTerm::Triple(Child::new(GroundTriple {
                    subject: GroundTerm::NamedNode(iri("s")),
                    predicate: iri("p"),
                    object: cell,
                }));
            }
            let value = ground_term_pattern_to_value(&deep, SITE).expect("a ground pattern");
            assert_eq!(nesting(&value), DEPTH);
            drop(value);
            let error = ground_term_pattern_to_value(&variable, SITE)
                .expect_err("a variable at the innermost object");
            assert!(error.to_string().contains(SITE), "{error}");
            let value = ground_term_to_value(&cell);
            assert_eq!(nesting(&value), DEPTH);
            drop(value);
            drop(deep);
            drop(variable);
            drop(cell);
        })
        .expect("spawn");
    }
}
