// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The declarative-AST codec: the byte form of a parsed SHACL shapes graph.
//!
//! [`encode_ast`] writes the declarative half of a [`Shapes`] — the node shapes,
//! the `sh:SPARQLTargetType` declarations, the box-role vocabulary, and the
//! custom node-expression function declarations — as one self-contained byte
//! string, and [`decode_ast`] reads that string back into [`AstParts`].
//! `AstParts` is deliberately NOT a `Shapes`: assembling one, re-establishing the
//! shared shape index and reattaching the retained dataset are a later stage's
//! work, and a codec that returned a half-built `Shapes` would be handing out a
//! value whose invariants it had not established.
//!
//! # Why hand-rolled, and what the rejected alternative cost
//!
//! The obvious alternative is a derived serializer — `serde` plus a
//! self-describing container such as CBOR. It was rejected, and not for
//! dependency hygiene. A derived codec is *total by construction*: it encodes
//! whatever the type happens to hold, which is exactly wrong for this model,
//! because four of its fields must NOT be carried. `Constraint::Pattern`'s
//! `compiled` is a lazy cache; `Constraint::NodeByExpression`'s and
//! `ShapeArg::Computed`'s `shapes` is one handle SHARED by every constraint of a
//! shapes graph; `CustomFunction`'s body graph is **cyclic by design** (§6.1/§6.2
//! hold a body by reference precisely so it can call its own function). A derived
//! tree walk over that last one does not terminate — it is not a slow encode, it
//! is a stack exhaustion — and a derived walk over the first three either invents
//! a per-value default where a shared handle belongs or serializes a compiled
//! regex nobody asked for. Every one of those decisions is a judgement about
//! MEANING, and the only place to state a judgement is code that was written to
//! state it. So each field is written by hand, and the table below records what
//! happens to every one of them.
//!
//! # The canonical form, and the equality the types lack
//!
//! [`Shape`], [`Constraint`], [`Path`] and [`NodeExpr`] deliberately do not
//! implement `PartialEq` — `expression.rs` says so in as many words, because they
//! embed a compiled regex and a shared `OnceLock`. So a round-trip through this
//! codec cannot be witnessed structurally: there is no `==` to assert. The
//! witness is therefore the BYTES, `encode(decode(b)) == b`, and that is not a
//! weaker test standing in for a stronger one. **This codec supplies the
//! canonical form, and hence the equality relation the types themselves lack**:
//! two values are equal exactly when they encode alike, and every hash-ordered
//! container is written key-sorted so that relation is a function of value
//! content and never of iteration order.
//!
//! # Bounded recursion is a security property
//!
//! `Shape`, `Constraint`, `Path`, `Term` and `NodeExpr` are mutually recursive
//! through `Box`/`Vec`, so a hostile product can nest them arbitrarily. Native
//! stack exhaustion is an **abort, not an error any caller can handle** — the
//! same failure shape `crate::engine`'s class-planning walk names for the cyclic
//! function graph. Both directions therefore carry an explicit depth counter and
//! refuse past [`MAX_DEPTH`] with [`ProductDimension::DepthLimit`].
//!
//! [`MAX_DEPTH`] is 128: twice [`crate::expression::MAX_RECURSION_DEPTH`], the
//! ceiling the evaluator itself enforces. The factor is the point. A ceiling
//! BELOW the evaluator's would refuse products the evaluator could have run,
//! which is over-refusal — the mirror of a silent drop, and the harder of the two
//! to notice, because a refusal looks like correct strictness right up until a
//! user writes the shapes graph that should load and doesn't. Authored SHACL
//! nests in single digits; 128 is a stack guard, not a second semantic limit.
//!
//! # No format version byte
//!
//! This stream carries no version counter of its own, deliberately. The census
//! gate (`crates/shapes/tests/product_model_census.rs`) already derives a *stage
//! id* from the model's own content, and its whole argument is that a
//! hand-incremented version is how an authenticated cache serves
//! stale-but-verified wrong answers: the bytes verify, the counter matches, and
//! the meaning moved underneath both. A second, hand-maintained counter here
//! would reintroduce exactly that. The container framing this section carries the
//! format version it needs.
//!
//! # Byte layout
//!
//! Every multi-byte integer is a little-endian LEB128 varint via
//! [`purrdf_core::ir::pack::bits`]; `f64` is the only fixed-width field and is
//! written as 8 little-endian bytes. Readers copy into an explicit `[u8; 8]`
//! before `from_le_bytes`, so the caller's buffer need not be aligned.
//!
//! ## Primitives
//!
//! | Form | Bytes |
//! |------|-------|
//! | `uint` | LEB128 varint, 1..=10 bytes |
//! | `bool` | one byte, `0` or `1`; any other value is [`Malformed`] |
//! | `str` | `uint` byte length, then that many UTF-8 bytes |
//! | `opt<X>` | one byte `0` (absent), or `1` followed by `X` |
//! | `seq<X>` | `uint` element count, then that many `X` |
//! | `tag` | one byte selecting a variant; an unknown value is [`UnsupportedCapability`] |
//! | `f64` | 8 bytes, `f64::to_bits().to_le_bytes()`, non-finite refused |
//!
//! ## Stream
//!
//! | # | Field | Form | Meaning |
//! |---|-------|------|---------|
//! | 1 | `fn_decls` | `seq<decl>` | the custom node-expression function declarations, **sorted by IRI** |
//! | 2 | `fn_bodies` | `opt<node_expr>` × `fn_decls` count | each declaration's `sh:bodyExpression`, in the same order |
//! | 3 | `node_shapes` | `seq<shape>` | the shapes graph's top-level node shapes |
//! | 4 | `box_role_vocab` | `opt<box_role_vocab>` | absent = the box-role feature is inactive |
//! | 5 | `target_types` | `seq<(str, sparql_target_type)>` | `sh:SPARQLTargetType` declarations, **key-sorted** |
//! | 6 | `shapes_graph` | `opt<str>` | the named-graph IRI the shapes dataset is exposed under |
//! | 7 | `class_catalog` | `seq<(str, uint)>` | the REUSABLE ANALYSIS: every planned class IRI and its binding-row position, **sorted by IRI** |
//!
//! Any byte after field 7 is [`Malformed`]: trailing bytes are a structure the
//! writer did not produce, and skipping them is how a silent truncation passes
//! for a successful load.
//!
//! The declaration table is split from the bodies because the function graph is
//! cyclic. Reading it is two passes for the same reason writing it is: every
//! declaration is materialized first, so a body that calls its own function — or
//! any other — resolves to an ALREADY EXISTING handle by index rather than
//! recursing into a definition that reaches back.
//!
//! ## `decl`
//!
//! | Field | Form |
//! |-------|------|
//! | `iri` | `str` |
//! | `kind` | `tag` — `0` = `sh:ListParameterExpressionFunction`, `1` = `sh:NamedParameterExpressionFunction` |
//! | `params` | `seq<arg_key>` |
//! | `required` | `uint` |
//!
//! ## Tag spaces
//!
//! Each tagged type's tags are `0..n` in the type's own DECLARATION order, so the
//! census's record of variant order is load-bearing: reordering arms in the model
//! remaps the bytes, and that is what moves the stage id. The counts are pinned
//! as `TAGS_*` constants next to the decoders that honour them.
//!
//! | Type | Tags |
//! |------|------|
//! | [`Term`] | `NamedNode` `BlankNode` `Literal` `Triple` |
//! | [`Severity`] | `Violation` `Warning` `Info` `Other` |
//! | [`NodeKindValue`] | `Iri` `BlankNode` `Literal` `BlankNodeOrIri` `BlankNodeOrLiteral` `IriOrLiteral` |
//! | [`Path`] | `Predicate` `Inverse` `Sequence` `Alternative` `ZeroOrMore` `OneOrMore` `ZeroOrOne` |
//! | [`Target`] | `Class` `SubjectsOf` `ObjectsOf` `Node` `ImplicitClass` `Sparql` |
//! | [`ComponentValidator`] | `Ask` `Select` |
//! | [`Constraint`] | 31 arms, `Class` … `Component` |
//! | [`NodeExpr`] | 32 arms, `Constant` … `Select` |
//! | [`ShapeArg`] | `Named` `Computed` |
//! | [`ArgKey`] | `Index` `Named` |
//! | [`CustomFnKind`] | `ListParameter` `NamedParameter` |
//! | [`FnCall`] | `Builtin` `UserDefined` `Sparql` |
//! | [`RuleBody`] | `Triple` `Sparql` |
//! | [`RuleSchedule`] | `Once` `General` |
//! | `NodeExpr::Select::key` | `sh:select` `sh:sparqlExpr` |
//! | `Literal::direction` | `Ltr` `Rtl` |
//!
//! # What is NOT written, and why
//!
//! Every omission below is a judgement, and an unexplained one is
//! indistinguishable from the silent drop this codec exists to avoid — so each
//! carries its reason here.
//!
//! * `Constraint::Pattern::compiled` — a pure lazy cache of `regex` + `flags`,
//!   both of which ARE written. Carrying a compiled automaton would pin one
//!   regex-engine version into a cached artifact for no gain.
//! * `Constraint::NodeByExpression::shapes`, `ShapeArg::Computed::shapes` — ONE
//!   `Arc<OnceLock<..>>` shared by every such constraint of a shapes graph.
//!   [`decode_ast`] mints a single empty handle and clones it into every site
//!   ([`AstParts::shape_index`] hands it to the stage that fills it). Defaulting
//!   it per constraint would type-check and would quietly give each constraint a
//!   private, permanently empty index.
//! * `FnCall::Sparql::expr` — re-lowered at decode from the call's own
//!   `sparql:<NAME>` local name through [`crate::expression::sparql_ns_lowering`],
//!   which is the seam over `purrdf_sparql_algebra::builtin_function_keyword`.
//!   That table is the source of truth for what the name MEANS; storing the
//!   rendered text would let a product keep claiming the old rendering after the
//!   table moved. A name it cannot resolve is [`UnsupportedCapability`].
//! * `Shapes::functions`, `Shapes::aggregates` — caller-supplied registries, not
//!   parse output. They are admission dimensions
//!   ([`ProductDimension::FunctionRegistry`], [`ProductDimension::AggregateRegistry`]),
//!   checked by the container rather than carried here.
//! * `Shapes::shapes_dataset`, `Shapes::parse_provenance` — the retained RDF and
//!   the recorded parse inputs. Both belong to other sections of the product.
//!
//! # Why field 7 is here and not in a section of its own
//!
//! The class catalog is ANALYSIS, not model, so a fourth container section would
//! describe it better. It cannot have one. The section directory is TOTAL — the
//! declared section count is part of the [`ArtifactSpec`], checked before a product
//! opens at all — so adding a fourth kind makes every product ever written fail to
//! OPEN rather than merely fail to admit, and the forward-compatibility seam this
//! codec is built around (`admit` refuses an unknown stage, `rebuild` rescues it
//! from the carried dataset) never gets the chance to run. Extending this stream
//! instead leaves the directory and the container format version exactly where they
//! are, so an older product stays rescuable.
//!
//! That extension is safe for the same reason this stream carries no version byte of
//! its own: the section has exactly ONE reader, and that reader is gated.
//! [`super::ShapesProductView::admit`] verifies the stage id before it decodes
//! anything here, [`super::ShapesProductView::rebuild`] never touches this section
//! at all, and carrying the analysis MOVES the stage id, because the class walk
//! behind field 7 is digested into it. So no product written before field 7 existed
//! is ever handed to a decoder that expects it: such a product is refused on
//! `stage-id` and rebuilt.
//!
//! Nothing here touches the filesystem, a clock, a thread or a source of
//! randomness: `wasm32-unknown-unknown`-clean by construction.
//!
//! [`Malformed`]: ProductDimension::Malformed
//! [`UnsupportedCapability`]: ProductDimension::UnsupportedCapability
//! [`ArtifactSpec`]: purrdf_core::artifact::ArtifactSpec

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use ::purrdf::{FastMap, RdfTextDirection};
use purrdf_core::ir::pack::bits::{PackBitsError, read_varint, write_varint};

use crate::expression::{
    ArgKey, CustomFnKind, CustomFunction, FnCall, NodeExpr, ShapeArg, sparql_ns_lowering,
};
use crate::model::{BoxRoleVocab, sparql_ns};
use crate::plan::ClassCatalog;
use crate::report::Severity;
use crate::rules::{OrderKey, Rule, RuleBody, RuleSchedule};
use crate::shapes::{
    ComponentValidator, Constraint, NodeKindValue, Path, PropertyShape, Shape, Shapes,
    SparqlTargetType, Target, TargetTypeParam,
};
use crate::term::{Literal, NamedNode, Term, Triple};

use super::error::{ProductDimension, ShapesProductError};

// ---------------------------------------------------------------------------
// Limits and tag spaces
// ---------------------------------------------------------------------------

/// The structural nesting ceiling both directions enforce.
///
/// Twice [`crate::expression::MAX_RECURSION_DEPTH`] on purpose — see the module
/// documentation on why a ceiling below the evaluator's would be over-refusal
/// rather than strictness.
pub(crate) const MAX_DEPTH: u32 = 128;

/// How many elements a declared sequence count may pre-allocate *speculatively*.
///
/// [`AstReader::count`] already refuses a count larger than the bytes that follow
/// it, and that bound is necessary — without it a two-byte varint could name four
/// billion elements and the reader would try to reserve room for all of them
/// before discovering the stream ends. It is **not sufficient**, because it bounds
/// the count in ELEMENTS by a number of BYTES, and the two are not the same unit.
/// A sequence element's minimum *encoding* is one byte; its minimum *footprint* is
/// `size_of` the decoded value, and for the widest element in this format
/// (`Shape`) that is well over a hundred bytes. So a well-formed 1 MiB artifact
/// may honestly declare a one-million-element sequence, and an unclamped
/// `Vec::with_capacity` would answer that declaration with a multi-hundred-megabyte
/// reservation — from an artifact a hundredth that size — before a single element
/// is read and found to be a lie. That is amplification, and it is exactly the
/// shape of denial of service a memo format must not hand an attacker.
///
/// The clamp separates the two jobs a capacity does. Pre-sizing exists to spare a
/// HONEST sequence its geometric doubling, and the sequences an authored shapes
/// graph produces are small — a shape's constraint list, a path's alternatives, a
/// call's arguments, all single- to low-double digits, with the top-level node
/// shape list the only one that plausibly reaches thousands. 4096 clears every one
/// of those with orders of magnitude to spare, so no real product is ever clamped
/// and no real product pays a reallocation this constant could have avoided.
/// Past it the `Vec` still grows to whatever the stream genuinely contains — the
/// clamp caps the SPECULATION, never the capacity, so a sequence that really does
/// hold a million elements still decodes, paying the doubling that a count nobody
/// has corroborated yet does not get to skip.
const MAX_SPECULATIVE_ELEMENTS: usize = 4096;

/// The capacity a declared element `count` is allowed to reserve before a single
/// element has been read — `count` itself, clamped by [`MAX_SPECULATIVE_ELEMENTS`].
///
/// The ONE place a sequence length turns into memory, so the clamp is stated once
/// and every sequence in the format inherits it.
const fn speculative_capacity(count: usize) -> usize {
    if count < MAX_SPECULATIVE_ELEMENTS {
        count
    } else {
        MAX_SPECULATIVE_ELEMENTS
    }
}

/// The capacity [`AstWriter`] starts its output buffer at.
///
/// Unlike [`MAX_SPECULATIVE_ELEMENTS`] this guards nothing — the writer's input is
/// a `Shapes` this process already holds, so there is no hostile length to defend
/// against, only a buffer that would otherwise start at zero and double its way up
/// to the section's size, copying everything written so far on each step. Nothing
/// in the format decides the number: it is one allocator page, below the section
/// any shapes graph with a handful of shapes in it produces.
///
/// **It is a trade, not a free win, and the bench prints both halves.** Measured
/// over `benches/support/product.rs`, whose AST section is 8,733 bytes: growing
/// from zero costs 10 allocations and 20,959 bytes of allocator traffic; starting
/// at one page costs 3 allocations and 28,672. Seven fewer trips through the
/// allocator, and roughly half the bytes memcpy'd between buffers, in exchange for
/// reserving room the smallest graphs will not fill. The count is what the restore
/// path is judged on, so the count is what this optimizes; a reader who later
/// decides transient traffic matters more should move this number, not the code
/// around it.
///
/// A writer that began at any other capacity emits identical bytes, which is what
/// the determinism fixture's pinned length asserts.
const WRITE_BUFFER_HINT: usize = 4096;

/// The number of [`Term`] tags.
const TAGS_TERM: u8 = 4;
/// The number of [`Severity`] tags.
const TAGS_SEVERITY: u8 = 4;
/// The number of [`NodeKindValue`] tags.
const TAGS_NODE_KIND: u8 = 7;
/// The number of [`Path`] tags.
const TAGS_PATH: u8 = 7;
/// The number of [`Target`] tags.
const TAGS_TARGET: u8 = 6;
/// The number of [`ComponentValidator`] tags.
const TAGS_COMPONENT_VALIDATOR: u8 = 2;
/// The number of [`Constraint`] tags.
const TAGS_CONSTRAINT: u8 = 40;
/// The number of [`NodeExpr`] tags.
const TAGS_NODE_EXPR: u8 = 32;
/// The number of [`ShapeArg`] tags.
const TAGS_SHAPE_ARG: u8 = 2;
/// The number of [`ArgKey`] tags.
const TAGS_ARG_KEY: u8 = 2;
/// The number of [`CustomFnKind`] tags.
const TAGS_CUSTOM_FN_KIND: u8 = 2;
/// The number of [`FnCall`] tags.
const TAGS_FN_CALL: u8 = 3;
/// The number of [`RuleBody`] tags.
const TAGS_RULE_BODY: u8 = 2;
/// The number of [`RuleSchedule`] tags.
const TAGS_RULE_SCHEDULE: u8 = 2;
/// The number of `NodeExpr::Select::key` tags.
const TAGS_SELECT_KEY: u8 = 2;
/// The number of `Literal::direction` tags.
const TAGS_DIRECTION: u8 = 2;

/// The `sh:select` spelling of a SPARQL-based node expression.
const SELECT_KEY_SELECT: &str = "sh:select";
/// The `sh:sparqlExpr` spelling of a SPARQL-based node expression.
const SELECT_KEY_SPARQL_EXPR: &str = "sh:sparqlExpr";

// ---------------------------------------------------------------------------
// Refusals
// ---------------------------------------------------------------------------

/// Refuse: the bytes are structurally invalid.
fn malformed(message: impl Into<String>) -> ShapesProductError {
    ShapesProductError::new(ProductDimension::Malformed, message)
}

/// Refuse: the bytes ask for something this build does not implement.
fn unsupported(message: impl Into<String>) -> ShapesProductError {
    ShapesProductError::new(ProductDimension::UnsupportedCapability, message)
}

/// Refuse: an unknown variant tag for `type_name`.
fn unknown_tag(type_name: &str, tag: u8, known: u8) -> ShapesProductError {
    unsupported(format!(
        "this product selects `{type_name}` variant {tag}, but this build implements only tags 0 \
         through {}; prepare the product with the SAME PurRDF build that will execute it, because \
         a variant tag is a capability of the model this build compiled, not an interchange number",
        known.saturating_sub(1)
    ))
}

/// Refuse: the structure nests past [`MAX_DEPTH`].
fn depth_limit() -> ShapesProductError {
    ShapesProductError::new(
        ProductDimension::DepthLimit,
        format!(
            "this product nests shapes, constraints or node expressions more than {MAX_DEPTH} \
             levels deep; flatten the shapes graph, because the ceiling is what stops untrusted \
             bytes from exhausting the native stack — an abort no caller could have handled"
        ),
    )
}

/// Translate a `pack::bits` decoding failure into an admission refusal.
///
/// `pub(crate)` because the product's identity section reads the same LEB128
/// primitives this codec does, and one mapping is what keeps the two sections from
/// reporting the same byte-level failure under two different dimensions.
pub(crate) fn from_pack(error: PackBitsError) -> ShapesProductError {
    match error {
        PackBitsError::Truncated { needed, found } => ShapesProductError::new(
            ProductDimension::Truncated,
            format!(
                "this product ends after {found} bytes but declared a structure needing at least \
                 {needed}; re-write the product, because a short read is an interrupted write \
                 rather than corruption in place"
            ),
        ),
        PackBitsError::Malformed(reason) => malformed(format!(
            "this product carries an integer this build cannot read ({reason}); re-prepare the \
             product from its shapes graph"
        )),
        // `PackBitsError` is `#[non_exhaustive]`, so a wildcard is REQUIRED here.
        // It is not a census enum: totality over it is the `pack` crate's
        // contract, and a failure this build cannot name is still a refusal.
        other => malformed(format!(
            "this product carries an integer this build cannot read ({other}); re-prepare the \
             product from its shapes graph"
        )),
    }
}

// ---------------------------------------------------------------------------
// AstParts
// ---------------------------------------------------------------------------

/// The declarative pieces [`decode_ast`] recovers from a product's AST section.
///
/// Not a [`Shapes`]: the retained dataset, the parse provenance and the
/// caller-supplied registries are carried by other sections, and the shared shape
/// index is still empty. [`Self::shape_index`] is the handle every
/// `sh:nodeByExpression` constraint and every computed [`ShapeArg`] in
/// [`Self::node_shapes`] already holds, so the stage that assembles a `Shapes`
/// fills it ONCE and every site sees the result.
#[derive(Debug)]
pub(crate) struct AstParts {
    /// The shapes graph's top-level node shapes, in their encoded order.
    pub(crate) node_shapes: Vec<Shape>,
    /// The caller-supplied box-role vocabulary, or `None` when the feature was
    /// inactive at preparation.
    pub(crate) box_role_vocab: Option<BoxRoleVocab>,
    /// The `sh:SPARQLTargetType` declarations, keyed by target-type IRI.
    pub(crate) target_types: BTreeMap<String, SparqlTargetType>,
    /// The named-graph IRI the shapes dataset is exposed under, when recorded.
    pub(crate) shapes_graph: Option<String>,
    /// The custom node-expression function declarations, sorted by IRI. Every
    /// [`NodeExpr::CustomCall`] in [`Self::node_shapes`] holds one of THESE
    /// handles, so installing a body here reaches every call site.
    pub(crate) custom_functions: Vec<Arc<CustomFunction>>,
    /// The single shared shape index every `sh:nodeByExpression` constraint and
    /// computed [`ShapeArg`] in [`Self::node_shapes`] was decoded against, left
    /// empty for the assembling stage to fill.
    pub(crate) shape_index: Arc<OnceLock<FastMap<Term, Shape>>>,
    /// The REUSABLE ANALYSIS: the cycle-aware class walk's result, carried rather
    /// than recomputed.
    ///
    /// This is the one field here that is not model. It is the product's answer to
    /// "restore without repeated shared analysis" — see [`encode_classes`] for why
    /// it travels in this section and what still has to be proven about it before a
    /// preparation may use it.
    pub(crate) classes: ClassCatalog,
}

// ---------------------------------------------------------------------------
// Writer
// ---------------------------------------------------------------------------

/// The encoding cursor: an output buffer, the live nesting depth, and the
/// custom-function index every [`NodeExpr::CustomCall`] is written against.
struct AstWriter {
    /// The bytes written so far.
    out: Vec<u8>,
    /// How many recursive structures are currently open.
    depth: u32,
    /// Function IRI → index into the key-sorted declaration table.
    fn_index: BTreeMap<String, u64>,
}

impl AstWriter {
    /// A writer over an empty buffer with the given declaration index.
    ///
    /// The buffer starts at [`WRITE_BUFFER_HINT`]. That is a reservation, not a
    /// limit and not a format fact: the writer appends exactly the same bytes in
    /// exactly the same order whatever capacity it began with, which is why the
    /// determinism fixture's pinned artifact length is unmoved by it.
    fn new(fn_index: BTreeMap<String, u64>) -> Self {
        Self {
            out: Vec::with_capacity(WRITE_BUFFER_HINT),
            depth: 0,
            fn_index,
        }
    }

    /// Open one level of recursion, refusing past [`MAX_DEPTH`].
    fn enter(&mut self) -> Result<(), ShapesProductError> {
        if self.depth >= MAX_DEPTH {
            return Err(depth_limit());
        }
        self.depth += 1;
        Ok(())
    }

    /// Close one level of recursion.
    fn leave(&mut self) {
        self.depth -= 1;
    }

    /// Write a variant tag.
    fn tag(&mut self, tag: u8) {
        self.out.push(tag);
    }

    /// Write a boolean as one byte.
    fn flag(&mut self, value: bool) {
        self.out.push(u8::from(value));
    }

    /// Write an unsigned integer as a varint.
    fn uint(&mut self, value: u64) {
        write_varint(&mut self.out, value);
    }

    /// Write a length or count as a varint.
    fn count(&mut self, value: usize) {
        write_varint(&mut self.out, value as u64);
    }

    /// Write a length-prefixed UTF-8 string.
    fn text(&mut self, value: &str) {
        self.count(value.len());
        self.out.extend_from_slice(value.as_bytes());
    }

    /// Write an optional string.
    fn opt_text(&mut self, value: Option<&str>) {
        match value {
            None => self.flag(false),
            Some(text) => {
                self.flag(true);
                self.text(text);
            }
        }
    }

    /// Write an optional unsigned integer.
    fn opt_uint(&mut self, value: Option<u64>) {
        match value {
            None => self.flag(false),
            Some(number) => {
                self.flag(true);
                self.uint(number);
            }
        }
    }

    // ── Terms ──────────────────────────────────────────────────────────────

    /// Write an IRI.
    fn named_node(&mut self, node: &NamedNode) {
        self.text(node.as_str());
    }

    /// Write an RDF literal as its four lexical components.
    fn literal(&mut self, literal: &Literal) {
        self.text(literal.value());
        self.text(literal.datatype_str());
        self.opt_text(literal.language());
        match literal.direction() {
            None => self.flag(false),
            Some(direction) => {
                self.flag(true);
                self.tag(match direction {
                    RdfTextDirection::Ltr => 0,
                    RdfTextDirection::Rtl => 1,
                });
            }
        }
    }

    /// Write a quoted triple.
    fn triple(&mut self, triple: &Triple) -> Result<(), ShapesProductError> {
        self.term(&triple.subject)?;
        self.named_node(&triple.predicate);
        self.term(&triple.object)
    }

    /// Write an RDF term.
    fn term(&mut self, term: &Term) -> Result<(), ShapesProductError> {
        self.enter()?;
        match term {
            Term::NamedNode(node) => {
                self.tag(0);
                self.named_node(node);
            }
            Term::BlankNode(label) => {
                self.tag(1);
                self.text(label);
            }
            Term::Literal(literal) => {
                self.tag(2);
                self.literal(literal);
            }
            Term::Triple(triple) => {
                self.tag(3);
                self.triple(triple)?;
            }
        }
        self.leave();
        Ok(())
    }

    // ── Leaf enums ─────────────────────────────────────────────────────────

    /// Write a severity level.
    fn severity(&mut self, severity: &Severity) {
        match severity {
            Severity::Violation => self.tag(0),
            Severity::Warning => self.tag(1),
            Severity::Info => self.tag(2),
            Severity::Other(iri) => {
                self.tag(3);
                self.named_node(iri);
            }
        }
    }

    /// Write an optional severity override.
    fn opt_severity(&mut self, severity: Option<&Severity>) {
        match severity {
            None => self.flag(false),
            Some(level) => {
                self.flag(true);
                self.severity(level);
            }
        }
    }

    /// Write a `sh:nodeKind` value.
    fn node_kind(&mut self, kind: &NodeKindValue) {
        self.tag(match kind {
            NodeKindValue::Iri => 0,
            NodeKindValue::BlankNode => 1,
            NodeKindValue::Literal => 2,
            NodeKindValue::BlankNodeOrIri => 3,
            NodeKindValue::BlankNodeOrLiteral => 4,
            NodeKindValue::IriOrLiteral => 5,
            NodeKindValue::TripleTerm => 6,
        });
    }

    /// Write a SHACL property path.
    fn path(&mut self, path: &Path) -> Result<(), ShapesProductError> {
        self.enter()?;
        match path {
            Path::Predicate(predicate) => {
                self.tag(0);
                self.named_node(predicate);
            }
            Path::Inverse(inner) => {
                self.tag(1);
                self.path(inner)?;
            }
            Path::Sequence(steps) => {
                self.tag(2);
                self.count(steps.len());
                for step in steps {
                    self.path(step)?;
                }
            }
            Path::Alternative(branches) => {
                self.tag(3);
                self.count(branches.len());
                for branch in branches {
                    self.path(branch)?;
                }
            }
            Path::ZeroOrMore(inner) => {
                self.tag(4);
                self.path(inner)?;
            }
            Path::OneOrMore(inner) => {
                self.tag(5);
                self.path(inner)?;
            }
            Path::ZeroOrOne(inner) => {
                self.tag(6);
                self.path(inner)?;
            }
        }
        self.leave();
        Ok(())
    }

    /// Write a target declaration.
    fn target(&mut self, target: &Target) -> Result<(), ShapesProductError> {
        match target {
            Target::Class(class) => {
                self.tag(0);
                self.named_node(class);
            }
            Target::SubjectsOf(predicate) => {
                self.tag(1);
                self.named_node(predicate);
            }
            Target::ObjectsOf(predicate) => {
                self.tag(2);
                self.named_node(predicate);
            }
            Target::Node(node) => {
                self.tag(3);
                self.term(node)?;
            }
            Target::ImplicitClass(class) => {
                self.tag(4);
                self.term(class)?;
            }
            Target::Sparql {
                select,
                substitutions,
            } => {
                self.tag(5);
                self.text(select);
                self.count(substitutions.len());
                for (name, value) in substitutions {
                    self.text(name);
                    self.term(value)?;
                }
            }
        }
        Ok(())
    }

    /// Write a custom constraint component's validator query.
    fn component_validator(&mut self, validator: &ComponentValidator) {
        match validator {
            ComponentValidator::Ask { ask } => {
                self.tag(0);
                self.text(ask);
            }
            ComponentValidator::Select { select } => {
                self.tag(1);
                self.text(select);
            }
        }
    }

    /// Write a `sh:SPARQLTargetType` parameter.
    fn target_type_param(&mut self, param: &TargetTypeParam) {
        self.named_node(&param.predicate);
        self.text(&param.var);
    }

    /// Write a `sh:SPARQLTargetType` declaration.
    fn sparql_target_type(
        &mut self,
        target_type: &SparqlTargetType,
    ) -> Result<(), ShapesProductError> {
        self.term(&target_type.id)?;
        self.count(target_type.params.len());
        for param in &target_type.params {
            self.target_type_param(param);
        }
        self.text(&target_type.select);
        Ok(())
    }

    // ── Node expressions ───────────────────────────────────────────────────

    /// Write a `shnex:arg` key.
    fn arg_key(&mut self, key: &ArgKey) {
        match key {
            ArgKey::Index(index) => {
                self.tag(0);
                self.uint(*index);
            }
            ArgKey::Named(iri) => {
                self.tag(1);
                self.text(iri);
            }
        }
    }

    /// Write a custom node-expression function's argument keying.
    fn custom_fn_kind(&mut self, kind: CustomFnKind) {
        self.tag(match kind {
            CustomFnKind::ListParameter => 0,
            CustomFnKind::NamedParameter => 1,
        });
    }

    /// Write the shape argument of `shnex:conformsToShape`.
    fn shape_arg(&mut self, arg: &ShapeArg) -> Result<(), ShapesProductError> {
        self.enter()?;
        match arg {
            ShapeArg::Named(shape) => {
                self.tag(0);
                self.shape(shape)?;
            }
            // `shapes` is the SHARED index; see this module's coverage table.
            ShapeArg::Computed { expr, shapes: _ } => {
                self.tag(1);
                self.node_expr(expr)?;
            }
        }
        self.leave();
        Ok(())
    }

    /// Write a function call.
    fn fn_call(&mut self, call: &FnCall) -> Result<(), ShapesProductError> {
        self.enter()?;
        match call {
            FnCall::Builtin { iri, args } => {
                self.tag(0);
                self.named_node(iri);
                self.node_exprs(args)?;
            }
            FnCall::UserDefined { iri, args } => {
                self.tag(1);
                self.named_node(iri);
                self.node_exprs(args)?;
            }
            // `expr` is re-lowered at decode from `iri`; see the coverage table.
            FnCall::Sparql { iri, expr: _, args } => {
                self.tag(2);
                self.named_node(iri);
                self.node_exprs(args)?;
            }
        }
        self.leave();
        Ok(())
    }

    /// Write a sequence of node expressions.
    fn node_exprs(&mut self, exprs: &[NodeExpr]) -> Result<(), ShapesProductError> {
        self.count(exprs.len());
        for expr in exprs {
            self.node_expr(expr)?;
        }
        Ok(())
    }

    /// Write a node expression.
    fn node_expr(&mut self, expr: &NodeExpr) -> Result<(), ShapesProductError> {
        self.enter()?;
        match expr {
            NodeExpr::Constant(term) => {
                self.tag(0);
                self.term(term)?;
            }
            NodeExpr::This => self.tag(1),
            NodeExpr::Path(path) => {
                self.tag(2);
                self.path(path)?;
            }
            NodeExpr::Filter { nodes, shape } => {
                self.tag(3);
                self.node_expr(nodes)?;
                self.shape(shape)?;
            }
            NodeExpr::Union(operands) => {
                self.tag(4);
                self.node_exprs(operands)?;
            }
            NodeExpr::Intersection(operands) => {
                self.tag(5);
                self.node_exprs(operands)?;
            }
            NodeExpr::If { cond, then, els } => {
                self.tag(6);
                self.node_expr(cond)?;
                self.node_expr(then)?;
                self.node_expr(els)?;
            }
            NodeExpr::Count { distinct, of } => {
                self.tag(7);
                self.flag(*distinct);
                self.node_expr(of)?;
            }
            NodeExpr::Distinct(of) => {
                self.tag(8);
                self.node_expr(of)?;
            }
            NodeExpr::Min(of) => {
                self.tag(9);
                self.node_expr(of)?;
            }
            NodeExpr::Max(of) => {
                self.tag(10);
                self.node_expr(of)?;
            }
            NodeExpr::Sum(of) => {
                self.tag(11);
                self.node_expr(of)?;
            }
            NodeExpr::Limit { of, n } => {
                self.tag(12);
                self.node_expr(of)?;
                self.uint(*n);
            }
            NodeExpr::Offset { of, n } => {
                self.tag(13);
                self.node_expr(of)?;
                self.uint(*n);
            }
            NodeExpr::OrderBy {
                of,
                key,
                descending,
            } => {
                self.tag(14);
                self.node_expr(of)?;
                self.node_expr(key)?;
                self.flag(*descending);
            }
            NodeExpr::Exists(of) => {
                self.tag(15);
                self.node_expr(of)?;
            }
            NodeExpr::Call(call) => {
                self.tag(16);
                self.fn_call(call)?;
            }
            NodeExpr::Arg(key) => {
                self.tag(17);
                self.arg_key(key);
            }
            NodeExpr::CustomCall { func, args } => {
                self.tag(18);
                let index = *self.fn_index.get(func.iri.as_str()).ok_or_else(|| {
                    malformed(format!(
                        "this shapes graph calls the custom node-expression function <{}> but does \
                         not declare it; re-parse the shapes graph, because a call whose \
                         declaration is missing has no body to evaluate",
                        func.iri.as_str()
                    ))
                })?;
                self.uint(index);
                self.count(args.len());
                for (key, arg) in args {
                    self.arg_key(key);
                    self.node_expr(arg)?;
                }
            }
            NodeExpr::Empty => self.tag(19),
            NodeExpr::Var(name) => {
                self.tag(20);
                self.text(name);
            }
            NodeExpr::List(members) => {
                self.tag(21);
                self.count(members.len());
                for member in members {
                    self.term(member)?;
                }
            }
            NodeExpr::PathValues { path, focus } => {
                self.tag(22);
                self.path(path)?;
                self.node_expr(focus)?;
            }
            NodeExpr::Concat(operands) => {
                self.tag(23);
                self.node_exprs(operands)?;
            }
            NodeExpr::Remove { nodes, remove } => {
                self.tag(24);
                self.node_expr(nodes)?;
                self.node_expr(remove)?;
            }
            NodeExpr::FlatMap { nodes, map } => {
                self.tag(25);
                self.node_expr(nodes)?;
                self.node_expr(map)?;
            }
            NodeExpr::FindFirst { nodes, shape } => {
                self.tag(26);
                self.node_expr(nodes)?;
                self.shape(shape)?;
            }
            NodeExpr::MatchAll { nodes, shape } => {
                self.tag(27);
                self.node_expr(nodes)?;
                self.shape(shape)?;
            }
            NodeExpr::InstancesOf(class) => {
                self.tag(28);
                self.named_node(class);
            }
            NodeExpr::NodesMatching(shape) => {
                self.tag(29);
                self.shape(shape)?;
            }
            NodeExpr::ConformsToShape { node, shape } => {
                self.tag(30);
                self.node_expr(node)?;
                self.shape_arg(shape)?;
            }
            NodeExpr::Select {
                query,
                variable,
                key,
            } => {
                self.tag(31);
                self.text(query);
                self.text(variable);
                self.tag(select_key_tag(key)?);
            }
        }
        self.leave();
        Ok(())
    }

    // ── Constraints and shapes ─────────────────────────────────────────────

    /// Write a sequence of shapes.
    fn shapes_seq(&mut self, shapes: &[Shape]) -> Result<(), ShapesProductError> {
        self.count(shapes.len());
        for shape in shapes {
            self.shape(shape)?;
        }
        Ok(())
    }

    /// Write a constraint.
    fn constraint(&mut self, constraint: &Constraint) -> Result<(), ShapesProductError> {
        self.enter()?;
        match constraint {
            Constraint::Class(classes) => {
                self.tag(0);
                self.count(classes.len());
                for class in classes {
                    self.named_node(class);
                }
            }
            Constraint::Datatype(datatypes) => {
                self.tag(1);
                self.count(datatypes.len());
                for datatype in datatypes {
                    self.named_node(datatype);
                }
            }
            Constraint::NodeKind(kinds) => {
                self.tag(2);
                self.count(kinds.len());
                for kind in kinds {
                    self.node_kind(kind);
                }
            }
            Constraint::MinCount(count) => {
                self.tag(3);
                self.uint(*count);
            }
            Constraint::MaxCount(count) => {
                self.tag(4);
                self.uint(*count);
            }
            Constraint::In(values) => {
                self.tag(5);
                self.count(values.len());
                for value in values {
                    self.term(value)?;
                }
            }
            Constraint::HasValue(value) => {
                self.tag(6);
                self.term(value)?;
            }
            // `compiled` is a lazy cache; see the coverage table.
            Constraint::Pattern {
                regex,
                flags,
                compiled: _,
            } => {
                self.tag(7);
                self.text(regex);
                self.opt_text(flags.as_deref());
            }
            Constraint::MinLength(length) => {
                self.tag(8);
                self.uint(*length);
            }
            Constraint::MaxLength(length) => {
                self.tag(9);
                self.uint(*length);
            }
            Constraint::UniqueLang(unique) => {
                self.tag(10);
                self.flag(*unique);
            }
            Constraint::LanguageIn(tags) => {
                self.tag(11);
                self.count(tags.len());
                for language in tags {
                    self.text(language);
                }
            }
            Constraint::Not(shape) => {
                self.tag(12);
                self.shape(shape)?;
            }
            Constraint::Closed { ignored } => {
                self.tag(13);
                self.count(ignored.len());
                for predicate in ignored {
                    self.named_node(predicate);
                }
            }
            Constraint::MinInclusive(bound) => {
                self.tag(14);
                self.term(bound)?;
            }
            Constraint::MaxInclusive(bound) => {
                self.tag(15);
                self.term(bound)?;
            }
            Constraint::MinExclusive(bound) => {
                self.tag(16);
                self.term(bound)?;
            }
            Constraint::MaxExclusive(bound) => {
                self.tag(17);
                self.term(bound)?;
            }
            Constraint::And(shapes) => {
                self.tag(18);
                self.shapes_seq(shapes)?;
            }
            Constraint::Or(shapes) => {
                self.tag(19);
                self.shapes_seq(shapes)?;
            }
            Constraint::Xone(shapes) => {
                self.tag(20);
                self.shapes_seq(shapes)?;
            }
            Constraint::Node(shape) => {
                self.tag(21);
                self.shape(shape)?;
            }
            Constraint::Sparql {
                select,
                message,
                severity,
            } => {
                self.tag(22);
                self.text(select);
                self.opt_text(message.as_deref());
                self.opt_severity(severity.as_ref());
            }
            Constraint::Equals(path) => {
                self.tag(23);
                self.path(path)?;
            }
            Constraint::Disjoint(path) => {
                self.tag(24);
                self.path(path)?;
            }
            Constraint::LessThan(path) => {
                self.tag(25);
                self.path(path)?;
            }
            Constraint::LessThanOrEquals(path) => {
                self.tag(26);
                self.path(path)?;
            }
            Constraint::QualifiedValueShape {
                shape,
                siblings,
                min_count,
                max_count,
                disjoint,
            } => {
                self.tag(27);
                self.shape(shape)?;
                self.shapes_seq(siblings)?;
                self.opt_uint(*min_count);
                self.opt_uint(*max_count);
                self.flag(*disjoint);
            }
            Constraint::Expression {
                expr,
                message,
                severity,
            } => {
                self.tag(28);
                self.node_expr(expr)?;
                self.opt_text(message.as_deref());
                self.opt_severity(severity.as_ref());
            }
            // `shapes` is the SHARED index; see the coverage table.
            Constraint::NodeByExpression {
                expr,
                shapes: _,
                message,
                severity,
            } => {
                self.tag(29);
                self.node_expr(expr)?;
                self.opt_text(message.as_deref());
                self.opt_severity(severity.as_ref());
            }
            Constraint::Component {
                component,
                source_shape,
                bindings,
                validator,
                message,
                severity,
            } => {
                self.tag(30);
                self.named_node(component);
                self.term(source_shape)?;
                self.count(bindings.len());
                for (name, value) in bindings {
                    self.text(name);
                    self.term(value)?;
                }
                self.component_validator(validator);
                self.opt_text(message.as_deref());
                self.opt_severity(severity.as_ref());
            }
            Constraint::MinListLength(length) => {
                self.tag(31);
                self.uint(*length);
            }
            Constraint::MaxListLength(length) => {
                self.tag(32);
                self.uint(*length);
            }
            Constraint::UniqueMembers(unique) => {
                self.tag(33);
                self.flag(*unique);
            }
            Constraint::MemberShape(shape) => {
                self.tag(34);
                self.shape(shape)?;
            }
            Constraint::SingleLine(flag) => {
                self.tag(35);
                self.flag(*flag);
            }
            Constraint::RootClass(roots) => {
                self.tag(36);
                self.count(roots.len());
                for root in roots {
                    self.named_node(root);
                }
            }
            Constraint::SomeValue(shape) => {
                self.tag(37);
                self.shape(shape)?;
            }
            Constraint::SubsetOf(path) => {
                self.tag(38);
                self.path(path)?;
            }
            Constraint::UniqueValuesFor {
                properties,
                targets,
            } => {
                self.tag(39);
                self.count(properties.len());
                for property in properties {
                    self.named_node(property);
                }
                self.count(targets.len());
                for target in targets {
                    self.target(target)?;
                }
            }
        }
        self.leave();
        Ok(())
    }

    /// Write a property shape.
    fn property_shape(&mut self, shape: &PropertyShape) -> Result<(), ShapesProductError> {
        self.enter()?;
        self.term(&shape.id)?;
        self.path(&shape.path)?;
        self.count(shape.constraints.len());
        for constraint in &shape.constraints {
            self.constraint(constraint)?;
        }
        self.count(shape.property_shapes.len());
        for nested in &shape.property_shapes {
            self.property_shape(nested)?;
        }
        self.shapes_seq(&shape.reifier_shapes)?;
        self.flag(shape.reification_required);
        self.severity(&shape.severity);
        self.opt_text(shape.message.as_deref());
        self.flag(shape.deactivated);
        self.count(shape.box_roles.len());
        for role in &shape.box_roles {
            self.named_node(role);
        }
        self.leave();
        Ok(())
    }

    /// Write a node shape.
    fn shape(&mut self, shape: &Shape) -> Result<(), ShapesProductError> {
        self.enter()?;
        self.term(&shape.id)?;
        self.count(shape.targets.len());
        for target in &shape.targets {
            self.target(target)?;
        }
        self.count(shape.constraints.len());
        for constraint in &shape.constraints {
            self.constraint(constraint)?;
        }
        self.count(shape.property_shapes.len());
        for property in &shape.property_shapes {
            self.property_shape(property)?;
        }
        self.severity(&shape.severity);
        self.opt_text(shape.message.as_deref());
        self.flag(shape.deactivated);
        self.count(shape.box_roles.len());
        for role in &shape.box_roles {
            self.named_node(role);
        }
        self.count(shape.rules.len());
        for rule in &shape.rules {
            self.rule(rule)?;
        }
        self.leave();
        Ok(())
    }

    // ── Rules ──────────────────────────────────────────────────────────────

    /// Write a rule's stratification half.
    fn rule_schedule(&mut self, schedule: RuleSchedule) {
        self.tag(match schedule {
            RuleSchedule::Once => 0,
            RuleSchedule::General => 1,
        });
    }

    /// Write a rule head.
    fn rule_body(&mut self, body: &RuleBody) -> Result<(), ShapesProductError> {
        self.enter()?;
        match body {
            RuleBody::Triple {
                subject,
                predicate,
                object,
            } => {
                self.tag(0);
                self.node_expr(subject)?;
                self.node_expr(predicate)?;
                self.node_expr(object)?;
            }
            RuleBody::Sparql { construct } => {
                self.tag(1);
                self.text(construct);
            }
        }
        self.leave();
        Ok(())
    }

    /// Write a `sh:order` key, canonically.
    fn order_key(&mut self, key: OrderKey) -> Result<(), ShapesProductError> {
        // `+ 0.0` is the same normalization `OrderKey::new` applies, repeated here
        // so the BYTES are canonical whatever produced the value: `-0.0` and
        // `+0.0` are one number with two IEEE-754 encodings, and two encodings of
        // one number would be two byte forms of one product.
        let value = key.value() + 0.0;
        if !value.is_finite() {
            return Err(malformed(format!(
                "this shapes graph carries a `sh:order` of {value}, which has no ordering value; \
                 give the rule a finite decimal order, because the scheduler makes equal orders \
                 mean one stratum and a non-finite key belongs to none"
            )));
        }
        self.out.extend_from_slice(&value.to_bits().to_le_bytes());
        Ok(())
    }

    /// Write a SHACL-AF rule.
    fn rule(&mut self, rule: &Rule) -> Result<(), ShapesProductError> {
        self.enter()?;
        self.term(&rule.id)?;
        self.rule_body(&rule.body)?;
        self.shapes_seq(&rule.conditions)?;
        match rule.order {
            None => self.flag(false),
            Some(key) => {
                self.flag(true);
                self.order_key(key)?;
            }
        }
        self.flag(rule.deactivated);
        self.rule_schedule(rule.schedule);
        self.leave();
        Ok(())
    }

    // ── Shapes-level ───────────────────────────────────────────────────────

    /// Write the caller-supplied box-role vocabulary.
    fn box_role_vocab(&mut self, vocab: &BoxRoleVocab) {
        self.text(&vocab.graph_box_role);
        self.text(&vocab.box_abox);
        self.text(&vocab.box_tbox);
        self.text(&vocab.box_rbox);
        self.text(&vocab.box_cbox);
        self.text(&vocab.box_config_box);
    }
}

/// The tag for a `NodeExpr::Select` authored key.
///
/// The key is a closed two-value set the parser assigns from `&'static str`
/// literals, so an unrecognised one is a model this build does not implement
/// rather than a string to carry through.
fn select_key_tag(key: &str) -> Result<u8, ShapesProductError> {
    match key {
        SELECT_KEY_SELECT => Ok(0),
        SELECT_KEY_SPARQL_EXPR => Ok(1),
        other => Err(unsupported(format!(
            "this shapes graph spells a SPARQL-based node expression `{other}`, but this build \
             implements only `{SELECT_KEY_SELECT}` and `{SELECT_KEY_SPARQL_EXPR}`; re-parse the \
             shapes graph with the build that will execute it"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Reader
// ---------------------------------------------------------------------------

/// The decoding cursor: the caller's bytes, the read position, the live nesting
/// depth, the materialized function table, and the one shared shape index.
struct AstReader<'a> {
    /// The bytes being admitted. Never assumed aligned.
    bytes: &'a [u8],
    /// The next byte to read.
    pos: usize,
    /// How many recursive structures are currently open.
    depth: u32,
    /// The declaration table, materialized before any body or shape is read.
    functions: Vec<Arc<CustomFunction>>,
    /// The ONE shape-index handle every decoded site is given a clone of.
    shape_index: Arc<OnceLock<FastMap<Term, Shape>>>,
}

impl<'a> AstReader<'a> {
    /// A reader positioned at the start of `bytes`.
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            pos: 0,
            depth: 0,
            functions: Vec::new(),
            shape_index: Arc::new(OnceLock::new()),
        }
    }

    /// How many bytes remain unread.
    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.pos)
    }

    /// Open one level of recursion, refusing past [`MAX_DEPTH`].
    fn enter(&mut self) -> Result<(), ShapesProductError> {
        if self.depth >= MAX_DEPTH {
            return Err(depth_limit());
        }
        self.depth += 1;
        Ok(())
    }

    /// Close one level of recursion.
    fn leave(&mut self) {
        self.depth -= 1;
    }

    /// Read one raw byte.
    fn byte(&mut self) -> Result<u8, ShapesProductError> {
        let value = *self.bytes.get(self.pos).ok_or_else(|| {
            from_pack(PackBitsError::Truncated {
                needed: self.pos + 1,
                found: self.bytes.len(),
            })
        })?;
        self.pos += 1;
        Ok(value)
    }

    /// Read a variant tag, refusing one this build does not implement.
    fn tag(&mut self, type_name: &str, known: u8) -> Result<u8, ShapesProductError> {
        let tag = self.byte()?;
        if tag >= known {
            return Err(unknown_tag(type_name, tag, known));
        }
        Ok(tag)
    }

    /// Read a boolean.
    fn flag(&mut self) -> Result<bool, ShapesProductError> {
        match self.byte()? {
            0 => Ok(false),
            1 => Ok(true),
            other => Err(malformed(format!(
                "this product spells a boolean as {other}; re-prepare the product, because only 0 \
                 and 1 are boolean byte forms and any other value means the reader is no longer \
                 at a field boundary"
            ))),
        }
    }

    /// Read an unsigned integer.
    fn uint(&mut self) -> Result<u64, ShapesProductError> {
        read_varint(self.bytes, &mut self.pos).map_err(from_pack)
    }

    /// Read a length or count, bounded by the bytes that could possibly hold it.
    ///
    /// Every element of every sequence in this format occupies at least one byte,
    /// so a declared count larger than the remaining input cannot be honest — and
    /// refusing it here is what stops a hostile length from driving a multi-
    /// gigabyte allocation before the truncation is discovered.
    fn count(&mut self) -> Result<usize, ShapesProductError> {
        let declared = self.uint()?;
        let count = usize::try_from(declared).map_err(|_| {
            malformed(format!(
                "this product declares a sequence of {declared} elements, more than this platform \
                 can address; re-prepare the product from its shapes graph"
            ))
        })?;
        if count > self.remaining() {
            return Err(malformed(format!(
                "this product declares a sequence of {count} elements but only {} bytes follow it; \
                 re-prepare the product, because every element occupies at least one byte",
                self.remaining()
            )));
        }
        Ok(count)
    }

    /// Read a length-prefixed UTF-8 string.
    fn text(&mut self) -> Result<String, ShapesProductError> {
        let len = self.count()?;
        let end = self.pos + len;
        let slice = self.bytes.get(self.pos..end).ok_or_else(|| {
            from_pack(PackBitsError::Truncated {
                needed: end,
                found: self.bytes.len(),
            })
        })?;
        let text = std::str::from_utf8(slice).map_err(|error| {
            malformed(format!(
                "this product carries a string that is not UTF-8 ({error}); re-prepare the \
                 product, because every string in a shapes graph is Unicode text"
            ))
        })?;
        self.pos = end;
        Ok(text.to_owned())
    }

    /// Read an optional string.
    fn opt_text(&mut self) -> Result<Option<String>, ShapesProductError> {
        if self.flag()? {
            Ok(Some(self.text()?))
        } else {
            Ok(None)
        }
    }

    /// Read an optional unsigned integer.
    fn opt_uint(&mut self) -> Result<Option<u64>, ShapesProductError> {
        if self.flag()? {
            Ok(Some(self.uint()?))
        } else {
            Ok(None)
        }
    }

    /// Read `count` elements with `read`.
    ///
    /// The reservation is clamped to [`MAX_SPECULATIVE_ELEMENTS`]; a longer honest
    /// sequence still decodes in full, growing as it is read.
    fn seq<T, F>(&mut self, mut read: F) -> Result<Vec<T>, ShapesProductError>
    where
        F: FnMut(&mut Self) -> Result<T, ShapesProductError>,
    {
        let count = self.count()?;
        let mut out = Vec::with_capacity(speculative_capacity(count));
        for _ in 0..count {
            out.push(read(self)?);
        }
        Ok(out)
    }

    // ── Terms ──────────────────────────────────────────────────────────────

    /// Read an IRI.
    fn named_node(&mut self) -> Result<NamedNode, ShapesProductError> {
        Ok(NamedNode::new_unchecked(self.text()?))
    }

    /// Read an RDF literal, rebuilding it through the constructor its recorded
    /// language and direction select.
    fn literal(&mut self) -> Result<Literal, ShapesProductError> {
        let lexical = self.text()?;
        let datatype = self.text()?;
        let language = self.opt_text()?;
        let direction = if self.flag()? {
            Some(match self.tag("RdfTextDirection", TAGS_DIRECTION)? {
                0 => RdfTextDirection::Ltr,
                _ => RdfTextDirection::Rtl,
            })
        } else {
            None
        };

        let literal = match (language, direction) {
            (Some(language), Some(direction)) => {
                Literal::new_directional_language_tagged_literal_unchecked(
                    lexical, language, direction,
                )
            }
            (Some(language), None) => {
                Literal::new_language_tagged_literal_unchecked(lexical, language)
            }
            // The datatype string is MOVED into the term, never copied alongside
            // it: a typed literal's datatype is the one just read, so the
            // agreement check below has nothing to compare and the clone it would
            // have needed is one heap allocation per literal in the shape tree.
            (None, None) => {
                return Ok(Literal::new_typed_literal(
                    lexical,
                    NamedNode::new_unchecked(datatype),
                ));
            }
            (None, Some(_)) => {
                return Err(malformed(
                    "this product carries a literal with a base direction but no language tag; \
                     re-prepare the product, because RDF 1.2 gives a direction only to a \
                     language-tagged literal",
                ));
            }
        };
        // A language-tagged literal's datatype is DECIDED by its language and
        // direction, so a recorded datatype that disagrees is a product whose two
        // halves describe different terms — and admitting it would silently
        // replace the one it names.
        if literal.datatype_str() != datatype {
            return Err(malformed(format!(
                "this product records the literal datatype <{datatype}> for a term whose language \
                 tag makes it <{}>; re-prepare the product from its shapes graph",
                literal.datatype_str()
            )));
        }
        Ok(literal)
    }

    /// Read a quoted triple.
    fn triple(&mut self) -> Result<Triple, ShapesProductError> {
        let subject = self.term()?;
        let predicate = self.named_node()?;
        let object = self.term()?;
        Ok(Triple::new(subject, predicate, object))
    }

    /// Read an RDF term.
    fn term(&mut self) -> Result<Term, ShapesProductError> {
        self.enter()?;
        let term = match self.tag("Term", TAGS_TERM)? {
            0 => Term::NamedNode(self.named_node()?),
            1 => Term::BlankNode(self.text()?),
            2 => Term::Literal(self.literal()?),
            _ => Term::Triple(Box::new(self.triple()?)),
        };
        self.leave();
        Ok(term)
    }

    // ── Leaf enums ─────────────────────────────────────────────────────────

    /// Read a severity level.
    fn severity(&mut self) -> Result<Severity, ShapesProductError> {
        Ok(match self.tag("Severity", TAGS_SEVERITY)? {
            0 => Severity::Violation,
            1 => Severity::Warning,
            2 => Severity::Info,
            _ => Severity::Other(self.named_node()?),
        })
    }

    /// Read an optional severity override.
    fn opt_severity(&mut self) -> Result<Option<Severity>, ShapesProductError> {
        if self.flag()? {
            Ok(Some(self.severity()?))
        } else {
            Ok(None)
        }
    }

    /// Read a `sh:nodeKind` value.
    fn node_kind(&mut self) -> Result<NodeKindValue, ShapesProductError> {
        Ok(match self.tag("NodeKindValue", TAGS_NODE_KIND)? {
            0 => NodeKindValue::Iri,
            1 => NodeKindValue::BlankNode,
            2 => NodeKindValue::Literal,
            3 => NodeKindValue::BlankNodeOrIri,
            4 => NodeKindValue::BlankNodeOrLiteral,
            5 => NodeKindValue::IriOrLiteral,
            _ => NodeKindValue::TripleTerm,
        })
    }

    /// Read a SHACL property path.
    fn path(&mut self) -> Result<Path, ShapesProductError> {
        self.enter()?;
        let path = match self.tag("Path", TAGS_PATH)? {
            0 => Path::Predicate(self.named_node()?),
            1 => Path::Inverse(Box::new(self.path()?)),
            2 => Path::Sequence(self.seq(Self::path)?),
            3 => Path::Alternative(self.seq(Self::path)?),
            4 => Path::ZeroOrMore(Box::new(self.path()?)),
            5 => Path::OneOrMore(Box::new(self.path()?)),
            _ => Path::ZeroOrOne(Box::new(self.path()?)),
        };
        self.leave();
        Ok(path)
    }

    /// Read a target declaration.
    fn target(&mut self) -> Result<Target, ShapesProductError> {
        Ok(match self.tag("Target", TAGS_TARGET)? {
            0 => Target::Class(self.named_node()?),
            1 => Target::SubjectsOf(self.named_node()?),
            2 => Target::ObjectsOf(self.named_node()?),
            3 => Target::Node(self.term()?),
            4 => Target::ImplicitClass(self.term()?),
            _ => Target::Sparql {
                select: self.text()?,
                substitutions: self.seq(|reader| {
                    let name = reader.text()?;
                    let value = reader.term()?;
                    Ok((name, value))
                })?,
            },
        })
    }

    /// Read a custom constraint component's validator query.
    fn component_validator(&mut self) -> Result<ComponentValidator, ShapesProductError> {
        Ok(
            match self.tag("ComponentValidator", TAGS_COMPONENT_VALIDATOR)? {
                0 => ComponentValidator::Ask { ask: self.text()? },
                _ => ComponentValidator::Select {
                    select: self.text()?,
                },
            },
        )
    }

    /// Read a `sh:SPARQLTargetType` parameter.
    fn target_type_param(&mut self) -> Result<TargetTypeParam, ShapesProductError> {
        Ok(TargetTypeParam {
            predicate: self.named_node()?,
            var: self.text()?,
        })
    }

    /// Read a `sh:SPARQLTargetType` declaration.
    fn sparql_target_type(&mut self) -> Result<SparqlTargetType, ShapesProductError> {
        Ok(SparqlTargetType {
            id: self.term()?,
            params: self.seq(Self::target_type_param)?,
            select: self.text()?,
        })
    }

    // ── Node expressions ───────────────────────────────────────────────────

    /// Read a `shnex:arg` key.
    fn arg_key(&mut self) -> Result<ArgKey, ShapesProductError> {
        Ok(match self.tag("ArgKey", TAGS_ARG_KEY)? {
            0 => ArgKey::Index(self.uint()?),
            _ => ArgKey::Named(self.text()?),
        })
    }

    /// Read a custom node-expression function's argument keying.
    fn custom_fn_kind(&mut self) -> Result<CustomFnKind, ShapesProductError> {
        Ok(match self.tag("CustomFnKind", TAGS_CUSTOM_FN_KIND)? {
            0 => CustomFnKind::ListParameter,
            _ => CustomFnKind::NamedParameter,
        })
    }

    /// Read the shape argument of `shnex:conformsToShape`.
    fn shape_arg(&mut self) -> Result<ShapeArg, ShapesProductError> {
        self.enter()?;
        let arg = match self.tag("ShapeArg", TAGS_SHAPE_ARG)? {
            0 => ShapeArg::Named(Box::new(self.shape()?)),
            _ => ShapeArg::Computed {
                expr: Box::new(self.node_expr()?),
                shapes: Arc::clone(&self.shape_index),
            },
        };
        self.leave();
        Ok(arg)
    }

    /// Read a function call, re-lowering a `sparql:<NAME>` call's surface text.
    fn fn_call(&mut self) -> Result<FnCall, ShapesProductError> {
        self.enter()?;
        let call = match self.tag("FnCall", TAGS_FN_CALL)? {
            0 => FnCall::Builtin {
                iri: self.named_node()?,
                args: self.seq(Self::node_expr)?,
            },
            1 => FnCall::UserDefined {
                iri: self.named_node()?,
                args: self.seq(Self::node_expr)?,
            },
            _ => {
                let iri = self.named_node()?;
                let args = self.seq(Self::node_expr)?;
                let expr = lower_sparql_call(&iri, args.len())?;
                FnCall::Sparql { iri, expr, args }
            }
        };
        self.leave();
        Ok(call)
    }

    /// Read a node expression.
    fn node_expr(&mut self) -> Result<NodeExpr, ShapesProductError> {
        self.enter()?;
        let expr = match self.tag("NodeExpr", TAGS_NODE_EXPR)? {
            0 => NodeExpr::Constant(self.term()?),
            1 => NodeExpr::This,
            2 => NodeExpr::Path(self.path()?),
            3 => NodeExpr::Filter {
                nodes: Box::new(self.node_expr()?),
                shape: Box::new(self.shape()?),
            },
            4 => NodeExpr::Union(self.seq(Self::node_expr)?),
            5 => NodeExpr::Intersection(self.seq(Self::node_expr)?),
            6 => NodeExpr::If {
                cond: Box::new(self.node_expr()?),
                then: Box::new(self.node_expr()?),
                els: Box::new(self.node_expr()?),
            },
            7 => NodeExpr::Count {
                distinct: self.flag()?,
                of: Box::new(self.node_expr()?),
            },
            8 => NodeExpr::Distinct(Box::new(self.node_expr()?)),
            9 => NodeExpr::Min(Box::new(self.node_expr()?)),
            10 => NodeExpr::Max(Box::new(self.node_expr()?)),
            11 => NodeExpr::Sum(Box::new(self.node_expr()?)),
            12 => NodeExpr::Limit {
                of: Box::new(self.node_expr()?),
                n: self.uint()?,
            },
            13 => NodeExpr::Offset {
                of: Box::new(self.node_expr()?),
                n: self.uint()?,
            },
            14 => NodeExpr::OrderBy {
                of: Box::new(self.node_expr()?),
                key: Box::new(self.node_expr()?),
                descending: self.flag()?,
            },
            15 => NodeExpr::Exists(Box::new(self.node_expr()?)),
            16 => NodeExpr::Call(self.fn_call()?),
            17 => NodeExpr::Arg(self.arg_key()?),
            18 => {
                let index = self.uint()?;
                let func = self.custom_function(index)?;
                let args = self.seq(|reader| {
                    let key = reader.arg_key()?;
                    let arg = reader.node_expr()?;
                    Ok((key, arg))
                })?;
                NodeExpr::CustomCall { func, args }
            }
            19 => NodeExpr::Empty,
            20 => NodeExpr::Var(self.text()?),
            21 => NodeExpr::List(self.seq(Self::term)?),
            22 => NodeExpr::PathValues {
                path: self.path()?,
                focus: Box::new(self.node_expr()?),
            },
            23 => NodeExpr::Concat(self.seq(Self::node_expr)?),
            24 => NodeExpr::Remove {
                nodes: Box::new(self.node_expr()?),
                remove: Box::new(self.node_expr()?),
            },
            25 => NodeExpr::FlatMap {
                nodes: Box::new(self.node_expr()?),
                map: Box::new(self.node_expr()?),
            },
            26 => NodeExpr::FindFirst {
                nodes: Box::new(self.node_expr()?),
                shape: Box::new(self.shape()?),
            },
            27 => NodeExpr::MatchAll {
                nodes: Box::new(self.node_expr()?),
                shape: Box::new(self.shape()?),
            },
            28 => NodeExpr::InstancesOf(self.named_node()?),
            29 => NodeExpr::NodesMatching(Box::new(self.shape()?)),
            30 => NodeExpr::ConformsToShape {
                node: Box::new(self.node_expr()?),
                shape: self.shape_arg()?,
            },
            _ => {
                let query = self.text()?;
                let variable = self.text()?;
                let key = match self.tag("NodeExpr::Select::key", TAGS_SELECT_KEY)? {
                    0 => SELECT_KEY_SELECT,
                    _ => SELECT_KEY_SPARQL_EXPR,
                };
                NodeExpr::Select {
                    query,
                    variable,
                    key,
                }
            }
        };
        self.leave();
        Ok(expr)
    }

    /// Resolve a custom-function table index to the handle every call site of
    /// that function shares.
    fn custom_function(&self, index: u64) -> Result<Arc<CustomFunction>, ShapesProductError> {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.functions.get(index))
            .map(Arc::clone)
            .ok_or_else(|| {
                malformed(format!(
                    "this product calls custom node-expression function {index}, but its \
                     declaration table holds {}; re-prepare the product from its shapes graph, \
                     because a call with no declaration has no body to evaluate",
                    self.functions.len()
                ))
            })
    }

    // ── Constraints and shapes ─────────────────────────────────────────────

    /// Read a constraint.
    fn constraint(&mut self) -> Result<Constraint, ShapesProductError> {
        self.enter()?;
        let constraint = match self.tag("Constraint", TAGS_CONSTRAINT)? {
            0 => Constraint::Class(self.seq(Self::named_node)?),
            1 => Constraint::Datatype(self.seq(Self::named_node)?),
            2 => Constraint::NodeKind(self.seq(Self::node_kind)?),
            3 => Constraint::MinCount(self.uint()?),
            4 => Constraint::MaxCount(self.uint()?),
            5 => Constraint::In(self.seq(Self::term)?),
            6 => Constraint::HasValue(self.term()?),
            7 => Constraint::Pattern {
                regex: self.text()?,
                flags: self.opt_text()?,
                compiled: Arc::new(OnceLock::new()),
            },
            8 => Constraint::MinLength(self.uint()?),
            9 => Constraint::MaxLength(self.uint()?),
            10 => Constraint::UniqueLang(self.flag()?),
            11 => Constraint::LanguageIn(self.seq(Self::text)?),
            12 => Constraint::Not(Box::new(self.shape()?)),
            13 => Constraint::Closed {
                ignored: self.seq(Self::named_node)?,
            },
            14 => Constraint::MinInclusive(self.term()?),
            15 => Constraint::MaxInclusive(self.term()?),
            16 => Constraint::MinExclusive(self.term()?),
            17 => Constraint::MaxExclusive(self.term()?),
            18 => Constraint::And(self.seq(Self::shape)?),
            19 => Constraint::Or(self.seq(Self::shape)?),
            20 => Constraint::Xone(self.seq(Self::shape)?),
            21 => Constraint::Node(Box::new(self.shape()?)),
            22 => Constraint::Sparql {
                select: self.text()?,
                message: self.opt_text()?,
                severity: self.opt_severity()?,
            },
            23 => Constraint::Equals(self.path()?),
            24 => Constraint::Disjoint(self.path()?),
            25 => Constraint::LessThan(self.path()?),
            26 => Constraint::LessThanOrEquals(self.path()?),
            27 => Constraint::QualifiedValueShape {
                shape: Box::new(self.shape()?),
                siblings: self.seq(Self::shape)?,
                min_count: self.opt_uint()?,
                max_count: self.opt_uint()?,
                disjoint: self.flag()?,
            },
            28 => Constraint::Expression {
                expr: self.node_expr()?,
                message: self.opt_text()?,
                severity: self.opt_severity()?,
            },
            29 => Constraint::NodeByExpression {
                expr: self.node_expr()?,
                shapes: Arc::clone(&self.shape_index),
                message: self.opt_text()?,
                severity: self.opt_severity()?,
            },
            30 => Constraint::Component {
                component: self.named_node()?,
                source_shape: self.term()?,
                bindings: self.seq(|reader| {
                    let name = reader.text()?;
                    let value = reader.term()?;
                    Ok((name, value))
                })?,
                validator: self.component_validator()?,
                message: self.opt_text()?,
                severity: self.opt_severity()?,
            },
            31 => Constraint::MinListLength(self.uint()?),
            32 => Constraint::MaxListLength(self.uint()?),
            33 => Constraint::UniqueMembers(self.flag()?),
            34 => Constraint::MemberShape(Box::new(self.shape()?)),
            35 => Constraint::SingleLine(self.flag()?),
            36 => Constraint::RootClass(self.seq(Self::named_node)?),
            37 => Constraint::SomeValue(Box::new(self.shape()?)),
            38 => Constraint::SubsetOf(self.path()?),
            _ => Constraint::UniqueValuesFor {
                properties: self.seq(Self::named_node)?,
                targets: self.seq(Self::target)?,
            },
        };
        self.leave();
        Ok(constraint)
    }

    /// Read a property shape.
    fn property_shape(&mut self) -> Result<PropertyShape, ShapesProductError> {
        self.enter()?;
        let shape = PropertyShape {
            id: self.term()?,
            path: self.path()?,
            constraints: self.seq(Self::constraint)?,
            property_shapes: self.seq(Self::property_shape)?,
            reifier_shapes: self.seq(Self::shape)?,
            reification_required: self.flag()?,
            severity: self.severity()?,
            message: self.opt_text()?,
            deactivated: self.flag()?,
            box_roles: self.seq(Self::named_node)?,
        };
        self.leave();
        Ok(shape)
    }

    /// Read a node shape.
    fn shape(&mut self) -> Result<Shape, ShapesProductError> {
        self.enter()?;
        let shape = Shape {
            id: self.term()?,
            targets: self.seq(Self::target)?,
            constraints: self.seq(Self::constraint)?,
            property_shapes: self.seq(Self::property_shape)?,
            severity: self.severity()?,
            message: self.opt_text()?,
            deactivated: self.flag()?,
            box_roles: self.seq(Self::named_node)?,
            rules: self.seq(Self::rule)?,
        };
        self.leave();
        Ok(shape)
    }

    // ── Rules ──────────────────────────────────────────────────────────────

    /// Read a rule's stratification half.
    fn rule_schedule(&mut self) -> Result<RuleSchedule, ShapesProductError> {
        Ok(match self.tag("RuleSchedule", TAGS_RULE_SCHEDULE)? {
            0 => RuleSchedule::Once,
            _ => RuleSchedule::General,
        })
    }

    /// Read a rule head.
    fn rule_body(&mut self) -> Result<RuleBody, ShapesProductError> {
        self.enter()?;
        let body = match self.tag("RuleBody", TAGS_RULE_BODY)? {
            0 => RuleBody::Triple {
                subject: self.node_expr()?,
                predicate: self.node_expr()?,
                object: self.node_expr()?,
            },
            _ => RuleBody::Sparql {
                construct: self.text()?,
            },
        };
        self.leave();
        Ok(body)
    }

    /// Read a `sh:order` key.
    fn order_key(&mut self) -> Result<OrderKey, ShapesProductError> {
        let end = self.pos + 8;
        let slice = self.bytes.get(self.pos..end).ok_or_else(|| {
            from_pack(PackBitsError::Truncated {
                needed: end,
                found: self.bytes.len(),
            })
        })?;
        // An explicit byte-slice copy, never a pointer cast: the caller's buffer
        // need not be 8-byte aligned. Same discipline as `pack::bits`.
        let mut raw = [0u8; 8];
        raw.copy_from_slice(slice);
        self.pos = end;
        let value = f64::from_bits(u64::from_le_bytes(raw));
        if !value.is_finite() {
            return Err(malformed(format!(
                "this product carries a `sh:order` of {value}, which has no ordering value; \
                 re-prepare the product from its shapes graph, because the scheduler makes equal \
                 orders mean one stratum and a non-finite key belongs to none"
            )));
        }
        Ok(OrderKey::new(value))
    }

    /// Read a SHACL-AF rule.
    fn rule(&mut self) -> Result<Rule, ShapesProductError> {
        self.enter()?;
        let rule = Rule {
            id: self.term()?,
            body: self.rule_body()?,
            conditions: self.seq(Self::shape)?,
            order: if self.flag()? {
                Some(self.order_key()?)
            } else {
                None
            },
            deactivated: self.flag()?,
            schedule: self.rule_schedule()?,
        };
        self.leave();
        Ok(rule)
    }

    // ── Shapes-level ───────────────────────────────────────────────────────

    /// Read the caller-supplied box-role vocabulary.
    fn box_role_vocab(&mut self) -> Result<BoxRoleVocab, ShapesProductError> {
        Ok(BoxRoleVocab {
            graph_box_role: self.text()?,
            box_abox: self.text()?,
            box_tbox: self.text()?,
            box_rbox: self.text()?,
            box_cbox: self.text()?,
            box_config_box: self.text()?,
        })
    }
}

/// Re-lower a `sparql:<NAME>` call to the SPARQL surface text it renders.
///
/// The call's IRI plus its argument count is everything the rendering needs, and
/// the rendering table is [`crate::expression::sparql_ns_lowering`] — the seam
/// over the SPARQL parser's own `builtin_function_keyword`. Deriving the text
/// here rather than carrying it is what keeps that table the single source of
/// truth for what `sparql:add` MEANS.
fn lower_sparql_call(iri: &NamedNode, arity: usize) -> Result<String, ShapesProductError> {
    let local = iri.as_str().strip_prefix(sparql_ns::NS).ok_or_else(|| {
        unsupported(format!(
            "this product records <{}> as a `sparql:` node-expression call, but the IRI is not in \
             the SPARQL 1.2 term vocabulary <{}>; re-prepare the product from its shapes graph",
            iri.as_str(),
            sparql_ns::NS
        ))
    })?;
    let form = sparql_ns_lowering(local).map_err(|reason| {
        unsupported(format!(
            "this product calls the SPARQL 1.2 name `{local}`, which this build cannot lower to a \
             SPARQL expression ({reason}); prepare the product with the SAME PurRDF build that \
             will execute it, because the built-in function table decides what the name means"
        ))
    })?;
    form.render(iri.as_str(), arity).map_err(|reason| {
        unsupported(format!(
            "this product calls `{local}` with {arity} arguments, which this build cannot render \
             ({reason}); re-prepare the product from its shapes graph"
        ))
    })
}

// ---------------------------------------------------------------------------
// The custom-function declaration table
// ---------------------------------------------------------------------------

/// Collect every custom node-expression function a shapes graph can call.
///
/// Keyed by IRI and inserted BEFORE its body is walked, so a function whose body
/// calls itself terminates. Two distinct handles sharing one IRI are refused
/// rather than unified: the parser interns one handle per declaration precisely
/// so every call site shares it, and quietly picking one of two would give half
/// the call sites a body they were never written against.
struct FnTable {
    /// IRI → declaration handle, in the key order the table is written in.
    by_iri: BTreeMap<String, Arc<CustomFunction>>,
    /// The live walk depth, bounded exactly as encoding is.
    depth: u32,
}

impl FnTable {
    /// Collect the table reachable from `shapes`.
    fn collect(shapes: &Shapes) -> Result<Vec<Arc<CustomFunction>>, ShapesProductError> {
        let mut table = Self {
            by_iri: BTreeMap::new(),
            depth: 0,
        };
        for shape in &shapes.node_shapes {
            table.shape(shape)?;
        }
        Ok(table.by_iri.into_values().collect())
    }

    /// Open one level of the walk.
    fn enter(&mut self) -> Result<(), ShapesProductError> {
        if self.depth >= MAX_DEPTH {
            return Err(depth_limit());
        }
        self.depth += 1;
        Ok(())
    }

    /// Close one level of the walk.
    fn leave(&mut self) {
        self.depth -= 1;
    }

    /// Record `func` and walk its body once.
    fn declare(&mut self, func: &Arc<CustomFunction>) -> Result<(), ShapesProductError> {
        if let Some(existing) = self.by_iri.get(func.iri.as_str()) {
            if Arc::ptr_eq(existing, func) {
                return Ok(());
            }
            return Err(malformed(format!(
                "this shapes graph declares the custom node-expression function <{}> twice, as two \
                 separate declarations; re-parse the shapes graph so every call site shares ONE \
                 declaration, because a product can record only one body per name",
                func.iri.as_str()
            )));
        }
        // Recorded BEFORE the body is walked: a self-recursive body is legal, and
        // this is what makes walking it terminate.
        self.by_iri
            .insert(func.iri.as_str().to_owned(), Arc::clone(func));
        if let Some(body) = func.body.get() {
            self.node_expr(body)?;
        }
        Ok(())
    }

    /// Walk a node shape.
    fn shape(&mut self, shape: &Shape) -> Result<(), ShapesProductError> {
        self.enter()?;
        for constraint in &shape.constraints {
            self.constraint(constraint)?;
        }
        for property in &shape.property_shapes {
            self.property_shape(property)?;
        }
        for rule in &shape.rules {
            match &rule.body {
                RuleBody::Triple {
                    subject,
                    predicate,
                    object,
                } => {
                    self.node_expr(subject)?;
                    self.node_expr(predicate)?;
                    self.node_expr(object)?;
                }
                RuleBody::Sparql { construct: _ } => {}
            }
            for condition in &rule.conditions {
                self.shape(condition)?;
            }
        }
        self.leave();
        Ok(())
    }

    /// Walk a property shape.
    fn property_shape(&mut self, shape: &PropertyShape) -> Result<(), ShapesProductError> {
        self.enter()?;
        for constraint in &shape.constraints {
            self.constraint(constraint)?;
        }
        for nested in &shape.property_shapes {
            self.property_shape(nested)?;
        }
        for reifier in &shape.reifier_shapes {
            self.shape(reifier)?;
        }
        self.leave();
        Ok(())
    }

    /// Walk a constraint.
    ///
    /// Wildcard-free, so a new `Constraint` arm that can reach a node expression
    /// cannot be added without deciding whether the table has to see it.
    fn constraint(&mut self, constraint: &Constraint) -> Result<(), ShapesProductError> {
        self.enter()?;
        match constraint {
            Constraint::Class(_)
            | Constraint::Datatype(_)
            | Constraint::NodeKind(_)
            | Constraint::MinCount(_)
            | Constraint::MaxCount(_)
            | Constraint::In(_)
            | Constraint::HasValue(_)
            | Constraint::Pattern { .. }
            | Constraint::MinLength(_)
            | Constraint::MaxLength(_)
            | Constraint::UniqueLang(_)
            | Constraint::LanguageIn(_)
            | Constraint::Closed { .. }
            | Constraint::MinInclusive(_)
            | Constraint::MaxInclusive(_)
            | Constraint::MinExclusive(_)
            | Constraint::MaxExclusive(_)
            | Constraint::Sparql { .. }
            | Constraint::Equals(_)
            | Constraint::Disjoint(_)
            | Constraint::SubsetOf(_)
            | Constraint::LessThan(_)
            | Constraint::LessThanOrEquals(_)
            | Constraint::MinListLength(_)
            | Constraint::MaxListLength(_)
            | Constraint::UniqueMembers(_)
            | Constraint::SingleLine(_)
            | Constraint::RootClass(_)
            | Constraint::UniqueValuesFor { .. }
            | Constraint::Component { .. } => {}
            Constraint::Not(shape)
            | Constraint::Node(shape)
            | Constraint::MemberShape(shape)
            | Constraint::SomeValue(shape) => {
                self.shape(shape)?;
            }
            Constraint::And(shapes) | Constraint::Or(shapes) | Constraint::Xone(shapes) => {
                for shape in shapes {
                    self.shape(shape)?;
                }
            }
            Constraint::QualifiedValueShape {
                shape, siblings, ..
            } => {
                self.shape(shape)?;
                for sibling in siblings {
                    self.shape(sibling)?;
                }
            }
            Constraint::Expression { expr, .. } | Constraint::NodeByExpression { expr, .. } => {
                self.node_expr(expr)?;
            }
        }
        self.leave();
        Ok(())
    }

    /// Walk the shape argument of `shnex:conformsToShape`.
    fn shape_arg(&mut self, arg: &ShapeArg) -> Result<(), ShapesProductError> {
        match arg {
            ShapeArg::Named(shape) => self.shape(shape),
            ShapeArg::Computed { expr, .. } => self.node_expr(expr),
        }
    }

    /// Walk a node expression.
    ///
    /// Wildcard-free for the same reason [`Self::constraint`] is.
    fn node_expr(&mut self, expr: &NodeExpr) -> Result<(), ShapesProductError> {
        self.enter()?;
        match expr {
            NodeExpr::Constant(_)
            | NodeExpr::This
            | NodeExpr::Path(_)
            | NodeExpr::Arg(_)
            | NodeExpr::Empty
            | NodeExpr::Var(_)
            | NodeExpr::List(_)
            | NodeExpr::InstancesOf(_)
            | NodeExpr::Select { .. } => {}
            NodeExpr::Filter { nodes, shape }
            | NodeExpr::FindFirst { nodes, shape }
            | NodeExpr::MatchAll { nodes, shape } => {
                self.node_expr(nodes)?;
                self.shape(shape)?;
            }
            NodeExpr::Union(operands)
            | NodeExpr::Intersection(operands)
            | NodeExpr::Concat(operands) => {
                for operand in operands {
                    self.node_expr(operand)?;
                }
            }
            NodeExpr::If { cond, then, els } => {
                self.node_expr(cond)?;
                self.node_expr(then)?;
                self.node_expr(els)?;
            }
            NodeExpr::Count { of, .. }
            | NodeExpr::Distinct(of)
            | NodeExpr::Min(of)
            | NodeExpr::Max(of)
            | NodeExpr::Sum(of)
            | NodeExpr::Limit { of, .. }
            | NodeExpr::Offset { of, .. }
            | NodeExpr::Exists(of) => self.node_expr(of)?,
            NodeExpr::OrderBy { of, key, .. } => {
                self.node_expr(of)?;
                self.node_expr(key)?;
            }
            NodeExpr::Call(
                FnCall::Builtin { args, .. }
                | FnCall::UserDefined { args, .. }
                | FnCall::Sparql { args, .. },
            ) => {
                for arg in args {
                    self.node_expr(arg)?;
                }
            }
            NodeExpr::CustomCall { func, args } => {
                self.declare(func)?;
                for (_, arg) in args {
                    self.node_expr(arg)?;
                }
            }
            NodeExpr::PathValues { focus, .. } => self.node_expr(focus)?,
            NodeExpr::Remove { nodes, remove } => {
                self.node_expr(nodes)?;
                self.node_expr(remove)?;
            }
            NodeExpr::FlatMap { nodes, map } => {
                self.node_expr(nodes)?;
                self.node_expr(map)?;
            }
            NodeExpr::NodesMatching(shape) => self.shape(shape)?,
            NodeExpr::ConformsToShape { node, shape } => {
                self.node_expr(node)?;
                self.shape_arg(shape)?;
            }
        }
        self.leave();
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// The public codec
// ---------------------------------------------------------------------------

/// Every custom node-expression function declaration reachable from `shapes`,
/// keyed and ordered by IRI — the same table [`encode_ast`] writes.
///
/// Exposed because the restore path has to assemble a
/// [`UserFunctionRegistry`](purrdf_sparql_eval::UserFunctionRegistry) from a
/// shapes graph it did NOT decode from this codec (the rebuild seam re-derives
/// its shapes from the carried dataset), and the registration it must reproduce
/// is a function of exactly this table. Sharing the walk is what keeps the two
/// paths from disagreeing about which declarations a shapes graph has.
///
/// # Errors
///
/// [`ProductDimension::DepthLimit`] when the walk nests past [`MAX_DEPTH`];
/// [`ProductDimension::Malformed`] when one IRI names two separate declarations.
pub(crate) fn custom_functions(
    shapes: &Shapes,
) -> Result<Vec<Arc<CustomFunction>>, ShapesProductError> {
    FnTable::collect(shapes)
}

/// Encode the declarative half of a parsed shapes graph.
///
/// # Errors
///
/// [`ProductDimension::DepthLimit`] when the model nests past [`MAX_DEPTH`];
/// [`ProductDimension::Malformed`] when the model itself is internally
/// inconsistent (two declarations of one custom function, a non-finite
/// `sh:order`); [`ProductDimension::UnsupportedCapability`] for a
/// SPARQL-based node expression spelled under a key this build does not know.
pub(crate) fn encode_ast(
    shapes: &Shapes,
    classes: &ClassCatalog,
) -> Result<Vec<u8>, ShapesProductError> {
    let functions = custom_functions(shapes)?;
    let fn_index: BTreeMap<String, u64> = functions
        .iter()
        .enumerate()
        .map(|(index, func)| (func.iri.as_str().to_owned(), index as u64))
        .collect();

    let mut writer = AstWriter::new(fn_index);

    // 1. Declarations, key-sorted (a `BTreeMap`'s own order).
    writer.count(functions.len());
    for func in &functions {
        writer.text(func.iri.as_str());
        writer.custom_fn_kind(func.kind);
        writer.count(func.params.len());
        for param in &func.params {
            writer.arg_key(param);
        }
        writer.count(func.required);
    }
    // 2. Bodies, in the same order — a separate pass, because a body may call any
    //    declaration including its own.
    for func in &functions {
        match func.body.get() {
            None => writer.flag(false),
            Some(body) => {
                writer.flag(true);
                writer.node_expr(body)?;
            }
        }
    }
    // 3. Node shapes.
    writer.count(shapes.node_shapes.len());
    for shape in &shapes.node_shapes {
        writer.shape(shape)?;
    }
    // 4. The caller-supplied box-role vocabulary.
    match &shapes.box_role_vocab {
        None => writer.flag(false),
        Some(vocab) => {
            writer.flag(true);
            writer.box_role_vocab(vocab);
        }
    }
    // 5. `sh:SPARQLTargetType` declarations, key-sorted (a `BTreeMap`'s own
    //    order — no hash-ordered container is ever written).
    writer.count(shapes.target_types.len());
    for (iri, target_type) in &shapes.target_types {
        writer.text(iri);
        writer.sparql_target_type(target_type)?;
    }
    // 6. The shapes-graph IRI.
    writer.opt_text(shapes.shapes_graph.as_deref());
    // 7. The reusable class analysis.
    encode_classes(&mut writer, classes);

    Ok(writer.out)
}

/// Write the reusable class analysis: every planned class IRI and the position its
/// resolved term id occupies in a validation plan's binding row, sorted by IRI.
///
/// # Why the product carries this at all
///
/// A prepared product exists so a consumer can restore a validator without RDF
/// reparsing, without shape extraction and without repeating the shared analysis.
/// The class walk IS that shared analysis: a cycle-aware traversal of every shape,
/// every nested property shape, every constraint and every node expression in the
/// graph. A product that carried only the walk's DIGEST would let a restore prove
/// its own walk agreed — after running it, on every restore, in every process,
/// forever. Proving the analysis is right is not the same as not having to do it,
/// and only the second is what the artifact was for.
///
/// # Why carrying it is not a stale-analysis hazard
///
/// It would be, if nothing bound the body to the build that reads it. The argument
/// against carrying a body was never about bytes: a product written by a build whose
/// reachability rule differed — a bug, or simply a later refinement — would restore
/// THAT build's catalog and validate against it, verified and wrong, with every
/// digest agreeing because the digest was written by the same wrong walk.
///
/// Two checks close that, and they close different halves of it:
///
/// * the STAGE ID covers the derivation. It is digested from the class walk's own
///   source — `ClassCatalog::from_walk` and the `lower_*` functions of the one
///   total shape walk — so a build whose reachability rule differs at all cannot share a
///   stage id with this one, and its products are refused by `admit` and sent to
///   `rebuild`, which re-derives and ignores what was carried. Before the analysis
///   travelled, that hazard was open in the other direction and unguarded: the walk
///   is an algorithm, not a type, so changing the rule without touching a model type
///   left the stage id standing while the meaning moved.
/// * the IDENTITY DIGEST covers the body. Row 10 of a product's identity is
///   `class_catalog_digest` over these same pairs, positions included, and the
///   admit seam checks the carried body against it before any preparation exists.
///   A body that is not the one this product was written with is refused on the
///   class-catalog dimension.
///
/// Byte-deterministic: the pairs go out sorted by class IRI, never in the backing
/// map's iteration order, which is not a fact about the catalog at all.
fn encode_classes(writer: &mut AstWriter, classes: &ClassCatalog) {
    let mut entries: Vec<(&str, usize)> = classes
        .entries()
        .map(|(class, position)| (class.as_str(), position))
        .collect();
    entries.sort_unstable();

    writer.count(entries.len());
    for (class, position) in entries {
        writer.text(class);
        // `uint`, not `count`: a position is an INDEX into the binding row, not a
        // count of elements that follow it, and the count reader's "every element
        // occupies at least one byte" bound is false of an index. The last entry's
        // position sits at the very end of the section with nothing behind it, so
        // reading it as a count refuses every catalog whose largest position
        // exceeds the bytes left — which is every catalog of more than one class.
        writer.uint(position as u64);
    }
}

/// Read the reusable class analysis back — see [`encode_classes`] for why it is
/// carried and what still binds it.
///
/// # Errors
///
/// [`ProductDimension::Truncated`] when the section ends inside the table;
/// [`ProductDimension::Malformed`] when the pairs are not a catalog any walk could
/// have produced — see [`ClassCatalog::from_entries`] for the two conditions and
/// for why the rank each class would have been GIVEN is deliberately not one of
/// them.
fn decode_classes(reader: &mut AstReader<'_>) -> Result<ClassCatalog, ShapesProductError> {
    let declared = reader.count()?;
    let mut entries = Vec::with_capacity(speculative_capacity(declared));
    for _ in 0..declared {
        let class = reader.named_node()?;
        // Read as a bare integer and let `from_entries` decide whether it is a
        // position at all. Bounding it here would be a second, weaker transcription
        // of the permutation rule, and the one thing an unchecked value could do —
        // drive an allocation — it cannot: the only vector it indexes is already
        // sized by `declared`, which the count reader bounded.
        let position = usize::try_from(reader.uint()?).unwrap_or(usize::MAX);
        entries.push((class, position));
    }
    ClassCatalog::from_entries(entries).ok_or_else(|| {
        malformed(format!(
            "this product carries a class analysis of {declared} entries whose positions are not \
             one arrangement of the {declared} slots a validation plan's binding row has, or whose \
             class IRIs are not distinct; re-prepare the product from its shapes graph, because \
             every class a plan resolves needs exactly one slot and a plan indexed past its own \
             row would abort rather than refuse"
        ))
    })
}

/// Encode a shapes graph alongside the class analysis its OWN shape tree derives.
///
/// The pairing [`PreparedShapes::to_product`](crate::engine::PreparedShapes::to_product)
/// makes, hoisted for the tests that are about this stream rather than about a
/// product: those hold a bare [`Shapes`] and no preparation, and re-deriving the
/// catalog at each call site would be dozens of transcriptions of the one line that
/// says what field 7 holds.
///
/// Test-only, and that is the point — production has a preparation in hand and must
/// write the catalog THAT preparation carries. A non-test helper that derived one
/// here would be a second way to answer the question, sitting next to the writer
/// that must not use it.
///
/// # Errors
///
/// Every dimension [`encode_ast`] refuses on.
#[cfg(test)]
pub(crate) fn encode_ast_derived(shapes: &Shapes) -> Result<Vec<u8>, ShapesProductError> {
    encode_ast(
        shapes,
        crate::plan::lower_shapes(shapes.node_shapes.iter()).classes(),
    )
}

/// Decode the declarative half of a prepared product.
///
/// # Errors
///
/// [`ProductDimension::UnsupportedCapability`] for a variant tag or a
/// `sparql:<NAME>` this build does not implement; [`ProductDimension::DepthLimit`]
/// past [`MAX_DEPTH`]; [`ProductDimension::Truncated`] when the bytes end inside a
/// declared structure; [`ProductDimension::Malformed`] for any other structural
/// inconsistency, trailing bytes included.
pub(crate) fn decode_ast(bytes: &[u8]) -> Result<AstParts, ShapesProductError> {
    let mut reader = AstReader::new(bytes);

    // 1. Materialize every declaration BEFORE any body or shape is read, so a
    //    call — including a body's call to its own function — resolves to an
    //    existing handle by index instead of recursing into a definition that
    //    reaches back.
    let declarations = reader.count()?;
    let mut functions = Vec::with_capacity(speculative_capacity(declarations));
    for _ in 0..declarations {
        let iri = reader.named_node()?;
        let kind = reader.custom_fn_kind()?;
        let params = reader.seq(AstReader::arg_key)?;
        let required = reader.count()?;
        functions.push(Arc::new(CustomFunction {
            iri,
            kind,
            params,
            required,
            body: OnceLock::new(),
        }));
    }
    reader.functions = functions;

    // 2. Bodies, installed into the handles minted above.
    for index in 0..declarations {
        if reader.flag()? {
            let body = reader.node_expr()?;
            let func = Arc::clone(&reader.functions[index]);
            func.body.set(body).map_err(|_| {
                malformed(format!(
                    "this product installs two bodies for the custom node-expression function <{}>; \
                     re-prepare the product from its shapes graph",
                    func.iri.as_str()
                ))
            })?;
        }
    }

    // 3..6. The shapes-level fields, in stream order.
    let node_shapes = reader.seq(AstReader::shape)?;
    let box_role_vocab = if reader.flag()? {
        Some(reader.box_role_vocab()?)
    } else {
        None
    };
    let target_type_count = reader.count()?;
    let mut target_types = BTreeMap::new();
    for _ in 0..target_type_count {
        let iri = reader.text()?;
        let target_type = reader.sparql_target_type()?;
        if target_types.insert(iri.clone(), target_type).is_some() {
            return Err(malformed(format!(
                "this product declares the `sh:SPARQLTargetType` <{iri}> twice; re-prepare the \
                 product from its shapes graph, because a target-type IRI names exactly one \
                 declaration"
            )));
        }
    }
    let shapes_graph = reader.opt_text()?;

    // 7. The reusable class analysis, after the model it was derived from.
    let classes = decode_classes(&mut reader)?;

    if reader.pos != bytes.len() {
        return Err(malformed(format!(
            "this product carries {} bytes after the end of its shapes AST; re-prepare the \
             product, because a writer that produced this section produced exactly {} bytes and \
             ignoring the remainder is how a partial write passes for a whole one",
            bytes.len() - reader.pos,
            reader.pos
        )));
    }

    Ok(AstParts {
        node_shapes,
        box_role_vocab,
        target_types,
        shapes_graph,
        custom_functions: reader.functions,
        shape_index: reader.shape_index,
        classes,
    })
}

#[cfg(test)]
mod tests;
