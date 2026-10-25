// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One compiled, anchored Unicode glob, with fallible compilation and metered,
//! allocation-free execution. See the shipped [language](../SPEC.md).
#![forbid(unsafe_code)]
#![doc = include_str!("../SPEC.md")]

mod compile;
mod execute;
pub mod rdf;
mod set;

pub use set::{CompiledGlobSet, SetCompileError, SetMatchError, SetMatchReport};

pub use purrdf_lex::allocation::{Admission, Resident, StorageError};

/// The exact language and execution law implemented by this crate.
pub const LANGUAGE_VERSION: &str = "purrdf-glob-v1";

/// Whether slashes are ordinary text or path-component boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Wildcards can consume any Unicode scalar, including a slash.
    Text,
    /// Ordinary wildcards cannot cross a slash; whole-component `**` can.
    Path,
}

/// Explicit selection from an already parsed IRI reference, without decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IriComponent {
    /// The original, complete lexical spelling.
    Lexical,
    /// Scheme without its colon.
    Scheme,
    /// Authority without its leading slashes.
    Authority,
    /// Path exactly as spelled, including its slashes.
    Path,
    /// Query without its leading question mark.
    Query,
    /// Fragment without its leading hash.
    Fragment,
}

impl purrdf_hash::error::StructuredError for IriComponent {
    purrdf_hash::error_visit! {
        Lexical => "glob-iri-lexical" {}, Scheme => "glob-iri-scheme" {},
        Authority => "glob-iri-authority" {}, Path => "glob-iri-path" {},
        Query => "glob-iri-query" {}, Fragment => "glob-iri-fragment" {},
    }
}

/// A closed syntax refusal; offsets always refer to the original pattern bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxReason {
    /// A backslash has no following scalar.
    TrailingEscape,
    /// A bracket class has no closing bracket.
    UnclosedClass,
    /// A bracket class has no member.
    EmptyClass,
    /// A range's final scalar precedes its first scalar.
    ReversedRange,
    /// Unescaped brace alternatives belong to a different language version.
    BraceAlternative,
    /// POSIX named bracket classes are not part of this language.
    NamedClass,
}

impl purrdf_hash::error::StructuredError for SyntaxReason {
    purrdf_hash::error_visit! {
        TrailingEscape => "glob-trailing-escape" {}, UnclosedClass => "glob-unclosed-class" {},
        EmptyClass => "glob-empty-class" {}, ReversedRange => "glob-reversed-range" {},
        BraceAlternative => "glob-brace-alternative" {},
        NamedClass => "glob-named-class" {},
    }
}

/// An original syntax or physical storage failure during compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileError {
    /// Invalid pattern, with the original byte offset and closed reason.
    Syntax {
        /// Zero-based pattern byte offset.
        offset: usize,
        /// The violated language rule.
        reason: SyntaxReason,
    },
    /// The shared native allocator/admission failure, preserved verbatim.
    Storage(StorageError),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Syntax { offset, reason } => {
                write!(f, "glob syntax at byte {offset}: {reason:?}")
            }
            Self::Storage(cause) => cause.fmt(f),
        }
    }
}
purrdf_hash::error_sources!(CompileError, Storage);
purrdf_lex::variant_from!(CompileError { Storage(StorageError) });
impl purrdf_hash::error::StructuredError for CompileError {
    purrdf_hash::error_visit! {
        Syntax { offset, reason } => "glob-syntax" {
            "offset" => ByteCount(*offset as u128), "reason" => Cause(reason)
        },
        Storage(cause) => "glob-storage" { "cause" => Cause(cause) },
    }
}

/// A completed decision and the exact deterministic work consumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchReport {
    /// Whether the entire selected surface matches.
    pub matched: bool,
    /// Number of charged token/predicate dispatches and retry scalars.
    pub work: u64,
}

/// Refusal to publish a decision, distinct from a completed negative match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchError {
    /// The next unit would exceed the stated work allowance.
    WorkExceeded {
        /// Work admitted before this refusal.
        consumed: u64,
        /// Original caller allowance.
        limit: u64,
        /// Byte offset in the selected input surface at the refused unit.
        offset: usize,
    },
}
impl std::fmt::Display for MatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WorkExceeded {
                consumed,
                limit,
                offset,
            } => write!(
                f,
                "glob work refused at input byte {offset}: {} units requested, {limit} permitted ({consumed} consumed)",
                u128::from(*consumed) + 1
            ),
        }
    }
}
impl std::error::Error for MatchError {}
impl purrdf_hash::error::StructuredError for MatchError {
    purrdf_hash::error_visit! {
        WorkExceeded { consumed, limit, offset } => "glob-work-exceeded" {
            "consumed" => Unsigned(u128::from(*consumed)),
            "attempted" => Unsigned(u128::from(*consumed) + 1),
            "limit" => Unsigned(u128::from(*limit)), "offset" => ByteCount(*offset as u128)
        },
    }
}

#[derive(Debug, Clone, Copy)]
enum Token {
    Literal(char),
    Separator,
    Any,
    Star,
    Directories,
    Tail,
    Class {
        start: usize,
        end: usize,
        negated: bool,
    },
}

type Range = purrdf_lex::terminals::ScalarRange;

#[derive(Debug)]
struct Account<S: Admission>(S);
impl<S: Admission> Drop for Account<S> {
    fn drop(&mut self) {
        // All payload fields have already died. A refusing admission retains
        // its own sticky cause/conservative charge until its owner is dropped.
        let _ = self.0.resize(0);
    }
}

/// One immutable reusable program, owning its original physical admission.
/// Matching borrows this value and allocates nothing. It deliberately has no
/// deep-clone operation or alternate regex implementation.
#[derive(Debug)]
pub struct CompiledGlob<S: Admission = Resident> {
    // Field order releases payloads before the account, including on unwind.
    tokens: Vec<Token>,
    ranges: Vec<Range>,
    mode: Mode,
    _account: Account<S>,
}

impl CompiledGlob {
    /// Compile under resident physical storage, without a logical pattern cap.
    /// # Errors
    /// Returns original syntax, checked layout or native allocation refusal.
    pub fn compile(pattern: &str, mode: Mode) -> Result<Self, CompileError> {
        Self::compile_with_storage(pattern, mode, Resident)
    }
}

impl<S: Admission> CompiledGlob<S> {
    /// Compile under a dedicated, initially empty physical admission account.
    /// The returned plan retains the account until its buffers have died.
    /// Validation/counting allocate nothing; exactly sized arrays are admitted
    /// before their fallible construction. No input text is retained or copied.
    /// # Errors
    /// Preserves the original syntax or native storage error. Failed buffers
    /// die before their grant is refunded. Account shrink failures remain with
    /// the original admission owner, including after an earlier refusal.
    pub fn compile_with_storage(
        pattern: &str,
        mode: Mode,
        storage: S,
    ) -> Result<Self, CompileError> {
        compile::compile(pattern, mode, storage)
    }

    /// Decide one literal's verbatim lexical form, with an explicit work limit.
    /// `u64::MAX` admits every representable work count. A refusal is no decision.
    /// # Errors
    /// Returns exact work exhaustion, including a further unit beyond `u64::MAX`.
    pub fn match_literal(&self, lexical: &str, max_work: u64) -> Result<MatchReport, MatchError> {
        execute::matches(&self.tokens, &self.ranges, self.mode, lexical, max_work)
    }

    /// Decide an explicitly selected component of a parsed IRI reference.
    /// No case fold, normalization or percent decoding is performed.
    /// # Errors
    /// An absent optional component is a completed negative decision, charged
    /// one selection predicate. A present empty component executes normally.
    /// Returns exact work exhaustion.
    pub fn match_iri(
        &self,
        iri: &purrdf_iri::Iri,
        component: IriComponent,
        max_work: u64,
    ) -> Result<MatchReport, MatchError> {
        match iri_surface(iri, component) {
            Some(text) => self.match_literal(text, max_work),
            None => execute::absent(max_work),
        }
    }
}

fn iri_surface(iri: &purrdf_iri::Iri, component: IriComponent) -> Option<&str> {
    match component {
        IriComponent::Lexical => Some(iri.as_str()),
        IriComponent::Scheme => iri.scheme(),
        IriComponent::Authority => iri.authority(),
        IriComponent::Path => Some(iri.path()),
        IriComponent::Query => iri.query(),
        IriComponent::Fragment => iri.fragment(),
    }
}
