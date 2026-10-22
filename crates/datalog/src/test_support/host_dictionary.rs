// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! One test-owned host dictionary and evidence scope for native and portable controls.

use super::dictionary::{DictionaryError, DictionaryTerm, TermDictionary};
use core::fmt;
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SourceLost(pub(crate) u64);

impl fmt::Display for SourceLost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "source lost: {}", self.0)
    }
}
impl std::error::Error for SourceLost {}

#[derive(Default)]
pub(crate) struct HostDictionary {
    pub(crate) surfaces: BTreeMap<u64, String>,
    pub(crate) extra_terms: u64,
    pub(crate) resolves: std::sync::atomic::AtomicUsize,
    pub(crate) lost: std::sync::atomic::AtomicBool,
    pub(crate) lookups: std::sync::atomic::AtomicUsize,
    pub(crate) lose_after_lookup: std::sync::atomic::AtomicBool,
    pub(crate) pins: std::sync::atomic::AtomicUsize,
}

pub(crate) struct HostPin<'a> {
    text: String,
    live: &'a std::sync::atomic::AtomicUsize,
}

impl AsRef<str> for HostPin<'_> {
    fn as_ref(&self) -> &str {
        &self.text
    }
}

impl Drop for HostPin<'_> {
    fn drop(&mut self) {
        self.live.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
}

impl TermDictionary for HostDictionary {
    type Id = u64;
    type Error = SourceLost;
    type Lexical<'a> = HostPin<'a>;
    type OrderKey = String;
    fn order_key(&self, id: u64) -> Result<String, SourceLost> {
        self.check_read()?;
        self.surfaces.get(&id).cloned().ok_or(SourceLost(id))
    }
    fn lookup_term(&self, surface: &str) -> Result<Option<u64>, SourceLost> {
        self.check_read()?;
        self.lookups
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let found = self
            .surfaces
            .iter()
            .find(|(_, value)| value.as_str() == surface)
            .map(|(&id, _)| id);
        if self
            .lose_after_lookup
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            self.lost.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        Ok(found)
    }
    fn resolve_term(&self, id: u64) -> Result<HostPin<'_>, SourceLost> {
        self.check_read()?;
        self.resolves
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let text = self.surfaces.get(&id).cloned().ok_or(SourceLost(id))?;
        self.pins.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(HostPin {
            text,
            live: &self.pins,
        })
    }
    fn compare_terms(
        &self,
        left: DictionaryTerm<'_, u64>,
        right: DictionaryTerm<'_, u64>,
    ) -> Result<core::cmp::Ordering, SourceLost> {
        super::dictionary::compare_lexical_terms(self, left, right)
    }
    fn with_interned<T, const N: usize>(
        &mut self,
        terms: [DictionaryTerm<'_, u64>; N],
        validate: impl FnOnce(&Self, [u64; N]) -> Result<T, DictionaryError<SourceLost>>,
    ) -> Result<T, DictionaryError<SourceLost>> {
        let ids = terms.map(|term| match term {
            DictionaryTerm::Id(id) => id,
            DictionaryTerm::Surface(_) => {
                panic!("direct-ID operations must not mint substitutes")
            }
        });
        validate(self, ids)
    }
    fn check_read(&self) -> Result<(), SourceLost> {
        if self.lost.load(std::sync::atomic::Ordering::Relaxed) {
            Err(SourceLost(99))
        } else {
            Ok(())
        }
    }
    fn term_count(&self) -> u64 {
        self.surfaces.len() as u64 + self.extra_terms
    }
    fn resident_bytes(&self) -> usize {
        self.surfaces.values().map(String::len).sum()
    }
}

pub(crate) fn evidence_scope() -> super::seminaive::NegationScope {
    use purrdf_core::{TermBox, TermValue, evidence::ReasonDescription};
    use purrdf_lex::allocation::{Memory, Resident};
    let cause = TermValue::iri("https://example.org/missing-scope");
    let concern = TermValue::Triple {
        s: TermBox::new(TermValue::iri("https://example.org/input")),
        p: TermBox::new(TermValue::iri("https://example.org/closed")),
        o: TermBox::new(TermValue::simple_literal("required")),
    };
    let mut admission = Resident;
    let mut memory = Memory::new(&mut admission);
    let missing_scope_reason = ReasonDescription {
        cause: &cause,
        concern: &concern,
        constraint: None,
        vault: None,
        watermark: None,
    }
    .identity(&mut memory)
    .unwrap();
    assert_eq!(memory.admitted_bytes(), 0);
    super::seminaive::NegationScope {
        completeness: purrdf_core::evidence::SearchCompleteness::Complete,
        closure_declared: true,
        missing_scope_reason,
    }
}
