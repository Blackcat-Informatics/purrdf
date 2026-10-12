// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Explicit field-scoped lexical arguments without changing a relation's arity.
//!
//! The named encoding is an RDF 1.2 triple term `(field, rdf:value, needle)`.
//! The field is a caller-selected absolute predicate IRI; the needle is the
//! original literal, including its language and direction. Producers opt into
//! this encoding in their request-facet declaration. Ordinary literal arguments
//! keep their existing unscoped meaning.

mod counters;

pub use counters::{ConstraintCounts, ConstraintDecision, ScopeCounters, ScopeError};

use crate::{TermBox, TermValue, vocab::rdf};

/// A malformed declared lexical argument.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LexicalScopeError {
    /// A field must name a caller-selected absolute predicate IRI.
    InvalidPredicate,
    /// The argument is neither a literal nor the explicit scoped encoding.
    InvalidEncoding,
}

impl core::fmt::Display for LexicalScopeError {
    fn fmt(&self, out: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        out.write_str(match self {
            Self::InvalidPredicate => "lexical field scope requires an absolute predicate IRI",
            Self::InvalidEncoding => {
                "lexical argument requires a literal or a (field, rdf:value, literal) triple term"
            }
        })
    }
}

impl std::error::Error for LexicalScopeError {}

/// Borrowed original lexical payload and its optional declared field.
#[derive(Clone, Copy, Debug)]
pub struct LexicalQuery<'a> {
    /// The original literal, preserving every literal facet.
    pub needle: &'a TermValue,
    /// The original field predicate; scope changes membership, never weight.
    pub predicate: Option<&'a TermValue>,
}

impl<'a> LexicalQuery<'a> {
    /// Decode the explicitly declared lexical argument without copying payloads.
    ///
    /// # Errors
    /// Refuses malformed encodings and nonabsolute field predicates.
    pub fn decode(value: &'a TermValue) -> Result<Self, LexicalScopeError> {
        match value {
            TermValue::Literal { .. } => Ok(Self {
                needle: value,
                predicate: None,
            }),
            TermValue::Triple { s, p, o }
                if matches!(p.as_ref(), TermValue::Iri(iri) if iri == rdf::VALUE)
                    && matches!(o.as_ref(), TermValue::Literal { .. }) =>
            {
                validate_predicate(s)?;
                Ok(Self {
                    needle: o,
                    predicate: Some(s),
                })
            }
            _ => Err(LexicalScopeError::InvalidEncoding),
        }
    }

    /// Move a compiled request's original terms into the explicit encoding.
    ///
    /// This constructor is for owned compile-time request terms. Query-time
    /// decoding borrows the evaluator's already admitted original term owner.
    /// No term clone or new RDF result column is required.
    ///
    /// # Errors
    /// Refuses nonliteral needles or malformed field predicates.
    pub fn encode(
        needle: TermValue,
        predicate: Option<TermValue>,
    ) -> Result<TermValue, LexicalScopeError> {
        if !matches!(needle, TermValue::Literal { .. }) {
            return Err(LexicalScopeError::InvalidEncoding);
        }
        let Some(predicate) = predicate else {
            return Ok(needle);
        };
        validate_predicate(&predicate)?;
        Ok(TermValue::Triple {
            s: TermBox::new(predicate),
            p: TermBox::new(TermValue::iri(rdf::VALUE)),
            o: TermBox::new(needle),
        })
    }
}

fn validate_predicate(value: &TermValue) -> Result<(), LexicalScopeError> {
    if matches!(value, TermValue::Iri(iri)
        if purrdf_iri::is_absolute(iri).is_ok_and(|absolute| absolute))
    {
        Ok(())
    } else {
        Err(LexicalScopeError::InvalidPredicate)
    }
}

#[cfg(test)]
mod tests {
    use super::{LexicalQuery, LexicalScopeError};
    use crate::{TermBox, TermValue};

    #[test]
    fn scoped_decoder_borrows_original_payload_and_rejects_malformed_fields() {
        let plain = TermValue::simple_literal("alpha missing");
        let unscoped = LexicalQuery::decode(&plain).unwrap();
        assert!(std::ptr::eq(unscoped.needle, &raw const plain));
        assert!(unscoped.predicate.is_none());
        let encoded =
            LexicalQuery::encode(plain, Some(TermValue::iri("https://example.org/from"))).unwrap();
        let TermValue::Triple { s, o, .. } = &encoded else {
            panic!("genuine triple term");
        };
        let scoped = LexicalQuery::decode(&encoded).unwrap();
        assert!(std::ptr::eq(scoped.needle, o.as_ref()));
        assert!(std::ptr::eq(scoped.predicate.unwrap(), s.as_ref()));
        for predicate in [
            TermValue::iri("relative"),
            TermValue::simple_literal("field"),
        ] {
            assert_eq!(
                LexicalQuery::encode(TermValue::simple_literal("alpha"), Some(predicate)),
                Err(LexicalScopeError::InvalidPredicate)
            );
        }
        let malformed = TermValue::Triple {
            s: TermBox::new(TermValue::iri("https://example.org/from")),
            p: TermBox::new(TermValue::iri("https://example.org/other")),
            o: TermBox::new(TermValue::simple_literal("alpha")),
        };
        assert!(matches!(
            LexicalQuery::decode(&malformed),
            Err(LexicalScopeError::InvalidEncoding)
        ));
        assert_eq!(
            LexicalQuery::encode(TermValue::iri("https://example.org/needle"), None),
            Err(LexicalScopeError::InvalidEncoding)
        );
    }
}
