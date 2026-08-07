// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The evaluator's typed error channel.
//!
//! Per the project `no-optionality` / hard-fail doctrine, every condition that is
//! not a valid in-scope result is a typed error — there is no lenient mode and no
//! silent degradation. An unsupported algebra node or an unimplemented builtin is
//! [`EvalError::Unsupported`], not a best-effort answer.
//!
//! # This channel is disjoint from the governor channel, and outranks it
//!
//! A governed execution can end in a **truncated** solution sequence
//! ([`GovernedOutcome::BudgetExhausted`](crate::GovernedOutcome)), which is not a
//! contradiction of the paragraph above and is deliberately not an [`EvalError`]. The
//! doctrine bans answering a question *wrongly*; a governor answers a different, honestly
//! labelled question — "what had been established when the ceiling was reached" — and it
//! can only do so because the certificate travelling with those rows says which bound they
//! are. A partial sequence that arrived here instead would be exactly the silent
//! degradation the doctrine forbids, because an [`EvalError`] carries no such certificate
//! and a caller reducing one to "the query failed" would discard rows it could have used.
//!
//! The two channels therefore never merge, and where they meet the precedence is fixed: an
//! [`EvalError`] outranks **every** governor. Reporting an exhausted budget for a query
//! that could not have been answered at all would hand a caller a partial answer to a
//! question that has none — which is the same falsehood the hard-fail rule exists to
//! prevent, merely wearing a receipt.

use purrdf_sparql_algebra::ParseError;

/// An error raised while evaluating a SPARQL query.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EvalError {
    /// A query failed to parse in [`purrdf_sparql_algebra`]. Carries the rendered
    /// parse error.
    Parse(String),

    /// A well-formed construct this evaluator does not (or cannot) evaluate.
    ///
    /// This is the hard-fail boundary. `SERVICE` federation, `LATERAL`, and SPARQL
    /// `UPDATE` are all evaluated in-engine, so none of them surfaces here; what
    /// remains is a narrow, enumerated residue: a variable-bound quoted-triple-term
    /// component in a BGP or property-path pattern (structural triple-term matching
    /// is out of scope), an unresolved custom SPARQL function or aggregate IRI,
    /// `heldIn` called without a caller-supplied standpoint-predicate configuration,
    /// and a manually constructed graph pattern whose nesting exceeds the parser's
    /// safety bound. The string names the unsupported construct. (Property paths are
    /// evaluated in-engine — S8 — and `DESCRIBE` evaluates via the canonical
    /// Symmetric CBD, so neither is here either.)
    Unsupported(String),

    /// An internal invariant was violated — e.g. a solution row whose width does
    /// not match its schema. This indicates a bug in the evaluator, not bad input
    /// (a frozen, validated dataset and a parsed algebra cannot legitimately cause
    /// it); it is surfaced rather than panicking so callers fail cleanly.
    Internal(String),

    /// A `SERVICE` federation step failed (transport error, undecodable remote
    /// response, or no remote source configured) and the `SERVICE` was **not**
    /// `SILENT`. Per the hard-fail doctrine a non-silent federation failure aborts
    /// the query rather than silently contributing no bindings; `SERVICE SILENT`
    /// instead swallows the failure to the join identity.
    Remote(String),

    /// The dataset carries structurally malformed RDF that a builtin cannot
    /// interpret — e.g. a cyclic `rdf:List` (a cell reachable from itself) or a
    /// list cell missing its `rdf:first`/`rdf:rest` edge. Distinct from
    /// [`EvalError::Internal`] (an evaluator bug over valid data) and
    /// [`EvalError::Unsupported`] (a valid construct out of scope): this is bad
    /// *input*. Per the hard-fail doctrine it aborts the query loudly rather than
    /// looping forever or guessing an answer.
    Data(String),

    /// A user function call was invalid — either a SHACL-AF SPARQL-based function
    /// (`sh:SPARQLFunction`: an arity mismatch, a
    /// `sh:datatype`/`sh:nodeKind`/`sh:returnType` violation, or exceeding the
    /// user-function recursion bound) or a native (host-Rust closure) function
    /// (an arity mismatch, the closure's own returned `Err`, or a caught panic
    /// inside the closure). Per the hard-fail doctrine a mis-invoked function
    /// aborts the query rather than yielding a wrong or unbound value.
    Function(String),

    /// A caller supplied an invalid evaluation-configuration parameter to an
    /// `EvalCtx` builder method -- e.g. a deterministic blank-mint prefix
    /// (`EvalCtx::with_bnode_mint_prefix`) that is not a legal
    /// `BLANK_NODE_LABEL` prefix. Distinct from [`EvalError::Data`], which is
    /// about the dataset being evaluated rather than the caller's evaluation
    /// configuration: per the hard-fail doctrine, an out-of-alphabet
    /// configuration parameter is rejected at the setter rather than left to
    /// surface later as a silently rewritten label at egress.
    Config(String),
}

impl EvalError {
    /// Construct an [`EvalError::Unsupported`] from any displayable construct name.
    pub fn unsupported(what: impl Into<String>) -> Self {
        Self::Unsupported(what.into())
    }

    /// Construct an [`EvalError::Internal`] from any displayable message.
    pub fn internal(what: impl Into<String>) -> Self {
        Self::Internal(what.into())
    }

    /// Construct an [`EvalError::Remote`] from any displayable message.
    pub fn remote(what: impl Into<String>) -> Self {
        Self::Remote(what.into())
    }

    /// Construct an [`EvalError::Data`] from any displayable message.
    pub fn data(what: impl Into<String>) -> Self {
        Self::Data(what.into())
    }

    /// Construct an [`EvalError::Function`] from any displayable message.
    pub fn function(what: impl Into<String>) -> Self {
        Self::Function(what.into())
    }

    /// Construct an [`EvalError::Config`] from any displayable message.
    pub fn config(what: impl Into<String>) -> Self {
        Self::Config(what.into())
    }
}

impl core::fmt::Display for EvalError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Parse(msg) => write!(f, "SPARQL parse error: {msg}"),
            Self::Unsupported(what) => {
                write!(f, "unsupported in sparql-eval (S6 scope): {what}")
            }
            Self::Internal(msg) => write!(f, "internal evaluator error: {msg}"),
            Self::Remote(msg) => write!(f, "SERVICE federation error: {msg}"),
            Self::Data(msg) => write!(f, "malformed RDF input: {msg}"),
            Self::Function(msg) => write!(f, "user function error: {msg}"),
            Self::Config(msg) => write!(f, "invalid evaluation configuration: {msg}"),
        }
    }
}

impl std::error::Error for EvalError {}

impl From<ParseError> for EvalError {
    fn from(err: ParseError) -> Self {
        Self::Parse(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_error_converts_and_renders() {
        let pe = ParseError::Unsupported("VALUES with mixed arity".to_owned());
        let ee: EvalError = pe.into();
        assert!(matches!(ee, EvalError::Parse(_)));
        assert!(ee.to_string().contains("parse error"));
    }

    #[test]
    fn unsupported_names_the_construct() {
        let e = EvalError::unsupported("SERVICE");
        assert!(e.to_string().contains("SERVICE"));
        assert!(e.to_string().contains("scope"));
    }
}
