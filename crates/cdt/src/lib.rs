// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `purrdf-cdt` — the native **SPARQL composite datatypes** (SEP-0009) for the
//! RDF 1.2 query stack.
//!
//! SEP-0009 adds two datatypes to SPARQL — `cdt:List` and `cdt:Map` — whose values
//! are ordered sequences and keyed collections of ordinary RDF terms, carried in the
//! *lexical form* of a single literal. This crate is the value layer for both: it
//! scans that lexical form, gives the result a canonical spelling, and implements
//! the spec's equality and ordering over it.
//!
//! # A closed leaf, on purpose
//!
//! `purrdf-cdt` depends on exactly two crates — [`purrdf_iri`] and [`purrdf_xsd`] —
//! and on **no** part of `purrdf-core`, in either direction. That is the whole point
//! of the shape: composite values must be reachable from the kernel, so the kernel
//! will depend on this crate, and a dependency in the other direction would close
//! the cycle. It therefore owns its own closed element type, [`CdtTerm`], which is
//! exactly what the grammar admits; converting to and from a host's term
//! representation happens in the consumer, above this crate.
//!
//! The crate is `#![no_std]` plus `alloc`, so it builds for
//! `wasm32-unknown-unknown` like every other release crate in the workspace.
//!
//! # PurRDF is not an ontology, and this is not minting
//!
//! Every IRI constant here is a fixed, third-party, spec-defined string:
//! [`CDT_LIST`] and [`CDT_MAP`] come from SEP-0009, and the `xsd:` / `rdf:` ones
//! from the W3C Recommendations the grammar's shorthands resolve into. They are the
//! spelling the grammar is written in, not caller-supplied vocabulary, and there is
//! no default being fabricated for anything.
//!
//! # Two PurRDF supersets of the SEP-0009 lexical space
//!
//! Both extend the *lexical* space only. No IRI is minted and the datatype stays
//! `cdt:List` / `cdt:Map`. Each form is emitted only when such a term is actually
//! present — that is, only for values SEP-0009 cannot express at all — so any value
//! SEP-0009 *can* express is written in SEP-0009's own lexical space and conformance
//! is preserved.
//!
//! ## Superset 1 — RDF 1.2 triple terms
//!
//! Productions `[3]` and `[8]` gain one alternative:
//!
//! ```text
//! TripleTerm ::= '<<(' Element Element Element ')>>'
//! ```
//!
//! Without it, folding a triple-term binding into a `cdt:List` would have no lexical
//! form at all and could only raise — refusing an RDF 1.2 term type outright. RDF 1.2
//! is the whole point of this toolkit and triple terms are first-class in it, so
//! refusal is not an admissible outcome; the toolkit carries the term and spells it.
//!
//! ## Superset 2 — directional language-tagged literals
//!
//! `RDFLiteral`'s `LANGTAG` gains RDF 1.2's base-direction suffix, in **both**
//! directions, spelled exactly as the rest of this workspace already writes it:
//!
//! ```text
//! LANGTAG ::= '@' [a-zA-Z]+ ('-' [a-zA-Z0-9]+)* ('--' ('ltr' | 'rtl'))?
//! ```
//!
//! So `"مرحبا"@ar--rtl` and `"hello"@en--ltr` are both admitted, and both round-trip
//! through the canonical form. As with triple terms, refusing an `rdf:dirLangString`
//! is not an admissible outcome for either direction.
//!
//! # Termination, and the three bounds
//!
//! A CDT lexical form is attacker-controlled data inside a literal, and exhausting
//! the stack in Rust is an `abort` that no caller can catch. **Every** walk in this
//! crate — the scanner, the renderer, equality, ordering and the canonical mapping —
//! is therefore iterative over an explicit heap worklist; nothing recurses. On top
//! of that the crate owns three bounds: [`MAX_NESTING_DEPTH`], [`MAX_ELEMENTS`] and
//! [`MAX_LEXICAL_BYTES`], each with its governor justification in the [`limits`]
//! module. Exceeding one is a typed [`CdtError`] carrying a byte offset, never a
//! panic and never an abort.
//!
//! # The function library
//!
//! SEP-0009 also defines fifteen functions over these values — `cdt:List`,
//! `cdt:concat`, `cdt:get`, `cdt:merge`, `cdt:put` and the rest. [`functions`] is
//! the closed registry ([`CdtFn`], with each member's spec IRI and arity) and the
//! pure value-space operation behind each one, written against SEP-0009's own
//! conformance corpus with the pinning test named in every rustdoc. Functions that
//! *mint* a composite check all three bounds against the prospective result before
//! allocating any of it.
//!
//! # The canonical lexical form
//!
//! SEP-0009 defines none, so this crate chooses one — for values PurRDF *computes*,
//! with exactly the standing `purrdf_xsd::XsdValue::canonical_lexical` has for
//! minted XSD literals. It is a pure, deterministic function of the value: a fixed
//! separator spelling, IRIs as `<…>`, literals under the SPARQL escape set with
//! numeric and boolean elements always in explicit `"…"^^<…>` form, `null` for the
//! null element, and map entries written in the crate's total key order. See
//! [`render`] for the full form.
//!
//! # Examples
//!
//! ```rust
//! use purrdf_cdt::{list_less_than, parse_list, CdtValue};
//!
//! // Parse, canonicalize, and re-parse: the canonical form is a fixpoint.
//! let value = parse_list("[ 1 , 'two'@en--ltr , [ true ] ]")?;
//! let canonical = value.canonical_lexical();
//! assert_eq!(parse_list(&canonical)?.canonical_lexical(), canonical);
//!
//! // The spec's ordering is partial and raises where SPARQL `<` has no answer.
//! let items = |value: CdtValue| match value {
//!     CdtValue::List(items) => items,
//!     CdtValue::Map(_) => unreachable!("parse_list yields a list"),
//! };
//! assert_eq!(
//!     list_less_than(&items(parse_list("[1,2]")?), &items(parse_list("[1,3]")?)),
//!     Ok(true)
//! );
//! # Ok::<(), purrdf_cdt::CdtError>(())
//! ```
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/Blackcat-Informatics/purrdf/main/docs/purrdf-logo.svg"
)]
#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

pub mod datatype;
pub mod error;
pub mod functions;
pub mod limits;
pub mod literal;
pub mod ops;
pub mod parse;
pub mod render;
pub mod term;
pub mod value;

pub use datatype::{
    CDT_LIST, CDT_MAP, CDT_NS, CdtDatatype, RDF_DIR_LANG_STRING, RDF_LANG_STRING, XSD_BOOLEAN,
    XSD_DECIMAL, XSD_DOUBLE, XSD_INTEGER, XSD_STRING,
};
pub use error::{CdtError, CdtTypeError};
pub use functions::{
    CDT_FUNCTIONS, CdtArity, CdtFn, CdtOutcome, MapRemoval, concat, contains, contains_key, get,
    head, integer_argument, keys, list_concat, list_constructor, list_contains, list_get,
    list_head, list_reverse, list_size, list_subseq, list_tail, map_constructor, map_contains_key,
    map_get, map_keys, map_merge, map_put, map_remove, map_size, merge, put, remove, reverse, size,
    subseq, tail,
};
pub use limits::{MAX_ELEMENTS, MAX_LEXICAL_BYTES, MAX_NESTING_DEPTH};
pub use literal::{LiteralValue, parse_literal};
pub use ops::{
    list_equal, list_less_than, map_equal, map_less_than, term_equal, term_less_than,
    total_key_cmp, total_term_cmp, total_value_cmp, value_equal, value_less_than,
};
pub use parse::{parse_cdt, parse_cdt_by_iri, parse_list, parse_map};
pub use render::{canonical_key_lexical, canonical_lexical, canonical_lexical_len};
pub use term::{CdtEntry, CdtKey, CdtLiteral, CdtTerm, CdtTripleTerm, TextDirection};
pub use value::CdtValue;
