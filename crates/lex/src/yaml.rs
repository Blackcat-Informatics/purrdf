// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The workspace's one YAML 1.2 grammar, lexical syntax and JSON projection.
//!
//! [`read_syntax`] retains original source bytes, ordered occurrences, decoded
//! lexical scalars, styles, properties and trivia before Core-schema resolution.
//! Its borrowed [`SyntaxDocument`] carries a byte identity and explicit input,
//! live allocation, occurrence and depth bounds. [`SyntaxDocument::to_core`]
//! projects through the same parser and eager builder as [`read_with`].
//! [`read_syntax_stream`] retains ordered document roots and exact absolute
//! prologues, envelopes and start/end markers in one [`SyntaxArena`]. Its explicit
//! [`SyntaxStreamLimits`] admit the complete input, document metadata, decoded
//! text, occurrences, trivia and parser state under aggregate bounds. An absent
//! stream has no documents; an explicit empty document has an empty scalar root.
//! No later failure publishes a partial stream.
//!
//! Explicit tags retain their raw span and a separate lexical identity:
//! document-local handles expand their admitted `%TAG` prefixes, percent escapes
//! decode as UTF-8, and the bare `!` remains non-specific. Untagged nodes have no
//! explicit identity. Unknown handles are refused; unsupported specific tags
//! survive lexically for a later semantic projection to judge. Anchor names
//! shadow at declaration and reset per document. Only acyclic aliases are
//! supported: an alias to an in-progress anchor has its own positioned refusal.
//! Scalar-key uniqueness remains the lexical text/raw-tag law, rather than a
//! complete Core representation-key equality or YAML-LD composition proof.
//! [`emit_scalar`] shares the native quoting and escape bodies with [`write`](fn@write),
//! reports the actual safe style, and admits its complete measured output before
//! writing. Layout-policy consumers can use it without a local scalar encoder.
//!
//! The semantic reader and writer carry JSON-shaped data — LinkML schemas,
//! OKF frontmatter, YAML-LD — through [`crate::json::Value`]. Lexical syntax
//! preserves authored constructs before that projection. One scalar resolver
//! decides both what a plain
//! scalar denotes when it is read and whether a string must be quoted when it
//! is written: whatever [`write`](fn@write) leaves plain, [`read`] reads back as the same
//! string.
//!
//! # The emitter
//!
//! [`write`](fn@write) is deterministic and produces the block layout `serde_yaml`
//! produces (its layout decisions follow the reference emitter it drives), with
//! these deliberate differences:
//!
//! * **Numbers are written as their lexemes.** `serde_yaml` wrote a JSON number
//!   through `binary64`, so `1.50` became `1.5` and `1e+20` became `1e20`; here
//!   the lexeme is written as it is, and nothing is rounded.
//! * **A string holding U+2028 or U+2029 is double-quoted**, with `\L` and
//!   `\P`. The reference emitter wrote those characters raw inside a
//!   single-quoted or literal scalar, where a YAML 1.2 reader keeps them as
//!   content and misreads the indentation that follows them.
//! * **Strings use exact Core-schema classification for quoting.** Numeric
//!   spellings such as `007`, `1e400` and arbitrarily wide unsigned octal or
//!   hexadecimal forms are quoted. Binary (`0b101`) and signed radix-looking
//!   spellings (`-0x1F`) remain plain strings.
//!
//! # The reader
//!
//! [`read`] accepts one YAML 1.2 document in block or flow style, with
//! comments, directives, document markers, plain, single-quoted, double-quoted,
//! literal and folded scalars, explicit (`?`) keys, and anchors and aliases
//! (expanded, and counted against [`Limits::max_nodes`]; refused when
//! [`Limits::aliases`] is off, as YAML-LD's JSON profile requires). What JSON
//! cannot hold is refused with a typed [`Error`] at a byte offset: a tag other
//! than the core schema's, a key that is not a string (a number, boolean or null
//! key is accepted, as its source text, only under [`Limits::scalar_keys`]; a
//! collection key never is), a repeated key (YAML 1.2 §3.2.1.1 requires mapping
//! keys to be unique), `.inf` and `.nan`, and a second document. A stream with
//! no document reads as `null`.
//!
//! [`read_stream`] instead constructs every document in source order under
//! explicit [`StreamLimits`]. No-document streams return an empty vector;
//! explicit empty documents contribute null values. The allocation and expanded
//! node bounds cover the complete retained stream, while anchor bindings and
//! directives reset between documents. Later failures return no partial result.
//! This API uses the same documented scalar rules as [`read_with`]; it does not
//! select a different schema or introduce YAML-LD processing options.
//!
//! The reader is graded against the official yaml-test-suite
//! (`crates/lex/tests/yaml_test_suite.rs`, over the vendored
//! `vectors/yaml-test-suite`): every valid case inside the subset reads as the
//! suite's JSON, every invalid case is refused, and every valid case outside it
//! is refused by one of the kinds above, never read wrongly.
//!
//! Plain scalars resolve by the YAML 1.2.2 Core schema: signed decimal digits,
//! unsigned octal and hexadecimal integers, and finite decimal/exponent forms
//! keep their numeric type at arbitrary width. JSON-compatible number lexemes
//! retain their spelling (`1.50` stays `1.50`); other forms are normalized
//! exactly (`007` is `7`, `0x1F` is `31`, `+.5` is `0.5`). No machine float is
//! used. [`Limits::max_numeric_work`] bounds classification and conversion;
//! [`Limits::max_numeric_bytes`] bounds newly materialized numeric spellings.
//! Both accounts cover the complete stream; aliases remain governed by node
//! and allocation bounds. Exhaustion is a positioned typed refusal. Lexical
//! syntax retains the authored spelling before projection.
//! Mapping keys stay in document order. Nesting is bounded by
//! [`Limits::max_depth`], and neither direction spends a machine-stack frame
//! per nesting level.

mod admission;
mod build;
mod number;
mod read;
mod resolve;
mod syntax;
mod write;

pub use read::{Error, ErrorKind, Limits, StreamLimits, read, read_stream, read_with};
pub use syntax::{
    Attachment, BlockHeader, Chomping, CollectionStyle, ExplicitTag, NodeId, NodeKind, Position,
    ProjectionLimits, ScalarStyle, Span, SyntaxArena, SyntaxDocument, SyntaxDocumentRoot,
    SyntaxEntry, SyntaxEscape, SyntaxLimits, SyntaxNode, SyntaxScalar, SyntaxStream,
    SyntaxStreamLimits, Trivia, TriviaKind, read_syntax, read_syntax_stream,
};
pub use write::{
    EmitError, EmitLimits, EmitStyle, EmittedScalar, ScalarContext, emit_scalar, write,
};

#[cfg(test)]
mod tests;
