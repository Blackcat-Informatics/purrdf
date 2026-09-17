// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The shapes-dataset section of a prepared product: the RDF a parsed
//! [`Shapes`](crate::shapes::Shapes) was built from, carried across the product
//! boundary in the workspace's existing succinct pack container.
//!
//! [`encode_dataset`] writes `Shapes::shapes_dataset` out, [`open_dataset`] reads
//! it back, and [`certify_dataset`] separately corroborates that what came back
//! is the dataset the section claims to hold.
//!
//! # Why the dataset is carried at all
//!
//! The declarative AST is not the whole of a parsed shapes graph. SHACL-SPARQL
//! exposes the shapes graph itself as a NAMED GRAPH — `$shapesGraph` is bound for
//! every `sh:select` body, and `crate::engine`'s
//! `validate_dataset_with_shapes_graph` builds that named graph out of
//! `Shapes::shapes_dataset` — so a product that dropped the dataset would restore
//! a validator that still loads, still verifies, and answers `GRAPH $shapesGraph
//! { … }` with zero rows instead of the rows the source shapes graph has. Nothing
//! about that failure is loud: the report is well formed, it simply describes a
//! shapes graph nobody asked about.
//!
//! Carrying the dataset is also what preserves RDF 1.2 TERM IDENTITY across the
//! boundary. Quoted triples, reifier bindings, statement annotations,
//! language-tagged literals with their full subtag sequence, and
//! `rdf:dirLangString` literals whose base direction is part of their identity
//! (`"x"@en--ltr` and `"x"@en--rtl` are two distinct terms) all survive because
//! the pack codec already round-trips every one of them through its unified
//! dictionary and side-tables. This module adds no term handling of its own,
//! which is precisely why it cannot lose a term kind the codec knows about.
//!
//! # The rejected alternative: re-serialize the shapes graph as text
//!
//! The obvious alternative was to write the shapes dataset out as N-Quads (or
//! Turtle) and reparse it on restore. It was rejected for two independent
//! reasons. First, it reintroduces exactly the cost this whole feature exists to
//! remove: a prepared product whose restore path runs the RDF parser has not
//! eliminated the parse, it has only moved it. Second, a text round trip is a
//! SECOND codec for RDF 1.2 term identity, and a second codec is a second place
//! for a term kind to go missing — the pack container's dictionary/side-table
//! layout is already frozen against golden vectors, and reusing it means the
//! shapes graph inherits that coverage instead of asking for its own.
//!
//! # 🔴 The two-tier rule: opening is not certifying
//!
//! Performance in the common case is paramount; conformance testing may be slow,
//! because it is never part of normal usage. That standing rule is why this
//! module exposes TWO entry points over the same bytes, and why they must stay
//! separate:
//!
//! | Path | Tier | What it costs | Who calls it |
//! |------|------|---------------|--------------|
//! | [`open_dataset`] | COMMON — every restore | `PackView::from_bytes` only: magic, version, each section's SHA-256, each section reader's own structural validation, then a term-by-term reconstruction | the product loader |
//! | [`certify_dataset`] | COLD — never in normal usage | all of the above PLUS an independent re-canonicalization of the reconstructed dataset and a digest comparison against the section's stored canonical identity | tests, conformance harnesses, a CLI verify subcommand |
//!
//! `verify_pack` is NOT reachable from [`open_dataset`], and that is a load-bearing
//! property rather than an optimization. Canonicalization is a graph-isomorphism
//! computation over the shapes graph's blank nodes; putting it on the restore path
//! would very plausibly cost MORE than the shapes parse this feature exists to
//! eliminate. The product would then be slower than the thing it replaces while
//! every test in this file still passed — the benchmark nobody ran is the only
//! place that regression shows up. `open_does_not_certify` in this module's tests
//! pins the split by construction: a section whose stored canonical digest has
//! been tampered with (leaving every section digest intact) MUST open and MUST
//! fail certification. If someone quietly moves the certification onto the common
//! path, that test fails.
//!
//! This mirrors the pack layer's own precedent exactly, rather than inventing a
//! policy: `PackView::from_bytes` is the cheap integrity check and `verify_pack`
//! is the separate, caller-invoked certification, for the reasons `purrdf_core`'s
//! `ir::pack::certify` module documentation gives.
//!
//! # Refusals
//!
//! Every pack failure is mapped onto an existing [`ProductDimension`] — no pack
//! error escapes untyped, and no new error type is minted. See
//! [`refuse`] for the mapping and its justification.
//!
//! Nothing here touches the filesystem, a clock, a thread, or a source of
//! randomness: it is a pure function of caller-supplied bytes and stays
//! `wasm32-unknown-unknown` compatible.

use std::sync::Arc;

use ::purrdf::{PackBuilder, PackDigest, PackError, RdfDataset, restore_pack, verify_pack};

use super::error::{ProductDimension, ShapesProductError};

// ---------------------------------------------------------------------------
// Pack error → admission dimension
// ---------------------------------------------------------------------------

/// Map a [`PackError`] onto the admission [`ProductDimension`] that names it, and
/// pair it with a message that states the fix.
///
/// # The mapping, and why each arm is the apt dimension
///
/// The dimensions are ordered from the outside of the container inward, and the
/// pack's own error vocabulary is ordered the same way, so most arms are a direct
/// correspondence:
///
/// * `BadMagic` → [`Magic`]. Both say "these bytes were never this format".
/// * `UnsupportedVersion` → [`FormatVersion`]. Both say "this build does not
///   decode that revision"; rebuilding is the fix in either vocabulary.
/// * `Truncated` → [`Truncated`]. The dimension exists to separate an interrupted
///   write from corruption in place, which is the same distinction the pack
///   variant draws.
/// * `SectionDigestMismatch` → [`SectionDigest`]. The pack container's sections
///   ARE the product's sections here; a section whose SHA-256 disagrees with its
///   bytes is corrupt in place under either name.
/// * `CanonRefused` and `RdfcDigestMismatch` → [`DatasetIdentity`]. Both are
///   failures of the shapes dataset's CANONICAL IDENTITY rather than of its
///   framing: either the identity could not be computed at all (a pathologically
///   symmetric blank-node graph exhausting the canonicalization budget, or a
///   reserved-namespace IRI whose acceptance would let a different dataset forge
///   this one's identity), or it was computed and disagreed with the identity the
///   section claims. Note that neither is reachable from [`open_dataset`] — only
///   [`certify_dataset`] performs the recompute, by design.
/// * `ViewNotReady` → [`Malformed`], as the residual. This one needs the most
///   justification because it is the least apt: it is not a statement about a
///   candidate product's bytes at all, it says the encoder never got a complete
///   reading of its SOURCE, so no product was produced. No dimension names "the
///   source was unreadable", and inventing one is out of scope for this module.
///   [`Malformed`] is documented as the dimension for a structural failure no
///   other dimension names, so it is where this lands. It is also unreachable
///   through [`encode_dataset`]: a frozen [`RdfDataset`] is a view whose status is
///   `Ready` by construction, so the checkpoints the pack builder samples cannot
///   observe a failure. The arm exists so the mapping is total, not because the
///   case occurs.
/// * `Dict`, `Triples`, `Side` → [`Malformed`]. Each is a section-internal
///   structural decode failure discovered AFTER that section's SHA-256 already
///   passed, so the bytes are intact and the structure they encode is invalid —
///   which is [`Malformed`]'s definition ("a field is out of range, a length
///   disagrees with its payload, or a reference points outside the container"),
///   not [`SectionDigest`]'s.
/// * Any future variant → [`Malformed`]. [`PackError`] is `#[non_exhaustive]`, so
///   a wildcard is mandatory. Mapping an unknown failure to the residual
///   structural dimension fails CLOSED: the product is refused, and refused under
///   a dimension whose documented meaning is "structurally invalid in a way no
///   other dimension names", which is exactly what an unrecognized container
///   failure is from this boundary's point of view.
///
/// [`Magic`]: ProductDimension::Magic
/// [`FormatVersion`]: ProductDimension::FormatVersion
/// [`Truncated`]: ProductDimension::Truncated
/// [`SectionDigest`]: ProductDimension::SectionDigest
/// [`DatasetIdentity`]: ProductDimension::DatasetIdentity
/// [`Malformed`]: ProductDimension::Malformed
fn refuse(error: &PackError) -> ShapesProductError {
    let (dimension, fix) = match error {
        PackError::BadMagic => (
            ProductDimension::Magic,
            "these bytes do not begin with the pack container's magic, so they are not a shapes \
             dataset section at all; hand this decoder the bytes the product's dataset section \
             actually holds, or re-prepare the product from its shapes graph",
        ),
        PackError::UnsupportedVersion(_) => (
            ProductDimension::FormatVersion,
            "the shapes dataset section was written in a container revision this build does not \
             decode; re-prepare the product with the same PurRDF build that will execute it",
        ),
        PackError::Truncated => (
            ProductDimension::Truncated,
            "the shapes dataset section ends before a structure it declared is complete, which \
             is an interrupted write rather than corruption in place; re-prepare the product and \
             write the whole section again",
        ),
        PackError::SectionDigestMismatch { .. } => (
            ProductDimension::SectionDigest,
            "a shapes dataset section's recorded SHA-256 disagrees with the bytes stored for it, \
             so the RDF behind these shapes is corrupt in place; discard this product and \
             re-prepare it from the shapes graph",
        ),
        PackError::CanonRefused(_) => (
            ProductDimension::DatasetIdentity,
            "the shapes dataset's canonical identity could not be computed, so nothing can \
             corroborate what this section claims to hold; re-prepare the product from a shapes \
             graph that canonicalizes — one whose blank-node structure is not pathologically \
             symmetric and that mints no IRI in the canonicalization profile's reserved \
             namespace",
        ),
        PackError::RdfcDigestMismatch { .. } => (
            ProductDimension::DatasetIdentity,
            "the shapes dataset section's stored canonical identity does not match the dataset \
             its own bytes decode to, so the identity is a claim nothing corroborates; discard \
             this product and re-prepare it from the shapes graph",
        ),
        PackError::ViewNotReady { .. } => (
            ProductDimension::Malformed,
            "the shapes dataset could not be read completely while the section was being \
             written, so no section was published rather than a truncated one; re-prepare the \
             product once the shapes dataset reads completely",
        ),
        PackError::Dict(_) | PackError::Triples(_) | PackError::Side(_) => (
            ProductDimension::Malformed,
            "a shapes dataset section decoded its own bytes intact but found them structurally \
             invalid; discard this product and re-prepare it from the shapes graph",
        ),
        // `PackError` is `#[non_exhaustive]`: a variant added upstream must refuse
        // here rather than fall through, and the residual structural dimension is
        // the fail-closed landing spot. See this function's doc comment.
        _ => (
            ProductDimension::Malformed,
            "the shapes dataset section failed a container check this build has no more specific \
             name for; discard this product and re-prepare it from the shapes graph",
        ),
    };

    ShapesProductError::new(dimension, format!("{fix} (container reported: {error})"))
}

// ---------------------------------------------------------------------------
// The section codec
// ---------------------------------------------------------------------------

/// Write the shapes dataset out as the product's dataset section.
///
/// Byte-deterministic: the pack container is a pure function of the dataset's
/// VALUE content, so two calls over the same shapes graph produce identical
/// bytes and no hash-iteration order, wall clock or RNG reaches the output.
///
/// # Errors
///
/// [`ProductDimension::DatasetIdentity`] if canonicalization refuses the shapes
/// dataset (an exhausted call budget on a pathologically symmetric blank-node
/// graph, or an IRI in the canonicalization profile's reserved namespace), so the
/// section's identity claim cannot be established. Every other container failure
/// is mapped by [`refuse`].
pub(crate) fn encode_dataset(dataset: &Arc<RdfDataset>) -> Result<Vec<u8>, ShapesProductError> {
    PackBuilder::build_bytes(dataset.as_ref()).map_err(|error| refuse(&error))
}

/// Restore the shapes dataset from the product's dataset section — the COMMON
/// path, taken on every product load.
///
/// Runs the container's cheap integrity tier ONLY: magic, format version, every
/// section's SHA-256 against its stored directory digest, each section reader's
/// own structural validation, and the header's recomputed capability/term-count
/// cross-check. It does NOT re-canonicalize the restored dataset — see the
/// [module documentation](self) for why that separation is load-bearing rather
/// than an optimization, and [`certify_dataset`] for the path that does.
///
/// Every RDF 1.2 component the section carries is restored: base quads, reifier
/// bindings, and statement annotations, with blank-node `(label, scope)` identity
/// unchanged.
///
/// # Errors
///
/// [`ProductDimension::Magic`], [`ProductDimension::FormatVersion`],
/// [`ProductDimension::Truncated`], [`ProductDimension::SectionDigest`] or
/// [`ProductDimension::Malformed`], per [`refuse`]. The two identity dimensions
/// are not reachable from here, because nothing on this path recomputes an
/// identity.
pub(crate) fn open_dataset(bytes: &[u8]) -> Result<Arc<RdfDataset>, ShapesProductError> {
    restore_pack(bytes).map_err(|error| refuse(&error))
}

/// Certify the product's dataset section — the COLD path, for tests, conformance
/// harnesses and a CLI verify subcommand. **Never call this from a restore.**
///
/// Everything [`open_dataset`] checks, and then an INDEPENDENT recompute: the
/// section's own decoded contents are replayed into a fresh dataset builder, that
/// reconstruction is canonicalized, and its digest is compared against the
/// canonical identity the section stores. Only a section that passes both is a
/// certified projection of the shapes graph it claims to carry — the stored
/// identity sits outside every section's SHA-256 coverage, so a tampered identity
/// is caught HERE and nowhere else.
///
/// The returned [`PackDigest`] is a verified `purrdf-rdfc12` SHA-256 digest, not
/// an RDFC-1.0 one; it is a distinct type from a bare `[u8; 32]` precisely so an
/// uncorroborated digest read straight off the section header cannot be confused
/// with one this function has certified.
///
/// # Errors
///
/// Everything [`open_dataset`] can return, plus
/// [`ProductDimension::DatasetIdentity`] when canonicalization refuses the
/// reconstruction or when the recomputed digest disagrees with the stored one.
pub(crate) fn certify_dataset(bytes: &[u8]) -> Result<PackDigest, ShapesProductError> {
    verify_pack(bytes).map_err(|error| refuse(&error))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Tests for the shapes-dataset section codec.
///
/// Deliberately function-only: the product-model census scans every
/// `crates/shapes/src/**/*.rs` file, and a `struct` or `type` alias declared here
/// would be read as part of the model it is testing.
///
/// Every refusal below is executed alongside a NEIGHBOURING VALID case. A refusal
/// is a claim, and an over-refusal — rejecting a section that is actually fine —
/// hides perfectly: every test still passes and the strictness looks correct,
/// right up until a user loads the product that should restore and doesn't.
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ::purrdf::{RdfDataset, pack_digest, try_canonicalize};

    use super::{ProductDimension, certify_dataset, encode_dataset, open_dataset};
    use crate::product::ast::encode_ast;
    use crate::shapes::Shapes;

    /// The pack container header's `rdfc_digest` field: 32 bytes at offset 32.
    ///
    /// Hard-coded from the container's documented fixed layout because the tamper
    /// below has to hit that field and NOTHING else — every section digest must
    /// stay intact for `open_does_not_certify` to mean what it claims. The tests
    /// that use it assert the window really is the stored digest before touching
    /// it, so a layout change fails loudly here instead of silently defanging the
    /// tamper.
    const RDFC_DIGEST_OFFSET: usize = 32;

    // ── Fixtures (example.org, per the repository's fixture rule) ────────────────

    /// A plain shapes graph: one node shape with a blank-node-identified property
    /// shape.
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
            sh:property ex:AgeShape .

        ex:AgeShape a sh:PropertyShape ;
            sh:path ex:age ;
            sh:maxCount 1 .
    ";

    /// A shapes graph carrying the RDF 1.2 term kinds whose identity must survive
    /// the section: a quoted triple inside a shape (`sh:hasValue`), directional
    /// language literals in BOTH directions (two distinct terms that differ only
    /// by base direction), a language tag with subtags, a reifier binding, a
    /// statement annotation, and a blank-node-identified property shape.
    const RDF12_SHAPES: &str = r#"
        @prefix ex: <https://example.org/> .
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

        ex:ThingShape a sh:NodeShape ;
            sh:targetClass ex:Thing ;
            sh:name "colour"@en-GB-oxendict ;
            sh:property [
                sh:path ex:cites ;
                sh:hasValue <<( ex:a ex:p ex:b )>> ;
                sh:minCount 1 ;
            ] .

        ex:meta ex:label "direction"@ar--ltr , "direction"@ar--rtl ;
            ex:tag "traditional"@de-CH-1901 .

        ex:reifier rdf:reifies <<( ex:a ex:p ex:b )>> .

        ex:s ex:p ex:o {| ex:note "annotated" |} .
    "#;

    /// A shapes graph whose SHACL-SPARQL constraint reads the shapes graph itself
    /// through `$shapesGraph`, so the restored dataset is observable in the rows
    /// the constraint produces rather than only in the bytes.
    const SHAPES_GRAPH_QUERY_SHAPES: &str = r#"
        @prefix ex: <https://example.org/> .
        @prefix sh: <http://www.w3.org/ns/shacl#> .

        ex:Shape a sh:NodeShape ;
            sh:targetNode ex:Node ;
            ex:prop 42 ;
            sh:sparql ex:Constraint .

        ex:Constraint sh:select """
            SELECT $this
            WHERE {
                FILTER bound($shapesGraph)
                GRAPH $shapesGraph {
                    FILTER bound($currentShape)
                    $currentShape ex:prop 42 .
                }
            }
        """ .
    "#;

    /// The IRI the shapes graph is exposed under to SHACL-SPARQL.
    const SHAPES_GRAPH_IRI: &str = "https://example.org/shapes";

    /// Parse Turtle into a frozen dataset.
    fn dataset_of(ttl: &str) -> Arc<RdfDataset> {
        crate::text_ingest::parse_turtle_to_dataset(ttl, None).expect("fixture Turtle parses")
    }

    /// Parse a shapes graph from an already-frozen dataset under `ttl`'s own
    /// document configuration: its `@prefix` map (SHACL-AF `sh:select` bodies
    /// resolve prefixed names against it) and the shapes-graph IRI.
    ///
    /// The configuration is read from the SOURCE TEXT, so it is identical on both
    /// sides of a round trip by construction — which is what makes the restored
    /// dataset the only variable in `pack_roundtrip_preserves_from_dataset_output`.
    fn shapes_from(dataset: &Arc<RdfDataset>, ttl: &str) -> Shapes {
        crate::shapes::from_dataset_with_config_and_graph(
            dataset,
            &crate::text_ingest::extract_prefixes(ttl),
            None,
            Some(SHAPES_GRAPH_IRI.to_owned()),
        )
        .expect("fixture shapes parse")
    }

    /// Parse a shapes graph straight from Turtle, keeping the retained dataset.
    fn shapes_of(ttl: &str) -> Shapes {
        shapes_from(&dataset_of(ttl), ttl)
    }

    /// The canonical N-Quads of a dataset — the value-level witness of equality
    /// the RDF types themselves do not provide.
    fn canonical(dataset: &RdfDataset) -> String {
        try_canonicalize(dataset)
            .expect("fixture datasets canonicalize")
            .nquads
    }

    // ── Round trip ──────────────────────────────────────────────────────────────

    #[test]
    fn dataset_section_round_trips() {
        let source = dataset_of(PLAIN_SHAPES);
        let bytes = encode_dataset(&source).expect("encode");
        let restored = open_dataset(&bytes).expect("open");

        assert_eq!(
            restored.quad_count(),
            source.quad_count(),
            "the section must restore every quad it carried",
        );
        assert_eq!(
            canonical(restored.as_ref()),
            canonical(source.as_ref()),
            "the restored dataset must be the same RDF value as the source",
        );

        // Determinism: the same dataset writes the same bytes, and the restored
        // dataset writes them too.
        assert_eq!(
            bytes,
            encode_dataset(&source).expect("re-encode"),
            "the section must be byte-deterministic",
        );
        assert_eq!(
            bytes,
            encode_dataset(&restored).expect("encode the restored dataset"),
            "re-encoding the restored dataset must reproduce the section bytes",
        );
    }

    #[test]
    fn empty_shapes_dataset_round_trips() {
        // The neighbouring valid case for every "these bytes are not a section"
        // refusal: a shapes graph with no statements at all is still a section
        // this codec must accept, not an edge the decoder may refuse.
        let source = Shapes::default().dataset().clone();
        let bytes = encode_dataset(&source).expect("encode the empty dataset");
        let restored = open_dataset(&bytes).expect("open the empty dataset");

        assert_eq!(restored.quad_count(), 0);
        assert_eq!(canonical(restored.as_ref()), canonical(source.as_ref()));
        certify_dataset(&bytes).expect("an empty section certifies");
    }

    // ── Certification ───────────────────────────────────────────────────────────

    #[test]
    fn certify_returns_pack_digest_equal_to_source() {
        let source = dataset_of(PLAIN_SHAPES);
        let bytes = encode_dataset(&source).expect("encode");

        let certified = certify_dataset(&bytes).expect("certify");
        let stored = pack_digest(&bytes).expect("read the stored identity");
        assert_eq!(
            certified.as_bytes(),
            &stored,
            "certification must return the identity it corroborated, not a new one",
        );

        // The identity is a function of the dataset's VALUE, so it survives a
        // round trip: re-encoding the restored dataset certifies to the same digest.
        let restored = open_dataset(&bytes).expect("open");
        let reencoded = encode_dataset(&restored).expect("re-encode");
        let recertified = certify_dataset(&reencoded).expect("re-certify");
        assert_eq!(
            certified.as_bytes(),
            recertified.as_bytes(),
            "the source dataset's canonical identity must survive the section",
        );

        // A DIFFERENT shapes graph must certify to a different identity — without
        // this the assertion above would hold for a constant.
        let other = encode_dataset(&dataset_of(RDF12_SHAPES)).expect("encode the other fixture");
        assert_ne!(
            certified.as_bytes(),
            certify_dataset(&other)
                .expect("certify the other fixture")
                .as_bytes(),
            "distinct shapes graphs must not share one certified identity",
        );
    }

    // ── The two-tier rule ───────────────────────────────────────────────────────

    #[test]
    fn open_does_not_certify() {
        let source = dataset_of(PLAIN_SHAPES);
        let bytes = encode_dataset(&source).expect("encode");

        // The tamper target really is the stored canonical identity. If the
        // container's layout moves, this assertion fails rather than the test
        // silently tampering with an unrelated byte and proving nothing.
        let stored = pack_digest(&bytes).expect("read the stored identity");
        assert_eq!(
            &bytes[RDFC_DIGEST_OFFSET..RDFC_DIGEST_OFFSET + 32],
            &stored[..],
            "the header's stored identity must sit where this test tampers",
        );

        let mut tampered = bytes.clone();
        tampered[RDFC_DIGEST_OFFSET] ^= 0xff;

        // TIER 1 — the common path still opens. The identity field is outside
        // every section's SHA-256 coverage, so nothing the cheap tier checks moved.
        let restored = open_dataset(&tampered).expect("a tampered identity must still OPEN");
        assert_eq!(
            canonical(restored.as_ref()),
            canonical(source.as_ref()),
            "the tamper touched an identity claim, not the RDF",
        );

        // TIER 2 — certification refuses, because only it recomputes.
        let refusal = certify_dataset(&tampered).expect_err("a tampered identity must NOT certify");
        assert_eq!(
            refusal.dimension(),
            ProductDimension::DatasetIdentity,
            "a forged identity is an identity refusal, got {refusal}",
        );

        // The neighbouring valid case: the untampered section certifies. Without
        // it, a `certify_dataset` that refused everything would pass the assertion
        // above — over-refusal is the mirror of a silent drop and hides just as well.
        certify_dataset(&bytes).expect("the untampered section must certify");
    }

    // ── Refusals, each with its neighbouring valid case ──────────────────────────

    #[test]
    fn refusals_name_their_dimension_and_neighbours_still_open() {
        let bytes = encode_dataset(&dataset_of(PLAIN_SHAPES)).expect("encode");

        // Not this format at all.
        let mut bad_magic = bytes.clone();
        bad_magic[0] ^= 0xff;
        assert_eq!(
            open_dataset(&bad_magic)
                .expect_err("a broken magic must refuse")
                .dimension(),
            ProductDimension::Magic,
        );

        // Ends before the structures it declared are complete.
        let truncated = &bytes[..bytes.len() / 2];
        let refusal = open_dataset(truncated).expect_err("a truncated section must refuse");
        assert_eq!(refusal.dimension(), ProductDimension::Truncated);

        // Corrupt in place: a section body byte flipped, so its stored SHA-256 no
        // longer matches. The last byte of the buffer is inside the final section.
        let mut corrupt = bytes.clone();
        let last = corrupt.len() - 1;
        corrupt[last] ^= 0xff;
        let refusal = open_dataset(&corrupt).expect_err("a corrupt section must refuse");
        assert_eq!(refusal.dimension(), ProductDimension::SectionDigest);

        // An empty buffer is not this format either.
        assert!(open_dataset(&[]).is_err(), "empty bytes are not a section");

        // THE NEIGHBOURING VALID CASE — the untouched bytes still open and still
        // certify. Each refusal above is one byte away from this.
        let restored = open_dataset(&bytes).expect("the untouched section must open");
        assert!(restored.quad_count() > 0);
        certify_dataset(&bytes).expect("the untouched section must certify");

        // Every refusal renders its own dimension label, so a caller branching on
        // the dimension is branching on a discriminant rather than on prose.
        let refusal = open_dataset(&bad_magic).expect_err("still refuses");
        assert!(
            refusal.to_string().starts_with("magic: "),
            "a refusal must lead with its dimension label, got {refusal}",
        );
        assert!(
            refusal.message().contains("re-prepare the product"),
            "a refusal must name the fix, got {refusal}",
        );
    }

    // ── The important one: re-interning must not move the AST ────────────────────

    #[test]
    fn pack_roundtrip_preserves_from_dataset_output() {
        // `Shape`, `Constraint`, `Path` and `NodeExpr` deliberately do not
        // implement equality, so the AST codec IS the equality relation here: two
        // parsed shapes graphs are the same exactly when they encode alike.
        //
        // This is the one place re-interning could legitimately diverge. Opening a
        // section rebuilds every term in a fresh dataset builder, which reassigns
        // `TermId`s; if any part of `from_dataset`'s shape walk were ordered by id
        // rather than by value, the restored shapes would encode differently while
        // every other test in this file still passed.
        for (name, ttl) in [
            ("plain", PLAIN_SHAPES),
            ("rdf-1.2", RDF12_SHAPES),
            ("shapes-graph query", SHAPES_GRAPH_QUERY_SHAPES),
        ] {
            let direct = shapes_of(ttl);
            let direct_bytes = encode_ast(&direct).expect("encode the directly parsed shapes");

            let section = encode_dataset(direct.dataset()).expect("encode the dataset section");
            let restored = open_dataset(&section).expect("open the dataset section");
            let reparsed = shapes_from(&restored, ttl);
            let reparsed_bytes = encode_ast(&reparsed).expect("encode the re-parsed shapes");

            assert_eq!(
                direct_bytes, reparsed_bytes,
                "{name}: parse -> encode_dataset -> open_dataset -> from_dataset must produce \
                 byte-identical AST bytes to parsing directly",
            );
        }
    }

    // ── RDF 1.2 term identity ───────────────────────────────────────────────────

    #[test]
    fn rdf_12_term_identity_survives_the_section() {
        let source = dataset_of(RDF12_SHAPES);

        // The fixture really does carry every term kind under test — otherwise
        // the round-trip assertion below would be vacuous. Language tags are
        // case-normalized to lowercase when a term is interned (BCP 47 tags are
        // case-insensitive), so the SUBTAG SEQUENCE is what this checks, not the
        // authored casing.
        let source_nquads = canonical(source.as_ref());
        for needle in [
            "\"direction\"@ar--ltr",
            "\"direction\"@ar--rtl",
            "@en-gb-oxendict",
            "@de-ch-1901",
            "<<(",
            "_:",
        ] {
            assert!(
                source_nquads.contains(needle),
                "the fixture must carry {needle} for this test to mean anything",
            );
        }

        let bytes = encode_dataset(&source).expect("encode");
        let restored = open_dataset(&bytes).expect("open");

        assert_eq!(
            canonical(restored.as_ref()),
            source_nquads,
            "every RDF 1.2 term kind must survive the section unchanged",
        );

        // Base direction is part of a literal's IDENTITY: `"x"@ar--ltr` and
        // `"x"@ar--rtl` are TWO terms. A dictionary that keyed on lexical form plus
        // language would merge them into one and lose a direction silently, so the
        // two spellings are asserted independently rather than trusted to the
        // whole-dataset comparison above.
        let restored_nquads = canonical(restored.as_ref());
        assert!(restored_nquads.contains("\"direction\"@ar--ltr"));
        assert!(restored_nquads.contains("\"direction\"@ar--rtl"));

        // The RDF 1.2 side-tables are a distinct structural component from the base
        // quads: a section that replayed only the quads would restore an equal-looking
        // dataset with empty side-tables.
        assert!(
            source.reifier_quads().count() > 0 && source.annotation_quads().count() > 0,
            "the fixture must carry side-table rows for this test to mean anything",
        );
        assert_eq!(
            restored.reifier_quads().count(),
            source.reifier_quads().count(),
            "reifier bindings must survive the section",
        );
        assert_eq!(
            restored.annotation_quads().count(),
            source.annotation_quads().count(),
            "statement annotations must survive the section",
        );

        // And the shapes still parse out of the restored dataset — a term identity
        // that survived the bytes but broke the parse would be no use.
        let reparsed = shapes_from(&restored, RDF12_SHAPES);
        assert_eq!(
            encode_ast(&reparsed).expect("encode the re-parsed shapes"),
            encode_ast(&shapes_of(RDF12_SHAPES)).expect("encode the directly parsed shapes"),
        );
    }

    // ── The named shapes graph stays queryable ──────────────────────────────────

    #[test]
    fn restored_shapes_graph_is_queryable() {
        let source = dataset_of(SHAPES_GRAPH_QUERY_SHAPES);
        let prefixes = crate::text_ingest::extract_prefixes(SHAPES_GRAPH_QUERY_SHAPES);

        let before = crate::shapes::from_dataset_with_config_and_graph(
            &source,
            &prefixes,
            None,
            Some(SHAPES_GRAPH_IRI.to_owned()),
        )
        .expect("parse the shapes");

        let bytes = encode_dataset(&source).expect("encode");
        let restored = open_dataset(&bytes).expect("open");
        let after = crate::shapes::from_dataset_with_config_and_graph(
            &restored,
            &prefixes,
            None,
            Some(SHAPES_GRAPH_IRI.to_owned()),
        )
        .expect("parse the restored shapes");

        let data = dataset_of(
            "@prefix ex: <https://example.org/> .\n\
             ex:Node ex:p ex:o .\n",
        );

        let report_before =
            crate::engine::validate_dataset_with_shapes_graph(data.as_ref(), &before, None)
                .expect("validate against the source shapes");
        let report_after =
            crate::engine::validate_dataset_with_shapes_graph(data.as_ref(), &after, None)
                .expect("validate against the restored shapes");

        // The constraint reads the shapes graph, so it can only fire when the
        // named graph carries the source's rows. A restore that dropped the
        // dataset would conform here — silently, with a well-formed report.
        assert!(
            !report_before.conforms,
            "the fixture constraint must fire before the round trip",
        );
        assert_eq!(report_before.results.len(), 1);

        assert_eq!(
            report_after.conforms, report_before.conforms,
            "the restored shapes graph must answer identically",
        );
        assert_eq!(
            report_after.results.len(),
            report_before.results.len(),
            "the restored shapes graph must produce the same rows",
        );
        for (after_result, before_result) in report_after.results.iter().zip(&report_before.results)
        {
            assert_eq!(after_result.focus_node, before_result.focus_node);
            assert_eq!(after_result.source_shape, before_result.source_shape);
            assert_eq!(after_result.severity, before_result.severity);
        }
    }
}
