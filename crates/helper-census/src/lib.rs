// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shared Rust source-scope inspection for host-side structural gates.

/// Whether one parsed `cfg` predicate is exactly `test`. Compound predicates
/// remain the responsibility of the caller's explicitly selected cfg policy.
#[must_use]
pub fn is_test_predicate(predicate: &syn::Meta) -> bool {
    matches!(predicate, syn::Meta::Path(path) if path.is_ident("test"))
}

/// Whether attributes carry exactly `#[cfg(test)]` or `#![cfg(test)]`.
/// This does not classify `#[test]` functions or evaluate compound predicates.
#[must_use]
pub fn is_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg")
            && attr
                .parse_args::<syn::Meta>()
                .is_ok_and(|predicate| is_test_predicate(&predicate))
    })
}
