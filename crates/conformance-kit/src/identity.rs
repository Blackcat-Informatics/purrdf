// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The one blank identity re-encoding shared by report and solution carriers.

use std::collections::BTreeMap;

use purrdf_core::blank_label::{LabelAlphabet, encode_blank_label};
use purrdf_core::cdt_blank::{BlankBinding, rewrite_cdt_blank_terms};
use purrdf_core::{BlankScope, TermValue};

/// An identity includes its original source domain and scope. Synthetic carrier
/// nodes use a separate namespace and cannot alias a caller's blank spelling.
#[derive(Default)]
pub(crate) struct Identities {
    names: BTreeMap<(String, String, BlankScope), String>,
}

impl Identities {
    pub(crate) fn link(
        &mut self,
        from_domain: &str,
        from: (&str, BlankScope),
        to_domain: &str,
        to: (&str, BlankScope),
    ) {
        let TermValue::Blank { label, .. } = self.blank(to_domain, to.0, to.1) else {
            unreachable!("blank identity mapping returns a blank")
        };
        self.names
            .insert((from_domain.to_owned(), from.0.to_owned(), from.1), label);
    }

    pub(crate) fn blank(&mut self, domain: &str, label: &str, scope: BlankScope) -> TermValue {
        let next = self.names.len();
        let label = self
            .names
            .entry((domain.to_owned(), label.to_owned(), scope))
            .or_insert_with(|| format!("value{next}"));
        TermValue::Blank {
            label: label.clone(),
            scope: BlankScope::DEFAULT,
        }
    }

    pub(crate) fn map(&mut self, domain: &str, value: &TermValue) -> TermValue {
        value.fold(
            |term| match term {
                TermValue::Blank { label, scope } => self.blank(domain, label, *scope),
                TermValue::Literal {
                    lexical_form,
                    datatype,
                    language,
                    direction,
                } => {
                    let lexical = rewrite_cdt_blank_terms(lexical_form, datatype, &mut |token| {
                        let (label, scope) =
                            BlankBinding::Decoded(LabelAlphabet::BlankNodeLabel).resolve(token);
                        let TermValue::Blank { label, scope } = self.blank(domain, &label, scope)
                        else {
                            unreachable!("blank identity mapping returns a blank")
                        };
                        Some(format!(
                            "_:{}",
                            encode_blank_label(&label, scope, LabelAlphabet::BlankNodeLabel)
                        ))
                    });
                    TermValue::Literal {
                        lexical_form: lexical.into_owned(),
                        datatype: datatype.clone(),
                        language: language.clone(),
                        direction: *direction,
                    }
                }
                term => term.clone(),
            },
            |s, p, o| TermValue::Triple {
                s: Box::new(s).into(),
                p: Box::new(p).into(),
                o: Box::new(o).into(),
            },
        )
    }
}
