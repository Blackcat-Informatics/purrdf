// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The prepared-shapes-product boundary every language binding routes through —
//! the string/bytes-in, bytes-out seam over [`purrdf_shapes::product`].
//!
//! The codec itself lives in `purrdf-shapes` and decides everything that matters:
//! what a product is, what admitting one requires, and which
//! [`ProductDimension`] a refusal names. This module adds nothing to that. It
//! composes the two-or-three-step sequences the CLI, Python, WebAssembly and C-ABI
//! surfaces would otherwise each open-code — parse a Turtle shapes graph, prepare
//! it, write the product; open a product, admit it, validate a data graph with it —
//! so those sequences exist ONCE and the bindings are left with their own
//! platform-specific wrapping.
//!
//! # A refusal keeps its dimension all the way to the host
//!
//! The whole point of [`ShapesProductError`] is that a caller branches on the
//! dimension rather than on the message text. A binding boundary that flattened
//! the refusal to a string would delete exactly that, and every host would be back
//! to substring-matching prose. So the error type here is
//! [`ShapesProductRefusal`], which keeps the dimension where there is one and is
//! honest about the one place there is not: a shapes DOCUMENT that does not parse
//! never reached the admission boundary at all, so no dimension names it, and
//! inventing one (`malformed`, say) would claim a product was inspected when none
//! was ever written.
//!
//! # Portability
//!
//! Pure in-memory work over the wasm-clean SHACL engine and the wasm-clean codec:
//! no filesystem, no clock, no randomness. The one surface that reads files is the
//! CLI, which owns its own `std::fs`.

use std::sync::Arc;

use purrdf_core::RdfDataset;
use purrdf_shapes::engine::{self, PreparedShapes};
use purrdf_shapes::product::{
    HostBindings, ProductDimension, STAGE_ID, ShapesProduct, ShapesProductError, ShapesProfile,
};

use crate::{SarifOptions, report_to_sarif_string};

// ---------------------------------------------------------------------------
// Refusal
// ---------------------------------------------------------------------------

/// Why a prepared-product operation did not produce what was asked of it.
///
/// Two arms, because there are exactly two kinds of failure on these paths and
/// collapsing them would lie about one of them:
///
/// * [`Shapes`](Self::Shapes) — the shapes DOCUMENT did not parse, so no product
///   was ever written or opened. There is no admission dimension because nothing
///   was admitted; [`dimension`](Self::dimension) is `None` and says so.
/// * [`Admission`](Self::Admission) — the codec refused, on a named
///   [`ProductDimension`] the host can branch on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShapesProductRefusal {
    /// The shapes document did not parse into a shapes graph. Carries the SHACL
    /// engine's own diagnostic.
    Shapes(String),
    /// The prepared-product admission boundary refused, on a named dimension.
    Admission(ShapesProductError),
}

impl ShapesProductRefusal {
    /// The admission dimension that refused, or `None` when the failure happened
    /// before any product existed.
    ///
    /// This is the stable, matchable half of the refusal — see the
    /// [module documentation](self).
    #[must_use]
    pub const fn dimension(&self) -> Option<ProductDimension> {
        match self {
            Self::Shapes(_) => None,
            Self::Admission(error) => Some(error.dimension()),
        }
    }

    /// The kebab-case label of [`dimension`](Self::dimension), or `None`.
    ///
    /// The label rather than the variant is what crosses a language boundary: it
    /// is the pinned contract `ProductDimension::label` documents, and it is a
    /// string every one of Python, JavaScript and C can carry unchanged.
    #[must_use]
    pub fn dimension_label(&self) -> Option<&'static str> {
        self.dimension().map(ProductDimension::label)
    }

    /// The explanation, WITHOUT the dimension label prefix.
    ///
    /// A host that carries the label in its own typed slot — a Python exception
    /// attribute, a JS class field, a C accessor — wants the prose alone, so that
    /// the label is not also duplicated into the message it renders beside.
    #[must_use]
    pub fn message(&self) -> &str {
        match self {
            Self::Shapes(message) => message,
            Self::Admission(error) => error.message(),
        }
    }
}

impl From<ShapesProductError> for ShapesProductRefusal {
    fn from(error: ShapesProductError) -> Self {
        Self::Admission(error)
    }
}

impl std::fmt::Display for ShapesProductRefusal {
    /// `<dimension>: <message>` for an admission refusal, and the engine's own
    /// diagnostic verbatim for a parse failure — the same rendering
    /// [`ShapesProductError`] itself uses, so a host with only a message channel
    /// still sees the label.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Shapes(message) => f.write_str(message),
            Self::Admission(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ShapesProductRefusal {}

// ---------------------------------------------------------------------------
// Writing
// ---------------------------------------------------------------------------

/// Parse a Turtle shapes graph and write it out as a prepared product under
/// [`ShapesProfile::CORE`].
///
/// `shapes_base` is the base IRI the shapes document's relative IRI references
/// resolve against; it is recorded in the product's parse provenance, so a restore
/// resolves them identically without the document.
///
/// The parse is the same two steps [`engine::parse_shapes`] performs — native
/// Turtle ingestion into a dataset, plus the `@prefix`/`PREFIX` map recovered from
/// the source text — so a product packed from a document and a validation run
/// directly against that document see the same `Shapes`. They are performed here
/// rather than by calling `parse_shapes` because this entry point additionally has
/// to inspect the resulting DATASET before it becomes `Shapes` — see the next
/// paragraph — and `parse_shapes` does not hand one back.
///
/// # An unresolved `owl:imports` is refused here, not silently dropped
///
/// This is the text-in/bytes-out surface the Python, C-ABI and WebAssembly hosts
/// call, and none of the three carries an import table or a place to print a
/// warning: there is no `--import IRI=FILE` for them to pass and no stderr for
/// them to read. The CLI's `shacl pack` is different — it has both, folds the
/// closure or reports each unresolved IRI on stderr, and calls
/// [`pack_shapes_product_from_dataset`] once that decision is made — but a host
/// with neither capability has exactly one non-silent option when the shapes
/// graph declares an import it has no way to honour: refuse. Packing the root
/// graph alone and calling it done is exactly the silent omission this whole
/// codec exists to rule out; see [`ProductDimension::UnsupportedCapability`],
/// which is also where a declared function nothing in the shapes model reaches
/// refuses.
///
/// # Errors
///
/// [`ShapesProductRefusal::Shapes`] when the document does not parse;
/// [`ShapesProductRefusal::Admission`] on [`ProductDimension::UnsupportedCapability`]
/// when the shapes graph declares an `owl:imports` this entry point has no table to
/// resolve it against, and on any other dimension the shapes graph declares
/// something the product format cannot carry.
pub fn pack_shapes_product(
    shapes_ttl: &str,
    shapes_base: Option<&str>,
) -> Result<Vec<u8>, ShapesProductRefusal> {
    let dataset = purrdf_shapes::text_ingest::parse_turtle_to_dataset(shapes_ttl, shapes_base)
        .map_err(|errors| ShapesProductRefusal::Shapes(errors.join("\n")))?;
    let unresolved = purrdf_entail::entails::imports::imported_iris(&dataset);
    if let Some(iri) = unresolved.first() {
        return Err(ShapesProductRefusal::Admission(ShapesProductError::new(
            ProductDimension::UnsupportedCapability,
            format!(
                "this shapes graph owl:imports <{iri}>, and this entry point has no \
                 `--import IRI=FILE` table and no diagnostic channel to report an unresolved \
                 one on; packing the shapes graph alone would silently validate against a \
                 different, smaller shapes graph than the one named. Fold the closure before \
                 calling this function — `purrdf shacl pack --import <{iri}>=FILE` does so on \
                 the command line, and {caller} does the same over a dataset it already holds",
                caller = "`pack_shapes_product_from_dataset`"
            ),
        )));
    }
    let prefixes = purrdf_shapes::text_ingest::extract_prefixes(shapes_ttl);
    pack_shapes_product_from_dataset(&dataset, &prefixes, shapes_base, None)
}

/// Write an already-READ shapes dataset out as a prepared product under
/// [`ShapesProfile::CORE`].
///
/// This is the dataset-level twin of [`pack_shapes_product`], and the two share
/// one seam by construction: `pack_shapes_product` parses `shapes_ttl` into a
/// dataset and its document prefix map, then calls straight through to this
/// function, so the two can never compile the shapes graph two different ways.
///
/// The intended caller is a host that has ALREADY turned a shapes document (or
/// several, merged) into a dataset before this point — the CLI's `shacl pack`,
/// which reads the shapes document, folds its `owl:imports` closure against an
/// `--import IRI=FILE` table exactly the way `validate --shapes` does, and only
/// then packs the merged graph. Import resolution is that caller's decision, not
/// this function's: by the time a dataset reaches here, whatever it does or does
/// not carry of an `owl:imports` closure is exactly what the caller decided
/// should be packed, and this function has no opinion about it.
///
/// `base` and `shapes_graph` are recorded into the product's parse provenance —
/// see [`purrdf_shapes::shapes::from_dataset_with_base`], the parser this calls —
/// so a restore resolves relative references and exposes `sh:shapesGraph`
/// identically to the document(s) the dataset was read from.
///
/// # Errors
///
/// [`ShapesProductRefusal::Shapes`] when the dataset does not parse as a shapes
/// graph; [`ShapesProductRefusal::Admission`] when it declares something the
/// product format cannot carry.
pub fn pack_shapes_product_from_dataset(
    dataset: &Arc<RdfDataset>,
    doc_prefixes: &[(String, String)],
    base: Option<&str>,
    shapes_graph: Option<String>,
) -> Result<Vec<u8>, ShapesProductRefusal> {
    let shapes = purrdf_shapes::shapes::from_dataset_with_base(
        dataset,
        base,
        doc_prefixes,
        None,
        shapes_graph,
    )
    .map_err(ShapesProductRefusal::Shapes)?;
    prepared_to_product(&PreparedShapes::new(Arc::new(shapes)))
        .map_err(ShapesProductRefusal::Admission)
}

/// Write an already-prepared shapes graph out as a product under
/// [`ShapesProfile::CORE`].
///
/// The profile is not a parameter, here or anywhere else in this module: a caller
/// cannot mint a profile (see [`ShapesProfile`]), `CORE` is the only one this build
/// implements, and a binding that took it as an argument would be offering a choice
/// with one arm.
///
/// # Errors
///
/// Any [`ProductDimension`] the writer refuses on.
pub fn prepared_to_product(prepared: &PreparedShapes) -> Result<Vec<u8>, ShapesProductError> {
    prepared.to_product(&ShapesProfile::CORE)
}

// ---------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------

/// **The common path.** Open `product` and admit it, restoring the preparation
/// from the product's memo.
///
/// The host bindings are [`HostBindings::empty`], which is what
/// [`ShapesProfile::CORE`] is defined to need: every capability a product of that
/// profile can exercise is declared by the shapes graph itself. A host that injects
/// native SPARQL functions has a richer boundary available to it in
/// `purrdf-shapes` directly; this is the seam the four bindings share, and none of
/// them can carry a host closure across its own language boundary.
///
/// # Errors
///
/// Any structural or identity [`ProductDimension`]. A
/// [`StageId`](ProductDimension::StageId) refusal specifically means these bytes
/// were written by another build of this format — [`rebuild_shapes_product`] is the
/// path for it.
pub fn admit_shapes_product(product: &[u8]) -> Result<PreparedShapes, ShapesProductError> {
    ShapesProduct::open(product)?.admit(&ShapesProfile::CORE, &HostBindings::empty())
}

/// **The common path, bound to the product the caller MEANT.** Open `product`,
/// confirm its input binding is `expected_identity`, and only then admit it.
///
/// Everything [`admit_shapes_product`] checks is a question about the executing
/// process — its build, its registries, its class analysis. This adds the one
/// question about the ARTIFACT: *is this the product I asked for?* Without it a
/// consumer that names the wrong file is handed a successful restore and a
/// well-formed report about a shapes graph nobody asked about, which is the silent
/// wrong answer the whole codec exists to rule out.
///
/// `expected_identity` is the 32-byte digest of the product's input binding, the
/// same value [`explain_shapes_product`] renders on its `identity-digest` line.
/// [`parse_identity_digest`] turns that rendering back into one, so every surface
/// that carries the selector as text reads and writes one spelling.
///
/// # Errors
///
/// [`ShapesGraph`](ProductDimension::ShapesGraph) when the product's binding is not
/// the required one, and otherwise every dimension [`admit_shapes_product`] refuses
/// on.
pub fn admit_shapes_product_expecting(
    product: &[u8],
    expected_identity: &[u8; 32],
) -> Result<PreparedShapes, ShapesProductError> {
    ShapesProduct::open(product)?.admit_expecting(
        &ShapesProfile::CORE,
        &HostBindings::empty(),
        expected_identity,
    )
}

/// Read a 32-byte identity selector back out of the text a host carries it as.
///
/// The accepted spelling is 64 hexadecimal digits — exactly what
/// [`explain_shapes_product`]'s `identity-digest` line prints, so the digest a
/// consumer reads off an artifact can be handed straight back without editing. Case
/// is not significant on the way in: the renderer emits lowercase, but a selector
/// that travelled through a shell, a spreadsheet or a CI variable may not have
/// stayed that way, and refusing `9F2C…` for a shape the mechanism does not care
/// about would be refusing input that is actually valid.
///
/// Nothing about a malformed selector is a statement about a product — no product
/// has been opened, and none may be, so this refuses with prose and no
/// [`ProductDimension`]. Each host raises it through its own argument-error channel
/// rather than through the admission one, for the same reason
/// [`ShapesProductRefusal::Shapes`] carries no dimension.
///
/// # Errors
///
/// A prescriptive message naming the fix when `text` is not 64 hexadecimal digits.
pub fn parse_identity_digest(text: &str) -> Result<[u8; 32], String> {
    let trimmed = text.trim();
    if trimmed.len() != 64 || !trimmed.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "an expected product identity is the 64 hexadecimal digits of the product's input \
             binding, and `{trimmed}` is not that; read the value off the product you mean — \
             `purrdf shacl explain` prints it on its `identity-digest` line — and pass it \
             unchanged"
        ));
    }

    let mut digest = [0u8; 32];
    for (slot, pair) in digest.iter_mut().zip(trimmed.as_bytes().as_chunks::<2>().0) {
        let text = std::str::from_utf8(pair)
            .expect("two ASCII hexadecimal digits are valid UTF-8 by the check above");
        *slot = u8::from_str_radix(text, 16)
            .expect("two ASCII hexadecimal digits parse as a byte by the check above");
    }
    Ok(digest)
}

/// **The forward-compatibility path.** Open `product` and re-derive the preparation
/// from the shapes dataset the product carries, ignoring its memo.
///
/// No RDF text is parsed and no file is read: the dataset travels inside the
/// product, under the envelope's own digests.
///
/// # Errors
///
/// Any structural [`ProductDimension`], and
/// [`Malformed`](ProductDimension::Malformed) when the carried dataset does not
/// re-derive as a shapes graph under the product's recorded parse inputs.
pub fn rebuild_shapes_product(product: &[u8]) -> Result<PreparedShapes, ShapesProductError> {
    ShapesProduct::open(product)?.rebuild(&ShapesProfile::CORE, &HostBindings::empty())
}

/// **The forward-compatibility path, bound to the product the caller MEANT.** Open
/// `product`, confirm its input binding is `expected_identity`, and only then
/// re-derive the preparation from its carried dataset.
///
/// A stage id this build does not know is not a reason to stop asking *is this the
/// product I asked for?* — see [`admit_shapes_product_expecting`] for why that
/// question is checked ahead of everything the codec itself decides, and
/// [`purrdf_shapes::product::ShapesProductView::rebuild_expecting`] for why the
/// same 32-byte comparison is exactly as free on this path as it is there: the
/// envelope's identity region is decoded and authenticated by
/// [`ShapesProduct::open`] independently of whether the stage id is one this build
/// recognizes.
///
/// # Errors
///
/// [`ShapesGraph`](ProductDimension::ShapesGraph) when the product's binding is not
/// the required one, and otherwise every dimension [`rebuild_shapes_product`]
/// refuses on.
pub fn rebuild_shapes_product_expecting(
    product: &[u8],
    expected_identity: &[u8; 32],
) -> Result<PreparedShapes, ShapesProductError> {
    ShapesProduct::open(product)?.rebuild_expecting(
        &ShapesProfile::CORE,
        &HostBindings::empty(),
        expected_identity,
    )
}

/// **The cold path.** Open `product` and independently corroborate its shapes
/// dataset's canonical identity against the one its binding claims.
///
/// Never called from a restore — canonicalizing a shapes graph's blank nodes can
/// cost more than the shapes parse a product exists to eliminate. This is the
/// surface a `verify` subcommand or a conformance harness reaches for.
///
/// # Errors
///
/// [`DatasetIdentity`](ProductDimension::DatasetIdentity) when the dataset does not
/// canonicalize to the digest the identity records, plus the structural dimensions
/// opening the product reports.
pub fn certify_shapes_product(product: &[u8]) -> Result<(), ShapesProductError> {
    ShapesProduct::open(product)?.certify()
}

/// Open `product` and render everything it says about itself, WITHOUT admitting it.
///
/// This is what makes a named refusal actionable rather than a log line: a caller
/// whose restore was refused on [`Prefixes`](ProductDimension::Prefixes) can read
/// which prefix map the product was actually compiled under and fix its own
/// configuration, instead of guessing or rebuilding blindly.
///
/// # The rendering
///
/// Deterministic `key value` lines, in a fixed order, terminated by a newline —
/// the same shape [`crate::regime::render_reasoning_report`] uses, and for the same
/// reason: it is the one form every host can carry across its own boundary
/// unchanged, and a consumer can split it on whitespace without a parser.
///
/// ```text
/// format-version 1
/// stage-id <64 lowercase hex>
/// stage-known true|false
/// identity-digest <64 lowercase hex>
/// identity-components <count>
/// identity <label> <value>        (one per component, in the identity's own order)
/// parse-base <iri>|none
/// parse-shapes-graph <iri>|none
/// parse-prefixes <count>
/// parse-prefix <prefix> <namespace>   (one per declaration, in parse order)
/// ```
///
/// `stage-known` is the fact a caller acts on: `false` says this build's
/// [`admit_shapes_product`] will refuse these bytes and
/// [`rebuild_shapes_product`] is the path that still restores them.
///
/// An identity component's value is rendered as `"text"` when it is printable
/// UTF-8 and `0x<hex>` otherwise, which is the envelope's own rule for the same
/// values (`purrdf_core::artifact::IdentityMismatch`'s `Display`) — most of them
/// are digests, and a digest shown as mojibake helps nobody.
///
/// # Errors
///
/// Any structural [`ProductDimension`]: these bytes must be a well-formed product
/// of this format before there is anything to describe.
pub fn explain_shapes_product(product: &[u8]) -> Result<String, ShapesProductError> {
    use std::fmt::Write as _;

    let view = ShapesProduct::open(product)?;
    let identity = view.declared_identity();
    let provenance = view.declared_provenance();

    let mut out = String::new();
    let _ = writeln!(out, "format-version {}", view.format_version());
    let _ = writeln!(out, "stage-id {}", hex(view.stage_id()));
    let _ = writeln!(out, "stage-known {}", view.stage_id() == &STAGE_ID);
    let _ = writeln!(out, "identity-digest {}", hex(identity.digest()));
    let _ = writeln!(out, "identity-components {}", identity.components().len());
    for component in identity.components() {
        let _ = writeln!(
            out,
            "identity {} {}",
            component.label(),
            render_component(component.value())
        );
    }
    let _ = writeln!(out, "parse-base {}", provenance.base().unwrap_or("none"));
    let _ = writeln!(
        out,
        "parse-shapes-graph {}",
        provenance.shapes_graph().unwrap_or("none")
    );
    let _ = writeln!(out, "parse-prefixes {}", provenance.doc_prefixes().len());
    for (prefix, namespace) in provenance.doc_prefixes() {
        let _ = writeln!(out, "parse-prefix {prefix} {namespace}");
    }
    Ok(out)
}

/// Admit `product` and validate `data_nt` (N-Triples) with it, rendering the SHACL
/// report to a SARIF 2.1.0 JSON string.
///
/// This is the capability's POINT on every binding: a product is compiled once and
/// then used, and a surface that could only write and inspect one would ship an
/// artifact with no consumer. The validation is
/// [`engine::validate_dataset_with_shapes_graph`] over the admitted shapes — the
/// same function `validate` and every other host reaches — so restoring a product
/// and parsing its shapes graph reach the identical verdict rather than two
/// independently-derived ones.
///
/// # Errors
///
/// [`ShapesProductRefusal::Admission`] when the product is refused, and
/// [`ShapesProductRefusal::Shapes`] when the DATA graph does not parse or the
/// validation hard-fails — the arm that carries "no product dimension names this",
/// which is exactly true of a malformed data graph.
pub fn validate_with_shapes_product(
    product: &[u8],
    data_nt: &str,
    options: &SarifOptions,
) -> Result<String, ShapesProductRefusal> {
    validate_with_product(product, data_nt, None, options)
}

/// Admit `product` — only if its input binding is `expected_identity` — and validate
/// `data_nt` (N-Triples) with it, rendering the SHACL report to a SARIF 2.1.0 JSON
/// string.
///
/// The bound twin of [`validate_with_shapes_product`], and the shape every non-Rust
/// host reaches for: the hosts validate through one call rather than holding a
/// restored preparation across their own language boundary, so the expectation has
/// to travel with the validation. See [`admit_shapes_product_expecting`] for why an
/// unbound restore is the one door the codec did not guard.
///
/// # Errors
///
/// [`ShapesProductRefusal::Admission`] on
/// [`ShapesGraph`](ProductDimension::ShapesGraph) when the product's binding is not
/// the required one, on any other dimension the product is refused for, and
/// [`ShapesProductRefusal::Shapes`] when the DATA graph does not parse or the
/// validation hard-fails.
pub fn validate_with_shapes_product_expecting(
    product: &[u8],
    data_nt: &str,
    expected_identity: &[u8; 32],
    options: &SarifOptions,
) -> Result<String, ShapesProductRefusal> {
    validate_with_product(product, data_nt, Some(expected_identity), options)
}

/// **The forward-compatibility path.** Rebuild `product` — re-deriving the
/// preparation from its carried dataset rather than admitting its memo — and
/// validate `data_nt` (N-Triples) with it, rendering the SHACL report to a SARIF
/// 2.1.0 JSON string.
///
/// The rebuild twin of [`validate_with_shapes_product`], for the one binding this
/// module serves that has no way to inspect a `stage-known false` line and retry:
/// a host driving this in one call needs the rescue to happen automatically when
/// the memo is one this build cannot read, and [`rebuild_shapes_product`] is that
/// rescue. It is also the answer a CURRENT product gets when a caller reaches for
/// this path anyway — [`rebuild`](purrdf_shapes::product::ShapesProductView::rebuild)
/// re-derives from the SAME carried dataset [`admit_shapes_product`] restores a
/// memo of, so the two reach the identical report.
///
/// # Errors
///
/// [`ShapesProductRefusal::Admission`] when the product is refused — most notably
/// [`Malformed`](ProductDimension::Malformed) when the carried dataset does not
/// re-derive as a shapes graph — and [`ShapesProductRefusal::Shapes`] when the DATA
/// graph does not parse or the validation hard-fails.
pub fn validate_with_rebuilt_shapes_product(
    product: &[u8],
    data_nt: &str,
    options: &SarifOptions,
) -> Result<String, ShapesProductRefusal> {
    validate_with_rebuilt_product(product, data_nt, None, options)
}

/// **The forward-compatibility path, bound to the product the caller MEANT.**
/// Rebuild `product` — only if its input binding is `expected_identity` — and
/// validate `data_nt` (N-Triples) with it, rendering the SHACL report to a SARIF
/// 2.1.0 JSON string.
///
/// The bound twin of [`validate_with_rebuilt_shapes_product`], for the same
/// reason [`validate_with_shapes_product_expecting`] exists beside
/// [`validate_with_shapes_product`]: a host driving this in one call has no
/// separate step to check an identity before it commits to the rescue, so the
/// expectation has to travel with the rebuild. An unknown stage id is not a
/// reason to stop asking *is this the product I asked for?* — see
/// [`rebuild_shapes_product_expecting`] for why the comparison runs first, ahead
/// of the re-derivation, exactly as it does on the admission path.
///
/// # Errors
///
/// [`ShapesProductRefusal::Admission`] on
/// [`ShapesGraph`](ProductDimension::ShapesGraph) when the product's binding is
/// not the required one, on any other dimension [`rebuild_shapes_product`]
/// refuses on, and [`ShapesProductRefusal::Shapes`] when the DATA graph does not
/// parse or the validation hard-fails.
pub fn validate_with_rebuilt_shapes_product_expecting(
    product: &[u8],
    data_nt: &str,
    expected_identity: &[u8; 32],
    options: &SarifOptions,
) -> Result<String, ShapesProductRefusal> {
    validate_with_rebuilt_product(product, data_nt, Some(expected_identity), options)
}

/// The ONE rebuild-validation body, with the caller's expectation as its only
/// variable — the same arrangement [`validate_with_product`] makes for the
/// admission path.
fn validate_with_rebuilt_product(
    product: &[u8],
    data_nt: &str,
    expected_identity: Option<&[u8; 32]>,
    options: &SarifOptions,
) -> Result<String, ShapesProductRefusal> {
    let prepared = match expected_identity {
        None => rebuild_shapes_product(product)?,
        Some(expected) => rebuild_shapes_product_expecting(product, expected)?,
    };
    validate_prepared(&prepared, data_nt, options)
}

/// The ONE product-validation body, with the caller's expectation as its only
/// variable — the same arrangement `purrdf-shapes` makes one layer down, where
/// `admit` and `admit_expecting` are two entry points over one admission sequence.
///
/// Two entry points rather than two bodies: a bound validation that restored the
/// product through a second sequence of steps would be a second answer about one
/// artifact, and the expectation exists precisely to stop a second answer.
fn validate_with_product(
    product: &[u8],
    data_nt: &str,
    expected_identity: Option<&[u8; 32]>,
    options: &SarifOptions,
) -> Result<String, ShapesProductRefusal> {
    let prepared = match expected_identity {
        None => admit_shapes_product(product)?,
        Some(expected) => admit_shapes_product_expecting(product, expected)?,
    };
    validate_prepared(&prepared, data_nt, options)
}

/// Validate `data_nt` (N-Triples) against an already-restored preparation,
/// rendering the SHACL report to a SARIF 2.1.0 JSON string.
///
/// The one tail every restore route shares — admitted, bound-admitted, or
/// rebuilt — so a data-graph parse failure or a validation hard-fail is reported
/// identically regardless of which door the preparation came through.
fn validate_prepared(
    prepared: &PreparedShapes,
    data_nt: &str,
    options: &SarifOptions,
) -> Result<String, ShapesProductRefusal> {
    let data = purrdf_shapes::text_ingest::parse_ntriples_to_dataset(data_nt)
        .map_err(|errors| ShapesProductRefusal::Shapes(errors.join("\n")))?;
    let report = engine::validate_dataset_with_shapes_graph(data.as_ref(), prepared.shapes(), None)
        .map_err(ShapesProductRefusal::Shapes)?;
    Ok(report_to_sarif_string(&report, options))
}

// ---------------------------------------------------------------------------
// Rendering helpers
// ---------------------------------------------------------------------------

/// Lowercase-hex a 32-byte digest.
fn hex(digest: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(64);
    for byte in digest {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// Render an identity component's value: quoted when it is printable UTF-8,
/// `0x`-prefixed lowercase hex otherwise. See [`explain_shapes_product`].
fn render_component(value: &[u8]) -> String {
    match std::str::from_utf8(value) {
        Ok(text) if !text.is_empty() && !text.chars().any(char::is_control) => {
            format!("\"{text}\"")
        }
        _ => {
            use std::fmt::Write as _;
            let mut out = String::with_capacity(value.len() * 2 + 2);
            out.push_str("0x");
            for byte in value {
                let _ = write!(out, "{byte:02x}");
            }
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ShapesProductRefusal, admit_shapes_product, admit_shapes_product_expecting,
        certify_shapes_product, explain_shapes_product, pack_shapes_product,
        pack_shapes_product_from_dataset, parse_identity_digest, rebuild_shapes_product,
        rebuild_shapes_product_expecting, validate_with_rebuilt_shapes_product,
        validate_with_rebuilt_shapes_product_expecting, validate_with_shapes_product,
        validate_with_shapes_product_expecting,
    };
    use crate::SarifOptions;
    use purrdf_shapes::product::ProductDimension;
    use purrdf_shapes::text_ingest::{extract_prefixes, parse_turtle_to_dataset};

    const SHAPES: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
        @prefix ex: <http://example.org/> .\n\
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\
        ex:PersonShape a sh:NodeShape ;\n\
          sh:targetClass ex:Person ;\n\
          sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] .\n";

    /// The same shapes graph as [`SHAPES`], plus an `owl:imports` this crate has no way
    /// to resolve — used to exercise [`pack_shapes_product`]'s refusal.
    const SHAPES_WITH_UNRESOLVED_IMPORT: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
        @prefix ex: <http://example.org/> .\n\
        @prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n\
        <http://example.org/> a owl:Ontology ;\n\
          owl:imports <http://example.org/lib> .\n\
        ex:PersonShape a sh:NodeShape ;\n\
          sh:targetClass ex:Person ;\n\
          sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] .\n";

    const DATA: &str = "<http://example.org/alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .\n\
        <http://example.org/alice> <http://example.org/age> \"nope\" .\n";

    #[test]
    fn a_product_round_trips_through_every_reader() {
        let product = pack_shapes_product(SHAPES, None).expect("shapes pack");
        admit_shapes_product(&product).expect("admit");
        rebuild_shapes_product(&product).expect("rebuild");
        certify_shapes_product(&product).expect("certify");

        let explained = explain_shapes_product(&product).expect("explain");
        assert!(explained.starts_with("format-version 1\n"));
        assert!(explained.contains("stage-known true\n"));
        assert!(explained.contains("identity profile \"purrdf-shacl-core-v1\"\n"));

        // The product validates the same data the shapes document does, through
        // the same engine entry point.
        let sarif = validate_with_shapes_product(&product, DATA, &SarifOptions::default())
            .expect("validate with the product");
        assert!(sarif.contains("\"version\": \"2.1.0\""));
        assert!(sarif.contains("\"level\": \"error\""));

        // The rebuild route reaches the identical report for a CURRENT product: a
        // caller who reaches for the forward-compatibility path anyway must not get
        // a second, divergent answer.
        let rebuilt_sarif =
            validate_with_rebuilt_shapes_product(&product, DATA, &SarifOptions::default())
                .expect("validate with the rebuilt product");
        assert_eq!(sarif, rebuilt_sarif);
    }

    #[test]
    fn a_corrupt_product_is_refused_on_a_named_dimension() {
        let product = pack_shapes_product(SHAPES, None).expect("shapes pack");

        // A whole, well-sized product whose MAGIC was overwritten: these bytes
        // were never a prepared shapes product, and the outermost check says so.
        let mut wrong_magic = product.clone();
        wrong_magic[0] = b'X';
        let refusal = certify_shapes_product(&wrong_magic).expect_err("a foreign magic is refused");
        assert_eq!(refusal.dimension(), ProductDimension::Magic);

        // The neighbouring VALID case still succeeds — a refusal is a claim too,
        // and a byte restored is a product admitted.
        certify_shapes_product(&product).expect("the unmodified product still certifies");
    }

    #[test]
    fn a_shapes_parse_failure_names_no_dimension() {
        let refusal = pack_shapes_product("@@@ not turtle", None).expect_err("refused");
        assert!(matches!(refusal, ShapesProductRefusal::Shapes(_)));
        assert_eq!(refusal.dimension(), None);
        assert_eq!(refusal.dimension_label(), None);

        // …while an admission refusal carries its label all the way out.
        let mut wrong_magic = pack_shapes_product(SHAPES, None).expect("shapes pack");
        wrong_magic[0] = b'X';
        let admission: ShapesProductRefusal = certify_shapes_product(&wrong_magic)
            .expect_err("refused")
            .into();
        assert_eq!(admission.dimension_label(), Some("magic"));
        assert!(admission.to_string().starts_with("magic: "));
        // The message channel carries the prose WITHOUT the label doubled into it.
        assert!(!admission.message().starts_with("magic: "));
    }

    /// The core equivalence [`pack_shapes_product_from_dataset`] exists to guarantee: a
    /// caller who parses the same Turtle into a dataset itself and calls the dataset-level
    /// entry point gets byte-IDENTICAL bytes to the text-level one, for a graph with no
    /// imports. `pack_shapes_product` is implemented in terms of this function precisely so
    /// the two can never drift — this test pins that down from the outside as well.
    #[test]
    fn the_dataset_entry_point_matches_the_text_entry_point_byte_for_byte() {
        let via_text = pack_shapes_product(SHAPES, None).expect("text entry point");

        let dataset = parse_turtle_to_dataset(SHAPES, None).expect("dataset parse");
        let prefixes = extract_prefixes(SHAPES);
        let via_dataset = pack_shapes_product_from_dataset(&dataset, &prefixes, None, None)
            .expect("dataset entry point");

        assert_eq!(
            via_text, via_dataset,
            "the two pack entry points must agree byte for byte on a graph with no imports"
        );
    }

    /// The neighbouring VALID case for the test above: a base and no imports still packs
    /// identically through both entry points, so the equivalence is not an artifact of
    /// `base = None`.
    #[test]
    fn the_dataset_entry_point_matches_the_text_entry_point_with_a_base() {
        let base = Some("https://example.org/shapes");
        let via_text = pack_shapes_product(SHAPES, base).expect("text entry point");

        let dataset = parse_turtle_to_dataset(SHAPES, base).expect("dataset parse");
        let prefixes = extract_prefixes(SHAPES);
        let via_dataset = pack_shapes_product_from_dataset(&dataset, &prefixes, base, None)
            .expect("dataset entry point");

        assert_eq!(via_text, via_dataset);
    }

    /// `pack_shapes_product` has no `--import` table and no stderr to warn on, so an
    /// `owl:imports` it cannot resolve is a HARD refusal rather than a silently smaller
    /// shapes graph — see the function's own doc comment for why packing the root graph
    /// alone would be exactly the silent omission this codec exists to rule out.
    #[test]
    fn refuses_an_unresolved_import_on_unsupported_capability() {
        let refusal = pack_shapes_product(SHAPES_WITH_UNRESOLVED_IMPORT, None)
            .expect_err("an unresolved owl:imports is refused");
        assert_eq!(
            refusal.dimension(),
            Some(ProductDimension::UnsupportedCapability)
        );
        assert!(refusal.to_string().contains("http://example.org/lib"));
    }

    /// The neighbouring VALID case: a graph with NO `owl:imports` at all still packs
    /// through the very same function — the refusal above is triggered by an unresolved
    /// import, never by the mere presence of an `owl:Ontology` header or of imports in
    /// general.
    #[test]
    fn accepts_a_graph_with_no_imports_neighbour() {
        pack_shapes_product(SHAPES, None).expect("a graph with no owl:imports still packs");
    }

    /// A second shapes graph, over different classes, so the two products genuinely
    /// carry two input bindings.
    const OTHER_SHAPES: &str = "@prefix sh: <http://www.w3.org/ns/shacl#> .\n\
        @prefix ex: <http://example.org/> .\n\
        ex:WidgetShape a sh:NodeShape ;\n\
          sh:targetClass ex:Widget ;\n\
          sh:property [ sh:path ex:maker ; sh:minCount 1 ] .\n";

    /// The `identity-digest` a product renders — the one spelling of the selector,
    /// read here exactly the way a consumer reads it off an artifact.
    fn rendered_selector(product: &[u8]) -> String {
        explain_shapes_product(product)
            .expect("explain")
            .lines()
            .find_map(|line| line.strip_prefix("identity-digest ").map(ToOwned::to_owned))
            .expect("the rendering carries an identity digest")
    }

    /// The whole point: a product that is perfectly valid and is NOT the one the
    /// caller required is refused, on a named dimension, before anything is decoded.
    #[test]
    fn refuses_a_product_that_is_not_the_expected_one() {
        let held = pack_shapes_product(SHAPES, None).expect("shapes pack");
        let wanted = pack_shapes_product(OTHER_SHAPES, None).expect("other shapes pack");
        let selector =
            parse_identity_digest(&rendered_selector(&wanted)).expect("a rendered selector parses");

        let refusal = admit_shapes_product_expecting(&held, &selector)
            .expect_err("the product held is not the product required");
        assert_eq!(refusal.dimension(), ProductDimension::ShapesGraph);

        // The gap this closes: the unbound path admits the very same bytes, because
        // nothing in them states which product was meant.
        admit_shapes_product(&held).expect("an unbound admit cannot ask which product was wanted");
    }

    /// The neighbouring VALID case: a product required to be ITSELF restores, and
    /// validating through the bound path reaches the byte-identical report the unbound
    /// path reaches. An expectation nobody can satisfy would send every consumer back
    /// to the unbound call it exists to replace.
    #[test]
    fn accepts_the_expected_product_neighbour() {
        let product = pack_shapes_product(SHAPES, None).expect("shapes pack");
        let selector = parse_identity_digest(&rendered_selector(&product))
            .expect("a rendered selector parses");

        admit_shapes_product_expecting(&product, &selector)
            .expect("a product required to be itself restores");

        let bound = validate_with_shapes_product_expecting(
            &product,
            DATA,
            &selector,
            &SarifOptions::default(),
        )
        .expect("the bound validation runs");
        let unbound = validate_with_shapes_product(&product, DATA, &SarifOptions::default())
            .expect("the unbound validation runs");
        assert_eq!(
            bound, unbound,
            "stating which product you meant must not change the answer, only the door",
        );
    }

    /// The composed rebuild-and-validate call answers the same "is this the
    /// product I asked for?" question as `validate_with_shapes_product_expecting`:
    /// a product that is not the one required is refused before its carried
    /// dataset is ever re-derived.
    #[test]
    fn refuses_a_rebuild_validation_that_is_not_the_expected_one() {
        let held = pack_shapes_product(SHAPES, None).expect("shapes pack");
        let wanted = pack_shapes_product(OTHER_SHAPES, None).expect("other shapes pack");
        let selector =
            parse_identity_digest(&rendered_selector(&wanted)).expect("a rendered selector parses");

        let refusal = validate_with_rebuilt_shapes_product_expecting(
            &held,
            DATA,
            &selector,
            &SarifOptions::default(),
        )
        .expect_err("the product held is not the product required");
        assert_eq!(refusal.dimension(), Some(ProductDimension::ShapesGraph));

        // The gap this closes: the unbound rebuild-and-validate call runs over the
        // very same bytes, because nothing in them states which product was meant.
        validate_with_rebuilt_shapes_product(&held, DATA, &SarifOptions::default())
            .expect("an unbound rebuild-and-validate cannot ask which product was wanted");
    }

    /// The neighbouring VALID case: a product required to be ITSELF still runs the
    /// composed rebuild-and-validate call, and reaches the byte-identical report
    /// the unbound rebuild-and-validate call and the bound admission both reach.
    #[test]
    fn accepts_the_expected_product_neighbour_on_rebuild_validation() {
        let product = pack_shapes_product(SHAPES, None).expect("shapes pack");
        let selector = parse_identity_digest(&rendered_selector(&product))
            .expect("a rendered selector parses");

        let bound = validate_with_rebuilt_shapes_product_expecting(
            &product,
            DATA,
            &selector,
            &SarifOptions::default(),
        )
        .expect("a product required to be itself rebuilds and validates");
        let unbound =
            validate_with_rebuilt_shapes_product(&product, DATA, &SarifOptions::default())
                .expect("the unbound rebuild-and-validate call runs");
        assert_eq!(
            bound, unbound,
            "stating which product you meant must not change the answer, only the door",
        );
    }

    /// The selector spelling is a ROUND TRIP, not two conventions that happen to
    /// agree today: what a product renders is what the boundary accepts back.
    #[test]
    fn the_rendered_selector_is_the_accepted_selector() {
        let product = pack_shapes_product(SHAPES, None).expect("shapes pack");
        let rendered = rendered_selector(&product);
        assert_eq!(rendered.len(), 64, "the rendering is 64 hexadecimal digits");

        let selector = parse_identity_digest(&rendered).expect("the rendering is accepted back");
        admit_shapes_product_expecting(&product, &selector)
            .expect("a product is required by the selector it renders");

        // Case is not significant on the way in. The renderer emits lowercase, but a
        // selector that travelled through a shell, a manifest or a CI variable may not
        // have stayed that way, and refusing it for a shape the mechanism does not care
        // about would be refusing input that is actually valid.
        let shouted = parse_identity_digest(&rendered.to_uppercase())
            .expect("an upper-case selector names the same product");
        assert_eq!(selector, shouted);
    }

    /// The rebuild path answers the same "is this the product I asked for?"
    /// question `admit_expecting` does: a product whose binding is not the one
    /// required is refused on `shapes-graph` even though its stage id is one this
    /// build knows and `rebuild` would otherwise happily re-derive it.
    #[test]
    fn refuses_a_rebuilt_product_that_is_not_the_expected_one() {
        let held = pack_shapes_product(SHAPES, None).expect("shapes pack");
        let wanted = pack_shapes_product(OTHER_SHAPES, None).expect("other shapes pack");
        let selector =
            parse_identity_digest(&rendered_selector(&wanted)).expect("a rendered selector parses");

        let refusal = rebuild_shapes_product_expecting(&held, &selector)
            .expect_err("the product held is not the product required");
        assert_eq!(refusal.dimension(), ProductDimension::ShapesGraph);

        // The gap this closes: the unbound rebuild restores the very same bytes,
        // because nothing in them states which product was meant.
        rebuild_shapes_product(&held).expect("an unbound rebuild cannot ask which product");
    }

    /// The neighbouring VALID case: a product required to be ITSELF still rebuilds,
    /// and reaches the byte-identical report the unbound rebuild and the bound
    /// `admit_expecting` both reach — stating which product you meant, and choosing
    /// to re-derive rather than restore the memo, must not change the answer.
    #[test]
    fn accepts_the_expected_product_neighbour_on_rebuild() {
        use purrdf_shapes::engine;

        let product = pack_shapes_product(SHAPES, None).expect("shapes pack");
        let selector = parse_identity_digest(&rendered_selector(&product))
            .expect("a rendered selector parses");

        let bound_rebuild = rebuild_shapes_product_expecting(&product, &selector)
            .expect("a product required to be itself rebuilds");
        let bound_admit = admit_shapes_product_expecting(&product, &selector)
            .expect("a product required to be itself admits");

        // The bound rebuild and the bound admit must reach the byte-identical
        // report over the same data: choosing to re-derive rather than restore
        // the memo — and stating which product you meant — must not change the
        // answer, only the door.
        let data = purrdf_shapes::text_ingest::parse_ntriples_to_dataset(DATA)
            .expect("the data graph parses");
        let rebuilt_report =
            engine::validate_dataset_with_shapes_graph(data.as_ref(), bound_rebuild.shapes(), None)
                .expect("validated against the rebuilt shapes");
        let admitted_report =
            engine::validate_dataset_with_shapes_graph(data.as_ref(), bound_admit.shapes(), None)
                .expect("validated against the admitted shapes");
        assert_eq!(
            crate::report_to_sarif_string(&rebuilt_report, &SarifOptions::default()),
            crate::report_to_sarif_string(&admitted_report, &SarifOptions::default()),
        );
    }

    /// A selector that is not 64 hexadecimal digits carries NO dimension, because no
    /// product was opened to name a dimension of. The two neighbouring valid spellings
    /// — the rendering itself, and the rendering with surrounding whitespace a shell or
    /// a file read leaves behind — must still parse.
    #[test]
    fn refuses_a_selector_that_is_not_a_digest() {
        for bad in ["", "not-a-digest", "abc", &"f".repeat(63), &"f".repeat(65)] {
            let why = parse_identity_digest(bad).expect_err("a non-digest selector is refused");
            assert!(
                why.contains("64 hexadecimal digits"),
                "the refusal names the accepted spelling, got {why:?}",
            );
        }

        let product = pack_shapes_product(SHAPES, None).expect("shapes pack");
        let rendered = rendered_selector(&product);
        let direct = parse_identity_digest(&rendered).expect("the rendering parses");
        let padded =
            parse_identity_digest(&format!("  {rendered}\n")).expect("a padded selector parses");
        assert_eq!(direct, padded);
    }
}
