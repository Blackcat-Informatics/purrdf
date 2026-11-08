// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Cold assertion insertion, separated from syntax decoding for positive-world updates.
//! Syntax decoding and frozen-schema admission belong to the parser; this owner
//! records the original warrant before publishing its corresponding assertion.

use std::collections::BTreeSet;

use purrdf_lex::allocation::StorageError;

use crate::interner::Interner;

use super::input_source::{Input, Inputs, Source};

#[derive(Clone, Copy)]
pub(crate) enum Assertion {
    Type(u32, u32),
    Role(u32, u32, u32),
    Equality(u32, u32),
    Distinct(u32, u32),
}

pub(crate) struct Target<'a> {
    pub(crate) inputs: &'a mut Inputs,
    pub(crate) types: &'a mut Vec<(u32, u32)>,
    pub(crate) roles: &'a mut Vec<(u32, u32, u32)>,
    pub(crate) equalities: &'a mut Vec<(u32, u32)>,
    pub(crate) distinctions: &'a mut Vec<(u32, u32)>,
    pub(crate) individuals: &'a mut BTreeSet<u32>,
}

impl Target<'_> {
    pub(crate) fn record(
        &mut self,
        interner: &Interner,
        assertion: Assertion,
        source: Source,
    ) -> Result<(), StorageError> {
        let (input, left, right) = match assertion {
            Assertion::Type(individual, _) => (Input::Type(self.types.len()), individual, None),
            Assertion::Role(subject, _, object) => {
                (Input::Role(self.roles.len()), subject, Some(object))
            }
            Assertion::Equality(left, right) => {
                (Input::Equality(self.equalities.len()), left, Some(right))
            }
            Assertion::Distinct(left, right) => {
                (Input::Distinct(self.distinctions.len()), left, Some(right))
            }
        };
        self.inputs.record(input, source)?;
        match assertion {
            Assertion::Type(individual, concept) => {
                self.types.push((individual, concept));
                self.individuals.insert(individual);
                return Ok(());
            }
            Assertion::Role(subject, property, object) => {
                self.roles.push((subject, property, object));
            }
            Assertion::Equality(left, right) => self.equalities.push((left, right)),
            Assertion::Distinct(left, right) => self.distinctions.push((left, right)),
        }
        for individual in [Some(left), right].into_iter().flatten() {
            if interner.is_subject(individual) {
                self.individuals.insert(individual);
            }
        }
        Ok(())
    }
}
