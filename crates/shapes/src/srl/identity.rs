// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Rule identities use the existing SHACL model codec and algebra token walk.

use super::ir::{Element, IrRuleBody, PatternTerm, RuleSet, Scheduling, TriplePattern};
use purrdf_hash::{blake3::Hasher, hex::Digest32};
use purrdf_lex::allocation::{Memory, Resident};

const PROGRAM: purrdf_hash::Domain = purrdf_hash::Domain::new(b"purrdf-shapes/rule-program/v1");

struct Encoder(Hasher);
impl Encoder {
    fn field(&mut self, value: &[u8]) {
        purrdf_hash::frame::frame_le_with(value, &mut |bytes| {
            self.0.update(bytes);
        });
    }
    fn count(&mut self, count: usize) {
        self.field(&(count as u64).to_le_bytes());
    }
    fn pattern(&mut self, pattern: &TriplePattern) {
        let mut pending = vec![&pattern.object, &pattern.predicate, &pattern.subject];
        while let Some(term) = pending.pop() {
            match term {
                PatternTerm::Variable(name) => {
                    self.field(b"variable");
                    self.field(name.as_bytes());
                }
                PatternTerm::BlankNode(name) => {
                    self.field(b"blank");
                    self.field(name.as_bytes());
                }
                PatternTerm::Term(term) => {
                    self.field(b"term");
                    self.field(term.to_string().as_bytes());
                }
                PatternTerm::Triple(triple) => {
                    self.field(b"triple");
                    pending.extend([&triple.object, &triple.predicate, &triple.subject]);
                }
            }
        }
    }
    fn expression(
        &mut self,
        expression: &purrdf_sparql_algebra::Expression,
    ) -> Result<(), crate::error::ShapesError> {
        let identity = purrdf_sparql_algebra::identity::node_with_memory(
            purrdf_sparql_algebra::NodeRef::Expr(expression),
            &mut Memory::new(&mut Resident),
        )?;
        self.field(identity.as_bytes());
        Ok(())
    }
    fn elements(&mut self, elements: &[Element]) -> Result<(), crate::error::ShapesError> {
        self.count(elements.len());
        let mut pending: Vec<_> = elements.iter().rev().collect();
        while let Some(element) = pending.pop() {
            match element {
                Element::Pattern(pattern) => {
                    self.field(b"pattern");
                    self.pattern(pattern);
                }
                Element::Filter(expression) => {
                    self.field(b"filter");
                    self.expression(expression)?;
                }
                Element::Assign {
                    variable,
                    expression,
                } => {
                    self.field(b"assign");
                    self.field(variable.as_bytes());
                    self.expression(expression)?;
                }
                Element::Negation { elements, data } => {
                    self.field(b"negation");
                    self.field(&[u8::from(*data)]);
                    self.count(elements.len());
                    pending.extend(elements.iter().rev());
                }
            }
        }
        Ok(())
    }
}

pub(super) fn program(
    set: &RuleSet<'_>,
    shapes: &crate::shapes::Shapes,
) -> Result<Digest32, crate::error::ShapesError> {
    let mut encoder = Encoder(Hasher::new());
    encoder.field(PROGRAM.as_bytes());
    encoder.field(match set.scheduling {
        Scheduling::Declared => b"declared",
        Scheduling::Stratified => b"stratified",
    });
    let producers: Vec<_> = set
        .rules
        .iter()
        .filter_map(|rule| match &rule.body {
            IrRuleBody::Shacl(producer) => Some(producer),
            IrRuleBody::Elements(_) => None,
        })
        .collect();
    let model = crate::product::ast::encode_rule_model(shapes, &producers).map_err(|error| {
        crate::error::ShapesError::internal(format!("rule model identity: {error}"))
    })?;
    encoder.field(&model);
    let options = &shapes.validation_options;
    encoder.field(options.shacl_profile.id().as_bytes());
    encoder.field(&[u8::from(options.subclass_of_in_shapes_graph)]);
    let disallows = options.conformance_disallows.iris();
    encoder.count(disallows.len());
    for iri in disallows {
        encoder.field(iri.as_bytes());
    }
    match options.recursion {
        None => encoder.field(b"no-recursion"),
        Some(recursion) => {
            encoder.field(match recursion.semantics {
                crate::recursion::RecursionSemantics::Lfp => b"lfp",
                crate::recursion::RecursionSemantics::Gfp => b"gfp",
            });
            encoder.field(&recursion.max_pairs.to_le_bytes());
            encoder.field(&recursion.max_iterations.to_le_bytes());
        }
    }
    match options.xpath_regex {
        None => encoder.field(b"no-xpath"),
        Some((profile, limits)) => {
            use purrdf_core::xsd_regex::xpath::Resource;
            encoder.field(profile.name().as_bytes());
            for resource in [
                Resource::PatternBytes,
                Resource::CompileSteps,
                Resource::ProgramNodes,
                Resource::CompileSlots,
                Resource::MatchSteps,
                Resource::MatchStates,
                Resource::MatchSlots,
                Resource::OutputBytes,
            ] {
                encoder.field(resource.code().as_bytes());
                encoder.field(&limits.limit(resource).to_le_bytes());
            }
        }
    }
    encoder.count(set.rules.len());
    for rule in &set.rules {
        encoder.field(rule.id.to_string().as_bytes());
        encoder.field(&rule.schedule.layer.value().to_bits().to_le_bytes());
        encoder.field(&rule.schedule.order.value().to_bits().to_le_bytes());
        encoder.field(&[u8::from(rule.schedule.run_once)]);
        encoder.count(rule.expected_predicates.len());
        for predicate in &rule.expected_predicates {
            encoder.field(predicate.as_str().as_bytes());
        }
        match &rule.body {
            IrRuleBody::Shacl(_) => encoder.field(b"shacl"),
            IrRuleBody::Elements(rule) => {
                encoder.field(b"elements");
                encoder.field(&[u8::from(rule.data)]);
                encoder.count(rule.head.len());
                for pattern in &rule.head {
                    encoder.pattern(pattern);
                }
                encoder.elements(&rule.body)?;
            }
        }
    }
    encoder.count(set.data.len());
    for triple in &set.data {
        for term in triple {
            encoder.field(term.to_string().as_bytes());
        }
    }
    Ok(Digest32::from(*encoder.0.finalize().as_bytes()))
}
