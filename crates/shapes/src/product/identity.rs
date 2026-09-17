// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The INPUT BINDING of a prepared SHACL product: the ordered tuple of everything
//! the product was derived from, which a restore checks before it trusts the bytes.
//!
//! [`build_identity`] assembles that tuple as a [`purrdf_core::artifact::Identity`]
//! and [`check_identity`] compares a decoded one against the environment about to
//! execute it, refusing the FIRST component that disagrees under the
//! [`ProductDimension`] that names it.
//!
//! # Why this exists, and what it prevents
//!
//! A prepared product is compiled once and executed later, elsewhere, by a process
//! that did not build it. The bytes carry compiled decisions that are only correct
//! under the inputs they were compiled from: which IRI `ex:Shape` denotes depends on
//! the prefix map, what a relative reference resolved to depends on the base, which
//! predicates are function calls depends on the registries, and whether box-role
//! annotations are collected at all depends on a vocabulary PurRDF refuses to
//! fabricate. Executing a product under different inputs does not crash. It
//! validates, and returns a well-formed report about a shapes graph nobody asked
//! for. The identity is what makes that mismatch loud at the boundary instead of
//! invisible in the results.
//!
//! # The rejected alternative: one opaque hash
//!
//! The obvious design is to fold every input into a single 32-byte digest and
//! compare it. It is smaller and it answers exactly one question — *do these
//! match?* — and when the answer is no, which is the only time anyone looks, it has
//! nothing further to say. So this module does NOT mint a second identity type and
//! does NOT reduce the identity to one hash: it reuses
//! [`purrdf_core::artifact::Identity`], which is DECODABLE (it carries the ordered
//! labelled components as well as their digest) and which enforces a canonical,
//! injective encoding. A refusal here can therefore name the component, the
//! dimension, and both sides' values, which is the difference between "digest
//! mismatch" and "the base moved from `https://example.org/a` to
//! `https://example.org/b`".
//!
//! The failure prevented is operational: an unexplainable refusal a caller can only
//! resolve by rebuilding everything and hoping — and, on the other side, the
//! silently-admitted product described above.
//!
//! # The components, in their FIXED order
//!
//! The order is part of the format: [`Identity`] is an ORDERED sequence and
//! [`check_identity`] maps a POSITION to a dimension, so reordering these rows is a
//! breaking change to every product already written, not a cosmetic edit.
//!
//! | # | Component | Label | Refuses under |
//! |---|-----------|-------|---------------|
//! | 0 | the source dataset's canonical pack digest | `source-dataset` | [`DatasetIdentity`] |
//! | 1 | the shapes-graph IRI (`Shapes::shapes_graph`) | `shapes-graph` | [`ShapesGraph`] |
//! | 2 | the document `@prefix` map, **key-sorted** | `doc-prefixes` | [`Prefixes`] |
//! | 3 | the base IRI | `base` | [`Base`] |
//! | 4 | the profile id ([`PROFILE_ID`]) | `profile` | [`Profile`] |
//! | 5 | the box-role vocabulary, **key-sorted** | `box-role-vocab` | [`Vocabulary`] |
//! | 6 | user functions, [`FnPopulation::Declared`] | `user-functions-declared` | [`FunctionRegistry`] |
//! | 7 | user functions, [`FnPopulation::Injected`] | `user-functions-injected` | [`FunctionRegistry`] |
//! | 8 | the custom-aggregate registry | `aggregate-registry` | [`AggregateRegistry`] |
//! | 9 | the property-function registry | `property-function-registry` | [`PropertyFunctionRegistry`] |
//! | 10 | the class catalog's digest | `class-catalog` | [`ClassCatalog`] |
//!
//! Rows 2 and 5 are sorted because both sources are maps whose *order* is not part
//! of their meaning: `ParseProvenance::doc_prefixes` deliberately preserves the
//! parser's own order (see [`crate::provenance`]), and two callers that built the
//! same logical prefix map in different orders must not fail to open each other's
//! products. Sorting at ENCODE time is where that canonicalization belongs — the
//! provenance keeps the observation, this module states the encoding.
//!
//! Rows 6 and 7 are two components rather than one because a restoring process
//! REBUILDS every declared entry by re-parsing the shapes graph, while the injected
//! half is something only its host can wire. One digest over the merged registry
//! would be unreproducible for the consumer and unusable as a requirement
//! statement; see [`purrdf_sparql_eval::user_fn::FnPopulation`].
//!
//! # Row 5 is not optional garnish
//!
//! `Shapes::box_role_vocab` is threaded straight into validation. A product that
//! omitted it would restore a `Shapes` carrying `box_role_vocab: None`, which
//! validates DIFFERENTLY from the parsed one — every box-role list stays empty and
//! no role lookup happens — and no other check in this codec catches it, because the
//! AST, the dataset and every registry are identical. PurRDF mints no vocabulary
//! IRIs, so this configuration has no default that could stand in for a missing one.
//! Hence its own component and its own [`Vocabulary`] refusal.
//!
//! Its encoding distinguishes ABSENT from PRESENT-AND-EMPTY, because those are two
//! different configurations: "the box-role feature is inactive" and "the feature is
//! active over a vocabulary of empty IRIs" are not the same parse, and an encoding
//! that conflated them would let one product open against the other's inputs.
//!
//! # Row 10 carries a digest and no body, deliberately
//!
//! `crate::engine::ClassCatalog` is a PURE DERIVATION of the shape tree: a
//! cycle-safe walk that collects every reachable `sh:class` / `sh:targetClass` /
//! `shnex:instancesOf` IRI and assigns each a position. So the product pins its
//! [`class_catalog_digest`] and carries no catalog section at all. A restore
//! re-derives the catalog from the AST it has already decoded and compares.
//!
//! That choice is free on the common path and strictly better on the cold one. The
//! walk is O(shapes), in memory, with no I/O and no SPARQL, and it is dominated by
//! the AST decode that has to happen on the same path anyway — so the recompute
//! costs nothing measurable. Carrying the body instead would add bytes, add a second
//! decoder, and — the actual defect — make a STALE ANALYSIS SERVABLE: a product
//! written by a build whose class walk had a bug (or simply a different reachability
//! rule) would restore that build's catalog and validate against it, verified and
//! wrong. Pinning the digest means a product whose analysis no longer matches this
//! build's walk is unservable, which is the outcome worth having.
//!
//! # Encoding
//!
//! A component value that holds SEVERAL parts (rows 2 and 5) is a flat sequence of
//! `varint(len) ‖ bytes` parts, reusing [`purrdf_core::ir::pack::bits`]'s LEB128
//! primitives exactly as [`purrdf_core::artifact::identity`] does for the component
//! frame itself. BOTH halves of every pair are length-prefixed: without that,
//! `{"a": "bc"}` and `{"ab": "c"}` both flatten to `abc` and two identities over
//! genuinely different prefix maps collide on one digest — a forged binding produced
//! by a formatting decision.
//!
//! A component value that holds ONE string (rows 1, 3 and 4) is written as printable
//! text instead, because it has no boundary to alias across and because a refusal
//! naming an IRI must show the IRI. See [`encode_optional_str`].
//!
//! Nothing here touches the filesystem, a clock, a thread, or a source of
//! randomness: it is a pure function of its arguments and stays
//! `wasm32-unknown-unknown` compatible.
//!
//! [`DatasetIdentity`]: ProductDimension::DatasetIdentity
//! [`ShapesGraph`]: ProductDimension::ShapesGraph
//! [`Prefixes`]: ProductDimension::Prefixes
//! [`Base`]: ProductDimension::Base
//! [`Profile`]: ProductDimension::Profile
//! [`Vocabulary`]: ProductDimension::Vocabulary
//! [`FunctionRegistry`]: ProductDimension::FunctionRegistry
//! [`AggregateRegistry`]: ProductDimension::AggregateRegistry
//! [`PropertyFunctionRegistry`]: ProductDimension::PropertyFunctionRegistry
//! [`ClassCatalog`]: ProductDimension::ClassCatalog

use ::purrdf::PackDigest;
use purrdf_core::ContentDigest;
use purrdf_core::artifact::identity::{Identity, IdentityMismatch};
use purrdf_core::ir::pack::bits::write_varint;
use purrdf_sparql_eval::user_fn::FnPopulation;
use purrdf_sparql_eval::{
    EvalError, PropertyFunctionRegistry, agg_fn, property_function_content_fingerprint, user_fn,
};

use crate::engine::ClassCatalog;
use crate::model::BoxRoleVocab;
use crate::shapes::Shapes;

use super::error::{ProductDimension, ShapesProductError};

// ---------------------------------------------------------------------------
// The component table
// ---------------------------------------------------------------------------

/// The preparation profile this module binds: the switchable behaviours compiled
/// into a product of the SHACL Core + SHACL-AF surface this crate implements.
///
/// A constant, not a computed value. It is the coarse "which product shape is this"
/// discriminant that sits ABOVE the content-derived stage id the census
/// (`crates/shapes/tests/product_model_census.rs`) derives from the model itself, and
/// it moves only when the profile's *meaning* is redefined — at which point every
/// product written under the old meaning must stop opening, which is exactly what
/// changing this string does.
///
/// The census mixes the SAME string into its stage id under its own `PROFILE_ID`.
/// It is an integration test and cannot see a `pub(crate)` item, so the two
/// declarations are necessarily separate; they name one profile and must be changed
/// together. `stage_id_matches_golden` fails the moment the census's copy moves, so
/// a divergence is loud rather than silent.
pub(crate) const PROFILE_ID: &str = "purrdf-shacl-core-v1";

/// Every identity component, in the fixed order of the table in the
/// [module docs](self): its label and the [`ProductDimension`] a disagreement at
/// that POSITION refuses under.
///
/// One table drives both [`build_identity`] (which pushes in this order) and
/// [`check_identity`] (which indexes by position), so the labels and the dimensions
/// cannot drift apart into two hand-maintained lists.
const COMPONENTS: [(&str, ProductDimension); 11] = [
    ("source-dataset", ProductDimension::DatasetIdentity),
    ("shapes-graph", ProductDimension::ShapesGraph),
    ("doc-prefixes", ProductDimension::Prefixes),
    ("base", ProductDimension::Base),
    ("profile", ProductDimension::Profile),
    ("box-role-vocab", ProductDimension::Vocabulary),
    (
        "user-functions-declared",
        ProductDimension::FunctionRegistry,
    ),
    (
        "user-functions-injected",
        ProductDimension::FunctionRegistry,
    ),
    ("aggregate-registry", ProductDimension::AggregateRegistry),
    (
        "property-function-registry",
        ProductDimension::PropertyFunctionRegistry,
    ),
    ("class-catalog", ProductDimension::ClassCatalog),
];

/// The tag byte an absent structured component value opens with. Distinct from
/// [`PRESENT`] so `None` can never encode like a present-but-empty value.
const ABSENT: u8 = 0;

/// The tag byte a present structured component value opens with. See [`ABSENT`].
const PRESENT: u8 = 1;

/// The whole encoding of an ABSENT single-string component — see
/// [`encode_optional_str`] for why those two components are spelled in text.
const ABSENT_MARKER: &str = "(absent)";

/// The prefix a PRESENT single-string component's encoding opens with. Chosen so no
/// present value can ever spell [`ABSENT_MARKER`]: every present encoding begins
/// `(present)` and the absent one begins `(absent)`.
const PRESENT_PREFIX: &str = "(present) ";

// ---------------------------------------------------------------------------
// Injective framing
// ---------------------------------------------------------------------------

/// Append one `varint(len) ‖ bytes` part.
///
/// Every variable-length piece of every component value goes through here, which is
/// what makes the encoding injective — see the [module docs](self) for the `abc`
/// collision this prevents.
fn push_part(out: &mut Vec<u8>, part: &[u8]) {
    write_varint(out, part.len() as u64);
    out.extend_from_slice(part);
}

/// Encode an optional single string as PRINTABLE text: [`ABSENT_MARKER`], or
/// [`PRESENT_PREFIX`] followed by the string itself.
///
/// The discrimination is what separates "the caller named no shapes graph" from "the
/// caller named the empty IRI". Both are representable configurations, so both must
/// encode distinctly.
///
/// # Why these two components are not length-framed like the rest
///
/// Length-prefixing exists to keep a sequence of SEVERAL parts from aliasing across
/// its own boundaries (the `abc` collision in the [module docs](self)). A component
/// whose whole content is ONE string has no boundary to alias across: the encoding
/// is injective as long as the present and absent spellings cannot coincide, which
/// the two constants guarantee by construction.
///
/// So the framing buys nothing here, and it costs something real: a LEB128 length
/// byte is a control character, which makes
/// [`purrdf_core::artifact::IdentityMismatch`] render the whole value as hex. These
/// are the two components whose content is a human-facing IRI — the thing a caller
/// has to look at and fix — and `artifact has 0x011a68747470…` is precisely the
/// unactionable refusal this module exists to avoid. Text in, text out.
fn encode_optional_str(value: Option<&str>) -> Vec<u8> {
    match value {
        None => ABSENT_MARKER.as_bytes().to_vec(),
        Some(text) => format!("{PRESENT_PREFIX}{text}").into_bytes(),
    }
}

/// Encode a prefix map as key-sorted framed `(prefix, namespace)` pairs.
///
/// Sorted on the WHOLE pair rather than on the prefix alone: the provenance is a
/// `Vec`, not a map, so it may in principle carry one prefix twice, and sorting on
/// the key alone would leave the tie broken by input order — which would make the
/// encoding depend on the very ordering this sort exists to erase.
fn encode_prefixes(prefixes: &[(String, String)]) -> Vec<u8> {
    let mut sorted: Vec<(&str, &str)> = prefixes
        .iter()
        .map(|(prefix, namespace)| (prefix.as_str(), namespace.as_str()))
        .collect();
    sorted.sort_unstable();

    let mut out = Vec::new();
    for (prefix, namespace) in sorted {
        push_part(&mut out, prefix.as_bytes());
        push_part(&mut out, namespace.as_bytes());
    }
    out
}

/// Encode the box-role vocabulary: [`ABSENT`], or [`PRESENT`] followed by its six
/// terms as key-sorted framed `(field, iri)` pairs.
///
/// Keyed and sorted rather than written positionally so the encoding says which term
/// is which, and so adding a term to [`BoxRoleVocab`] is a change the sort absorbs
/// rather than a silent re-interpretation of an existing slot. See the
/// [module docs](self) for why absent and present-and-empty must differ.
fn encode_vocab(vocab: Option<&BoxRoleVocab>) -> Vec<u8> {
    let mut out = Vec::new();
    let Some(vocab) = vocab else {
        out.push(ABSENT);
        return out;
    };
    out.push(PRESENT);

    let mut fields = [
        ("box-abox", vocab.box_abox.as_str()),
        ("box-cbox", vocab.box_cbox.as_str()),
        ("box-config-box", vocab.box_config_box.as_str()),
        ("box-rbox", vocab.box_rbox.as_str()),
        ("box-tbox", vocab.box_tbox.as_str()),
        ("graph-box-role", vocab.graph_box_role.as_str()),
    ];
    fields.sort_unstable();

    for (field, iri) in fields {
        push_part(&mut out, field.as_bytes());
        push_part(&mut out, iri.as_bytes());
    }
    out
}

// ---------------------------------------------------------------------------
// The class-catalog digest
// ---------------------------------------------------------------------------

/// The domain separator the class-catalog digest opens with, so its preimage can
/// never coincide with another content fingerprint's.
const CLASS_CATALOG_DOMAIN: &str = "purrdf-shapes/product/class-catalog";

/// A content digest over `catalog`'s key-sorted entries: every planned class IRI and
/// the position it was assigned.
///
/// The product carries THIS and no catalog body — see the [module docs](self) for
/// why re-deriving is free and why pinning the digest is what keeps a stale analysis
/// unservable.
///
/// The position is folded in, not just the IRI set: the position is what a
/// `ValidationPlan` indexes its resolved-`TermId` row by, so two catalogs over the
/// same classes with different assignments are different analyses.
pub(crate) fn class_catalog_digest(catalog: &ClassCatalog) -> ContentDigest {
    let mut bytes = Vec::new();
    push_part(&mut bytes, CLASS_CATALOG_DOMAIN.as_bytes());

    let mut entries: Vec<(&str, usize)> = catalog
        .entries()
        .map(|(class, position)| (class.as_str(), position))
        .collect();
    entries.sort_unstable();

    for (class, position) in entries {
        push_part(&mut bytes, class.as_bytes());
        push_part(&mut bytes, &(position as u64).to_be_bytes());
    }
    ContentDigest::of(&bytes)
}

// ---------------------------------------------------------------------------
// Building
// ---------------------------------------------------------------------------

/// Assemble the identity of a prepared product from the inputs it was derived from.
///
/// `dataset` is the certified canonical digest of the shapes dataset
/// (`super::dataset::certify_dataset`); `shapes` supplies the shapes-graph IRI, the
/// parse provenance, the box-role vocabulary and the two registries a shapes graph
/// carries; `property_functions` is the host-supplied relation table, which no
/// shapes graph can describe; and `classes` is the derived class analysis
/// (`crate::engine::PreparedShapes::class_catalog`).
///
/// # Errors
///
/// [`ProductDimension::FunctionRegistry`],
/// [`ProductDimension::AggregateRegistry`] or
/// [`ProductDimension::PropertyFunctionRegistry`] when a registry's own declaration
/// methods fail, so its content fingerprint cannot be computed. An identity that
/// silently omitted an unfingerprintable registry would be a binding that does not
/// bind, so this fails closed rather than skipping the component.
pub(crate) fn build_identity(
    dataset: &PackDigest,
    shapes: &Shapes,
    property_functions: &PropertyFunctionRegistry,
    classes: &ClassCatalog,
) -> Result<Identity, ShapesProductError> {
    let provenance = shapes.provenance();

    // In the fixed order of `COMPONENTS`, which is the order documented at the top
    // of this module and pinned by `identity_component_order_is_the_documented_one`.
    let values: [Vec<u8>; COMPONENTS.len()] = [
        dataset.as_bytes().to_vec(),
        encode_optional_str(shapes.shapes_graph.as_deref()),
        encode_prefixes(provenance.doc_prefixes()),
        encode_optional_str(provenance.base()),
        PROFILE_ID.as_bytes().to_vec(),
        encode_vocab(shapes.box_role_vocab.as_ref()),
        fingerprint(
            user_fn::content_fingerprint(&shapes.functions, FnPopulation::Declared),
            ProductDimension::FunctionRegistry,
            "SPARQL function registry's declared population",
        )?,
        fingerprint(
            user_fn::content_fingerprint(&shapes.functions, FnPopulation::Injected),
            ProductDimension::FunctionRegistry,
            "SPARQL function registry's host-injected population",
        )?,
        fingerprint(
            agg_fn::content_fingerprint(&shapes.aggregates),
            ProductDimension::AggregateRegistry,
            "custom-aggregate registry",
        )?,
        fingerprint(
            property_function_content_fingerprint(property_functions),
            ProductDimension::PropertyFunctionRegistry,
            "property-function registry",
        )?,
        class_catalog_digest(classes).as_bytes().to_vec(),
    ];

    let mut identity = Identity::new();
    for ((label, _), value) in COMPONENTS.iter().zip(values.iter()) {
        identity.push(label, value);
    }
    Ok(identity)
}

/// Unwrap a registry content fingerprint into its raw digest bytes, or refuse under
/// `dimension` naming `what` could not be fingerprinted.
fn fingerprint(
    computed: Result<ContentDigest, EvalError>,
    dimension: ProductDimension,
    what: &str,
) -> Result<Vec<u8>, ShapesProductError> {
    computed
        .map(|digest| digest.as_bytes().to_vec())
        .map_err(|error| {
            ShapesProductError::new(
                dimension,
                format!(
                    "this product's {what} could not be fingerprinted, so the product cannot \
                     state which registry it was prepared against and nothing at restore could \
                     check it; prepare the product with a registry whose declarations can be \
                     read (registry reported: {error})"
                ),
            )
        })
}

// ---------------------------------------------------------------------------
// Checking
// ---------------------------------------------------------------------------

/// Compare a decoded identity against the executing environment's, refusing the
/// FIRST component that disagrees under the [`ProductDimension`] its POSITION names.
///
/// `expected` is the identity the product carries; `actual` is the one
/// [`build_identity`] produces from the environment about to execute it. The
/// refusal's message carries what EACH side holds, which is the whole reason
/// [`Identity`] is decodable rather than a bare digest.
///
/// Only the first disagreement is reported. The components are ordered from the
/// outside in, so the first one is the outermost unmet precondition: a caller whose
/// dataset moved is told that, not that ten downstream digests also differ as a
/// consequence.
///
/// # Errors
///
/// The [`ProductDimension`] at the first disagreeing position — see the table in the
/// [module docs](self). A disagreement past the last known position (an identity
/// from a build that binds more inputs than this one) refuses under
/// [`ProductDimension::Malformed`], fail-closed.
pub(crate) fn check_identity(
    expected: &Identity,
    actual: &Identity,
) -> Result<(), ShapesProductError> {
    let mismatches = expected.mismatches(actual);
    let Some(first) = mismatches.first() else {
        return Ok(());
    };

    let dimension = mismatch_position(first)
        .and_then(|position| COMPONENTS.get(position))
        .map_or(ProductDimension::Malformed, |(_, dimension)| *dimension);

    Err(ShapesProductError::new(
        dimension,
        format!("{}; {first}", fix_for(dimension)),
    ))
}

/// The position an [`IdentityMismatch`] reports, or `None` when this build cannot
/// read one.
///
/// [`IdentityMismatch`] is `#[non_exhaustive]`, so the wildcard arm is mandatory. A
/// variant added upstream lands on `None` and [`check_identity`] refuses it under
/// [`ProductDimension::Malformed`] — fail-closed, and refused under the dimension
/// documented as "invalid in a way no other dimension names" rather than silently
/// attributed to whichever component happens to sit first.
fn mismatch_position(mismatch: &IdentityMismatch) -> Option<usize> {
    match mismatch {
        IdentityMismatch::LabelDiffers { position, .. }
        | IdentityMismatch::ValueDiffers { position, .. }
        | IdentityMismatch::MissingFromCaller { position, .. }
        | IdentityMismatch::UnexpectedFromCaller { position, .. } => Some(*position),
        _ => None,
    }
}

/// The prescriptive half of a refusal: what the caller does about a disagreement on
/// `dimension`.
///
/// Written in the register [`ShapesProductError`] documents — name the ACTION, not
/// only the fault — and deliberately per-dimension, because the actions genuinely
/// differ: a moved dataset means re-prepare, a wrong registry means fix the host
/// wiring, and a class-catalog disagreement means this build's shape analysis is not
/// the one the product was compiled against.
fn fix_for(dimension: ProductDimension) -> &'static str {
    match dimension {
        ProductDimension::DatasetIdentity => {
            "this product was prepared from a different shapes dataset than the one supplied for \
             its execution; re-prepare it from the SAME shapes graph the execution loads"
        }
        ProductDimension::ShapesGraph => {
            "this product was prepared under a different shapes-graph IRI than the one supplied \
             for its execution; expose the shapes dataset under the SAME named graph, because \
             that IRI is what `$shapesGraph` binds in every SHACL-SPARQL body"
        }
        ProductDimension::Prefixes => {
            "this product was prepared against a different prefix map than the one supplied for \
             its execution; prepare it under the SAME prefixes the execution uses, because the \
             prefix map is what decides which IRI a prefixed name denotes"
        }
        ProductDimension::Base => {
            "this product was prepared against a different base IRI than the one supplied for \
             its execution; prepare it under the SAME base, because the base is what relative \
             references resolve against"
        }
        ProductDimension::Profile => {
            "this product was prepared under a different preparation profile than the one in \
             force for its execution; re-prepare it with the PurRDF build that will execute it"
        }
        ProductDimension::Vocabulary => {
            "this product was prepared under a different box-role vocabulary than the one \
             supplied for its execution; supply the SAME vocabulary, because PurRDF mints no \
             vocabulary IRIs and a product restored without one validates with every box-role \
             lookup silently disabled"
        }
        ProductDimension::FunctionRegistry => {
            "this product was prepared against a different SPARQL function registry than the one \
             supplied for its execution; wire the SAME functions into the executing host, \
             because the registry is what decides how a function IRI resolves"
        }
        ProductDimension::AggregateRegistry => {
            "this product was prepared against a different custom-aggregate registry than the \
             one supplied for its execution; wire the SAME aggregates into the executing host, \
             because the registry is what an `AGG(<iri>, …)` call resolves against"
        }
        ProductDimension::PropertyFunctionRegistry => {
            "this product was prepared against a different property-function registry than the \
             one supplied for its execution; wire the SAME relations into the executing host, \
             because the registry is what decides which predicates are calls rather than \
             ordinary triple patterns"
        }
        ProductDimension::ClassCatalog => {
            "this product pinned a class analysis this build does not re-derive from the same \
             shape tree, so the analysis it compiled its class-membership decisions against is \
             stale; re-prepare the product with the PurRDF build that will execute it"
        }
        // The residual. Reached only through the `#[non_exhaustive]` fallback in
        // `mismatch_position` or a position past the last known component, both of
        // which mean this build cannot say WHICH input moved.
        _ => {
            "this product's input binding disagrees with the environment supplied for its \
             execution in a way this build has no more specific name for; re-prepare the \
             product with the PurRDF build that will execute it"
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Tests for the prepared-product identity.
///
/// Every refusal is executed alongside a NEIGHBOURING VALID case, because
/// over-refusal — rejecting an environment that really does match — hides perfectly:
/// every test still passes and the strictness looks correct, right up until a caller
/// restores the product that should open and doesn't.
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ::purrdf::{PackDigest, RdfDataset, TermValue};
    use purrdf_sparql_eval::{
        AggregateAccumulator, AggregateRegistry, AlgebraicClass, Arity, BindingPattern,
        CustomAggregate, EvalError, PfArgs, PfArity, PfCursor, PfRow, PropertyFunction,
        PropertyFunctionRegistry, UserFunctionRegistry, Volatility,
    };

    use purrdf_core::artifact::identity::{Identity, IdentityComponent};

    use super::{COMPONENTS, PROFILE_ID, build_identity, check_identity, class_catalog_digest};
    use crate::engine::{ClassCatalog, PreparedShapes, parse_shapes};
    use crate::model::BoxRoleVocab;
    use crate::product::dataset::{certify_dataset, encode_dataset};
    use crate::product::error::ProductDimension;
    use crate::shapes::{Shapes, from_dataset_with_config_and_graph};

    // ── Fixtures (example.org, per the repository's fixture rule) ────────────────

    /// A plain shapes graph with two class references, so the class catalog is not
    /// empty and the digest over it is not a constant.
    const PLAIN_SHAPES: &str = r"
        @prefix ex: <https://example.org/> .
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

        ex:ThingShape a sh:NodeShape ;
            sh:targetClass ex:Thing ;
            sh:property [
                sh:path ex:name ;
                sh:minCount 1 ;
                sh:datatype xsd:string ;
            ] ;
            sh:property [
                sh:path ex:owner ;
                sh:class ex:Person ;
            ] .
    ";

    /// [`PLAIN_SHAPES`] plus one triple that declares NO shape and names NO class.
    /// It moves the dataset's canonical identity and nothing else, which is what
    /// makes `identity_differs_on_source_dataset` an isolated single-input change.
    const PLAIN_SHAPES_PLUS_NOTE: &str = r#"
        @prefix ex: <https://example.org/> .
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

        ex:ThingShape a sh:NodeShape ;
            sh:targetClass ex:Thing ;
            sh:property [
                sh:path ex:name ;
                sh:minCount 1 ;
                sh:datatype xsd:string ;
            ] ;
            sh:property [
                sh:path ex:owner ;
                sh:class ex:Person ;
            ] .

        ex:note ex:text "an unrelated statement" .
    "#;

    /// A shapes graph whose class references are DIFFERENT from [`PLAIN_SHAPES`]'s,
    /// so its class catalog is a different analysis.
    const OTHER_CLASS_SHAPES: &str = r"
        @prefix ex: <https://example.org/> .
        @prefix sh: <http://www.w3.org/ns/shacl#> .

        ex:WidgetShape a sh:NodeShape ;
            sh:targetClass ex:Widget ;
            sh:property [
                sh:path ex:maker ;
                sh:class ex:Maker ;
            ] .
    ";

    const SHAPES_GRAPH_IRI: &str = "https://example.org/shapes";
    const OTHER_SHAPES_GRAPH_IRI: &str = "https://example.org/other-shapes";

    const EX_FN: &str = "https://example.org/fn";
    const EX_AGG: &str = "https://example.org/agg";
    const EX_REL: &str = "https://example.org/rel";

    // ── Environment assembly ────────────────────────────────────────────────────

    /// Parse Turtle into a frozen dataset.
    fn dataset_of(ttl: &str) -> Arc<RdfDataset> {
        crate::text_ingest::parse_turtle_to_dataset(ttl, None).expect("fixture Turtle parses")
    }

    /// Parse a shapes graph under an explicit configuration — the four inputs the
    /// dataset entry point takes, so a test can move exactly one of them.
    fn shapes_with(
        ttl: &str,
        prefixes: &[(String, String)],
        vocab: Option<BoxRoleVocab>,
        shapes_graph: Option<&str>,
    ) -> Shapes {
        from_dataset_with_config_and_graph(
            &dataset_of(ttl),
            prefixes,
            vocab,
            shapes_graph.map(ToOwned::to_owned),
        )
        .expect("fixture shapes parse")
    }

    /// The default configuration: the document's own prefixes, no box-role
    /// vocabulary, and the fixture shapes-graph IRI.
    fn shapes_of(ttl: &str) -> Shapes {
        shapes_with(
            ttl,
            &crate::text_ingest::extract_prefixes(ttl),
            None,
            Some(SHAPES_GRAPH_IRI),
        )
    }

    /// The certified canonical digest of a parsed shapes graph's retained dataset —
    /// identity component 0, taken through the real section codec rather than
    /// fabricated.
    fn digest_of(shapes: &Shapes) -> PackDigest {
        let bytes = encode_dataset(shapes.dataset()).expect("encode the dataset section");
        certify_dataset(&bytes).expect("certify the dataset section")
    }

    /// The class catalog a preparation derives from `shapes` — identity component
    /// 10, taken through the real analysis rather than rebuilt here.
    fn catalog_of(shapes: &Shapes) -> Arc<ClassCatalog> {
        PreparedShapes::new(Arc::new(clone_shapes(shapes))).class_catalog()
    }

    /// Re-parse `shapes` from its own retained dataset and provenance.
    ///
    /// `Shapes` is not `Clone` (it owns registries full of `Arc<dyn Fn>` and a
    /// shared shape index), and `PreparedShapes::new` takes ownership, so a test
    /// that needs both the shapes and their catalog re-derives rather than clones.
    /// That is also the honest thing to do here: the catalog is defined as a
    /// derivation of the shape tree, so deriving it from an independently parsed
    /// tree is a stronger witness than deriving it from the same object.
    fn clone_shapes(shapes: &Shapes) -> Shapes {
        from_dataset_with_config_and_graph(
            shapes.dataset(),
            shapes.provenance().doc_prefixes(),
            shapes.box_role_vocab.clone(),
            shapes.shapes_graph.clone(),
        )
        .expect("re-parse the retained dataset")
    }

    /// The identity of `shapes` under an empty host property-function registry.
    fn identity_of(shapes: &Shapes) -> Identity {
        identity_with(shapes, &PropertyFunctionRegistry::new())
    }

    /// The identity of `shapes` under a caller-supplied property-function registry.
    fn identity_with(shapes: &Shapes, relations: &PropertyFunctionRegistry) -> Identity {
        build_identity(&digest_of(shapes), shapes, relations, &catalog_of(shapes))
            .expect("the fixture environment fingerprints")
    }

    /// Rebuild `identity` with the value at `position` replaced.
    ///
    /// The only way to move a component whose input is a CONSTANT (the profile), and
    /// the uniform way to drive `check_identity` across every position.
    fn with_component_replaced(identity: &Identity, position: usize, value: &[u8]) -> Identity {
        let mut out = Identity::new();
        for (index, component) in identity.components().iter().enumerate() {
            if index == position {
                out.push(component.label(), value);
            } else {
                out.push(component.label(), component.value());
            }
        }
        out
    }

    /// The single component position at which two identities disagree, asserting
    /// that there is EXACTLY one — which is what makes a "differs on X" test a
    /// single-input change rather than a change that moved several things at once.
    fn sole_divergence(left: &Identity, right: &Identity) -> usize {
        let mismatches = left.mismatches(right);
        assert_eq!(
            mismatches.len(),
            1,
            "exactly one component may differ, got: {mismatches:?}",
        );
        super::mismatch_position(&mismatches[0]).expect("a readable mismatch position")
    }

    // ── Registry fixtures ───────────────────────────────────────────────────────

    /// A registry carrying one expression-bodied function — the DECLARED population,
    /// which a restore rebuilds by re-parsing the shapes graph.
    fn declared_functions() -> UserFunctionRegistry {
        let mut registry = UserFunctionRegistry::new();
        registry.register_expr(EX_FN, Arity::Exact(1), Arc::new(|_call| Ok(None)));
        registry
    }

    /// A registry carrying one native function — the INJECTED population, which only
    /// a host can wire.
    fn injected_functions() -> UserFunctionRegistry {
        let mut registry = UserFunctionRegistry::new();
        registry.register_native(
            EX_FN,
            Arity::Exact(1),
            Volatility::Stable,
            Arc::new(|_args: &[&TermValue]| Ok(None)),
        );
        registry
    }

    /// An aggregate that answers nothing; it exists to be DECLARED.
    #[derive(Debug)]
    struct NullAggregate;

    /// An accumulator that accumulates nothing; see [`NullAggregate`].
    #[derive(Debug)]
    struct NullAccumulator;

    impl AggregateAccumulator for NullAccumulator {
        fn step(&mut self, _args: &[TermValue]) -> Result<(), EvalError> {
            Ok(())
        }

        fn combine(&mut self, _other: Box<dyn AggregateAccumulator>) -> Result<(), EvalError> {
            Ok(())
        }

        fn into_any(self: Box<Self>) -> Box<dyn std::any::Any + Send> {
            self
        }

        fn finish(self: Box<Self>) -> Result<Option<TermValue>, EvalError> {
            Ok(None)
        }
    }

    impl CustomAggregate for NullAggregate {
        fn arity(&self) -> Arity {
            Arity::Exact(1)
        }

        fn volatility(&self) -> Volatility {
            Volatility::Stable
        }

        fn algebraic_class(&self) -> AlgebraicClass {
            AlgebraicClass::Commutative
        }

        fn state_bound(&self) -> u64 {
            0
        }

        fn init(&self, _scalarvals: &[(String, TermValue)]) -> Box<dyn AggregateAccumulator> {
            Box::new(NullAccumulator)
        }
    }

    /// A FRESH registry instance carrying [`NullAggregate`] under [`EX_AGG`].
    fn aggregate_registry() -> AggregateRegistry {
        let mut registry = AggregateRegistry::new();
        registry.register(EX_AGG, Arc::new(NullAggregate));
        registry
    }

    /// A relation that yields no rows; it exists to be DECLARED.
    #[derive(Debug)]
    struct NullRelation {
        modes: [BindingPattern; 1],
    }

    impl NullRelation {
        fn new() -> Self {
            Self {
                modes: [BindingPattern::from_code("bf")],
            }
        }
    }

    /// A cursor over no rows; see [`NullRelation`].
    #[derive(Debug)]
    struct NullCursor;

    impl PfCursor for NullCursor {
        fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
            Ok(None)
        }
    }

    impl PropertyFunction for NullRelation {
        fn volatility(&self) -> Volatility {
            Volatility::Stable
        }

        fn arity(&self) -> PfArity {
            PfArity::new(1, 1)
        }

        fn modes(&self) -> &[BindingPattern] {
            &self.modes
        }

        fn rows_per_invocation(&self, _mode: BindingPattern) -> u64 {
            0
        }

        fn open(
            &self,
            _args: &PfArgs<'_>,
            _ceiling: Option<u64>,
        ) -> Result<Box<dyn PfCursor>, EvalError> {
            Ok(Box::new(NullCursor))
        }
    }

    /// A FRESH registry instance carrying [`NullRelation`] under [`EX_REL`].
    fn relation_registry() -> PropertyFunctionRegistry {
        let mut registry = PropertyFunctionRegistry::new();
        registry.register(EX_REL, Arc::new(NullRelation::new()));
        registry
    }

    // ── The order is the format ─────────────────────────────────────────────────

    #[test]
    fn identity_component_order_is_the_documented_one() {
        let identity = identity_of(&shapes_of(PLAIN_SHAPES));
        let labels: Vec<&str> = identity
            .components()
            .iter()
            .map(IdentityComponent::label)
            .collect();

        assert_eq!(
            labels,
            vec![
                "source-dataset",
                "shapes-graph",
                "doc-prefixes",
                "base",
                "profile",
                "box-role-vocab",
                "user-functions-declared",
                "user-functions-injected",
                "aggregate-registry",
                "property-function-registry",
                "class-catalog",
            ],
            "the component order is part of the format; reordering breaks every product \
             already written",
        );
        assert_eq!(labels.len(), COMPONENTS.len());
        assert_eq!(
            identity.component("profile"),
            Some(PROFILE_ID.as_bytes()),
            "the profile component carries the profile id verbatim",
        );
    }

    // ── One test per component: changing ONLY that input moves the digest ───────

    #[test]
    fn identity_differs_on_source_dataset() {
        // The extra triple declares no shape and names no class, so every other
        // component is identical by construction.
        let before = identity_of(&shapes_of(PLAIN_SHAPES));
        let after = identity_of(&shapes_of(PLAIN_SHAPES_PLUS_NOTE));

        assert_ne!(before.digest(), after.digest());
        assert_eq!(sole_divergence(&before, &after), 0);
    }

    #[test]
    fn identity_differs_on_shapes_graph() {
        let prefixes = crate::text_ingest::extract_prefixes(PLAIN_SHAPES);
        let before = identity_of(&shapes_with(
            PLAIN_SHAPES,
            &prefixes,
            None,
            Some(SHAPES_GRAPH_IRI),
        ));
        let after = identity_of(&shapes_with(
            PLAIN_SHAPES,
            &prefixes,
            None,
            Some(OTHER_SHAPES_GRAPH_IRI),
        ));

        assert_ne!(before.digest(), after.digest());
        assert_eq!(sole_divergence(&before, &after), 1);

        // Absent must not encode like present-and-empty here either: "the caller
        // named no shapes graph" and "the caller named the empty IRI" are two
        // configurations, and only one of them binds `$shapesGraph`.
        let unnamed = identity_of(&shapes_with(PLAIN_SHAPES, &prefixes, None, None));
        let empty = identity_of(&shapes_with(PLAIN_SHAPES, &prefixes, None, Some("")));
        assert_ne!(unnamed.digest(), empty.digest());
        assert_eq!(sole_divergence(&unnamed, &empty), 1);

        // …and the absent spelling cannot be forged by naming a graph after it.
        let forged = identity_of(&shapes_with(
            PLAIN_SHAPES,
            &prefixes,
            None,
            Some(super::ABSENT_MARKER),
        ));
        assert_ne!(unnamed.digest(), forged.digest());
    }

    #[test]
    fn identity_differs_on_doc_prefixes() {
        let before = identity_of(&shapes_with(
            PLAIN_SHAPES,
            &[("ex".to_owned(), "https://example.org/".to_owned())],
            None,
            Some(SHAPES_GRAPH_IRI),
        ));
        let after = identity_of(&shapes_with(
            PLAIN_SHAPES,
            &[("ex".to_owned(), "https://example.org/other#".to_owned())],
            None,
            Some(SHAPES_GRAPH_IRI),
        ));

        assert_ne!(before.digest(), after.digest());
        assert_eq!(sole_divergence(&before, &after), 2);
    }

    #[test]
    fn identity_differs_on_base() {
        // The base only reaches a parse through the TEXT entry point, and the
        // fixture has no relative references, so the dataset is identical and the
        // base is the only moving input.
        let before = parse_shapes(PLAIN_SHAPES, Some("https://example.org/a")).expect("parse");
        let after = parse_shapes(PLAIN_SHAPES, Some("https://example.org/b")).expect("parse");

        let before = identity_of(&before);
        let after = identity_of(&after);
        assert_ne!(before.digest(), after.digest());
        assert_eq!(sole_divergence(&before, &after), 3);
    }

    #[test]
    fn identity_differs_on_profile() {
        // The profile is a CONSTANT of this build, so the only way to move it is to
        // substitute the component — which is exactly what a product written by a
        // build with a different profile would present at restore.
        let before = identity_of(&shapes_of(PLAIN_SHAPES));
        let after = with_component_replaced(&before, 4, b"purrdf-shacl-core-v2");

        assert_ne!(before.digest(), after.digest());
        assert_eq!(sole_divergence(&before, &after), 4);
    }

    #[test]
    fn identity_differs_on_box_role_vocab() {
        let prefixes = crate::text_ingest::extract_prefixes(PLAIN_SHAPES);
        let before = identity_of(&shapes_with(
            PLAIN_SHAPES,
            &prefixes,
            Some(BoxRoleVocab::for_namespace("https://example.org/roles#")),
            Some(SHAPES_GRAPH_IRI),
        ));
        let after = identity_of(&shapes_with(
            PLAIN_SHAPES,
            &prefixes,
            Some(BoxRoleVocab::for_namespace("https://example.org/other#")),
            Some(SHAPES_GRAPH_IRI),
        ));

        assert_ne!(before.digest(), after.digest());
        assert_eq!(sole_divergence(&before, &after), 5);
    }

    #[test]
    fn identity_differs_on_declared_functions() {
        let mut shapes = shapes_of(PLAIN_SHAPES);
        let before = identity_of(&shapes);

        shapes.functions = Arc::new(declared_functions());
        let after = identity_of(&shapes);

        assert_ne!(before.digest(), after.digest());
        assert_eq!(sole_divergence(&before, &after), 6);
    }

    #[test]
    fn identity_differs_on_injected_functions() {
        let mut shapes = shapes_of(PLAIN_SHAPES);
        let before = identity_of(&shapes);

        shapes.functions = Arc::new(injected_functions());
        let after = identity_of(&shapes);

        assert_ne!(before.digest(), after.digest());
        assert_eq!(
            sole_divergence(&before, &after),
            7,
            "a host-injected native must move the INJECTED component, not the declared one",
        );
    }

    #[test]
    fn identity_differs_on_aggregate_registry() {
        let mut shapes = shapes_of(PLAIN_SHAPES);
        let before = identity_of(&shapes);

        shapes.aggregates = Arc::new(aggregate_registry());
        let after = identity_of(&shapes);

        assert_ne!(before.digest(), after.digest());
        assert_eq!(sole_divergence(&before, &after), 8);
    }

    #[test]
    fn identity_differs_on_property_function_registry() {
        let shapes = shapes_of(PLAIN_SHAPES);
        let before = identity_with(&shapes, &PropertyFunctionRegistry::new());
        let after = identity_with(&shapes, &relation_registry());

        assert_ne!(before.digest(), after.digest());
        assert_eq!(sole_divergence(&before, &after), 9);
    }

    #[test]
    fn identity_differs_on_class_catalog() {
        // The catalog is an ARGUMENT, so a catalog derived from a different shape
        // tree moves component 10 and nothing else — which is precisely the stale
        // analysis the pinned digest exists to make unservable.
        let shapes = shapes_of(PLAIN_SHAPES);
        let before = identity_of(&shapes);

        let stale = catalog_of(&shapes_of(OTHER_CLASS_SHAPES));
        let after = build_identity(
            &digest_of(&shapes),
            &shapes,
            &PropertyFunctionRegistry::new(),
            &stale,
        )
        .expect("the fixture environment fingerprints");

        assert_ne!(before.digest(), after.digest());
        assert_eq!(sole_divergence(&before, &after), 10);
    }

    // ── Injectivity ─────────────────────────────────────────────────────────────

    #[test]
    fn identity_is_injective_across_field_boundaries() {
        // The exact collision length-prefixing exists to prevent: without a prefix
        // on BOTH halves of every pair, each of these prefix maps flattens to `abc`
        // and two genuinely different configurations share one digest.
        let prefixes = crate::text_ingest::extract_prefixes(PLAIN_SHAPES);
        let left = identity_of(&shapes_with(
            PLAIN_SHAPES,
            &[("a".to_owned(), "bc".to_owned())],
            None,
            Some(SHAPES_GRAPH_IRI),
        ));
        let right = identity_of(&shapes_with(
            PLAIN_SHAPES,
            &[("ab".to_owned(), "c".to_owned())],
            None,
            Some(SHAPES_GRAPH_IRI),
        ));

        assert_ne!(
            left.digest(),
            right.digest(),
            "{{\"a\": \"bc\"}} and {{\"ab\": \"c\"}} must not share an identity",
        );
        assert_eq!(sole_divergence(&left, &right), 2);

        // The same aliasing across a PAIR boundary: two entries whose concatenation
        // is one longer entry's.
        let split = identity_of(&shapes_with(
            PLAIN_SHAPES,
            &[
                ("a".to_owned(), "b".to_owned()),
                ("c".to_owned(), "d".to_owned()),
            ],
            None,
            Some(SHAPES_GRAPH_IRI),
        ));
        let joined = identity_of(&shapes_with(
            PLAIN_SHAPES,
            &[("ab".to_owned(), "cd".to_owned())],
            None,
            Some(SHAPES_GRAPH_IRI),
        ));
        assert_ne!(split.digest(), joined.digest());

        // The neighbouring valid case: the document's own prefixes still produce a
        // stable identity, so the framing above refuses nothing real.
        assert_eq!(
            identity_of(&shapes_with(
                PLAIN_SHAPES,
                &prefixes,
                None,
                Some(SHAPES_GRAPH_IRI)
            ))
            .digest(),
            identity_of(&shapes_of(PLAIN_SHAPES)).digest(),
        );
    }

    // ── Stability ───────────────────────────────────────────────────────────────

    #[test]
    fn identity_is_stable_across_processes() {
        // Every input is re-derived from the SOURCE TEXT: a fresh Turtle parse (so
        // every term id is reassigned), a fresh parse of the shapes, a fresh class
        // analysis and fresh registry instances. Nothing is shared between the two
        // sides but the bytes of the fixture.
        let first = identity_with(&shapes_of(PLAIN_SHAPES), &relation_registry());
        let second = identity_with(&shapes_of(PLAIN_SHAPES), &relation_registry());

        assert_eq!(first, second);
        assert_eq!(first.digest(), second.digest());
        check_identity(&first, &second).expect("a re-derived identity must open");
    }

    // ── Component 5's distinctness ──────────────────────────────────────────────

    #[test]
    fn absent_vocabulary_differs_from_empty_vocabulary() {
        let prefixes = crate::text_ingest::extract_prefixes(PLAIN_SHAPES);
        let empty = BoxRoleVocab {
            graph_box_role: String::new(),
            box_abox: String::new(),
            box_tbox: String::new(),
            box_rbox: String::new(),
            box_cbox: String::new(),
            box_config_box: String::new(),
        };

        let absent = identity_of(&shapes_with(
            PLAIN_SHAPES,
            &prefixes,
            None,
            Some(SHAPES_GRAPH_IRI),
        ));
        let present_and_empty = identity_of(&shapes_with(
            PLAIN_SHAPES,
            &prefixes,
            Some(empty),
            Some(SHAPES_GRAPH_IRI),
        ));

        assert_ne!(
            absent.digest(),
            present_and_empty.digest(),
            "an INACTIVE box-role feature and an active one over empty IRIs are two different \
             parses and must not share an identity",
        );
        assert_eq!(sole_divergence(&absent, &present_and_empty), 5);

        let refusal =
            check_identity(&absent, &present_and_empty).expect_err("the two must not open");
        assert_eq!(refusal.dimension(), ProductDimension::Vocabulary);
    }

    // ── The class-catalog digest ────────────────────────────────────────────────

    #[test]
    fn class_catalog_digest_matches_rederived() {
        let shapes = shapes_of(PLAIN_SHAPES);
        let pinned = class_catalog_digest(&catalog_of(&shapes));

        // A restore re-derives the catalog from the AST it already decoded. That
        // re-derivation must land on the pinned digest, or the common path would
        // refuse every valid product.
        let rederived = class_catalog_digest(&catalog_of(&shapes));
        assert_eq!(pinned, rederived);

        // …and it is not a constant: a different shape tree is a different analysis.
        assert_ne!(
            pinned,
            class_catalog_digest(&catalog_of(&shapes_of(OTHER_CLASS_SHAPES))),
        );

        // The catalog really does carry the fixture's classes, or the assertions
        // above would hold for an empty digest.
        let catalog = catalog_of(&shapes);
        let mut classes: Vec<&str> = catalog.entries().map(|(class, _)| class.as_str()).collect();
        classes.sort_unstable();
        assert_eq!(
            classes,
            vec!["https://example.org/Person", "https://example.org/Thing"],
        );
    }

    // ── Refusals name their dimension and carry both values ─────────────────────

    #[test]
    fn check_identity_names_the_violated_dimension() {
        let expected = identity_of(&shapes_of(PLAIN_SHAPES));

        for (position, (label, dimension)) in COMPONENTS.iter().enumerate() {
            let actual = with_component_replaced(
                &expected,
                position,
                b"https://example.org/a-different-value",
            );
            let Err(refusal) = check_identity(&expected, &actual) else {
                panic!("component `{label}` (position {position}) must refuse")
            };

            assert_eq!(
                refusal.dimension(),
                *dimension,
                "component `{label}` (position {position}) must refuse under its own dimension",
            );

            // BOTH values, via the library's own rendering — that is the whole
            // reason the identity is decodable rather than one opaque digest.
            let rendered = expected.mismatches(&actual)[0].to_string();
            assert!(
                refusal.message().contains(&rendered),
                "component `{label}`: the refusal must carry what each side holds, got {refusal}",
            );
            assert!(
                refusal
                    .message()
                    .contains("https://example.org/a-different-value"),
                "component `{label}`: the refusal must name the supplied value, got {refusal}",
            );
            assert!(
                refusal.to_string().starts_with(dimension.label()),
                "component `{label}`: a refusal must lead with its dimension label, got {refusal}",
            );
        }

        // Two components share the FunctionRegistry dimension by design, and the
        // refusal still names the position through the rendered mismatch.
        let injected = with_component_replaced(&expected, 7, b"https://example.org/other");
        let refusal = check_identity(&expected, &injected).expect_err("refuses");
        assert_eq!(refusal.dimension(), ProductDimension::FunctionRegistry);
        assert!(refusal.message().contains("position 7"));

        // A text component shows both sides literally, not only as a digest.
        let moved = with_component_replaced(&expected, 1, b"https://example.org/moved");
        let refusal = check_identity(&expected, &moved).expect_err("refuses");
        assert!(refusal.message().contains(SHAPES_GRAPH_IRI));
        assert!(refusal.message().contains("https://example.org/moved"));

        // THE NEIGHBOURING VALID CASE — the untouched identity opens. Without it a
        // `check_identity` that refused everything would pass every assertion above.
        check_identity(&expected, &identity_of(&shapes_of(PLAIN_SHAPES)))
            .expect("an identical environment must open");
    }

    #[test]
    fn check_identity_refuses_an_identity_of_a_different_shape() {
        let expected = identity_of(&shapes_of(PLAIN_SHAPES));

        // An identity from a build that binds MORE inputs than this one: the
        // disagreement sits past the last known position, so it refuses fail-closed
        // under the residual dimension rather than being attributed to a component
        // this build can name.
        let longer = expected.clone().with("future-input", b"x");
        let refusal = check_identity(&longer, &expected).expect_err("a shorter identity refuses");
        assert_eq!(refusal.dimension(), ProductDimension::Malformed);
        assert!(refusal.message().contains("caller supplied nothing"));

        // And the reverse direction, for the same reason.
        let refusal = check_identity(&expected, &longer).expect_err("a longer identity refuses");
        assert_eq!(refusal.dimension(), ProductDimension::Malformed);
    }

    // ── Valid neighbours: environments that MUST still open ─────────────────────

    #[test]
    fn prefixes_reordered_in_the_source_text_still_match() {
        // The same logical prefix map, declared in a different order in the source.
        // The `@prefix` order is not a fact about the shapes graph, and a product
        // that refused here would refuse a caller who did nothing wrong.
        const REORDERED: &str = r"
            @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
            @prefix sh: <http://www.w3.org/ns/shacl#> .
            @prefix ex: <https://example.org/> .

            ex:ThingShape a sh:NodeShape ;
                sh:targetClass ex:Thing ;
                sh:property [
                    sh:path ex:name ;
                    sh:minCount 1 ;
                    sh:datatype xsd:string ;
                ] ;
                sh:property [
                    sh:path ex:owner ;
                    sh:class ex:Person ;
                ] .
        ";

        let straight = identity_of(&shapes_of(PLAIN_SHAPES));
        let reordered = identity_of(&shapes_of(REORDERED));
        check_identity(&straight, &reordered)
            .expect("a reordered @prefix block is the same configuration and must open");
        assert_eq!(straight.digest(), reordered.digest());

        // The same map handed over in reverse order through the DATASET entry point,
        // which does no sorting of its own — so this exercises the encoder's sort
        // rather than `extract_prefixes`'.
        let forward = vec![
            ("a".to_owned(), "https://example.org/a#".to_owned()),
            ("b".to_owned(), "https://example.org/b#".to_owned()),
        ];
        let backward: Vec<(String, String)> = forward.iter().rev().cloned().collect();
        assert_ne!(forward, backward, "the two orders must actually differ");
        check_identity(
            &identity_of(&shapes_with(
                PLAIN_SHAPES,
                &forward,
                None,
                Some(SHAPES_GRAPH_IRI),
            )),
            &identity_of(&shapes_with(
                PLAIN_SHAPES,
                &backward,
                None,
                Some(SHAPES_GRAPH_IRI),
            )),
        )
        .expect("one prefix map in two orders must open");
    }

    #[test]
    fn absent_vocabulary_matches_absent_vocabulary() {
        // The valid neighbour of `absent_vocabulary_differs_from_empty_vocabulary`:
        // an inactive box-role feature on BOTH sides is a match, not a refusal.
        let prefixes = crate::text_ingest::extract_prefixes(PLAIN_SHAPES);
        let left = shapes_with(PLAIN_SHAPES, &prefixes, None, Some(SHAPES_GRAPH_IRI));
        let right = shapes_with(PLAIN_SHAPES, &prefixes, None, Some(SHAPES_GRAPH_IRI));
        assert!(left.box_role_vocab.is_none() && right.box_role_vocab.is_none());

        check_identity(&identity_of(&left), &identity_of(&right))
            .expect("two inactive box-role configurations must open");

        // And two INDEPENDENTLY BUILT equal vocabularies match too.
        let vocab = || Some(BoxRoleVocab::for_namespace("https://example.org/roles#"));
        check_identity(
            &identity_of(&shapes_with(
                PLAIN_SHAPES,
                &prefixes,
                vocab(),
                Some(SHAPES_GRAPH_IRI),
            )),
            &identity_of(&shapes_with(
                PLAIN_SHAPES,
                &prefixes,
                vocab(),
                Some(SHAPES_GRAPH_IRI),
            )),
        )
        .expect("two equal box-role vocabularies must open");
    }

    #[test]
    fn fresh_registry_instances_with_identical_declarations_match() {
        // This is what the CONTENT fingerprints exist for. A restoring host wires its
        // registries from scratch, so every `Arc` differs and — for the registries
        // that carry one — the process-lifetime instance id restarted at 1. An
        // identity that bound instance identity would refuse every valid restore.
        let mut left = shapes_of(PLAIN_SHAPES);
        left.functions = Arc::new(declared_functions());
        left.aggregates = Arc::new(aggregate_registry());

        let mut right = shapes_of(PLAIN_SHAPES);
        right.functions = Arc::new(declared_functions());
        right.aggregates = Arc::new(aggregate_registry());

        assert!(
            !Arc::ptr_eq(&left.aggregates, &right.aggregates),
            "the two registries must really be separate instances",
        );

        check_identity(
            &identity_with(&left, &relation_registry()),
            &identity_with(&right, &relation_registry()),
        )
        .expect("equal declarations in fresh registry instances must open");

        // The contrast that keeps the assertion above from holding for a constant:
        // a registry with DIFFERENT declarations still refuses.
        let mut other = shapes_of(PLAIN_SHAPES);
        other.functions = Arc::new(injected_functions());
        other.aggregates = Arc::new(aggregate_registry());
        let refusal = check_identity(
            &identity_with(&left, &relation_registry()),
            &identity_with(&other, &relation_registry()),
        )
        .expect_err("different declarations must refuse");
        assert_eq!(refusal.dimension(), ProductDimension::FunctionRegistry);
    }
}
