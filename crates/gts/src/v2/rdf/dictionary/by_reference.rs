// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use crate::FastMap;
use crate::v2::{
    DefinitionFindings, FoldError, LiteralDirection, TermDefinition, TermId, ValueName,
};

use super::{Position, ResolvedForm, RowScope, SegmentDictionary};

/// A term available by its value reference rather than rebuilt octets.
#[derive(Debug)]
#[non_exhaustive]
pub enum ByReferenceTerm {
    /// Lexical bytes and the form to apply to their UTF-8 text.
    Literal {
        /// Digest and declared byte length.
        value: ValueName,
        /// Datatype or language form.
        form: ByReferenceForm,
    },
    /// Octets of a hex-binary literal.
    Value(ValueName),
    /// Triple whose object contains a referenced value.
    Triple([u64; 3]),
}

/// Literal metadata known without materializing its lexical text.
#[derive(Debug)]
#[non_exhaustive]
pub enum ByReferenceForm {
    /// The resolved datatype IRI.
    Typed(TermId),
    /// The language and optional base direction as written.
    Language(String, Option<LiteralDirection>),
}

#[derive(Debug, Default)]
pub(in crate::v2) struct ByReferenceTerms {
    terms: FastMap<u64, ByReferenceTerm>,
}

impl ByReferenceTerms {
    pub(in crate::v2) fn get(&self, id: u64, scope: RowScope) -> Option<&ByReferenceTerm> {
        if !scope.defines(id) || !scope.admits(id) {
            return None;
        }
        self.terms.get(&id)
    }

    pub(super) fn rollback(&mut self, start: u128) {
        self.terms.retain(|&id, _| u128::from(id) < start);
    }

    /// Prepare the value reference while the dictionary holds exactly the
    /// earlier definitions. For a forward read, the caller selects pairs
    /// already seen in its value table. A sparse read admits every pair.
    pub(super) fn preserve(
        &mut self,
        dictionary: &SegmentDictionary,
        id: u64,
        definition: &TermDefinition<'_>,
        languages: &[&str],
        names_value: impl Fn(ValueName) -> bool,
        findings: &mut DefinitionFindings,
    ) -> Result<bool, FoldError> {
        if dictionary.in_doubt {
            return Ok(false);
        }
        let term = match definition {
            TermDefinition::LiteralRef(reference, form) if names_value((*reference).into()) => {
                let Some(form) = dictionary.literal_form(id, *form, languages, findings)? else {
                    return Ok(false);
                };
                let form = match form {
                    ResolvedForm::Typed(datatype) => ByReferenceForm::Typed(datatype),
                    ResolvedForm::Language(text, direction) => {
                        let mut language = String::new();
                        language.try_reserve_exact(text.len())?;
                        language.push_str(text);
                        ByReferenceForm::Language(language, direction)
                    }
                };
                ByReferenceTerm::Literal {
                    value: (*reference).into(),
                    form,
                }
            }
            TermDefinition::ValueRef(reference) if names_value((*reference).into()) => {
                ByReferenceTerm::Value((*reference).into())
            }
            TermDefinition::Triple([subject, predicate, object])
                if *object < id && self.terms.contains_key(object) =>
            {
                if dictionary
                    .dependencies(
                        id,
                        [*subject, *predicate],
                        [Position::Subject, Position::Iri],
                        findings,
                    )
                    .is_none()
                {
                    return Ok(false);
                }
                ByReferenceTerm::Triple([*subject, *predicate, *object])
            }
            _ => return Ok(false),
        };
        self.terms.try_reserve(1)?;
        self.terms.insert(id, term);
        Ok(true)
    }
}
