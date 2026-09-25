// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The SHACL 1.2 spec symbol table, and the linker that binds a shapes graph's
//! declarations against it.
//!
//! # Declarations are signatures; the engine provides implementations
//!
//! The W3C SHACL 1.2 vocabularies — `shacl.ttl`, `shnex.ttl` and
//! `shnex-sparql.ttl` — DECLARE every built-in constraint component and every
//! built-in node-expression function: `sh:SPARQLExprExpression a
//! sh:NamedParameterExpressionFunction` with its `sh:parameter` list,
//! `sparql:abs a sh:ListParameterExpressionFunction`,
//! `sh:MinCountConstraintComponent a sh:ConstraintComponent`. A shapes graph that
//! merges those vocabularies carries the declarations, and none of them has a
//! `sh:bodyExpression` or a validator — the implementation is the engine's.
//!
//! So a declaration is a SIGNATURE, and loading a shapes graph resolves symbols.
//! The table ([`implemented`]) lists every term this engine implements, and the
//! linker (run by the custom-function discovery and by the constraint-component
//! registry, on both the fresh-parse and the prepared-product restore path) gives
//! every declaration exactly one outcome:
//!
//! | Declaration | Outcome |
//! |---|---|
//! | a native IRI, bare | binds natively; no custom-function or component-registry entry |
//! | a native IRI carrying `sh:bodyExpression`, a validator, `sh:ask` or `sh:select` | load error: duplicate definition |
//! | a native IRI under another declaring class, or stating a parameter or key the native signature does not have | load error: kind or signature mismatch |
//! | a non-native IRI with no body / validator | load error (functions); ignored as SHACL-SPARQL requires (components) |
//! | a custom named function keyed by node-expression vocabulary | load error: key clash |
//!
//! Nativeness is decided by IMPLEMENTATION, never by namespace: `sh:NotABuiltin`
//! is as custom as `ex:f`, and a `sparql:<NAME>` IRI is native exactly when
//! [`crate::expression::sparql_ns_lowering`] resolves `NAME` — the one resolver the
//! call sites already use, not a second list.
//!
//! A declared-but-unimplemented component (see [`ComponentStatus::Unimplemented`])
//! binds as such: its bare declaration loads, and a shape that USES one of its
//! parameters is a load error naming it, instead of silently conforming.
//!
//! # SPARQL registration (SHACL 1.2 SPARQL Extensions §7.3)
//!
//! The specification's registration rule, verbatim: "SPARQL engines SHOULD
//! register a function for any SHACL instance of
//! sh:ListParameterExpressionFunction from any provided shapes graph. If a
//! function with the same IRI is already registered, SHACL engines MUST ignore
//! the attempt to redefine it unless the function was previously added as a custom
//! SPARQL function."
//!
//! A built-in list-parameter function the shapes graph declares — the vocabulary's
//! `shnex:conformsToShape`, or any `sparql:<NAME>` — is therefore registered with
//! its NATIVE implementation, so `shnex:conformsToShape(?x, ex:S)` is callable
//! from `sh:select` and `sh:sparqlExpr`. A custom list-parameter function keeps its
//! body-evaluating registration. An IRI a host has already registered is not
//! redefined.
//!
//! # The ratchet
//!
//! [`declared`] reads the vendored vocabularies (`crates/shapes/spec/`, shipped
//! inside the package) and [`implemented`] is the table; a test asserts the two
//! agree exactly, component by component, function by function, parameter by
//! parameter.

pub mod census;
mod link;
mod table;
mod vocab;

use std::collections::BTreeSet;
use std::sync::LazyLock;

pub(crate) use link::{
    bind_native_function, bind_spec_component, refuse_component_declared_function,
    refuse_function_declared_component,
};
pub(crate) use table::ExprKind;
pub use table::{
    AliasSource, Carrier, ComponentParam, ComponentRow, ComponentStatus, FunctionClass,
    FunctionParam, FunctionRow, KeyAlias, SPEC_TEXT_OPTIONALITY, SparqlAlias, TargetRow, ValueRule,
};
pub use vocab::{DeclaredFunction, DeclaredParam, Vocabulary, declared, declared_terms};

use crate::expression::SparqlCallForm;
use crate::model::sparql_ns;
use table::{AliasRole, Implementation};

/// The table of every spec term this engine implements.
#[must_use]
pub const fn implemented() -> Implemented {
    Implemented { _private: () }
}

/// A read-only handle on the spec symbol table. See the [module docs](self).
#[derive(Clone, Copy, Debug)]
pub struct Implemented {
    _private: (),
}

impl Implemented {
    /// Every built-in `sh:` / `shnex:` node-expression function row.
    #[must_use]
    pub fn functions(&self) -> &'static [FunctionRow] {
        table::FUNCTIONS
    }

    /// Every declared constraint component, with the engine's status for it.
    #[must_use]
    pub fn components(&self) -> &'static [ComponentRow] {
        table::COMPONENTS
    }

    /// The accepted SHACL Advanced Features spellings of node-expression keys.
    #[must_use]
    pub fn key_aliases(&self) -> &'static [KeyAlias] {
        table::KEY_ALIASES
    }

    /// The accepted alias spellings of SPARQL function names.
    #[must_use]
    pub fn sparql_aliases(&self) -> &'static [SparqlAlias] {
        table::SPARQL_ALIASES
    }

    /// The built-in severities.
    #[must_use]
    pub fn severities(&self) -> &'static [&'static str] {
        table::SEVERITIES
    }

    /// The implemented target predicates.
    #[must_use]
    pub fn targets(&self) -> &'static [TargetRow] {
        table::TARGETS
    }

    /// The implemented rule types.
    #[must_use]
    pub fn rule_types(&self) -> &'static [&'static str] {
        table::RULE_TYPES
    }

    /// The signature `iri` binds to natively, if the engine implements it: a table
    /// row, or a `sparql:<NAME>` the SPARQL lowering resolves (whose vocabulary
    /// declaration states no parameter).
    #[must_use]
    pub fn function(&self, iri: &str) -> Option<(FunctionClass, &'static [FunctionParam])> {
        native_function(iri).map(|native| (native.class(), native.params()))
    }

    /// The row for component `iri`, if the vocabulary declares it.
    #[must_use]
    pub fn component(&self, iri: &str) -> Option<&'static ComponentRow> {
        component(iri)
    }

    /// The canonical text of the whole table, one fact per line.
    ///
    /// A prepared shapes product's stage id folds this in, so a product prepared
    /// under one table is refused by a build whose table says something else.
    #[must_use]
    pub fn canonical_text(&self) -> String {
        let mut out = String::new();
        for line in table::canonical_lines() {
            out.push_str(&line);
            out.push('\n');
        }
        out
    }
}

// ── Crate-internal lookups ────────────────────────────────────────────────────

/// A built-in function a declaration can bind to.
#[derive(Clone, Copy, Debug)]
pub(crate) enum NativeFunction {
    /// A `sh:` / `shnex:` table row.
    Row(&'static FunctionRow),
    /// A `sparql:<NAME>` the SPARQL lowering resolves.
    Sparql,
}

impl NativeFunction {
    /// The declaring class the built-in has.
    pub(crate) const fn class(self) -> FunctionClass {
        match self {
            Self::Row(row) => row.class,
            Self::Sparql => FunctionClass::ListParameter,
        }
    }

    /// The built-in's signature.
    pub(crate) const fn params(self) -> &'static [FunctionParam] {
        match self {
            Self::Row(row) => row.params,
            Self::Sparql => &[],
        }
    }
}

/// The built-in function `iri` names, if any.
pub(crate) fn native_function(iri: &str) -> Option<NativeFunction> {
    if let Some(local) = iri.strip_prefix(sparql_ns::NS) {
        return crate::expression::sparql_ns_lowering(local)
            .ok()
            .map(|_| NativeFunction::Sparql);
    }
    table::FUNCTIONS
        .iter()
        .find(|row| row.iri == iri)
        .map(NativeFunction::Row)
}

/// The component row `iri` names, if the vocabulary declares it.
pub(crate) fn component(iri: &str) -> Option<&'static ComponentRow> {
    table::COMPONENTS.iter().find(|row| row.iri == iri)
}

/// The SPARQL surface form an alias spelling of a SPARQL function name lowers to.
pub(crate) fn sparql_alias(local: &str) -> Option<SparqlCallForm> {
    table::SPARQL_ALIASES
        .iter()
        .find(|alias| alias.local == local)
        .map(|alias| alias.form)
}

/// Every IRI that selects a node-expression kind, with the kind it selects:
/// every native function's dispatching key, every AF primary alias, and the
/// argument reference.
///
/// Both spec surfaces appear here, so the parser's single "exactly one key" check
/// rejects a node carrying two DIFFERENT kinds and a node carrying two spellings
/// of the SAME kind with the same message.
pub(crate) fn primary_keys() -> &'static [(&'static str, ExprKind)] {
    static KEYS: LazyLock<Vec<(&'static str, ExprKind)>> = LazyLock::new(|| {
        let mut keys: Vec<(&'static str, ExprKind)> = Vec::new();
        for alias in table::KEY_ALIASES {
            if let AliasRole::Primary(kind) = alias.role {
                keys.push((alias.iri, kind));
            }
        }
        for row in table::FUNCTIONS {
            if let Implementation::Keyed { key, kind } = row.implementation {
                keys.push((key, kind));
            }
        }
        keys.push(table::ARGUMENT_REFERENCE);
        keys
    });
    &KEYS
}

/// Every vocabulary term that structures a node expression — none of them can be
/// the predicate of a function-call expression, and none can key a custom
/// named-parameter function.
///
/// The union of every native function's parameters (list-parameter functions
/// contribute their own IRI, which is their call key), every AF alias, and the
/// argument reference.
pub(crate) fn non_function_keys() -> &'static [&'static str] {
    static KEYS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
        let mut keys: BTreeSet<&'static str> = BTreeSet::new();
        for row in table::FUNCTIONS {
            keys.extend(row.params.iter().map(|p| p.path));
            if let Implementation::Keyed { key, .. } = row.implementation {
                keys.insert(key);
            }
        }
        keys.extend(table::KEY_ALIASES.iter().map(|alias| alias.iri));
        keys.insert(table::ARGUMENT_REFERENCE.0);
        keys.into_iter().collect()
    });
    &KEYS
}

/// Why `path` cannot key a custom named-parameter function, if it cannot: it is a
/// key parameter of a built-in, an AF alias of one, or other node-expression
/// vocabulary a call site is never recognised by.
pub(crate) fn reserved_key(path: &str) -> Option<String> {
    for row in table::FUNCTIONS {
        if row.params.iter().any(|p| p.key && p.path == path)
            || matches!(row.implementation, Implementation::Keyed { key, .. } if key == path)
        {
            return Some(format!("a key parameter of the built-in <{}>", row.iri));
        }
    }
    if let Some(alias) = table::KEY_ALIASES.iter().find(|alias| alias.iri == path) {
        return Some(match alias.role {
            AliasRole::Primary(kind) => format!(
                "the SHACL Advanced Features spelling of the built-in {} key",
                kind.label()
            ),
            AliasRole::Operand => {
                "a SHACL Advanced Features node-expression operand, never a call key".to_owned()
            }
        });
    }
    if non_function_keys().contains(&path) {
        return Some("built-in node-expression vocabulary, never a call key".to_owned());
    }
    None
}

/// The target predicates the engine implements, in table order.
pub(crate) fn target_predicates() -> impl Iterator<Item = &'static str> {
    table::TARGETS.iter().map(|target| target.predicate)
}

/// Every constraint parameter a shape has at most one value for, in table order
/// and without repeats (a parameter shared by two components appears once).
pub(crate) fn single_valued_params() -> &'static [&'static str] {
    static PARAMS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
        let mut out: Vec<&'static str> = Vec::new();
        for row in table::COMPONENTS {
            for p in row.params {
                if p.single && !out.contains(&p.path) {
                    out.push(p.path);
                }
            }
        }
        out
    });
    &PARAMS
}

/// The parameters of every declared component this engine does not evaluate,
/// each with its component.
pub(crate) fn unimplemented_component_params()
-> impl Iterator<Item = (&'static str, &'static ComponentRow)> {
    table::COMPONENTS
        .iter()
        .filter(|row| row.status == ComponentStatus::Unimplemented)
        .flat_map(|row| row.params.iter().map(move |p| (p.path, row)))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{non_function_keys, primary_keys, reserved_key};
    use crate::model::{rdf, sh, shnex};

    /// The keys the parser dispatched on before the table existed, spelled out by
    /// hand: deriving them from the table must neither add nor drop one.
    #[test]
    fn derived_primary_keys_are_exactly_the_accepted_keys() {
        let expected: BTreeSet<&str> = [
            sh::PATH,
            sh::FILTER_SHAPE,
            sh::UNION,
            sh::INTERSECTION,
            sh::IF,
            sh::COUNT,
            sh::DISTINCT,
            sh::MIN,
            sh::MAX,
            sh::SUM,
            sh::EXISTS,
            shnex::PATH_VALUES,
            shnex::FILTER_SHAPE,
            shnex::INTERSECTION,
            shnex::CONCAT,
            shnex::IF,
            shnex::COUNT,
            shnex::DISTINCT,
            shnex::MIN,
            shnex::MAX,
            shnex::SUM,
            shnex::EXISTS,
            shnex::VAR,
            rdf::FIRST,
            shnex::REMOVE,
            shnex::LIMIT,
            shnex::OFFSET,
            shnex::ORDER_BY,
            shnex::FLAT_MAP,
            shnex::FIND_FIRST,
            shnex::MATCH_ALL,
            shnex::INSTANCES_OF,
            shnex::NODES_MATCHING,
            shnex::CONFORMS_TO_SHAPE,
            shnex::ARG,
            sh::SELECT,
            sh::SPARQL_EXPR,
        ]
        .into_iter()
        .collect();
        let derived: BTreeSet<&str> = primary_keys().iter().map(|&(iri, _)| iri).collect();
        assert_eq!(derived, expected);
        assert_eq!(derived.len(), primary_keys().len(), "no key twice");
    }

    /// Likewise for the structural vocabulary a call site can never be keyed by.
    #[test]
    fn derived_non_function_keys_are_exactly_the_structural_vocabulary() {
        let expected: BTreeSet<&str> = [
            sh::PATH,
            sh::FILTER_SHAPE,
            sh::NODES,
            sh::UNION,
            sh::INTERSECTION,
            sh::IF,
            sh::THEN,
            sh::ELSE,
            sh::COUNT,
            sh::DISTINCT,
            sh::MIN,
            sh::MAX,
            sh::SUM,
            sh::LIMIT,
            sh::OFFSET,
            sh::ORDERBY,
            sh::DESC,
            sh::EXISTS,
            shnex::VAR,
            shnex::PATH_VALUES,
            shnex::FOCUS_NODE,
            shnex::EXISTS,
            shnex::IF,
            shnex::THEN,
            shnex::ELSE,
            shnex::DISTINCT,
            shnex::INTERSECTION,
            shnex::CONCAT,
            shnex::REMOVE,
            shnex::NODES,
            shnex::FILTER_SHAPE,
            shnex::LIMIT,
            shnex::OFFSET,
            shnex::ORDER_BY,
            shnex::DESC,
            shnex::FLAT_MAP,
            shnex::FIND_FIRST,
            shnex::MATCH_ALL,
            shnex::COUNT,
            shnex::MIN,
            shnex::MAX,
            shnex::SUM,
            shnex::INSTANCES_OF,
            shnex::NODES_MATCHING,
            shnex::CONFORMS_TO_SHAPE,
            shnex::ARG,
            sh::SELECT,
            sh::SPARQL_EXPR,
            sh::PREFIXES,
            rdf::FIRST,
            rdf::REST,
        ]
        .into_iter()
        .collect();
        let derived: BTreeSet<&str> = non_function_keys().iter().copied().collect();
        assert_eq!(derived, expected);
    }

    /// Every structural key is reserved against custom functions; an application
    /// IRI is not.
    #[test]
    fn every_structural_key_is_reserved_and_an_application_iri_is_not() {
        for key in non_function_keys() {
            assert!(reserved_key(key).is_some(), "{key}");
        }
        assert!(reserved_key("http://example.org/ns#myCount").is_none());
    }

    /// The severities and target predicates the engine reads are the table's.
    #[test]
    fn severity_rows_are_the_built_in_severities() {
        for severity in super::implemented().severities() {
            assert!(crate::report::Severity::from_iri(severity).is_some());
        }
        assert_eq!(super::implemented().severities().len(), 3);
    }
}
