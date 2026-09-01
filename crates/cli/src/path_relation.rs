// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `--path-relation`: the CLI's property-function registration surface.
//!
//! A **path witness** relation binds not merely the endpoints of a traversal but the
//! traversal itself — every hop, in order, with the traversed statement as a first-class
//! RDF 1.2 term. The kernel type is
//! [`purrdf_sparql_eval::PathWitnessRelation`] (every simple-prefix walk) and its
//! polynomial sibling [`purrdf_sparql_eval::ShortestPathWitnessRelation`] (one shortest
//! witness per reachable pair); this module is the command-line spelling that reaches
//! them, and it is the ONLY property-function registration surface this binary has.
//!
//! A call reads
//!
//! ```text
//! ?start <caller-iri> ( ?end ?pathId ?len ?step ?node ?edge )
//! ```
//!
//! and emits ONE ROW PER HOP: row `i` of a `k`-hop walk binds `?len = k`, `?step = i`,
//! `?node` to the node hop `i` arrived at and `?edge` to the statement it traversed.
//! `GROUP BY ?pathId` reassembles one walk from its hop rows, and `ORDER BY ?step` puts
//! them back in traversal order (`?step` and `?len` are `xsd:integer` literals precisely
//! so that ordering is numeric).
//!
//! # The value grammar, and why every key is mandatory
//!
//! ```text
//! --path-relation 'iri=IRI;forward=IRI;inverse=IRI;min-hops=N;max-hops=N;\
//!                  max-paths-per-seed=N;max-expansions=N;mode=walk|shortest'
//! ```
//!
//! Semicolon-separated `key=value` pairs. `forward` and `inverse` may each repeat and at
//! least one must appear (they are the step's ordered alternation of directed
//! predicates); every other key appears exactly once and none of them has a default.
//!
//! That is not austerity, it is the two project rules meeting. **PurRDF mints no
//! vocabulary IRIs**, so `iri=` — the name a query spells in predicate position — is
//! caller-supplied with no default namespace to fall back on, exactly as
//! `--aggregate-namespace` and `--provenance-namespace` are. And
//! [`PathLimits`](purrdf_sparql_eval::PathLimits) deliberately has no `Default`: a
//! zero-hop path has no witness, and an unbounded traversal depth is a stack overflow,
//! which is an ABORT and so escapes the property-function seam's panic containment
//! entirely. A number this binary invented and the operator never read is precisely the
//! fabricated configuration the project forbids, so the envelope is stated every time.
//!
//! Every malformed spelling names the offending token. An unknown key, a missing
//! mandatory key, a repeated key, zero predicates, a repeated `(predicate, direction)`
//! pair, a relative IRI, a non-numeric or out-of-range count, and an unknown `mode` are
//! each their own message; nothing is coerced and nothing is defaulted.
//!
//! # Why the registry is built inside the view operation
//!
//! [`PathGraph::from_dataset`] snapshots the step's edges out of the dataset being
//! queried, so a registry cannot exist before the data source has been opened. The
//! command line is therefore parsed into [`PathRelationSpec`]s — pure, dataset-free
//! configuration — and [`build_registry`] turns them into relations inside
//! [`ViewOp::run`](crate::source::ViewOp::run), where the concrete view is in hand. That
//! also keeps the snapshot's lifetime equal to the view's, which is the pairing
//! `purrdf_sparql_eval::path_relation`'s own documentation requires of a host: a relation
//! answers about the dataset it was built from, and nothing at the seam can check that
//! for you.

use std::sync::Arc;

use purrdf_core::{DatasetView, GraphMatch, TermValue};
use purrdf_sparql_eval::{
    PathDirection, PathGraph, PathLimits, PathStep, PathWitnessRelation, PropertyFunctionRegistry,
    ShortestPathWitnessRelation,
};

use crate::error::CliError;

/// Which of the two relation types a spec registers.
///
/// Two types rather than one type with a mode flag is the kernel's own decision: the
/// planner reads cardinality off the relation, and "exponential" vs "polynomial" must be
/// a property of the registration rather than of a runtime value the planner cannot see.
/// This enum is only the command-line spelling of that choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PathRelationMode {
    /// [`PathWitnessRelation`]: every simple-prefix walk.
    Walk,
    /// [`ShortestPathWitnessRelation`]: one shortest witness per reachable pair.
    Shortest,
}

/// One `--path-relation` value, parsed and validated but not yet bound to a dataset.
///
/// Every field is required on the command line; there is no default for any of them.
#[derive(Debug, Clone)]
pub(crate) struct PathRelationSpec {
    /// The caller's IRI for the relation — the name a query spells in predicate
    /// position. PurRDF mints none.
    pub(crate) iri: String,
    /// The step's ordered alternation of directed predicates, in the order the
    /// `forward=` / `inverse=` keys appeared.
    steps: Vec<(String, PathDirection)>,
    /// `min-hops`: the shortest walk length the relation accepts (never zero).
    min_hops: u32,
    /// `max-hops`: the longest walk length the relation accepts.
    max_hops: u32,
    /// `max-paths-per-seed`: candidate walks one seed may enumerate before failing.
    max_paths_per_seed: u64,
    /// `max-expansions`: edges one invocation may traverse before failing.
    max_expansions: u64,
    /// `mode`: which of the two relation types to register.
    mode: PathRelationMode,
}

/// The keys the grammar accepts, in the order the help text lists them.
const KEYS: [&str; 8] = [
    "iri",
    "forward",
    "inverse",
    "min-hops",
    "max-hops",
    "max-paths-per-seed",
    "max-expansions",
    "mode",
];

/// Parse one `--path-relation` value, as clap's `value_parser`.
///
/// # Errors
///
/// A rendered message naming the offending token: an unknown key, a missing mandatory
/// key, a duplicated key, a spec with no `forward=`/`inverse=` predicate at all, a
/// repeated `(predicate, direction)` pair, a relative IRI in any IRI position, a
/// non-numeric or out-of-range count, or a `mode` that is neither `walk` nor `shortest`.
pub(crate) fn parse_path_relation(text: &str) -> Result<PathRelationSpec, String> {
    let mut iri: Option<String> = None;
    let mut steps: Vec<(String, PathDirection)> = Vec::new();
    let mut min_hops: Option<u32> = None;
    let mut max_hops: Option<u32> = None;
    let mut max_paths_per_seed: Option<u64> = None;
    let mut max_expansions: Option<u64> = None;
    let mut mode: Option<PathRelationMode> = None;

    for field in text.split(';') {
        let field = field.trim();
        if field.is_empty() {
            continue;
        }
        let (key, value) = field.split_once('=').ok_or_else(|| {
            format!(
                "--path-relation field `{field}` is not `key=value`; the value is \
                 semicolon-separated `key=value` pairs over {}",
                KEYS.join(", ")
            )
        })?;
        let key = key.trim();
        let value = value.trim();
        match key {
            "iri" => set_once(&mut iri, absolute_iri("iri", value)?, "iri")?,
            "forward" => push_step(&mut steps, value, PathDirection::Forward)?,
            "inverse" => push_step(&mut steps, value, PathDirection::Inverse)?,
            "min-hops" => set_once(&mut min_hops, number(key, value)?, key)?,
            "max-hops" => set_once(&mut max_hops, number(key, value)?, key)?,
            "max-paths-per-seed" => set_once(&mut max_paths_per_seed, number(key, value)?, key)?,
            "max-expansions" => set_once(&mut max_expansions, number(key, value)?, key)?,
            "mode" => set_once(&mut mode, parse_mode(value)?, key)?,
            other => {
                return Err(format!(
                    "--path-relation key `{other}` is not one of {}",
                    KEYS.join(", ")
                ));
            }
        }
    }

    if steps.is_empty() {
        return Err(
            "--path-relation names no predicate: at least one `forward=IRI` or `inverse=IRI` \
             is required, because a step that can traverse nothing defines no hop"
                .to_owned(),
        );
    }
    Ok(PathRelationSpec {
        iri: required(iri, "iri")?,
        steps,
        min_hops: required(min_hops, "min-hops")?,
        max_hops: required(max_hops, "max-hops")?,
        max_paths_per_seed: required(max_paths_per_seed, "max-paths-per-seed")?,
        max_expansions: required(max_expansions, "max-expansions")?,
        mode: required(mode, "mode")?,
    })
}

/// Record a single-valued key, refusing a second spelling of it.
///
/// A repeated key is refused rather than last-wins because both spellings are in the
/// operator's own command line and only one of them takes effect — a silent choice
/// between two stated intentions.
fn set_once<T>(slot: &mut Option<T>, value: T, key: &str) -> Result<(), String> {
    if slot.is_some() {
        return Err(format!(
            "--path-relation key `{key}` is given more than once; only `forward` and `inverse` \
             may repeat"
        ));
    }
    *slot = Some(value);
    Ok(())
}

/// Read a mandatory key that the grammar has no default for.
fn required<T>(slot: Option<T>, key: &str) -> Result<T, String> {
    slot.ok_or_else(|| {
        format!(
            "--path-relation is missing the mandatory key `{key}`; every key of {} is required \
             and none has a default (PurRDF mints no vocabulary IRIs, and a traversal envelope \
             this binary invented would be a limit the operator never read)",
            KEYS.join(", ")
        )
    })
}

/// Append one directed predicate to the step's alternation, refusing a repeat.
///
/// A duplicated `(predicate, direction)` pair records every matching statement as two
/// edges, so every walk through the hop is enumerated twice under two identifiers, with
/// no spelling of the query able to tell that apart from a graph that genuinely has two
/// derivations. [`PathStep::new`] refuses it too; refusing it here names the command-line
/// token instead of the interned term.
fn push_step(
    steps: &mut Vec<(String, PathDirection)>,
    value: &str,
    direction: PathDirection,
) -> Result<(), String> {
    let label = match direction {
        PathDirection::Forward => "forward",
        PathDirection::Inverse => "inverse",
    };
    let iri = absolute_iri(label, value)?;
    if steps
        .iter()
        .any(|(seen, seen_direction)| *seen == iri && *seen_direction == direction)
    {
        return Err(format!(
            "--path-relation repeats `{label}={iri}`; a duplicated alternative doubles every \
             walk that traverses it, with no observable difference at the call site"
        ));
    }
    steps.push((iri, direction));
    Ok(())
}

/// Validate one IRI-position value as an ABSOLUTE IRI.
///
/// Relative is refused rather than resolved: there is no base to resolve against here
/// (`--base` is the DATA and QUERY base, and silently borrowing it for a registration
/// key would make the relation's name depend on which file was loaded).
fn absolute_iri(key: &str, value: &str) -> Result<String, String> {
    let parsed = purrdf_iri::parse(value)
        .map_err(|e| format!("--path-relation `{key}={value}` is not a valid IRI: {e}"))?;
    if !parsed.has_scheme() {
        return Err(format!(
            "--path-relation `{key}={value}` is a relative IRI reference (no scheme); every IRI \
             position here must be absolute"
        ));
    }
    Ok(value.to_owned())
}

/// Read one count key as a non-negative integer of its own width.
///
/// The RANGE check that matters — a zero `min-hops`, an empty length interval, a
/// `max-hops` past [`purrdf_sparql_eval::MAX_HOPS_CAP`], a zero guard — belongs to
/// [`PathLimits::new`] and is left there rather than restated: a second copy of a bound
/// is a second opinion about it.
fn number<T: std::str::FromStr>(key: &str, value: &str) -> Result<T, String> {
    value.parse::<T>().map_err(|_| {
        format!(
            "--path-relation `{key}={value}` is not a whole number in range for that key \
             (`min-hops`/`max-hops` are 32-bit counts; `max-paths-per-seed`/`max-expansions` \
             are 64-bit)"
        )
    })
}

/// Read the `mode` key.
fn parse_mode(value: &str) -> Result<PathRelationMode, String> {
    match value {
        "walk" => Ok(PathRelationMode::Walk),
        "shortest" => Ok(PathRelationMode::Shortest),
        other => Err(format!(
            "--path-relation `mode={other}` is neither `walk` (every simple-prefix witness, \
             exponential in the worst case) nor `shortest` (one shortest witness per reachable \
             pair, polynomial)"
        )),
    }
}

/// Refuse the same relation IRI declared twice across repeated `--path-relation` flags.
///
/// [`PropertyFunctionRegistry::register`] **panics** on a duplicate, deliberately: a
/// shadowed relation silently changes which rows a graph pattern produces, and both
/// spellings of the call are identical. A command line is a host misconfiguration, so it
/// is a usage error here rather than an abort there.
///
/// # Errors
///
/// [`CliError::Usage`] naming the IRI declared twice.
pub(crate) fn refuse_duplicate_iris(specs: &[PathRelationSpec]) -> Result<(), CliError> {
    for (index, spec) in specs.iter().enumerate() {
        if specs[..index].iter().any(|seen| seen.iri == spec.iri) {
            return Err(CliError::Usage(format!(
                "--path-relation declares <{}> twice; a relation may not be silently shadowed, \
                 because both spellings of the call are identical and the only observable \
                 difference is which rows the query returns",
                spec.iri
            )));
        }
    }
    Ok(())
}

/// Snapshot every spec over `view` and register it, or `None` when no
/// `--path-relation` was given.
///
/// `None` is the ABSENCE of a registry rather than an empty one, so a command line that
/// names no relation evaluates byte-for-byte as it did before this flag existed.
///
/// The snapshot is scoped to [`GraphMatch::Default`], matching the Python surface's
/// `relations_from_graph` and the Rust harness: a relation table is *configuration*
/// written beside the data, and the default graph is where a source loaded without a
/// graph name puts it. Reading `Any` instead would let a traversal silently gain edges
/// from an unrelated named graph.
///
/// # An alternative the data has no edges for is not an error
///
/// [`PathGraph::from_dataset`] treats a predicate the dataset carries no in-scope quad
/// for — interned or not — as contributing zero edges, exactly as the core grammar's
/// `p|q` does not fail when `q` matches nothing. This flag inherits that: an operator
/// with a FIXED step vocabulary running the same command line across many datasets is
/// supplying valid configuration every time, and refusing here would key a failure on
/// which dataset happened to be in front of it. The configurations that are wrong
/// INDEPENDENTLY of any dataset — an empty alternation, a non-IRI predicate, a repeated
/// `(predicate, direction)` pair — are refused by [`parse_path_relation`] and
/// [`PathStep::new`], where the operator committed them.
///
/// # Errors
///
/// [`CliError::Usage`] carrying the kernel's own diagnostic when a step or an envelope is
/// malformed.
pub(crate) fn build_registry<D: DatasetView>(
    view: &D,
    specs: &[PathRelationSpec],
) -> Result<Option<PropertyFunctionRegistry>, CliError> {
    if specs.is_empty() {
        return Ok(None);
    }
    let mut registry = PropertyFunctionRegistry::new();
    for spec in specs {
        let alternatives = spec
            .steps
            .iter()
            .map(|(iri, direction)| (TermValue::iri(iri.clone()), *direction))
            .collect();
        let step = PathStep::new(alternatives).map_err(|e| named(&spec.iri, &e))?;
        let graph = Arc::new(
            PathGraph::from_dataset(view, &step, GraphMatch::Default)
                .map_err(|e| named(&spec.iri, &e))?,
        );
        let limits = PathLimits::new(
            spec.min_hops,
            spec.max_hops,
            spec.max_paths_per_seed,
            spec.max_expansions,
        )
        .map_err(|e| named(&spec.iri, &e))?;
        match spec.mode {
            PathRelationMode::Walk => {
                registry.register(
                    spec.iri.clone(),
                    Arc::new(PathWitnessRelation::new(graph, limits)),
                );
            }
            PathRelationMode::Shortest => {
                registry.register(
                    spec.iri.clone(),
                    Arc::new(ShortestPathWitnessRelation::new(graph, limits)),
                );
            }
        }
    }
    Ok(Some(registry))
}

/// Attach the relation's IRI to a kernel diagnostic, so a multi-relation command line
/// says which `--path-relation` was wrong.
fn named(iri: &str, error: &purrdf_sparql_eval::EvalError) -> CliError {
    CliError::Usage(format!("--path-relation <{iri}>: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole grammar, once: every key read, the alternation ordered as written.
    #[test]
    fn a_full_spec_parses_every_key() {
        let spec = parse_path_relation(
            "iri=http://example.org/pf#walk;forward=http://example.org/p;\
             inverse=http://example.org/q;min-hops=1;max-hops=4;max-paths-per-seed=1024;\
             max-expansions=100000;mode=shortest",
        )
        .expect("a complete spec parses");
        assert_eq!(spec.iri, "http://example.org/pf#walk");
        assert_eq!(
            spec.steps,
            vec![
                ("http://example.org/p".to_owned(), PathDirection::Forward),
                ("http://example.org/q".to_owned(), PathDirection::Inverse),
            ]
        );
        assert_eq!(spec.min_hops, 1);
        assert_eq!(spec.max_hops, 4);
        assert_eq!(spec.max_paths_per_seed, 1024);
        assert_eq!(spec.max_expansions, 100_000);
        assert_eq!(spec.mode, PathRelationMode::Shortest);
    }

    /// A minimal, valid spec, for the error tests to mutate one key of.
    fn minimal(extra: &str) -> String {
        format!(
            "iri=http://example.org/pf#walk;forward=http://example.org/p;min-hops=1;\
             max-hops=4;max-paths-per-seed=8;max-expansions=64;mode=walk{extra}"
        )
    }

    #[test]
    fn an_unknown_key_names_itself() {
        let error = parse_path_relation(&minimal(";depth=3")).expect_err("unknown key");
        assert!(error.contains("`depth`"), "{error}");
    }

    #[test]
    fn a_missing_key_is_named() {
        let error = parse_path_relation(
            "iri=http://example.org/pf#walk;forward=http://example.org/p;min-hops=1;\
             max-hops=4;max-paths-per-seed=8;mode=walk",
        )
        .expect_err("max-expansions is mandatory");
        assert!(error.contains("`max-expansions`"), "{error}");
    }

    #[test]
    fn a_repeated_single_valued_key_is_refused() {
        let error = parse_path_relation(&minimal(";max-hops=9")).expect_err("max-hops repeats");
        assert!(error.contains("more than once"), "{error}");
    }

    #[test]
    fn a_spec_with_no_predicate_is_refused() {
        let error = parse_path_relation(
            "iri=http://example.org/pf#walk;min-hops=1;max-hops=4;max-paths-per-seed=8;\
             max-expansions=64;mode=walk",
        )
        .expect_err("no predicate");
        assert!(error.contains("names no predicate"), "{error}");
    }

    #[test]
    fn a_repeated_directed_predicate_is_refused() {
        let error = parse_path_relation(&minimal(";forward=http://example.org/p"))
            .expect_err("duplicate alternative");
        assert!(error.contains("repeats"), "{error}");
    }

    /// The SAME predicate in the two directions is NOT a duplicate: it is the ordinary
    /// undirected step, and refusing it would refuse the commonest spelling there is.
    #[test]
    fn the_same_predicate_in_both_directions_is_accepted() {
        let spec = parse_path_relation(&minimal(";inverse=http://example.org/p"))
            .expect("both directions of one predicate is one alternation of two");
        assert_eq!(spec.steps.len(), 2);
    }

    #[test]
    fn a_relative_iri_is_refused() {
        let error = parse_path_relation(
            "iri=pf#walk;forward=http://example.org/p;min-hops=1;max-hops=4;\
             max-paths-per-seed=8;max-expansions=64;mode=walk",
        )
        .expect_err("relative iri");
        assert!(error.contains("relative IRI reference"), "{error}");
    }

    #[test]
    fn a_non_numeric_count_is_refused() {
        let error = parse_path_relation(
            "iri=http://example.org/pf#walk;forward=http://example.org/p;min-hops=one;\
             max-hops=4;max-paths-per-seed=8;max-expansions=64;mode=walk",
        )
        .expect_err("min-hops=one");
        assert!(error.contains("`min-hops=one`"), "{error}");
    }

    #[test]
    fn an_unknown_mode_is_refused() {
        let error = parse_path_relation(
            "iri=http://example.org/pf#walk;forward=http://example.org/p;min-hops=1;\
             max-hops=4;max-paths-per-seed=8;max-expansions=64;mode=cheapest",
        )
        .expect_err("mode=cheapest");
        assert!(error.contains("`mode=cheapest`"), "{error}");
    }

    #[test]
    fn a_field_without_an_equals_sign_is_refused() {
        let error = parse_path_relation(&minimal(";shortest")).expect_err("bare token");
        assert!(error.contains("is not `key=value`"), "{error}");
    }

    #[test]
    fn one_iri_declared_twice_across_flags_is_refused() {
        let spec = parse_path_relation(&minimal("")).expect("valid");
        let error = refuse_duplicate_iris(&[spec.clone(), spec]).expect_err("duplicate IRI");
        assert!(format!("{error}").contains("twice"), "{error}");
    }

    /// No `--path-relation` is the ABSENCE of a registry, not an empty one.
    #[test]
    fn no_spec_attaches_no_registry() {
        let dataset =
            purrdf_rdf::parse_dataset(b"", "application/n-triples", None).expect("empty dataset");
        assert!(
            build_registry(&*dataset, &[])
                .expect("no relations is not an error")
                .is_none()
        );
    }

    /// An alternative the data has no edges for contributes zero edges rather than
    /// failing — an operator with a fixed step vocabulary running the same command line
    /// across many datasets is supplying valid configuration every time. The relation is
    /// registered and answers nothing, which is the honest answer for a graph with no
    /// such edges.
    #[test]
    fn a_predicate_the_data_has_no_edges_for_registers_an_empty_traversal() {
        let dataset =
            purrdf_rdf::parse_dataset(b"", "application/n-triples", None).expect("empty dataset");
        let spec = parse_path_relation(&minimal("")).expect("valid");
        let registry = build_registry(&*dataset, &[spec])
            .expect("an edgeless alternative is not a misconfiguration")
            .expect("one relation was declared, so a registry exists");
        assert!(
            registry.resolve("http://example.org/pf#walk").is_some(),
            "the relation is registered under the caller's IRI"
        );
    }

    /// An envelope the kernel refuses is refused here too, naming the relation it came
    /// from so a multi-relation command line says which one was wrong.
    #[test]
    fn an_unbuildable_envelope_names_the_relation() {
        let dataset =
            purrdf_rdf::parse_dataset(b"", "application/n-triples", None).expect("empty dataset");
        let spec = parse_path_relation(
            "iri=http://example.org/pf#walk;forward=http://example.org/p;min-hops=0;\
             max-hops=4;max-paths-per-seed=8;max-expansions=64;mode=walk",
        )
        .expect("the grammar accepts it; the envelope does not");
        let error = build_registry(&*dataset, &[spec]).expect_err("min-hops of zero");
        assert!(
            format!("{error}").contains("http://example.org/pf#walk"),
            "{error}"
        );
    }
}
