// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The prepared-product codec for SHACL: the admission boundary where PurRDF
//! decides whether a sequence of bytes is a prepared shapes product it will
//! execute.
//!
//! # Why an admission boundary exists at all
//!
//! A prepared product is compiled once and then handed around — cached on disk,
//! shipped between processes, restored after a restart. By the time the bytes
//! come back they are **untrusted**: nothing in them is evidence of its own
//! provenance, and the validator that loads them cannot assume it produced them,
//! produced them at this version, or produced them against the same
//! caller-supplied configuration. Decoding is therefore not parsing — it is
//! admission. Every field is a claim the loader has to check before any of it
//! reaches the validator.
//!
//! # The surface
//!
//! [`PreparedShapes::to_product`] writes a preparation out;
//! [`ShapesProduct::open`] performs the envelope's own integrity checks and hands
//! back a [`ShapesProductView`] that can be INSPECTED without being admitted; and
//! the view's two consuming methods are the two ways those bytes become a
//! validator again.
//!
//! | Path | Tier | Re-derives | When |
//! |------|------|------------|------|
//! | [`admit`](ShapesProductView::admit) | COMMON — every restore | nothing expensive | the stage id is one this build knows |
//! | [`admit_expecting`](ShapesProductView::admit_expecting) | COMMON — every restore | nothing expensive | as `admit`, and the caller knows which product it wants |
//! | [`rebuild`](ShapesProductView::rebuild) | fallback | the whole shapes graph, from the carried dataset | the stage id is one this build does NOT know |
//! | [`certify`](ShapesProductView::certify) | COLD — never in normal usage | the shapes dataset's canonical identity | a verify subcommand, a conformance harness |
//!
//! `admit` and `admit_expecting` are two entry points over ONE body: the second
//! adds a single 32-byte comparison against the identity the caller requires, ahead
//! of every other check. Everything `admit` verifies is a question about the
//! executing ENVIRONMENT; the expectation is the one question about the ARTIFACT —
//! *is this the product I asked for?* — and without it a caller that names the
//! wrong file gets a successful restore and a report about a shapes graph nobody
//! asked about.
//!
//! Performance in the common case is paramount; conformance testing may be slow,
//! because it is never part of normal usage. That standing rule is the whole
//! reason these are three entry points and not one: `admit` must never reach the
//! canonicalization `certify` performs, because canonicalizing a shapes graph's
//! blank nodes would very plausibly cost more than the shapes parse this feature
//! exists to eliminate — a product slower than the thing it replaces, with every
//! test still passing.
//!
//! # `rebuild` is a SEAM, not a mode flag
//!
//! The writer has exactly ONE path. A product always carries every section, so
//! there is no "with dataset" and "without dataset" variant to choose between,
//! and no writer-side switch anyone can set wrong. The READER has two total entry
//! points at one boundary: a reader that meets a [`stage_id`] it does not know
//! refuses `admit` — the memo was written against a model this build no longer
//! has — and can still `rebuild` the preparation from the authenticated primitive
//! the product carries.
//!
//! **`rebuild` is not a Turtle fallback.** No RDF text is parsed and nothing is
//! re-read from disk: the shapes dataset travels inside the product, under the
//! envelope's per-section SHA-256 and whole-container digest, and rebuilding
//! re-derives the shapes from that. The rejected alternative — carrying only the
//! compiled model and telling a caller to keep the source document around for the
//! day the format moves — makes forward compatibility depend on a file the
//! product does not own and cannot authenticate. That is not a fallback; it is an
//! instruction to hope.
//!
//! # Why a refusal names a dimension
//!
//! A refusal here is itself a claim, so it carries the *exact* thing that failed:
//! [`error::ProductDimension`] is the closed set of ways a candidate product can
//! fail admission, and [`error::ShapesProductError`] pairs one of those with a
//! message that names the fix rather than only the fault. A caller can branch on
//! the dimension — a [`Malformed`] product is a corrupt cache to discard and
//! rebuild, a [`FormatVersion`] mismatch is a stale artifact to recompile, a
//! [`FunctionRegistry`] mismatch is a *configuration* error in the caller's own
//! code that rebuilding will not fix — and each of those is a different action.
//!
//! The rejected alternative was a single opaque error carrying only a string.
//! It types perfectly well and it is the thing this module exists to avoid: it
//! collapses "these bytes are garbage" and "you supplied a different function
//! registry than the one this product was prepared against" into one outcome, so
//! the only available recovery is the pessimistic one. Callers then do the
//! predictable thing and match on substrings of the message, which converts
//! every wording improvement into a silent behaviour change downstream.
//!
//! The failure this design prevents is the dangerous half of that collapse:
//! admitting a product whose *identity* no longer matches the environment
//! executing it. A product prepared under one vocabulary, prefix map, or
//! function registry encodes decisions that are only correct under those
//! inputs. Loading it under different ones does not crash — it validates, and
//! quietly returns a report for a shapes graph nobody asked about. Refusing on a
//! named dimension is what makes that mismatch loud at the boundary instead of
//! invisible in the results.
//!
//! # Container layout
//!
//! The bytes are a [`purrdf_core::artifact`] envelope under the magic `PURRSHP1`,
//! so the framing, the per-section digests, the trailer and the identity region
//! are the workspace's one implementation of those rules rather than a third
//! transcription of them. The section directory is TOTAL: all three kinds are
//! present in every product, possibly zero-length.
//!
//! | Kind | Section | Carries |
//! |------|---------|---------|
//! | 0 | `SECTION_IDENTITY` | the stage id, the profile id and the parse provenance |
//! | 1 | `SECTION_DATASET` | the shapes dataset, in the pack container (`dataset`) |
//! | 2 | `SECTION_AST` | the declarative model (`ast`) |
//!
//! The envelope's own identity REGION carries the [`Identity`] — which inputs the
//! product was compiled from (`identity`). That is a different fact from the
//! identity SECTION, which is what the product says it *is*: a binding versus a
//! self-description, and a restore needs both.
//!
//! Nothing in this module touches the filesystem, a clock, a thread, or a source
//! of randomness: it names outcomes over caller-supplied bytes and stays
//! `wasm32-unknown-unknown` compatible.
//!
//! [`Malformed`]: error::ProductDimension::Malformed
//! [`FormatVersion`]: error::ProductDimension::FormatVersion
//! [`FunctionRegistry`]: error::ProductDimension::FunctionRegistry
//! [`stage_id`]: ShapesProductView::stage_id

use std::collections::BTreeMap;
use std::sync::Arc;

use purrdf_core::artifact::{ArtifactBuilder, ArtifactError, ArtifactSpec, ArtifactView, Identity};
use purrdf_core::governor::{ResourceDimension, TrippedGovernor};
use purrdf_core::ir::pack::bits::{read_varint, write_varint};
use purrdf_sparql_eval::{AggregateRegistry, PropertyFunctionRegistry, UserFunctionRegistry};

use crate::engine::PreparedShapes;
use crate::model::BoxRoleVocab;
use crate::provenance::ParseProvenance;
use crate::shapes::{Shapes, link};

use certified::CertifiedParts;

pub(crate) mod ast;
mod certified;
pub(crate) mod dataset;
pub mod error;
pub(crate) mod identity;

#[cfg(test)]
mod tests;

pub use error::{ProductDimension, ShapesProductError};

// ---------------------------------------------------------------------------
// The format
// ---------------------------------------------------------------------------

/// The 8-byte header magic of a prepared SHACL product. No other artifact format
/// in this workspace may share it, or each would open the other's bytes.
const MAGIC: [u8; 8] = *b"PURRSHP1";

/// The container format version this build writes and requires. A buffer carrying
/// any other value is refused rather than best-effort parsed.
const FORMAT_VERSION: u32 = 1;

/// The section carrying the product's self-description: its stage id, its profile
/// id and the parse provenance a rebuild needs.
const SECTION_IDENTITY: u32 = 0;

/// The section carrying the shapes dataset, in the pack container.
const SECTION_DATASET: u32 = 1;

/// The section carrying the declarative model.
const SECTION_AST: u32 = 2;

/// The complete artifact declaration. The section count is the totality law: all
/// three kinds on every build, all three required on every open.
const SPEC: ArtifactSpec = ArtifactSpec::new(MAGIC, FORMAT_VERSION, 3);

/// The PREPARATION STAGE ID: a content-derived capability digest over the whole
/// declarative model plus the tables the model's meaning depends on.
///
/// Derived, never hand-incremented. `crates/shapes/tests/product_model_census.rs`
/// computes it from the live sources with `syn` and pins the result as
/// `STAGE_ID_GOLDEN`; the bytes here are that digest. Re-derive them from a
/// failing `stage_id_matches_golden` rather than editing either by hand. A
/// hand-maintained version counter is precisely how an authenticated cache serves
/// stale-but-verified wrong answers: the bytes verify, the counter matches, and
/// the meaning moved underneath both. Here the digest IS the meaning, so it
/// cannot.
pub const STAGE_ID: [u8; 32] = [
    0x5b, 0x7a, 0x53, 0xe6, 0x12, 0xe1, 0xa0, 0xe6, 0x63, 0x6f, 0xb0, 0xeb, 0xe8, 0x38, 0x68, 0xd7,
    0x9d, 0x25, 0x0a, 0xa7, 0x21, 0x99, 0x4b, 0x0e, 0xd3, 0x75, 0x86, 0x99, 0x0c, 0x9c, 0xc2, 0x7c,
];

/// The canonical empty SPARQL function registry a [`HostBindings::empty`] borrows.
///
/// A `static` rather than `&UserFunctionRegistry::EMPTY`: the registry owns hash
/// maps, so the constant is not const-promotable to a `'static` reference.
static EMPTY_FUNCTIONS: UserFunctionRegistry = UserFunctionRegistry::EMPTY;

/// The canonical empty custom-aggregate registry. See [`EMPTY_FUNCTIONS`].
static EMPTY_AGGREGATES: AggregateRegistry = AggregateRegistry::EMPTY;

/// The canonical empty property-function registry. See [`EMPTY_FUNCTIONS`].
static EMPTY_PROPERTY_FUNCTIONS: PropertyFunctionRegistry = PropertyFunctionRegistry::EMPTY;

// ---------------------------------------------------------------------------
// Profile
// ---------------------------------------------------------------------------

/// The preparation PROFILE: the switchable behaviours compiled into a product.
///
/// A coarse "which product shape is this" discriminant that sits above the
/// content-derived [`stage_id`](ShapesProductView::stage_id). The stage id moves
/// whenever the model's content moves; the profile moves only when the profile's
/// *meaning* is redefined, at which point every product written under the old
/// meaning must stop opening — which is exactly what changing it does.
///
/// Opaque on purpose. A caller names [`ShapesProfile::CORE`]; it cannot mint a
/// profile of its own, because a profile a caller can spell is a claim about a
/// product rather than a fact about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShapesProfile {
    /// The stable profile identifier this value names.
    id: &'static str,
}

impl ShapesProfile {
    /// SHACL Core + SHACL-SPARQL + SHACL-AF with **zero host-injected
    /// dependencies**: every capability a product of this profile can exercise is
    /// declared by the shapes graph itself.
    ///
    /// That is a statement about what a product NEEDS, not a ban on host bindings.
    /// [`HostBindings::empty`] always suffices to restore a product of this
    /// profile's own capabilities; a host that additionally wires custom
    /// aggregates or native functions may still do so, and the product's
    /// [`Identity`] binds whichever it was prepared against, so a restore under
    /// different ones is refused rather than silently executed.
    ///
    /// What the profile genuinely excludes is the one binding no shapes graph can
    /// describe: a product of this profile is prepared against the EMPTY
    /// property-function registry, because `sh:sparql` bodies that call host
    /// relations depend on wiring the shapes graph cannot state and a product
    /// therefore cannot carry.
    pub const CORE: Self = Self {
        id: identity::PROFILE_ID,
    };

    /// This profile's stable identifier.
    #[must_use]
    pub const fn id(&self) -> &'static str {
        self.id
    }
}

// ---------------------------------------------------------------------------
// Host bindings
// ---------------------------------------------------------------------------

/// The registries the executing HOST supplies to a restore: the native SPARQL
/// functions it injects, the custom aggregates its `AGG(<iri>, …)` calls resolve
/// against, and the property functions its `sh:sparql` bodies may call.
///
/// These are the components of a product's [`Identity`] that come from the process
/// rather than from the product, so they are the ones a caller can get wrong — and
/// the ones a restore must **install**, not merely fingerprint.
/// `Shapes::aggregates` and `Shapes::functions` are public fields every validation
/// entry point reads directly, so a restore that checked them and left the
/// restored value's own empty registries in place would pass every check and then
/// fail at evaluation with "no custom aggregate is registered".
#[derive(Debug, Clone, Copy)]
pub struct HostBindings<'a> {
    /// The host's injected SPARQL function table.
    functions: &'a UserFunctionRegistry,
    /// The host's custom-aggregate table.
    aggregates: &'a AggregateRegistry,
    /// The host's property-function table.
    property_functions: &'a PropertyFunctionRegistry,
}

impl<'a> HostBindings<'a> {
    /// Bind all three registries. Total by construction: there is no "unset"
    /// spelling, because an absent registry and an empty one are the same fact and
    /// a second spelling of it is a second branch nobody exercises.
    #[must_use]
    pub const fn new(
        functions: &'a UserFunctionRegistry,
        aggregates: &'a AggregateRegistry,
        property_functions: &'a PropertyFunctionRegistry,
    ) -> Self {
        Self {
            functions,
            aggregates,
            property_functions,
        }
    }

    /// The host's injected SPARQL function table.
    #[must_use]
    pub const fn functions(&self) -> &'a UserFunctionRegistry {
        self.functions
    }

    /// The host's custom-aggregate table.
    #[must_use]
    pub const fn aggregates(&self) -> &'a AggregateRegistry {
        self.aggregates
    }

    /// The host's property-function table.
    #[must_use]
    pub const fn property_functions(&self) -> &'a PropertyFunctionRegistry {
        self.property_functions
    }
}

impl HostBindings<'static> {
    /// A host that injects nothing — the value that suffices for every product of
    /// [`ShapesProfile::CORE`].
    #[must_use]
    pub const fn empty() -> Self {
        Self::new(
            &EMPTY_FUNCTIONS,
            &EMPTY_AGGREGATES,
            &EMPTY_PROPERTY_FUNCTIONS,
        )
    }
}

// ---------------------------------------------------------------------------
// Refusals
// ---------------------------------------------------------------------------

/// Refuse: the product's bytes are structurally invalid in a way no other
/// dimension names.
fn malformed(message: impl Into<String>) -> ShapesProductError {
    ShapesProductError::new(ProductDimension::Malformed, message)
}

/// Map an [`ArtifactError`] onto the admission [`ProductDimension`] that names it.
///
/// The envelope's error vocabulary was written for the same boundary this module
/// guards, so most arms are a direct correspondence: `BadMagic` is [`Magic`],
/// `UnsupportedVersion` is [`FormatVersion`], `Truncated` is [`Truncated`], a
/// section digest is [`SectionDigest`], the container digest is
/// [`ContainerDigest`], the trailer is [`Trailer`].
///
/// The three DIRECTORY failures — a duplicate kind, a missing kind, a section
/// count that is not the spec's — all land on [`Malformed`]. That is not a
/// shrug: the directory is TOTAL, so a product whose directory disagrees with the
/// spec was not written by a build of this format at all, and [`Malformed`] is the
/// dimension documented as "structurally invalid in a way no other dimension
/// names". `ArtifactError` is `#[non_exhaustive]`, so the wildcard arm is
/// mandatory and lands there too, fail-closed.
///
/// [`Magic`]: ProductDimension::Magic
/// [`FormatVersion`]: ProductDimension::FormatVersion
/// [`Truncated`]: ProductDimension::Truncated
/// [`SectionDigest`]: ProductDimension::SectionDigest
/// [`ContainerDigest`]: ProductDimension::ContainerDigest
/// [`Trailer`]: ProductDimension::Trailer
/// [`Malformed`]: ProductDimension::Malformed
fn refuse_artifact(error: &ArtifactError) -> ShapesProductError {
    let (dimension, fix) = match error {
        ArtifactError::BadMagic => (
            ProductDimension::Magic,
            "these bytes do not open with the prepared-shapes-product magic, so they were never \
             one; hand this loader a product written by `PreparedShapes::to_product`",
        ),
        ArtifactError::UnsupportedVersion(_) => (
            ProductDimension::FormatVersion,
            "this product's container format is not the one this build decodes; re-prepare the \
             product with the PurRDF build that will execute it",
        ),
        ArtifactError::Truncated => (
            ProductDimension::Truncated,
            "this product ends before a structure it declared is complete; re-prepare it, because \
             a truncated product is an interrupted write rather than corruption in place",
        ),
        ArtifactError::SectionDigestMismatch { .. } => (
            ProductDimension::SectionDigest,
            "a section of this product no longer matches the digest recorded for it, so the \
             product is corrupt in place; discard it and re-prepare it from the shapes graph",
        ),
        ArtifactError::ContainerDigestMismatch { .. } => (
            ProductDimension::ContainerDigest,
            "this product's whole-container digest disagrees with its own bytes, so something \
             outside the section bodies moved; discard it and re-prepare it from the shapes graph",
        ),
        ArtifactError::TrailerMismatch => (
            ProductDimension::Trailer,
            "this product's trailer is absent or inconsistent with the bytes ahead of it, so its \
             own directory cannot be trusted; discard it and re-prepare it from the shapes graph",
        ),
        _ => (
            ProductDimension::Malformed,
            "this product's container framing is not one this build wrote; discard it and \
             re-prepare it from the shapes graph, because a directory that disagrees with the \
             format's own section table describes a different format",
        ),
    };
    ShapesProductError::new(dimension, format!("{fix} (envelope reported: {error})"))
}

// ---------------------------------------------------------------------------
// The identity section
// ---------------------------------------------------------------------------

/// Write the product's self-description: the stage id, the profile id and the
/// parse provenance.
///
/// The provenance travels here rather than in the AST because [`rebuild`] must
/// re-derive the shapes graph WITHOUT decoding the AST, and the base and the
/// document prefix map are parse inputs the declarative model does not carry —
/// `from_dataset_with_prefixes` bakes the prefix map into every SHACL-AF query
/// body, so a rebuild that guessed at it would produce different query text.
///
/// The box-role vocabulary and the shapes-graph IRI appear here AND in the AST
/// section, because both carriers need them independently; [`decode_ast`] and this
/// section are cross-checked on the admit path, so a product whose two carriers
/// disagree is refused rather than restored inconsistently.
///
/// Byte-deterministic: the prefix pairs go out in the provenance's own order,
/// which is the order the parser consumed them.
///
/// [`rebuild`]: ShapesProductView::rebuild
/// [`decode_ast`]: ast::decode_ast
fn encode_preamble(shapes: &Shapes) -> Vec<u8> {
    let provenance = shapes.provenance();
    let mut out = Vec::new();
    out.extend_from_slice(&STAGE_ID);
    write_text(&mut out, identity::PROFILE_ID);
    write_opt_text(&mut out, provenance.base());
    write_opt_text(&mut out, provenance.shapes_graph());
    match provenance.box_role_vocab() {
        None => out.push(0),
        Some(vocab) => {
            out.push(1);
            for iri in [
                &vocab.graph_box_role,
                &vocab.box_abox,
                &vocab.box_tbox,
                &vocab.box_rbox,
                &vocab.box_cbox,
                &vocab.box_config_box,
            ] {
                write_text(&mut out, iri);
            }
        }
    }
    write_varint(&mut out, provenance.doc_prefixes().len() as u64);
    for (prefix, namespace) in provenance.doc_prefixes() {
        write_text(&mut out, prefix);
        write_text(&mut out, namespace);
    }
    out
}

/// Append a length-prefixed UTF-8 string.
fn write_text(out: &mut Vec<u8>, text: &str) {
    write_varint(out, text.len() as u64);
    out.extend_from_slice(text.as_bytes());
}

/// Append an optional string: one tag byte, then the string when present.
fn write_opt_text(out: &mut Vec<u8>, text: Option<&str>) {
    match text {
        None => out.push(0),
        Some(text) => {
            out.push(1);
            write_text(out, text);
        }
    }
}

/// Read the product's self-description back.
///
/// # Errors
///
/// [`ProductDimension::Truncated`] when the section ends inside a declared
/// structure, [`ProductDimension::Malformed`] for any other structural
/// inconsistency, trailing bytes included — a writer that produced this section
/// produced exactly these bytes, and ignoring a remainder is how a partial write
/// passes for a whole one.
fn decode_preamble(
    bytes: &[u8],
) -> Result<([u8; 32], String, ParseProvenance), ShapesProductError> {
    let mut pos = 0usize;
    let stage_id: [u8; 32] = bytes
        .get(..32)
        .ok_or_else(|| {
            ShapesProductError::new(
                ProductDimension::Truncated,
                "this product's identity section ends before its 32-byte stage id; re-prepare the \
                 product, because a stage id is the first thing a restore has to read",
            )
        })?
        .try_into()
        .expect("a 32-byte slice converts to a 32-byte array");
    pos += 32;

    let profile = read_text(bytes, &mut pos)?;
    let base = read_opt_text(bytes, &mut pos)?;
    let shapes_graph = read_opt_text(bytes, &mut pos)?;
    let box_role_vocab = match read_byte(bytes, &mut pos)? {
        0 => None,
        1 => Some(BoxRoleVocab {
            graph_box_role: read_text(bytes, &mut pos)?,
            box_abox: read_text(bytes, &mut pos)?,
            box_tbox: read_text(bytes, &mut pos)?,
            box_rbox: read_text(bytes, &mut pos)?,
            box_cbox: read_text(bytes, &mut pos)?,
            box_config_box: read_text(bytes, &mut pos)?,
        }),
        other => {
            return Err(malformed(format!(
                "this product spells the presence of its box-role vocabulary as {other}; \
                 re-prepare the product, because only 0 and 1 are presence byte forms and any \
                 other value means the reader is no longer at a field boundary"
            )));
        }
    };

    let declared = read_count(bytes, &mut pos)?;
    let mut doc_prefixes = Vec::with_capacity(declared);
    for _ in 0..declared {
        let prefix = read_text(bytes, &mut pos)?;
        let namespace = read_text(bytes, &mut pos)?;
        doc_prefixes.push((prefix, namespace));
    }

    if pos != bytes.len() {
        return Err(malformed(format!(
            "this product carries {} bytes after the end of its identity section; re-prepare the \
             product, because the writer that produced this section produced exactly {pos} bytes",
            bytes.len() - pos
        )));
    }

    Ok((
        stage_id,
        profile,
        ParseProvenance::new(base, doc_prefixes, box_role_vocab, shapes_graph),
    ))
}

/// Read one raw byte.
fn read_byte(bytes: &[u8], pos: &mut usize) -> Result<u8, ShapesProductError> {
    let value = *bytes.get(*pos).ok_or_else(|| {
        ShapesProductError::new(
            ProductDimension::Truncated,
            "this product's identity section ends inside a structure it declared; re-prepare the \
             product from its shapes graph",
        )
    })?;
    *pos += 1;
    Ok(value)
}

/// Read a length or count, bounded by the bytes that could possibly hold it.
///
/// Every element of every sequence here occupies at least one byte, so a declared
/// count larger than the remaining input cannot be honest — and refusing it here is
/// what stops a hostile length from driving a multi-gigabyte allocation before the
/// truncation is discovered.
fn read_count(bytes: &[u8], pos: &mut usize) -> Result<usize, ShapesProductError> {
    let declared = read_varint(bytes, pos).map_err(ast::from_pack)?;
    let count = usize::try_from(declared).map_err(|_| {
        malformed(format!(
            "this product declares a sequence of {declared} elements, more than this platform can \
             address; re-prepare the product from its shapes graph"
        ))
    })?;
    let remaining = bytes.len().saturating_sub(*pos);
    if count > remaining {
        return Err(malformed(format!(
            "this product declares a sequence of {count} elements but only {remaining} bytes \
             follow it; re-prepare the product, because every element occupies at least one byte"
        )));
    }
    Ok(count)
}

/// Read a length-prefixed UTF-8 string.
fn read_text(bytes: &[u8], pos: &mut usize) -> Result<String, ShapesProductError> {
    let len = read_count(bytes, pos)?;
    let end = *pos + len;
    let slice = bytes.get(*pos..end).ok_or_else(|| {
        ShapesProductError::new(
            ProductDimension::Truncated,
            "this product's identity section ends inside a string it declared; re-prepare the \
             product from its shapes graph",
        )
    })?;
    let text = std::str::from_utf8(slice)
        .map_err(|error| {
            malformed(format!(
                "this product carries a string that is not UTF-8 ({error}); re-prepare the \
                 product, because every string a shapes graph parses from is Unicode text"
            ))
        })?
        .to_owned();
    *pos = end;
    Ok(text)
}

/// Read an optional string.
fn read_opt_text(bytes: &[u8], pos: &mut usize) -> Result<Option<String>, ShapesProductError> {
    match read_byte(bytes, pos)? {
        0 => Ok(None),
        1 => Ok(Some(read_text(bytes, pos)?)),
        other => Err(malformed(format!(
            "this product spells an optional string's presence as {other}; re-prepare the \
             product, because only 0 and 1 are presence byte forms and any other value means the \
             reader is no longer at a field boundary"
        ))),
    }
}

// ---------------------------------------------------------------------------
// The writer
// ---------------------------------------------------------------------------

impl PreparedShapes {
    /// Write this preparation out as a prepared product.
    ///
    /// One path, always: every section is written on every build, so there is no
    /// writer-side option a caller can set wrong and no product shape a reader has
    /// to branch on. The output is a pure function of the preparation and the
    /// profile — no hash-iteration order, no wall clock and no randomness reach it,
    /// so two calls over equal inputs produce byte-identical buffers.
    ///
    /// # Refusal comes before the bytes
    ///
    /// Every capability check runs before the container is assembled. A shapes
    /// graph whose declared functions a product cannot carry, a node expression
    /// this build cannot spell, a shapes dataset whose canonical identity cannot be
    /// established — each refuses here, with nothing emitted. The alternative,
    /// writing a product that no `admit` could ever accept, moves a writer-side
    /// defect into a reader-side mystery at some later date on some other machine.
    ///
    /// # Errors
    ///
    /// [`ProductDimension::Profile`] when `profile` is not the profile this build
    /// prepares; [`ProductDimension::UnsupportedCapability`] when the shapes graph
    /// declares something the product format cannot represent;
    /// [`ProductDimension::DatasetIdentity`] when the shapes dataset's canonical
    /// identity cannot be established; [`ProductDimension::DepthLimit`] when the
    /// model nests past the codec's ceiling; [`ProductDimension::Malformed`] when
    /// the model is internally inconsistent.
    pub fn to_product(&self, profile: &ShapesProfile) -> Result<Vec<u8>, ShapesProductError> {
        if profile.id() != identity::PROFILE_ID {
            return Err(ShapesProductError::new(
                ProductDimension::Profile,
                format!(
                    "this build prepares products of the profile `{}`, not `{}`; prepare the \
                     product under the profile the executing build implements",
                    identity::PROFILE_ID,
                    profile.id()
                ),
            ));
        }

        let shapes = self.shapes();

        // Prove the product about to be written can be restored with every
        // declared capability intact, BEFORE a single byte exists. The empty host
        // is the right comparison basis — the declared population is exactly the
        // part a restore reinstates from the product's own content.
        let restorable = certified::assemble_functions(shapes, &HostBindings::empty())?;
        certified::verify_declared_functions(&shapes.functions, &restorable)?;

        let ast_bytes = ast::encode_ast(shapes)?;
        let dataset_bytes = dataset::encode_dataset(shapes.dataset())?;
        // The PRODUCER certifies; the consumer trusts the digest chain. This is the
        // one canonicalization in the whole codec that is not on a cold path, and it
        // is here because a product cannot state an identity it has not established.
        let digest = dataset::certify_dataset(&dataset_bytes)?;
        let identity = identity::build_identity(
            &digest,
            shapes,
            // The CORE profile binds the empty property-function registry: host
            // relations are wiring no shapes graph can describe. See
            // `ShapesProfile::CORE`.
            &EMPTY_PROPERTY_FUNCTIONS,
            &self.class_catalog(),
        )?;

        let mut builder = ArtifactBuilder::new(SPEC);
        builder
            .identity(identity)
            .section(SECTION_IDENTITY, &encode_preamble(shapes))
            .section(SECTION_DATASET, &dataset_bytes)
            .section(SECTION_AST, &ast_bytes);
        builder.build_bytes().map_err(|error| {
            malformed(format!(
                "this preparation could not be framed as a product (envelope reported: {error})"
            ))
        })
    }
}

// ---------------------------------------------------------------------------
// The reader
// ---------------------------------------------------------------------------

/// The entry point for reading prepared products.
#[derive(Debug, Clone, Copy)]
pub struct ShapesProduct;

impl ShapesProduct {
    /// Open `bytes` as a prepared product: verify the envelope and decode the
    /// product's self-description, without admitting anything.
    ///
    /// This runs the container's cheap integrity tier only — magic, format version,
    /// section directory, canonical offsets, zero padding, every section's SHA-256,
    /// the identity region's digest and framing, the trailer, and the whole-container
    /// digest. It does NOT restore the dataset, decode the model, or check the
    /// identity against anything; those are [`ShapesProductView::admit`]'s work.
    ///
    /// # Errors
    ///
    /// Any structural [`ProductDimension`]: the envelope's own refusals, and the
    /// identity section's framing refusals.
    pub fn open(bytes: &[u8]) -> Result<ShapesProductView<'_>, ShapesProductError> {
        let view =
            ArtifactView::from_bytes(SPEC, bytes).map_err(|error| refuse_artifact(&error))?;
        let preamble = view
            .section(SECTION_IDENTITY)
            .map_err(|error| refuse_artifact(&error))?;
        let (stage_id, profile, provenance) = decode_preamble(preamble)?;
        Ok(ShapesProductView {
            view,
            stage_id,
            profile,
            provenance,
        })
    }
}

/// A prepared product whose envelope has been verified and whose self-description
/// has been decoded — but which has NOT been admitted.
///
/// Holding one is a statement about framing and integrity, never about fitness:
/// the bytes are a well-formed product of this format, and nothing has yet claimed
/// they are a product this process may execute. That separation is what makes
/// [`declared_identity`](Self::declared_identity) useful — a caller can read what a
/// product says it was prepared from in order to decide what to do about it,
/// without any of it reaching a validator.
#[derive(Debug)]
pub struct ShapesProductView<'a> {
    /// The verified envelope. Section bodies alias the caller's buffer.
    view: ArtifactView<'a>,
    /// The preparation stage id the product declares.
    stage_id: [u8; 32],
    /// The profile id the product declares.
    profile: String,
    /// The parse inputs the product records.
    provenance: ParseProvenance,
}

impl<'a> ShapesProductView<'a> {
    /// The container format version these bytes were written under.
    ///
    /// A constant rather than a field read, and not a shortcut: the envelope
    /// refuses every other version before this view exists, so a product in hand
    /// has exactly one possible answer and reading a stored field would only
    /// create a second place for it to disagree.
    #[must_use]
    pub const fn format_version(&self) -> u32 {
        FORMAT_VERSION
    }

    /// The preparation stage id the product declares.
    ///
    /// Compare it against a product you wrote to decide whether
    /// [`admit`](Self::admit) can succeed at all — a stage id this build does not
    /// know means the memo describes a model this build no longer has, and
    /// [`rebuild`](Self::rebuild) is the path for it.
    #[must_use]
    pub const fn stage_id(&self) -> &[u8; 32] {
        &self.stage_id
    }

    /// The inputs the product says it was compiled from — readable WITHOUT
    /// admitting it.
    ///
    /// The binding is decodable rather than a bare digest precisely so this is
    /// possible: a caller holding a product whose restore was refused can read
    /// which base, which prefix map and which registries it was prepared against,
    /// and fix its own configuration instead of guessing.
    #[must_use]
    pub fn declared_identity(&self) -> &Identity {
        self.view.identity()
    }

    /// The parse inputs recorded when this product's shapes graph was parsed.
    #[must_use]
    pub const fn declared_provenance(&self) -> &ParseProvenance {
        &self.provenance
    }

    /// Every section kind this product carries, in ascending order.
    ///
    /// A closed, total set: exactly `SECTION_IDENTITY`, `SECTION_DATASET` and
    /// `SECTION_AST`, always all three, any of them possibly zero-length. There is
    /// no target-resolution section and there never will be — target resolution is a
    /// property of a data graph, and a product that cached it would validate a new
    /// snapshot against the focus nodes of an old one.
    ///
    /// The kinds are, in order: the identity section, the dataset section and the
    /// AST section.
    #[must_use]
    pub fn section_kinds(&self) -> Vec<u32> {
        self.view.section_kinds()
    }

    /// **The common path.** Restore the preparation from the product's memo,
    /// re-deriving nothing expensive.
    ///
    /// In order, and nothing partial escapes: verify the profile, verify the stage
    /// id, check the host-supplied half of the identity, refuse an oversized decode
    /// against the governor in force, restore the dataset (cheap tier only), decode
    /// the model, re-derive the shapes graph's own SPARQL function declarations from
    /// that dataset, link it, then hand the assembled shapes graph to the type gate
    /// that installs the host's bindings, re-derives the class catalog, and checks
    /// the whole identity before any [`PreparedShapes`] exists (`certified`).
    ///
    /// The canonicalization [`certify`](Self::certify) performs is NOT reachable
    /// from here, by construction: the identity's dataset component is taken from
    /// the product's own binding rather than recomputed. The envelope's per-section
    /// SHA-256 and whole-container digest still cover the dataset's bytes on this
    /// path, so a swapped section is refused; what is not established is that those
    /// bytes canonicalize to the digest the identity claims.
    ///
    /// # Errors
    ///
    /// [`ProductDimension::Profile`] or [`ProductDimension::StageId`] when the
    /// product was not prepared for this build; any identity dimension when the
    /// executing environment is not the one it was prepared against; any structural
    /// dimension when a section refuses.
    pub fn admit(
        self,
        profile: &ShapesProfile,
        host: &HostBindings<'_>,
    ) -> Result<PreparedShapes, ShapesProductError> {
        self.admit_bound(profile, host, None)
    }

    /// **The common path, bound to the product the caller MEANT.** Restore the
    /// preparation exactly as [`admit`](Self::admit) does, but only after the
    /// product's own input binding is confirmed to be `expected_identity` — the
    /// 32-byte digest of the [`Identity`] the caller requires.
    ///
    /// # Why an expectation is a separate entry point rather than a flag
    ///
    /// Everything [`admit`](Self::admit) checks is a question about the EXECUTING
    /// ENVIRONMENT: is this build the one that wrote the memo, are these the
    /// registries the product was prepared against, does this build's class walk
    /// re-derive the analysis the product pinned. Not one of them asks the
    /// question a consumer holding a product actually has, which is *is this the
    /// product I asked for?* A caller that hands the loader the wrong file gets a
    /// perfectly successful restore and a well-formed report about a shapes graph
    /// nobody asked about — the same silent-wrong-answer failure this whole codec
    /// exists to rule out, arriving through the one door the codec did not guard.
    ///
    /// The expectation is checked FIRST, ahead of the profile and the stage id,
    /// because it is the outermost precondition of the three: a caller who named
    /// the wrong artifact is told THAT, rather than being sent to re-pack a
    /// product that was never the one they wanted. It is also free — the identity
    /// digest is decoded by [`ShapesProduct::open`], so the comparison is 32 bytes
    /// against 32 bytes with nothing restored and nothing decoded.
    ///
    /// `expected_identity` is exactly the digest that
    /// [`declared_identity`](Self::declared_identity) reports for the
    /// product a caller intends, and the same value `purrdf shacl explain` renders
    /// on its `identity-digest` line. There is no second digest and no second
    /// spelling: a selector a consumer cannot read off the artifact it names is a
    /// mechanism nobody can use.
    ///
    /// # Errors
    ///
    /// [`ProductDimension::ShapesGraph`] when the product's binding is not the one
    /// required — the dimension documented as "the shapes-graph identity the
    /// product was prepared against is not the identity supplied for execution",
    /// which is precisely the disagreement an expectation miss reports. Otherwise
    /// every dimension [`admit`](Self::admit) refuses on.
    pub fn admit_expecting(
        self,
        profile: &ShapesProfile,
        host: &HostBindings<'_>,
        expected_identity: &[u8; 32],
    ) -> Result<PreparedShapes, ShapesProductError> {
        self.admit_bound(profile, host, Some(expected_identity))
    }

    /// The ONE admission path, with the caller's expectation as its only variable.
    ///
    /// [`admit`](Self::admit) and [`admit_expecting`](Self::admit_expecting) are
    /// two entry points over this body rather than two bodies, for the reason the
    /// link pass and the shared pack seam are each one function called from two
    /// places: a second transcription of an ordered sequence of checks is how the
    /// bound and unbound restores come to disagree about what one of them means.
    fn admit_bound(
        self,
        profile: &ShapesProfile,
        host: &HostBindings<'_>,
        expected_identity: Option<&[u8; 32]>,
    ) -> Result<PreparedShapes, ShapesProductError> {
        if let Some(expected) = expected_identity {
            self.verify_expected_identity(expected)?;
        }
        self.verify_profile(profile)?;
        self.verify_stage_id()?;
        identity::check_host_bindings(
            self.declared_identity(),
            host.functions(),
            host.aggregates(),
            host.property_functions(),
        )?;
        self.refuse_ungovernable_decode()?;

        let dataset = dataset::open_dataset(self.section(SECTION_DATASET)?)?;
        let parts = ast::decode_ast(self.section(SECTION_AST)?)?;

        // The shapes graph's own SPARQL-bodied function declarations, re-derived
        // from the dataset the product carries — the same move the class catalog
        // makes, and for the same reason: the input is already in hand, so a second
        // transcription of it would only be something to drift.
        //
        // This happens BEFORE linking because that is the order a parse performs it
        // in, and the order decides nothing else: `parse_sparql_functions` skips an
        // IRI the graph also declares as a custom node-expression function, so the
        // two populations are disjoint by the time either is registered.
        //
        // `certified::install` derives the same declarations again, from the same
        // dataset, and on this path the two derivations cannot disagree. That
        // repetition is deliberate and it is the cheaper of the two options: the
        // alternative is to hand `install` a registry this function already built,
        // which would make the one site that decides what a restore installs depend
        // on a caller having remembered to build it — exactly the "a later path
        // returns early with a value that looks finished" defect the type gate
        // exists to rule out. The value built here is not redundant either way: it
        // is what the restored `Shapes` must carry for identity row 6 to be the row
        // a parse of this graph would have produced.
        let mut functions = UserFunctionRegistry::new();
        crate::shapes::register_declared_sparql_functions(
            &dataset,
            &self.provenance,
            &mut functions,
        )
        .map_err(|error| {
            malformed(format!(
                "this product's carried shapes dataset does not re-derive its own SPARQL function \
                 declarations under the parse inputs the product records ({error}); re-prepare \
                 the product from a shapes graph this build parses"
            ))
        })?;
        link::link_shapes(
            &parts.node_shapes,
            &parts.shape_index,
            &parts.custom_functions,
            BTreeMap::new(),
            &mut functions,
        )?;

        // The two carriers of the parse configuration must agree. Both are
        // authenticated, so a disagreement is not tampering — it is a product whose
        // two halves were written from different shapes graphs, and restoring it
        // would produce a `Shapes` whose provenance describes neither.
        if parts.box_role_vocab.as_ref() != self.provenance.box_role_vocab() {
            return Err(malformed(
                "this product's model and its recorded parse inputs name different box-role \
                 vocabularies; re-prepare the product from its shapes graph, because PurRDF mints \
                 no vocabulary IRIs and the two halves must describe one parse",
            ));
        }
        if parts.shapes_graph.as_deref() != self.provenance.shapes_graph() {
            return Err(malformed(
                "this product's model and its recorded parse inputs name different shapes-graph \
                 IRIs; re-prepare the product from its shapes graph, because that IRI is what \
                 `$shapesGraph` binds in every SHACL-SPARQL body",
            ));
        }

        let shapes = Shapes {
            node_shapes: parts.node_shapes,
            box_role_vocab: parts.box_role_vocab,
            functions: Arc::new(functions),
            aggregates: Arc::new(AggregateRegistry::new()),
            target_types: parts.target_types,
            shapes_graph: parts.shapes_graph,
            shapes_dataset: dataset,
            parse_provenance: self.provenance.clone(),
        };

        CertifiedParts::from_admitted(self.declared_identity(), shapes, host)
            .map(CertifiedParts::into_prepared)
    }

    /// **The forward-compatibility path.** Ignore the memo and re-derive the
    /// preparation from the authenticated primitive the product carries.
    ///
    /// The shapes dataset travels inside the product, under the envelope's
    /// per-section SHA-256 and whole-container digest, so this parses no RDF text
    /// and reads no file: it restores that dataset and re-derives the shapes from
    /// it under the product's own recorded parse inputs. A reader meeting a stage id
    /// it does not know — a product written by another build of this format — takes
    /// this path.
    ///
    /// The stage id and the identity are deliberately not checked; see
    /// `certified::CertifiedParts::from_rebuilt` for why checking them would refuse
    /// exactly the products this path exists to rescue. The profile IS checked,
    /// because the profile moves only when the format's meaning is redefined, at
    /// which point the carried dataset no longer means what this build would make
    /// of it.
    ///
    /// # Errors
    ///
    /// [`ProductDimension::Profile`] when the product was prepared under another
    /// profile; [`ProductDimension::Malformed`] when the carried dataset does not
    /// parse as a shapes graph under the recorded inputs; any structural dimension
    /// when a section refuses.
    pub fn rebuild(
        self,
        profile: &ShapesProfile,
        host: &HostBindings<'_>,
    ) -> Result<PreparedShapes, ShapesProductError> {
        self.verify_profile(profile)?;
        self.refuse_ungovernable_decode()?;

        let dataset = dataset::open_dataset(self.section(SECTION_DATASET)?)?;
        let shapes = crate::shapes::from_dataset_with_base(
            &dataset,
            self.provenance.base(),
            self.provenance.doc_prefixes(),
            self.provenance.box_role_vocab().cloned(),
            self.provenance.shapes_graph().map(ToOwned::to_owned),
        )
        .map_err(|error| {
            malformed(format!(
                "this product's carried shapes dataset does not re-derive as a shapes graph under \
                 the parse inputs the product records ({error}); re-prepare the product from a \
                 shapes graph this build parses"
            ))
        })?;

        CertifiedParts::from_rebuilt(shapes, host).map(CertifiedParts::into_prepared)
    }

    /// **The cold path.** Independently corroborate the shapes dataset's canonical
    /// identity against the one this product's binding claims.
    ///
    /// Never reachable from [`admit`](Self::admit) or [`rebuild`](Self::rebuild),
    /// and that is load-bearing rather than an optimization. Canonicalization is a
    /// graph-isomorphism computation over the shapes graph's blank nodes; putting it
    /// on the restore path would very plausibly cost more than the shapes parse this
    /// whole feature exists to eliminate, and the product would be slower than the
    /// thing it replaces while every test still passed.
    ///
    /// Call it from a verify subcommand, a conformance harness, or a test — not from
    /// a restore.
    ///
    /// # Errors
    ///
    /// [`ProductDimension::DatasetIdentity`] when the dataset does not canonicalize
    /// to the digest the identity records, or when canonicalization refuses;
    /// otherwise the structural dimensions the dataset section reports.
    pub fn certify(&self) -> Result<(), ShapesProductError> {
        let certified = dataset::certify_dataset(self.section(SECTION_DATASET)?)?;
        identity::certify_dataset_component(self.declared_identity(), &certified)
    }

    /// One section's raw bytes.
    ///
    /// The directory is total, so a missing kind means these bytes are not a
    /// product of this format — never that a section was "left out".
    fn section(&self, kind: u32) -> Result<&'a [u8], ShapesProductError> {
        self.view
            .section(kind)
            .map_err(|error| refuse_artifact(&error))
    }

    /// Refuse a product whose input binding is not the one the caller required.
    ///
    /// The comparison is over the whole [`Identity`] digest, which is what makes it
    /// a SELECTOR rather than a second opinion about one component: it covers the
    /// shapes dataset, the shapes-graph IRI, the prefix map, the base, the profile,
    /// the vocabulary, all three registries and the class catalog at once, and it
    /// is authenticated by the envelope's identity region before this view exists.
    ///
    /// The refusal cannot name WHICH component moved, and it deliberately does not
    /// pretend to: a caller holding an expectation holds a digest, not a decoded
    /// identity, so there is nothing to diff against. What it does carry is both
    /// digests and the command that prints the required one, so the caller can put
    /// the two products side by side — which is the actionable half.
    fn verify_expected_identity(&self, expected: &[u8; 32]) -> Result<(), ShapesProductError> {
        let declared = self.declared_identity().digest();
        if declared == expected {
            return Ok(());
        }
        Err(ShapesProductError::new(
            ProductDimension::ShapesGraph,
            format!(
                "this product was prepared from the shapes graph whose input binding is {}, and \
                 the caller required the product bound to {}; restore the product prepared from \
                 the shapes graph you named, or read the binding of the product you meant off \
                 the artifact itself — `purrdf shacl explain` prints it on its `identity-digest` \
                 line, in exactly this spelling",
                hex(declared),
                hex(expected)
            ),
        ))
    }

    /// Refuse a product prepared under another profile.
    fn verify_profile(&self, profile: &ShapesProfile) -> Result<(), ShapesProductError> {
        if self.profile == profile.id() && profile.id() == identity::PROFILE_ID {
            return Ok(());
        }
        Err(ShapesProductError::new(
            ProductDimension::Profile,
            format!(
                "this product was prepared under the profile `{}`, but `{}` was requested and this \
                 build implements `{}`; restore the product under the profile it was prepared \
                 with, or re-prepare it with the build that will execute it",
                self.profile,
                profile.id(),
                identity::PROFILE_ID
            ),
        ))
    }

    /// Refuse a product whose memo describes a model this build no longer has.
    fn verify_stage_id(&self) -> Result<(), ShapesProductError> {
        if self.stage_id == STAGE_ID {
            return Ok(());
        }
        Err(ShapesProductError::new(
            ProductDimension::StageId,
            format!(
                "this product was prepared at stage {}, and this build is stage {}; re-prepare the \
                 product, or restore it with `rebuild`, which re-derives the shapes graph from the \
                 dataset the product carries instead of trusting a memo written against a model \
                 this build no longer has",
                hex(&self.stage_id),
                hex(&STAGE_ID)
            ),
        ))
    }

    /// Refuse, at ADMISSION, a decode whose declared size already exceeds the
    /// scratch ceiling in force on this thread.
    ///
    /// # Why this is a refusal and not a charge
    ///
    /// Restoring a product materializes the dataset and the model into owned
    /// values, and the sections' declared lengths are a lower bound on that: the
    /// decoded form of a section is never smaller than its bytes. So the ceiling
    /// can be decided BEFORE the work, from the directory alone, and
    /// [`TrippedGovernor::Refused`] is the variant that exists for exactly that —
    /// "refused at admission, before the operation ran", carrying an estimate in a
    /// slot that is documented as not being a measurement. Charging
    /// [`GovernorState::charge`](purrdf_sparql_eval::GovernorState::charge) instead
    /// would report a [`Budget`](TrippedGovernor::Budget) trip, whose `consumed`
    /// field promises a number something actually spent.
    ///
    /// The estimate is deliberately the sum of the declared section lengths and
    /// nothing cleverer. An estimator that guesses high refuses work that would
    /// have fit, which is over-refusal — the mirror of a silent drop, and the
    /// harder of the two to notice. A lower bound can only ever refuse a decode
    /// that genuinely could not have fit.
    ///
    /// The other half of a governed decode — bounded nesting — is the codec's own
    /// `ast::MAX_DEPTH`, refusing under [`ProductDimension::DepthLimit`]. It needs
    /// no governor: stack exhaustion is an abort no caller could handle, so its
    /// ceiling is unconditional rather than budgeted.
    ///
    /// An ungoverned thread, or one whose scratch dimension carries no ceiling,
    /// pays one `Option` check and one array read.
    fn refuse_ungovernable_decode(&self) -> Result<(), ShapesProductError> {
        let Some(state) = crate::sparql::current_governors() else {
            return Ok(());
        };
        if !state.is_engaged_in(ResourceDimension::ScratchBytes) {
            return Ok(());
        }
        let limit = state.limits().get(ResourceDimension::ScratchBytes);
        let estimate = [SECTION_IDENTITY, SECTION_DATASET, SECTION_AST]
            .into_iter()
            .map(|kind| self.section(kind).map_or(0, <[u8]>::len) as u64)
            .fold(0u64, u64::saturating_add);
        if estimate <= limit {
            return Ok(());
        }
        let tripped = state.record_trip(TrippedGovernor::Refused {
            dimension: ResourceDimension::ScratchBytes,
            limit,
            estimate,
        });
        Err(ShapesProductError::new(
            ProductDimension::UnsupportedCapability,
            format!(
                "restoring this product would materialize at least {estimate} bytes, and the \
                 scratch ceiling in force on this thread is {limit}; raise the ceiling for the \
                 restore or prepare a smaller shapes graph, because the bytes are well formed and \
                 it is this execution's budget — not this build — that cannot honour them \
                 (governor reported: {tripped})"
            ),
        ))
    }
}

/// Lowercase-hex a 32-byte digest for a refusal message.
///
/// A local helper rather than a shared utility: this module owes nothing to any
/// layer above it, and a digest renderer is four lines.
fn hex(digest: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(64);
    for byte in digest {
        let _ = write!(out, "{byte:02x}");
    }
    out
}
