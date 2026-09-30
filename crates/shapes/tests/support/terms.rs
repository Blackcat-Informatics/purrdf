// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native SHACL terms the integration tests build by hand, in the two
//! `example.org` namespaces the fixtures use. A test imports the one it names
//! under its own local spelling (`use terms::ex_ns as ex;`).

// The module is included into more than one integration-test binary, and no single binary
// uses every helper; an unused-here helper is used there.
#![allow(dead_code, unreachable_pub)]

use purrdf_shapes::term::{Literal, NamedNode, Term};
use purrdf_xsd::datatype::XSD_INTEGER;

/// `<http://example.org/ns#{local}>`, the `ex:` namespace of the Turtle fixtures.
pub fn ex_ns(local: &str) -> Term {
    Term::NamedNode(NamedNode::new_unchecked(format!(
        "http://example.org/ns#{local}"
    )))
}

/// `<http://example.org/{local}>`.
pub fn example_org(local: &str) -> Term {
    Term::NamedNode(NamedNode::new_unchecked(format!(
        "http://example.org/{local}"
    )))
}

/// `value` as an `xsd:integer` literal.
pub fn integer(value: i64) -> Term {
    Term::Literal(Literal::new_typed_literal(
        value.to_string(),
        NamedNode::from(XSD_INTEGER),
    ))
}
