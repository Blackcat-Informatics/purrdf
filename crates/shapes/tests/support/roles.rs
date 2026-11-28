// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Box-role fixtures for the SHACL integration tests.

// The module is included into more than one integration-test binary, and no single binary
// uses every helper; an unused-here helper is used there.
#![allow(dead_code, unreachable_pub)]

use purrdf_shapes::term::NamedNode;

/// A role vector as plain strings, for content comparison against an expected
/// list that a reader can check by eye.
pub fn role_iris(roles: &[NamedNode]) -> Vec<String> {
    roles.iter().map(|role| role.as_str().to_owned()).collect()
}
