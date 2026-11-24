// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Generated admission and fixed-point checks through native SHACL validation.

use std::fmt::Write;

use purrdf_shapes::engine::{ValidationOptions, validate_graphs_with_options};
use purrdf_shapes::recursion::{RecursionOptions, RecursionSemantics};
use purrdf_shapes::{RecursionRefusalReason, ShapesError, ShapesImports};
use purrdf_testkit::prop::prelude::*;

const PREFIXES: &str = "@prefix : <https://example.org/> .
    @prefix sh: <http://www.w3.org/ns/shacl#> .";

fn validate(catalog: &str, selection: Option<RecursionSemantics>) -> Result<bool, ShapesError> {
    let options = ValidationOptions::default();
    let options = if let Some(semantics) = selection {
        options.with_recursion(RecursionOptions {
            semantics,
            max_pairs: 1_024,
            max_iterations: 1_024,
        })
    } else {
        options
    };
    validate_graphs_with_options("", catalog, None, &options, &ShapesImports::new())
        .map(|report| report.conforms)
}

fn signed_catalog(negative_edges: &[bool]) -> String {
    let mut catalog = format!("{PREFIXES} :s0 sh:targetNode :focus .");
    for (shape, negative) in negative_edges.iter().copied().enumerate() {
        let successor = (shape + 1) % negative_edges.len();
        let predicate = if negative { "not" } else { "node" };
        write!(
            catalog,
            " :s{shape} a sh:NodeShape ; sh:{predicate} :s{successor} ."
        )
        .expect("write Turtle to String");
    }
    catalog
}

fn positive_catalog(equations: &[(bool, u8, bool)], dual: bool) -> String {
    let mut catalog = format!(
        "{PREFIXES} :s0 sh:targetNode :focus .
         :yes a sh:NodeShape ; sh:nodeKind sh:IRI .
         :no a sh:NodeShape ; sh:nodeKind sh:Literal ."
    );
    for (shape, &(conjunction, extra, constant)) in equations.iter().enumerate() {
        let successor = (shape + 1) % equations.len();
        let extra = usize::from(extra) % equations.len();
        let operator = if conjunction ^ dual { "and" } else { "or" };
        let constant = if constant ^ dual { "yes" } else { "no" };
        write!(
            catalog,
            " :s{shape} a sh:NodeShape ; sh:{operator} (:s{successor} :s{extra} :{constant}) ."
        )
        .expect("write Turtle to String");
    }
    catalog
}

fn fixed_models_at_target(equations: &[(bool, u8, bool)]) -> (bool, bool) {
    let at = |model: u16, index| model & (1_u16 << index) != 0;
    let model_limit = 1_u16 << equations.len();
    let fixed = (0..model_limit).filter(|&model| {
        equations
            .iter()
            .enumerate()
            .all(|(shape, &(and, extra, constant))| {
                let successor = at(model, (shape + 1) % equations.len());
                let extra = at(model, usize::from(extra) % equations.len());
                let evaluated = if and {
                    successor && extra && constant
                } else {
                    successor || extra || constant
                };
                evaluated == at(model, shape)
            })
    });
    // A monotone system's least and greatest models are fixed points themselves.
    // Enumerating fixed models gives the target's value at both extremes.
    fixed.fold((true, false), |(least, greatest), model| {
        (least && at(model, 0), greatest || at(model, 0))
    })
}

prop_test! {
    #![prop_config(Config::with_env_cases(64))]

    /// Every generated catalog contains a recursive component through the target.
    #[test]
    fn recursive_signed_catalogs_return_reports_or_typed_refusals(
        edges in prop::collection::vec(any::<bool>(), 1..9),
    ) {
        let catalog = signed_catalog(&edges);
        let default = validate(&catalog, None);
        prop_assert!(matches!(
            &default, Err(ShapesError::Recursion(refusal))
            if refusal.reason == RecursionRefusalReason::MissingSelection
        ), "{default:?}");
        for semantics in [RecursionSemantics::Lfp, RecursionSemantics::Gfp] {
            let report = validate(&catalog, Some(semantics));
            if edges.contains(&true) {
                prop_assert!(matches!(
                    &report, Err(ShapesError::Recursion(refusal))
                    if refusal.reason == RecursionRefusalReason::NonStratified { semantics }
                ), "{report:?}");
            } else {
                prop_assert_eq!(report.unwrap(), semantics == RecursionSemantics::Gfp);
            }
        }
    }

    /// De Morgan duality exchanges the least and greatest fixed points.
    #[test]
    fn positive_recursive_catalogs_obey_fixed_point_duality(
        equations in prop::collection::vec((any::<bool>(), any::<u8>(), any::<bool>()), 1..9),
    ) {
        let direct = positive_catalog(&equations, false);
        let dual = positive_catalog(&equations, true);
        let least = validate(&direct, Some(RecursionSemantics::Lfp)).unwrap();
        let greatest = validate(&direct, Some(RecursionSemantics::Gfp)).unwrap();
        let dual_least = validate(&dual, Some(RecursionSemantics::Lfp)).unwrap();
        let dual_greatest = validate(&dual, Some(RecursionSemantics::Gfp)).unwrap();
        prop_assert_eq!((least, greatest), fixed_models_at_target(&equations));
        prop_assert_eq!(least, !dual_greatest);
        prop_assert_eq!(greatest, !dual_least);
    }
}
