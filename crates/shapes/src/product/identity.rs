// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
//! | 7 | user functions, [`FnPopulation::Injected`], + the implementation identity | `user-functions-injected` | [`FunctionRegistry`] |
//! | 8 | the custom-aggregate registry, + the implementation identity | `aggregate-registry` | [`AggregateRegistry`] |
//! | 9 | the property-function registry, + the implementation identity | `property-function-registry` | [`PropertyFunctionRegistry`] |
//! | 10 | the class catalog's digest | `class-catalog` | [`ClassCatalog`] |
//! | 11 | the declared parser options: relation lists **sorted**, extension namespaces in **declaration order** | `parse-configuration` | [`ParseConfiguration`] |
//!
//! Row 11 is the OTHER half of the seam row 9 covers. A registry's keys decide
//! which EXACT predicate IRIs are calls; a declared namespace decides it for a whole
//! prefix, including IRIs no registry names — and an IRI under a declared namespace
//! that no registry answers is a hard error rather than a silent data triple. So a
//! product written under a declared relation namespace and restored under a host that
//! declares nothing would read every prefixed relation IRI in its shapes graph as
//! ordinary data, match nothing, and report conformance. Row 9 alone does not catch
//! that: both hosts can hold the identical registry and still disagree about which
//! predicates are calls.
//!
//! Its three lists are NOT folded alike, and that asymmetry is load-bearing. The two
//! RELATION lists are sorted for the reason rows 2 and 5 are — recognition there is
//! order-independent and nothing is stripped, so a declaration is a SET and two
//! callers who declared the same namespaces in different order must not fail to open
//! each other's products. `extension_fn_namespaces` is folded in DECLARATION ORDER,
//! because its order is first-match-wins for prefix STRIPPING: with
//! `["http://example.org/a/", "http://example.org/a/b/"]` the IRI
//! `http://example.org/a/b/f` strips to `b/f`, and reversed it strips to `f` — two
//! different function names for one IRI. Sorting it would encode both orders
//! identically and admit a product into a host that resolves its extension-function
//! calls differently, which is the very substitution this row exists to refuse.
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
//! # Rows 7, 8 and 9 carry a second half: WHICH implementations
//!
//! The three host rows are content fingerprints of DECLARATIONS — an entry's IRI,
//! its arity, its volatility, the metadata a registry can state about itself. That
//! is all a fingerprint reproducible in another process can ever be, and
//! `purrdf_sparql_eval`'s own registry-identity documentation states the limit
//! plainly: two registries built independently can register the same IRI to two
//! DIFFERENT trait-object implementations that declare identically, and no
//! declaration digest can tell them apart. A binding built from declarations alone
//! would therefore admit a product under a same-named, same-arity,
//! differently-behaving native and validate green under someone else's semantics —
//! the silent wrong answer this whole codec exists to rule out, arriving through
//! the one door a fingerprint cannot guard.
//!
//! The same documentation states the caller's half of the obligation: a caller
//! crossing a process boundary binds declarations and must pair that binding with
//! whatever separately identifies the implementations BEHIND those declarations.
//! `implementation_identity` is that pairing — an opaque caller-chosen byte string
//! naming the build of the native code being wired — and
//! [`bind_implementations`] folds it into each of the three host rows.
//!
//! It is folded into all three rather than carried as a row of its own because the
//! three registries are wired by ONE host build: the natives, the aggregates and
//! the relations come out of the same binary, so an identity that moved for one of
//! them has moved for all three, and a row a restore could compare independently of
//! the registry it qualifies would be a fact about nothing. Folding also keeps the
//! refusal attributable — a mismatch still reports the registry dimension whose row
//! disagreed first, rather than a twelfth dimension meaning "something about your
//! host".
//!
//! An ABSENT identity (the empty byte string, which is what
//! [`HostBindings::empty`](super::HostBindings::empty) carries) encodes as nothing
//! at all: the row is the bare fingerprint, byte for byte what a build that had
//! never heard of implementation identities would write. That is what keeps every
//! product with no injected population — which is every product of
//! [`ShapesProfile::CORE`](super::ShapesProfile::CORE) a host wires nothing into —
//! encoding exactly as before. A product whose injected population is NOT empty
//! cannot be written without one at all; see
//! [`injected_population_is_empty`] and the writer's refusal.
//!
//! What the identity does NOT do is verify itself. PurRDF cannot read a host's
//! machine code and confirm the bytes name it; the identity is the caller's claim
//! about its own build, and the codec's guarantee is exactly and only that a
//! restore claiming a DIFFERENT one is refused. A host that spells two different
//! builds with one identity has told the binding they are the same build, and the
//! binding believes it.
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
//! # Row 10 is a digest over a body the product CARRIES
//!
//! `crate::plan::ClassCatalog` is a derivation of the shape tree: a cycle-safe
//! walk that collects every reachable `sh:class` / `sh:targetClass` /
//! `shnex:instancesOf` IRI and assigns each a position. The product carries that
//! walk's RESULT — in the AST section, as field 7 (`super::ast`) — and row 10 pins
//! its [`class_catalog_digest`]. A restore reads the body and checks it here.
//!
//! The body travels because "restore without repeated shared analysis" is what a
//! prepared product is FOR, and the class walk is that shared analysis. An earlier
//! arrangement pinned the digest and carried nothing, on the argument that the walk
//! is cheap and that carrying a body makes a STALE ANALYSIS SERVABLE — a product
//! written by a build whose walk had a bug, or simply a different reachability rule,
//! would restore that build's catalog and validate against it, verified and wrong.
//! The hazard is real; the conclusion was not, because pinning-only does not avoid
//! the work, it only proves the work agreed after doing it again. What actually
//! closes the hazard is a pair of checks, and they cover two different halves:
//!
//! * the STAGE ID covers the DERIVATION. `super::STAGE_ID` is digested from the
//!   class walk's own source, so a build whose reachability rule differs cannot
//!   share a stage id with the writer's, its products are refused by `admit`, and
//!   `rebuild` re-derives the analysis rather than reading the carried one. Note
//!   which way this cuts: before the body travelled, that hazard was OPEN and
//!   unguarded — the walk is an algorithm and not a model type, so changing the
//!   rule moved the meaning while leaving the stage id exactly where it was.
//! * row 10 covers the BODY. The digest folds in each class IRI and the position it
//!   was given, so a carried catalog that is not the one the writer wrote is refused
//!   on [`ClassCatalog`] before any preparation exists.
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
//! [`ParseConfiguration`]: ProductDimension::ParseConfiguration

use ::purrdf::PackDigest;
use purrdf_core::ContentDigest;
use purrdf_core::artifact::identity::{Identity, IdentityMismatch};
use purrdf_core::ir::pack::bits::write_varint;
use purrdf_sparql_algebra::ParserOptions;
use purrdf_sparql_eval::user_fn::FnPopulation;
use purrdf_sparql_eval::{
    AggregateRegistry, EvalError, PropertyFunctionRegistry, UserFunctionRegistry, agg_fn,
    property_function_content_fingerprint, user_fn,
};

use crate::model::BoxRoleVocab;
use crate::plan::ClassCatalog;
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
/// together. `stage_id_matches_shipped_constant` fails the moment the census's copy moves, so
/// a divergence is loud rather than silent.
pub(crate) const PROFILE_ID: &str = "purrdf-shacl-core-v1";

/// Every identity component, in the fixed order of the table in the
/// [module docs](self): its label and the [`ProductDimension`] a disagreement at
/// that POSITION refuses under.
///
/// One table drives both [`build_identity`] (which pushes in this order) and
/// [`check_identity`] (which indexes by position), so the labels and the dimensions
/// cannot drift apart into two hand-maintained lists.
const COMPONENTS: [(&str, ProductDimension); 12] = [
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
    ("parse-configuration", ProductDimension::ParseConfiguration),
];

/// The positions of the three HOST-supplied rows in [`COMPONENTS`]: the injected
/// user functions, the custom aggregates and the property functions.
///
/// Named by index against the one table that also drives [`assemble`], so the three
/// sites that build these rows — the identity assembly, the cheap restore check and
/// the emptiness test — can never come to mean other rows.
const HOST_POSITIONS: [usize; 3] = [7, 8, 9];

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

/// Qualify one host registry's declaration fingerprint with the caller's
/// implementation identity — the second half of rows 7, 8 and 9.
///
/// `fingerprint` is the registry's 32-byte content digest and
/// `implementation_identity` is the opaque byte string the caller uses to name the
/// build of the native code behind those declarations. An EMPTY identity is the
/// absent one and appends nothing, so a product wired to no host implementations
/// encodes each row as the bare digest — byte for byte what a build that bound
/// declarations alone would write.
///
/// A present identity is appended through [`push_part`], which makes the pair
/// injective in both halves and makes the two spellings impossible to confuse: the
/// digest is a fixed 32 bytes, [`push_part`] writes at least one length byte, and an
/// identity that is present is non-empty by definition — so a qualified row is never
/// 32 bytes long and can never be read as an unqualified one.
fn bind_implementations(fingerprint: &[u8], implementation_identity: &[u8]) -> Vec<u8> {
    let mut out = fingerprint.to_vec();
    if implementation_identity.is_empty() {
        return out;
    }
    push_part(&mut out, implementation_identity);
    out
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
/// The declared parser options, encoded for row 11.
///
/// The two RELATION lists are SORTED and deduplicated before framing: recognition
/// there is order-independent — an IRI is a property function iff it prefix-matches
/// `property_fn_namespaces` or exactly matches `property_fn_iris`, and nothing is
/// stripped — so order is not part of their meaning, and two callers who declared the
/// same namespaces in a different order must open each other's products.
///
/// `extension_fn_namespaces` is NOT sorted, and that asymmetry is deliberate. Its
/// order is first-match-wins for prefix STRIPPING, so the order is part of the
/// meaning: with `["http://example.org/a/", "http://example.org/a/b/"]` the IRI
/// `http://example.org/a/b/f` strips to `b/f`, and with the two entries reversed it
/// strips to `f` — two different function names for one IRI. Sorting them would
/// encode both orders identically, so a product prepared under one would be admitted
/// under the other and its restored preparation would resolve extension-function
/// calls differently from the environment it was written for. That is precisely the
/// silent-wrong-answer this row exists to refuse, so this list is folded in
/// DECLARATION order, deduplicated keeping the first occurrence (a later duplicate
/// can never win a first-match, so dropping it changes no parse).
///
/// The lists are framed separately rather than concatenated, because the same string
/// declared as an extension-function namespace and as a relation namespace configures
/// two different seams.
///
/// The options are DESTRUCTURED rather than read field by field, and that is the
/// load-bearing detail. This row exists to stop a product prepared under one parse
/// configuration from restoring green under another, so a parse-affecting field added
/// to `ParserOptions` later and not folded here would silently reopen exactly the hole
/// the row was added to close — and nothing would fail. Destructuring makes the
/// omission a compile error instead: a fourth field lands here as "missing structure
/// field" before it can land in production as a wrong answer.
fn encode_parser_options(options: &ParserOptions) -> Vec<u8> {
    let ParserOptions {
        extension_fn_namespaces,
        property_fn_namespaces,
        property_fn_iris,
    } = options;
    // Sized up front rather than grown. The three labels and three 8-byte counts are
    // always written, even when every list is empty, so an empty configuration used to
    // walk the doubling ladder from zero for a payload whose size is known before the
    // loop starts -- allocations bought on the once-per-restore path for nothing. The
    // reserve covers the fixed part; a host that actually declares namespaces grows
    // past it, which is the case worth paying for.
    const FIXED_PART: usize = 128;
    let declared: usize = extension_fn_namespaces
        .iter()
        .chain(property_fn_namespaces)
        .chain(property_fn_iris)
        .map(|value| value.len() + 2)
        .sum();
    let mut out = Vec::with_capacity(FIXED_PART + declared);
    // `true` = this list's order is part of its meaning and is preserved.
    for (label, list, ordered) in [
        ("extension-fn-namespaces", extension_fn_namespaces, true),
        ("property-fn-namespaces", property_fn_namespaces, false),
        ("property-fn-iris", property_fn_iris, false),
    ] {
        push_part(&mut out, label.as_bytes());
        let mut values: Vec<&str> = list.iter().map(String::as_str).collect();
        if ordered {
            // Declaration order kept; only exact repeats dropped, first occurrence
            // winning, because a later duplicate can never win a first-match.
            let mut seen = std::collections::BTreeSet::new();
            values.retain(|value| seen.insert(*value));
        } else {
            values.sort_unstable();
            values.dedup();
        }
        push_part(&mut out, &(values.len() as u64).to_be_bytes());
        for value in values {
            push_part(&mut out, value.as_bytes());
        }
    }
    out
}

// The class-catalog digest
// ---------------------------------------------------------------------------

/// The domain separator the class-catalog digest opens with, so its preimage can
/// never coincide with another content fingerprint's.
const CLASS_CATALOG_DOMAIN: &str = "purrdf-shapes/product/class-catalog";

/// A content digest over `catalog`'s key-sorted entries: every planned class IRI and
/// the position it was assigned.
///
/// The product carries the catalog BODY too (`super::ast` field 7); this is what
/// binds that body to the product it arrived in — see the [module docs](self) for
/// which half of the stale-analysis hazard this closes and which half the stage id
/// closes.
///
/// The position is folded in, not just the IRI set: the position is what a
/// a dataset binding indexes its resolved-`TermId` class row by, so two catalogs over the
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
/// shapes graph can describe; `implementation_identity` is the caller's opaque name
/// for the build of the native implementations behind the injected declarations,
/// empty when there are none (see the [module docs](self)); and `classes` is the
/// derived class analysis (`crate::engine::PreparedShapes::class_catalog`).
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
    parser_options: &ParserOptions,
    implementation_identity: &[u8],
    classes: &ClassCatalog,
) -> Result<Identity, ShapesProductError> {
    assemble(
        dataset.as_bytes().as_slice(),
        shapes,
        property_functions,
        parser_options,
        implementation_identity,
        classes,
    )
}

/// Whether the preparation about to be written injects NOTHING a host would have to
/// wire — no native functions, no custom aggregates, no host relations.
///
/// Defined the way the binding itself defines it, rather than by probing the three
/// registries' internals: the injected population is empty exactly when the three
/// host rows are the rows the canonical EMPTY registries produce. A future registry
/// kind, or a future entry a fingerprint learns to distinguish, therefore reaches
/// this answer without anyone remembering to add a branch — and the answer can never
/// disagree with the rows [`assemble`] actually writes, because both come out of
/// [`host_components`].
///
/// The writer asks this to decide whether a product may be written with NO
/// implementation identity; see [`PreparedShapes::to_product`](crate::engine::PreparedShapes::to_product).
///
/// # Errors
///
/// The registry dimension whose fingerprint could not be computed, exactly as
/// [`build_identity`] reports it.
pub(crate) fn injected_population_is_empty(
    shapes: &Shapes,
    property_functions: &PropertyFunctionRegistry,
) -> Result<bool, ShapesProductError> {
    let wired = host_components(
        &shapes.functions,
        &shapes.aggregates,
        property_functions,
        &[],
    )?;
    let nothing = host_components(
        &UserFunctionRegistry::EMPTY,
        &AggregateRegistry::EMPTY,
        &PropertyFunctionRegistry::EMPTY,
        &[],
    )?;
    Ok(wired == nothing)
}

/// Compare a decoded identity against one rebuilt from the RESTORED shapes graph,
/// the host's registries and the re-derived class catalog — **without**
/// re-canonicalizing the shapes dataset.
///
/// # Why component 0 is copied rather than recomputed
///
/// Row 0 is the shapes dataset's canonical pack digest, and computing it is a
/// graph-isomorphism canonicalization over the shapes graph's blank nodes — the
/// COLD tier `super::dataset::certify_dataset` owns. Running it on every restore
/// would very plausibly cost more than the shapes parse a prepared product exists
/// to eliminate, so this check takes row 0's value from `declared` verbatim and
/// every other row from the environment. The dataset section is still protected on
/// the common path: the artifact envelope's per-section SHA-256 and whole-container
/// digest both cover its bytes, so the section cannot be swapped without the
/// container refusing first. What is deliberately NOT established here is that
/// those bytes CANONICALIZE to the digest row 0 claims — that is the one statement
/// only certification makes, and `ShapesProductView::certify` is the only place
/// that makes it.
///
/// Copying row 0 means a disagreement can never be reported at position 0 from
/// here, which is exactly the property `certify_is_not_reachable_from_admit` pins:
/// a product whose stored canonical digest has been tampered with ADMITS and FAILS
/// certification.
///
/// # Errors
///
/// The [`ProductDimension`] at the first disagreeing position, per
/// [`check_identity`], or a registry dimension when a fingerprint cannot be
/// computed at all.
pub(crate) fn check_restored_identity(
    declared: &Identity,
    shapes: &Shapes,
    property_functions: &PropertyFunctionRegistry,
    parser_options: &ParserOptions,
    implementation_identity: &[u8],
    classes: &ClassCatalog,
) -> Result<(), ShapesProductError> {
    // An identity whose row 0 carries another label has already failed the format's
    // own rules; the empty value below lands as a position-0 disagreement, which
    // `check_identity` reports under `DatasetIdentity`. Fail-closed, never skipped.
    let declared_dataset = declared.component(COMPONENTS[0].0).unwrap_or(&[]);
    let actual = assemble(
        declared_dataset,
        shapes,
        property_functions,
        parser_options,
        implementation_identity,
        classes,
    )?;
    check_identity(declared, &actual)
}

/// The CHEAP half of the restore check: the three components the executing HOST
/// supplies, compared before anything expensive is decoded.
///
/// Rows 7, 8 and 9 are the only ones a caller can get wrong by wiring their own
/// process differently — every other row is a property of the product's own
/// content, and re-deriving those means decoding the model first. Each of the three
/// carries the host's declarations AND the identity it gave the implementations
/// behind them, so a host that wired a same-named, same-arity native out of a
/// DIFFERENT build is refused here too, not merely one that wired a different IRI.
/// [`check_restored_identity`] covers all eleven and is what actually binds the
/// restore; this runs first so a host that supplied the wrong registries is told so
/// without paying for a dataset restore and an AST decode it is going to discard.
///
/// It is therefore a strictly redundant early exit, and deliberately so: it can
/// only refuse what the total check would refuse a moment later, so it can never
/// turn a valid product away. That property is what makes the optimization safe.
///
/// # Errors
///
/// [`ProductDimension::FunctionRegistry`],
/// [`ProductDimension::AggregateRegistry`] or
/// [`ProductDimension::PropertyFunctionRegistry`] when the host's registry is not
/// the one the product was prepared against, or when a registry cannot state its
/// own declarations.
pub(crate) fn check_host_bindings(
    declared: &Identity,
    functions: &UserFunctionRegistry,
    aggregates: &AggregateRegistry,
    property_functions: &PropertyFunctionRegistry,
    implementation_identity: &[u8],
) -> Result<(), ShapesProductError> {
    let host = host_components(
        functions,
        aggregates,
        property_functions,
        implementation_identity,
    )?;

    for (position, computed) in HOST_POSITIONS.into_iter().zip(host.iter()) {
        let (label, dimension) = COMPONENTS[position];
        if declared.component(label) != Some(computed.as_slice()) {
            return Err(ShapesProductError::new(dimension, fix_for(dimension)));
        }
    }
    Ok(())
}

/// The three HOST-supplied component values — rows 7, 8 and 9 of [`COMPONENTS`], in
/// that order — each a registry's declaration fingerprint qualified by the caller's
/// implementation identity.
///
/// One body for all three sites that need these rows: [`assemble`] writes them into
/// an identity, [`check_host_bindings`] compares them against a declared one, and
/// [`injected_population_is_empty`] compares them against the empty registries'. A
/// second transcription of "what a host row is" is precisely how the writer and the
/// restore check would drift into two answers about one product.
///
/// # Errors
///
/// [`ProductDimension::FunctionRegistry`],
/// [`ProductDimension::AggregateRegistry`] or
/// [`ProductDimension::PropertyFunctionRegistry`] when the corresponding registry
/// cannot state its own declarations.
fn host_components(
    functions: &UserFunctionRegistry,
    aggregates: &AggregateRegistry,
    property_functions: &PropertyFunctionRegistry,
    implementation_identity: &[u8],
) -> Result<[Vec<u8>; 3], ShapesProductError> {
    Ok([
        bind_implementations(
            &fingerprint(
                user_fn::content_fingerprint(functions, FnPopulation::Injected),
                ProductDimension::FunctionRegistry,
                "host-injected SPARQL function registry",
            )?,
            implementation_identity,
        ),
        bind_implementations(
            &fingerprint(
                agg_fn::content_fingerprint(aggregates),
                ProductDimension::AggregateRegistry,
                "custom-aggregate registry",
            )?,
            implementation_identity,
        ),
        bind_implementations(
            &fingerprint(
                property_function_content_fingerprint(property_functions),
                ProductDimension::PropertyFunctionRegistry,
                "property-function registry",
            )?,
            implementation_identity,
        ),
    ])
}

/// Corroborate a CERTIFIED dataset digest against the one the product's identity
/// claims — the statement no restore path makes.
///
/// [`check_restored_identity`] copies row 0 rather than recomputing it, so this is
/// the only comparison in the codec that can tell a product whose shapes dataset no
/// longer canonicalizes to the identity it was written under.
///
/// # Errors
///
/// [`ProductDimension::DatasetIdentity`] when the certified digest is not the one
/// the identity records.
pub(crate) fn certify_dataset_component(
    declared: &Identity,
    certified: &PackDigest,
) -> Result<(), ShapesProductError> {
    let (label, dimension) = COMPONENTS[0];
    if declared.component(label) == Some(certified.as_bytes().as_slice()) {
        return Ok(());
    }
    Err(ShapesProductError::new(
        dimension,
        format!(
            "this product's shapes dataset canonicalizes to {}, which is not the identity the \
             product claims it was prepared from; discard this product and re-prepare it from the \
             shapes graph, because the section and the binding over it no longer describe one \
             dataset",
            certified.to_hex()
        ),
    ))
}

/// Assemble the eleven component values in the fixed order of [`COMPONENTS`].
///
/// One body shared by the writer ([`build_identity`], which supplies a CERTIFIED
/// dataset digest) and the restore check ([`check_restored_identity`], which
/// supplies the declared one). Two transcriptions of an ordered tuple is exactly
/// how a writer and a reader drift into disagreeing about what position 6 means.
fn assemble(
    dataset: &[u8],
    shapes: &Shapes,
    property_functions: &PropertyFunctionRegistry,
    parser_options: &ParserOptions,
    implementation_identity: &[u8],
    classes: &ClassCatalog,
) -> Result<Identity, ShapesProductError> {
    let provenance = shapes.provenance();
    let [injected_functions, aggregates, relations] = host_components(
        &shapes.functions,
        &shapes.aggregates,
        property_functions,
        implementation_identity,
    )?;

    // In the fixed order of `COMPONENTS`, which is the order documented at the top
    // of this module and pinned by `identity_component_order_is_the_documented_one`.
    let values: [Vec<u8>; COMPONENTS.len()] = [
        dataset.to_vec(),
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
        injected_functions,
        aggregates,
        relations,
        class_catalog_digest(classes).as_bytes().to_vec(),
        encode_parser_options(parser_options),
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
             supplied for its execution; wire the SAME functions into the executing host, under \
             the SAME implementation identity, because the registry is what decides how a \
             function IRI resolves and the implementation identity is what says which build of \
             the native code stands behind it"
        }
        ProductDimension::AggregateRegistry => {
            "this product was prepared against a different custom-aggregate registry than the \
             one supplied for its execution; wire the SAME aggregates into the executing host, \
             under the SAME implementation identity, because the registry is what an \
             `AGG(<iri>, …)` call resolves against and the implementation identity is what says \
             which build of the native code stands behind it"
        }
        ProductDimension::PropertyFunctionRegistry => {
            "this product was prepared against a different property-function registry than the \
             one supplied for its execution; wire the SAME relations into the executing host, \
             under the SAME implementation identity, because the registry is what decides which \
             predicates are calls rather than ordinary triple patterns and the implementation \
             identity is what says which build of the native code stands behind them"
        }
        ProductDimension::ParseConfiguration => {
            "this product was prepared under a different parse configuration than the one \
             supplied for its execution; declare the SAME extension-function and relation \
             NAMESPACES on the executing host, because a declared namespace decides which \
             predicate IRIs are calls for a whole prefix — including IRIs no registry names, \
             where an unregistered one is a hard error rather than an ordinary data triple — so \
             two hosts holding the identical registries can still disagree about which \
             predicates are calls"
        }
        ProductDimension::ClassCatalog => {
            "the class analysis this product carries is not the one its own identity pins, so the \
             analysis a restore would compile its class-membership decisions against is not the \
             analysis the product was written with; discard this product and re-prepare it from \
             its shapes graph, because the section and the binding over it no longer describe one \
             analysis"
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

    use std::fmt::Write as _;

    use super::{COMPONENTS, PROFILE_ID, build_identity, check_identity, class_catalog_digest};
    use crate::engine::{PreparedShapes, parse_shapes};
    use crate::model::BoxRoleVocab;
    use crate::plan::ClassCatalog;
    use crate::product::dataset::{certify_dataset, encode_dataset};
    use crate::product::error::ProductDimension;
    use crate::shapes::{Shapes, from_dataset_with_config_and_graph};
    use purrdf_sparql_algebra::ParserOptions;

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
            &crate::text_ingest::parse_turtle_document(ttl, None)
                .expect("fixture parses")
                .prefixes,
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

    /// The identity of `shapes` under an empty host property-function registry and
    /// no implementation identity — the shape every product with nothing injected
    /// carries.
    fn identity_of(shapes: &Shapes) -> Identity {
        identity_with(shapes, &PropertyFunctionRegistry::new())
    }

    /// The identity of `shapes` under a caller-supplied property-function registry.
    fn identity_with(shapes: &Shapes, relations: &PropertyFunctionRegistry) -> Identity {
        identity_identified_by(shapes, relations, &[])
    }

    /// The identity of `shapes` under a caller-supplied property-function registry
    /// and a caller-supplied implementation identity.
    fn identity_identified_by(
        shapes: &Shapes,
        relations: &PropertyFunctionRegistry,
        implementation_identity: &[u8],
    ) -> Identity {
        build_identity(
            &digest_of(shapes),
            shapes,
            relations,
            &ParserOptions::default(),
            implementation_identity,
            &catalog_of(shapes),
        )
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
                "parse-configuration",
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
        let prefixes = crate::text_ingest::parse_turtle_document(PLAIN_SHAPES, None)
            .expect("fixture parses")
            .prefixes;
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
        let prefixes = crate::text_ingest::parse_turtle_document(PLAIN_SHAPES, None)
            .expect("fixture parses")
            .prefixes;
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

    // ── The implementation identity ─────────────────────────────────────────────

    /// The identity a caller gives its host implementations moves ALL THREE host
    /// rows and nothing else. Two hosts that declare the same natives, aggregates
    /// and relations out of two different builds must not share a binding — that is
    /// the whole gap a declaration fingerprint cannot close.
    #[test]
    fn identity_differs_on_the_implementation_identity() {
        let mut shapes = shapes_of(PLAIN_SHAPES);
        shapes.functions = Arc::new(injected_functions());
        let relations = relation_registry();

        let left = identity_identified_by(&shapes, &relations, b"build-a");
        let right = identity_identified_by(&shapes, &relations, b"build-b");

        assert_ne!(
            left.digest(),
            right.digest(),
            "two builds of the same declarations are two bindings",
        );
        let moved: Vec<usize> = left
            .mismatches(&right)
            .iter()
            .filter_map(super::mismatch_position)
            .collect();
        assert_eq!(
            moved,
            vec![7, 8, 9],
            "the three host rows carry the identity; no other row may move",
        );
    }

    /// An ABSENT implementation identity is the empty byte string, and it encodes as
    /// NOTHING: the three host rows are the bare declaration fingerprints, byte for
    /// byte what a build binding declarations alone would have written.
    ///
    /// This is what keeps every product with nothing injected — which is every
    /// product the common path writes — encoding exactly as before, and it is why
    /// the frozen release artifact still opens.
    #[test]
    fn an_absent_implementation_identity_encodes_as_nothing() {
        let shapes = shapes_of(PLAIN_SHAPES);
        let identity = identity_of(&shapes);

        for label in [
            "user-functions-injected",
            "aggregate-registry",
            "property-function-registry",
        ] {
            let value = identity.component(label).expect("the row is present");
            assert_eq!(
                value.len(),
                32,
                "{label} must be the bare 32-byte fingerprint when nothing is identified",
            );
        }

        // ...and the empty slice is the same fact as no identity at all, so there is
        // exactly one spelling of "nothing injected" and no second branch.
        assert_eq!(
            identity.digest(),
            identity_identified_by(&shapes, &PropertyFunctionRegistry::new(), b"").digest(),
        );
    }

    /// A PRESENT identity can never be read as an absent one, whatever bytes the
    /// caller chooses: the qualified row is a fixed 32-byte digest plus a
    /// length-framed value, so it is never 32 bytes long.
    ///
    /// The neighbouring valid case is the one a mechanism this strict must not
    /// break: two hosts naming the SAME build agree exactly, so an identity is a
    /// binding and not a nonce.
    #[test]
    fn a_present_implementation_identity_cannot_forge_an_absent_one() {
        let mut shapes = shapes_of(PLAIN_SHAPES);
        shapes.functions = Arc::new(injected_functions());
        let relations = PropertyFunctionRegistry::new();

        for identity in [b"x".as_slice(), b"\x00".as_slice(), &[0u8; 64]] {
            let built = identity_identified_by(&shapes, &relations, identity);
            let row = built
                .component("user-functions-injected")
                .expect("the row is present");
            assert_ne!(
                row.len(),
                32,
                "a qualified row must never be readable as an unqualified one",
            );
        }

        assert_eq!(
            identity_identified_by(&shapes, &relations, b"build-a").digest(),
            identity_identified_by(&shapes, &relations, b"build-a").digest(),
            "the same build named twice is one binding, not two",
        );
    }

    /// The injected population is empty exactly when nothing a host would have to
    /// wire is present — and non-empty the moment a native or an aggregate is.
    ///
    /// The writer reads this to decide whether a product may be written with no
    /// implementation identity at all, so an answer that drifted from the rows
    /// `assemble` writes would either refuse a product nobody needed to identify or
    /// emit one nothing could check.
    #[test]
    fn the_injected_population_is_empty_only_when_nothing_is_wired() {
        let relations = PropertyFunctionRegistry::new();

        let plain = shapes_of(PLAIN_SHAPES);
        assert!(
            super::injected_population_is_empty(&plain, &relations)
                .expect("the fixture fingerprints"),
            "a shapes graph nobody wired anything into injects nothing",
        );

        // A DECLARED function is the neighbouring valid case: it comes out of the
        // shapes graph's own content and a restore rebuilds it, so it is not
        // something a host has to identify.
        let mut declared = shapes_of(PLAIN_SHAPES);
        declared.functions = Arc::new(declared_functions());
        assert!(
            super::injected_population_is_empty(&declared, &relations)
                .expect("the fixture fingerprints"),
            "a declared function is rebuilt from the product, not wired by a host",
        );

        let mut native = shapes_of(PLAIN_SHAPES);
        native.functions = Arc::new(injected_functions());
        assert!(
            !super::injected_population_is_empty(&native, &relations)
                .expect("the fixture fingerprints"),
        );

        let mut aggregating = shapes_of(PLAIN_SHAPES);
        aggregating.aggregates = Arc::new(aggregate_registry());
        assert!(
            !super::injected_population_is_empty(&aggregating, &relations)
                .expect("the fixture fingerprints"),
        );

        assert!(
            !super::injected_population_is_empty(&plain, &relation_registry())
                .expect("the fixture fingerprints"),
            "a host relation is wiring no shapes graph can describe",
        );
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
            &ParserOptions::default(),
            &[],
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
        let prefixes = crate::text_ingest::parse_turtle_document(PLAIN_SHAPES, None)
            .expect("fixture parses")
            .prefixes;
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
        let prefixes = crate::text_ingest::parse_turtle_document(PLAIN_SHAPES, None)
            .expect("fixture parses")
            .prefixes;
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

    /// The class-catalog digest of [`PLAIN_SHAPES`], as lowercase hex.
    ///
    /// Pinned as a constant for the same reason `STAGE_ID` and `GOLDEN_LEN` are:
    /// this digest must NOT move. It is a fingerprint of the class walk's OUTPUT —
    /// the `(class, position)` pairs — under a domain separator, and it does not
    /// incorporate the stage id. The stage id is free to move whenever the walk's
    /// SOURCE changes, because that is what it is for; this one may only move when
    /// the walk reaches a different set of classes or ranks them differently, which
    /// is a semantic change to what a prepared product carries and never a
    /// re-pinning.
    ///
    /// A re-derivation check cannot state that. [`class_catalog_digest_matches_rederived`]
    /// computes both of its operands from the same call in the same process, so it
    /// proves DETERMINISM and would pass unchanged if the digest were altered
    /// completely. Only a committed value is stable across a code change, so here
    /// one is.
    const PLAIN_SHAPES_CLASS_CATALOG_DIGEST: &str =
        "eaa6b85267318d663007f5c7435b56ab6194a2b70631dd5ff5599c9a4e4182cc";

    /// **The class-catalog digest has not moved.**
    ///
    /// If this fails, the walk has changed WHAT IT COLLECTS — a class it no longer
    /// reaches, one it now reaches, or a different position for one it always had.
    /// Every one of those is a semantic regression in the analysis a prepared
    /// product carries, and every product ever minted disagrees with this build
    /// about it. Diagnose the walk. Do not re-pin the constant.
    ///
    /// The value is not merely whatever this build happened to produce when the
    /// constant was written. It was checked out of the tree as it stood BEFORE
    /// the walk was rewritten, built there, and computed: that build emits this
    /// same hex. So the constant records what the walk produced beforehand, and
    /// the rewrite is measured against it rather than described as equal to it.
    #[test]
    fn class_catalog_digest_matches_committed_constant() {
        let digest = class_catalog_digest(&catalog_of(&shapes_of(PLAIN_SHAPES)));
        let hex: String =
            digest
                .as_bytes()
                .iter()
                .fold(String::with_capacity(64), |mut out, byte| {
                    let _ = write!(out, "{byte:02x}");
                    out
                });
        assert_eq!(
            hex, PLAIN_SHAPES_CLASS_CATALOG_DIGEST,
            "the class-catalog digest moved, so the class walk now reaches a different set of \
             classes or ranks them differently; this is a semantic change to the analysis a \
             prepared product carries, not a constant to update"
        );
    }

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
        // rather than the codec's.
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
        let prefixes = crate::text_ingest::parse_turtle_document(PLAIN_SHAPES, None)
            .expect("fixture parses")
            .prefixes;
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
