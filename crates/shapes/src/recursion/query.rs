// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Dependencies in SPARQL node expressions, read from the production algebra.

use purrdf_lex::allocation::{Memory, Resident};
use purrdf_lex::walk::WorkList;
use purrdf_sparql_algebra::{
    Expression, Function, GraphPattern, NodeRef, ParseError, Query, SparqlParser,
};

use crate::error::{RecursionPolarity, ShapesError};
use crate::model::shnex;

pub(crate) enum Dependency<'a> {
    Shape(&'a str),
    Function(&'a str),
}

pub(crate) fn visit(
    text: &str,
    variable: &str,
    polarity: RecursionPolarity,
    mut dependency: impl FnMut(Dependency<'_>, RecursionPolarity) -> Result<(), ShapesError>,
) -> Result<(), ShapesError> {
    let parser = SparqlParser::new();
    let options = crate::sparql::current_parser_options();
    let parsed = match options.as_deref() {
        Some(options) => parser.parse_query_with(text, options),
        None => parser.parse_query(text),
    }
    .map_err(|error| match error {
        ParseError::Storage(error) => ShapesError::Storage(error),
        error => ShapesError::Invalid(format!("SPARQL dependency admission: {error}")),
    })?;
    let Query::Select { pattern, .. } = &parsed else {
        return Err(ShapesError::Invalid(
            "a SPARQL node expression requires a SELECT query".to_owned(),
        ));
    };
    // Only this exact single-row projection transfers its output's truth law.
    // Filters, joins, grouping and slicing can change cardinality/vacuous truth;
    // their calls remain Both until their own transfer is proved.
    let root = match projection(pattern, variable) {
        Some(expression) => (NodeRef::Expr(expression), polarity),
        None => (NodeRef::Pattern(pattern), unknown(polarity)),
    };
    let mut pending = WorkList::<_, 8>::with(root);
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    while let Some((node, polarity)) = pending.pop() {
        if let NodeRef::Expr(Expression::FunctionCall(Function::Custom(function), args)) = node {
            if function.as_str() == shnex::CONFORMS_TO_SHAPE {
                if args.len() == 2
                    && let Some(Expression::NamedNode(shape)) = args.get(1)
                {
                    dependency(Dependency::Shape(shape.as_str()), polarity)?;
                }
            } else {
                dependency(Dependency::Function(function.as_str()), polarity)?;
            }
        }
        let mut index = 0usize;
        let mut failure = None;
        node.for_each_child(|child| {
            let child_polarity = match node {
                NodeRef::Expr(Expression::Not(_)) => polarity.inverted(),
                NodeRef::Expr(Expression::And(_) | Expression::Or(_)) => polarity,
                NodeRef::Expr(Expression::If(_, _, _)) if index > 0 => polarity,
                _ => unknown(polarity),
            };
            index += 1;
            if failure.is_none() {
                failure = pending
                    .try_push_admitted((child, child_polarity), &mut memory)
                    .err();
            }
        });
        if let Some(error) = failure {
            return Err(ShapesError::Storage(error));
        }
    }
    Ok(())
}

fn projection<'a>(pattern: &'a GraphPattern, variable: &str) -> Option<&'a Expression> {
    let GraphPattern::Project { inner, variables } = pattern else {
        return None;
    };
    if variables.len() != 1 || variables[0].as_str() != variable {
        return None;
    }
    let GraphPattern::Extend {
        inner,
        variable: bound,
        expression,
    } = &**inner
    else {
        return None;
    };
    (bound.as_str() == variable && inner.is_empty_bgp()).then_some(expression)
}

fn unknown(polarity: RecursionPolarity) -> RecursionPolarity {
    if polarity == RecursionPolarity::Target {
        polarity
    } else {
        RecursionPolarity::Both
    }
}
