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
/// which is also where a `sh:SPARQLFunction` this format cannot carry refuses.
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
    let prepared = admit_shapes_product(product)?;
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
        ShapesProductRefusal, admit_shapes_product, certify_shapes_product, explain_shapes_product,
        pack_shapes_product, pack_shapes_product_from_dataset, rebuild_shapes_product,
        validate_with_shapes_product,
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
}
