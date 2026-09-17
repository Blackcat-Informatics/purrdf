// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The `shacl` subcommands: prepared-shapes-product utilities.
//!
//! A SHACL shapes graph is parsed, analyzed and compiled before a single focus node
//! is looked at, and that work is identical every time the same document is
//! validated against. `shacl pack` performs it once and writes the result out as a
//! **prepared product**; `validate --shapes-product` restores it instead of
//! re-parsing. The codec is `purrdf_shapes::product`, reached through the shared
//! [`purrdf_validate::product`] boundary the Python, WebAssembly and C-ABI hosts
//! also call — there is no CLI-local writer and no CLI-local reader.
//!
//! # A product is UNTRUSTED bytes, so this lane is an admission boundary
//!
//! By the time a product comes back it is a file on disk like any other: nothing in
//! it is evidence of its own provenance, and the build restoring it cannot assume it
//! wrote it, wrote it at this version, or wrote it against the same caller-supplied
//! configuration. So `verify` and `explain` are not diagnostics bolted onto a cache
//! — they are the two things a caller holding a refused product needs:
//!
//! * `verify` runs the codec's COLD path, [`certify_shapes_product`], which
//!   independently re-derives the shapes dataset's canonical identity and compares it
//!   against the one the product's binding claims. That is a graph-isomorphism
//!   computation over blank nodes and is deliberately unreachable from a restore, so
//!   this verb is where it lives.
//! * `explain` decodes what the product says it was compiled FROM — the base, the
//!   prefix map, the vocabulary, the registries, the class catalog — without
//!   admitting any of it. This is what makes a named refusal actionable: a restore
//!   refused on `prefixes` is answered by reading which prefix map the product
//!   actually carries, not by guessing.
//!
//! Both print a refusal's DIMENSION, because the dimension is the stable thing a
//! caller branches on: `stage-id` means re-pack, or restore with `validate
//! --shapes-product --rebuild`; `container-digest` means the file is corrupt in
//! place; `function-registry` means the caller's own configuration is wrong and
//! re-packing will not help. Collapsing those to one exit code and one prose
//! string is exactly what the typed boundary exists to prevent, so the label is
//! written to stderr as a `key value` line alongside the message.
//!
//! # Exit codes
//!
//! **0** when the product is written, certified, or explained. **1** when it is
//! refused — a refusal is a decided verdict about an ARTIFACT, not about data, so
//! unlike a non-conforming validation it really is a failure of the thing asked for.
//! **2** for a usage error.
//!
//! # Where the bytes come from
//!
//! A product is read through [`source::acquire_product_input`], the same sealed-mmap
//! immutable-input authority `pack verify` reads a pack through, and for the same
//! reason: the integrity check runs over the bytes in hand, so a hostile concurrent
//! pathname writer must not be able to move them afterwards. The product's view
//! borrows that owner, so the owner is held for the whole operation.
//!
//! [`purrdf_validate::product`]: purrdf_validate::product
//! [`certify_shapes_product`]: purrdf_validate::certify_shapes_product

use purrdf::shapes::product::ShapesProductError;
use purrdf_rdf::{NativeRdfFormat, SourceFormat};

use crate::error::CliError;
use crate::{sink, source};

/// Run `shacl pack`: read the shapes document at `shapes`, fold its `owl:imports` closure
/// against `imports`, prepare the result, and write the prepared product to `out`.
///
/// `base` overrides the base IRI the shapes document's relative IRI references resolve
/// against; omitted, the document's own RFC-8089 `file://` retrieval IRI is derived,
/// exactly as `validate --shapes` derives it. That agreement is load-bearing: a product
/// packed from a document must restore to the shapes that document parses to, and a
/// lane that derived a different base would produce a product whose `<PersonShape>`
/// denotes a different IRI than the one the operator wrote.
///
/// `shapes_graph` is `--shapes-graph`, resolved against that SAME base through
/// [`crate::shapes_source::resolve_shapes_graph`] — the identical function
/// `validate --shapes --shapes-graph` calls — and recorded into the product's identity. A
/// product packed with `--shapes-graph IRI` and a document validated with
/// `--shapes --shapes-graph IRI` therefore expose `$shapesGraph` under the same absolute
/// IRI and reach the byte-identical report: before this parameter existed, `shacl pack` had
/// no way to record ANY `--shapes-graph` override, so a shapes graph whose SHACL-SPARQL
/// bodies read `GRAPH $shapesGraph { … }` validated one answer through `--shapes` and a
/// different one through a restored product, with no flag on `shacl pack` able to close the
/// gap.
///
/// The document is Turtle. That is not a restriction this lane invents for its own
/// convenience — it is the one syntax that carries a `@prefix`/`PREFIX` map recoverable
/// from source text, which is the fallback prefix environment every SHACL-AF
/// `sh:select` body resolves against and which the product records in its parse
/// provenance. A product packed from a syntax with no such map would restore SHACL-AF
/// query bodies that resolve differently than the ones the shapes graph declared.
///
/// # This lane and `validate --shapes` share ONE seam
///
/// The read and the import fold are [`crate::shapes_source::read_shapes_document`] and
/// [`crate::shapes_source::fold_shapes_imports`] — the identical functions
/// `validate --shapes --import` calls — rather than a second copy of the same walk. That is
/// what makes a product written by `purrdf shacl pack --import IRI=FILE` and a run of
/// `purrdf validate --shapes --import IRI=FILE` agree by construction: there is
/// exactly one implementation of "what does folding this closure mean", so the two commands
/// cannot silently diverge on it the way they used to, when this lane parsed raw text with no
/// import table at all and dropped an unresolved `owl:imports` with nothing printed.
///
/// A shapes graph naming no `--import` at all is not refused for it: every unresolved
/// `owl:imports` is reported on stderr as a `shacl warning` line and the product is packed
/// from the shapes graph alone — see [`crate::shapes_source`]'s module documentation for why
/// that asymmetry is deliberate. Naming any `--import` pair makes the closure mandatory: an
/// `owl:imports` no pair resolves, or a pair the closure never reaches, is then a usage error.
///
/// # Errors
///
/// [`CliError::Usage`] when `--shapes -` is given (stdin has no retrieval IRI to derive
/// a base from, so a relative reference in it would silently resolve to nothing), when a
/// relative `--shapes-graph` has no base to resolve against, or when an `--import` pair is
/// malformed, resolves nothing the shapes graph imports, or is never reached by the import
/// closure; [`CliError::Runtime`] when a document cannot be read, is not UTF-8, does not
/// parse, an `owl:imports` no pair resolves, or the shapes graph declares a capability the
/// product format cannot carry.
pub(crate) fn pack(
    shapes: &str,
    base: Option<&str>,
    imports: &[String],
    shapes_graph: Option<&str>,
    out: &str,
) -> Result<(), CliError> {
    let effective_base = source::effective_base(shapes, NativeRdfFormat::Turtle, base)?;
    if effective_base.is_none() {
        return Err(CliError::Usage(format!(
            "--shapes {shapes}: a prepared product RECORDS the base its shapes graph was \
             parsed under, so that a restore resolves the same relative IRI references \
             without the document. Standard input has no retrieval IRI to derive one from — \
             give --shapes a PATH, or name the base with --base"
        )));
    }
    // Resolved against the SAME base the document parses under, and before either document
    // is read — a `--shapes-graph` that names no graph is a malformed request and should fail
    // against the command line rather than after the shapes graph has been folded, exactly
    // the ordering `validate --shapes` decides in.
    let shapes_graph =
        crate::shapes_source::resolve_shapes_graph(shapes_graph, effective_base.as_deref())?;

    let root = crate::shapes_source::read_shapes_document(
        shapes,
        SourceFormat::Native(NativeRdfFormat::Turtle),
        effective_base.as_deref(),
        "--shapes",
    )?;
    let folded = crate::shapes_source::fold_shapes_imports(root, imports)?;

    let product = purrdf_validate::pack_shapes_product_from_dataset(
        &folded.dataset,
        &folded.prefixes,
        effective_base.as_deref(),
        shapes_graph,
    )
    .map_err(|refusal| refusal_error(&format!("--shapes {shapes}"), &refusal))?;

    let written = product.len();
    sink::write_out(out, &product)?;
    // The receipt AFTER the artifact, so a `> file` redirect has the product on disk by
    // the time the operator reads the line describing it — the same ordering `validate`
    // uses for its verdict.
    eprintln!("shacl product bytes {written}");
    Ok(())
}

/// Run `shacl verify`: open the product at `product` and run the codec's cold-path
/// certification over it.
///
/// Prints the product's own identity digest on success, the way `pack verify` prints the
/// digest it verified: the answer to "which artifact did you just certify" is the digest,
/// not the word `ok`.
///
/// # Errors
///
/// [`CliError::Runtime`] naming the refused [`ProductDimension`] when the product is not
/// a well-formed product of this format, or when its shapes dataset does not canonicalize
/// to the digest its binding claims.
///
/// [`ProductDimension`]: purrdf_shapes::product::ProductDimension
pub(crate) fn verify(product: &str) -> Result<(), CliError> {
    let owner = source::acquire_product_input(product)?;
    purrdf_validate::certify_shapes_product(owner.as_bytes())
        .map_err(|error| admission_error(product, &error))?;
    // `explain` already renders the identity digest; reading it back from the same
    // rendering is what keeps the two verbs from reporting two independently-derived
    // answers about one product.
    let explained = purrdf_validate::explain_shapes_product(owner.as_bytes())
        .map_err(|error| admission_error(product, &error))?;
    for line in explained.lines() {
        if let Some(digest) = line.strip_prefix("identity-digest ") {
            println!("{digest}");
            return Ok(());
        }
    }
    Err(CliError::Runtime(format!(
        "{product}: this product certified, but its rendering carries no identity digest. \
         That is a defect in this build rather than in the product; re-run `shacl explain` \
         and report what it prints"
    )))
}

/// Run `shacl explain`: open the product at `product` and print everything it says about
/// itself, without admitting it.
///
/// The rendering is [`purrdf_validate::explain_shapes_product`]'s — deterministic
/// `key value` lines — written verbatim to stdout. The CLI reformats nothing, because a
/// second rendering is a second answer, and this one is the identical text the
/// WebAssembly and C-ABI hosts receive.
///
/// # Errors
///
/// [`CliError::Runtime`] naming the refused dimension when the bytes are not a
/// well-formed product of this format.
pub(crate) fn explain(product: &str) -> Result<(), CliError> {
    let owner = source::acquire_product_input(product)?;
    let explained = purrdf_validate::explain_shapes_product(owner.as_bytes())
        .map_err(|error| admission_error(product, &error))?;
    sink::write_out("-", explained.as_bytes())
}

/// Turn an admission refusal into a [`CliError`], writing the DIMENSION to stderr as its
/// own `key value` line first.
///
/// The dimension is on its own line rather than only inside the prose because it is the
/// part a shell can branch on: `shacl dimension stage-id` is a stable token, and the
/// message beside it is prose that may be reworded. That is the same split the codec
/// itself draws between `ShapesProductError::dimension` and its message, carried across
/// the process boundary instead of flattened at it.
pub(crate) fn admission_error(what: &str, error: &ShapesProductError) -> CliError {
    eprintln!("shacl dimension {}", error.dimension().label());
    CliError::Runtime(format!("{what}: {error}"))
}

/// Turn a [`purrdf_validate::ShapesProductRefusal`] into a [`CliError`].
///
/// The dimension line is written only when there IS one: a shapes document that did not
/// parse never reached the admission boundary, and printing a dimension for it would
/// claim a product was inspected when none was ever written.
pub(crate) fn refusal_error(
    what: &str,
    refusal: &purrdf_validate::ShapesProductRefusal,
) -> CliError {
    if let Some(label) = refusal.dimension_label() {
        eprintln!("shacl dimension {label}");
    }
    CliError::Runtime(format!("{what}: {refusal}"))
}
