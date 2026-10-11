// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native admitted dictionary borrowing the original ledger's term payloads.

use crate::dataset_view::lock_read_state;
use crate::{BlankScope, TermRef, TermValue, ViewTermId};
use core::{cmp::Ordering, ops::ControlFlow};
use purrdf_lex::allocation::{Admission, Memory, StorageError};
use std::sync::Mutex;

/// A query-local ledger dictionary id, without an application capacity ceiling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LedgerTermId(usize);

impl ViewTermId for LedgerTermId {
    type JoinKeyAtom = u128;

    fn encode(self) -> u128 {
        self.0 as u128
    }

    fn encode_computed(scratch_index: u32) -> u128 {
        (1_u128 << 127) | u128::from(scratch_index)
    }
}

/// Original term strings stay borrowed; flat descriptors and their lookup order
/// are allocated. Components are interned before parents, so resolving triple
/// ids terminates even for terms deeper than a resident dataset's ingress bound.
/// Decoded composite blank labels are retained by this immutable dictionary;
/// every other string remains borrowed. The original admission outlives all
/// physical arrays and decoded labels. Only its mutable account is locked.
#[derive(Debug)]
pub struct BorrowedLedgerTerms<'a, S: Admission> {
    terms: Vec<LedgerTerm<'a>>,
    order: Vec<LedgerTermId>,
    account: Mutex<LedgerAccount<S>>,
}

#[derive(Debug)]
struct LedgerAccount<S> {
    admitted: usize,
    admission: S,
}

#[derive(Debug)]
enum LedgerTerm<'a> {
    Borrowed(TermRef<'a, LedgerTermId>),
    DecodedBlank { label: String, scope: BlankScope },
}

impl LedgerTerm<'_> {
    fn as_ref(&self) -> TermRef<'_, LedgerTermId> {
        match self {
            Self::Borrowed(term) => *term,
            Self::DecodedBlank { label, scope } => TermRef::Blank {
                label,
                scope: *scope,
            },
        }
    }
}

impl<'a, S: Admission> BorrowedLedgerTerms<'a, S> {
    /// Intern original roots and borrowed predicate IRIs without copying payloads.
    ///
    /// # Errors
    /// Preserves native admission or allocator refusal before each array growth.
    pub fn new(
        roots: impl IntoIterator<Item = &'a TermValue>,
        iris: impl IntoIterator<Item = &'a str>,
        admission: S,
    ) -> Result<Self, StorageError> {
        Self::try_new(
            roots.into_iter().map(Ok::<_, core::convert::Infallible>),
            iris.into_iter().map(Ok),
            admission,
        )
        .map_err(|error| match error {
            TermBuildError::Storage(error) => error,
            TermBuildError::Source(never) => match never {},
        })
    }

    pub(super) fn try_new<E>(
        roots: impl IntoIterator<Item = Result<&'a TermValue, E>>,
        iris: impl IntoIterator<Item = Result<&'a str, E>>,
        mut admission: S,
    ) -> Result<Self, TermBuildError<E>> {
        let mut memory = Memory::new(&mut admission);
        let (terms, order) = memory.try_scope(|memory| {
            let mut terms = Vec::new();
            let mut order = Vec::new();
            for iri in iris {
                intern(
                    LedgerTerm::Borrowed(TermRef::Iri(iri.map_err(TermBuildError::Source)?)),
                    &mut terms,
                    &mut order,
                    memory,
                )?;
            }
            for root in roots {
                let root = root.map_err(TermBuildError::Source)?;
                walk_ids(root, memory, |term, children, memory| {
                    let descriptor = describe(term, children, |iri| {
                        intern(
                            LedgerTerm::Borrowed(TermRef::Iri(iri)),
                            &mut terms,
                            &mut order,
                            memory,
                        )
                        .map(Some)
                    })?
                    .expect("interned children are present");
                    let unseen = find(descriptor, &terms, &order).is_err();
                    let id = intern(
                        LedgerTerm::Borrowed(descriptor),
                        &mut terms,
                        &mut order,
                        memory,
                    )?;
                    if unseen
                        && let TermValue::Literal {
                            lexical_form,
                            datatype,
                            ..
                        } = term
                    {
                        register_composite(lexical_form, datatype, &mut terms, &mut order, memory)?;
                    }
                    Ok(Some(id))
                })?;
            }
            Ok::<_, TermBuildError<E>>((terms, order))
        })?;
        let admitted = memory.admitted_bytes();
        Ok(Self {
            terms,
            order,
            account: Mutex::new(LedgerAccount {
                admitted,
                admission,
            }),
        })
    }

    /// Resolve a flat descriptor borrowing the original term's strings.
    #[must_use]
    pub fn resolve(&self, id: LedgerTermId) -> Option<TermRef<'_, LedgerTermId>> {
        self.terms.get(id.0).map(LedgerTerm::as_ref)
    }

    /// Resolve a predicate spelling without constructing an owned IRI term.
    #[must_use]
    pub fn iri(&self, iri: &str) -> Option<LedgerTermId> {
        find(TermRef::Iri(iri), &self.terms, &self.order).ok()
    }

    /// Look up any structurally equal value, including a separately owned triple
    /// and exact blank scope. The native walk never clones the query term.
    ///
    /// # Errors
    /// Returns original scratch admission or allocator refusal; absent values
    /// are `None`, independently of operational failures.
    pub fn lookup(&self, value: &TermValue) -> Result<Option<LedgerTermId>, StorageError> {
        let terms = &self.terms;
        let order = &self.order;
        if !matches!(value, TermValue::Triple { .. }) {
            return describe(value, &[], |iri| {
                Ok(find(TermRef::Iri(iri), terms, order).ok())
            })
            .map(|descriptor| descriptor.and_then(|term| find(term, terms, order).ok()));
        }
        let mut account = lock_read_state(&self.account);
        let LedgerAccount {
            admitted,
            admission,
        } = &mut *account;
        let mut memory = Memory::resume(admission, *admitted);
        let result = walk_ids(value, &mut memory, |term, children, _| {
            let descriptor = describe(term, children, |iri| {
                Ok(find(TermRef::Iri(iri), terms, order).ok())
            })?;
            Ok(descriptor.and_then(|term| find(term, terms, order).ok()))
        });
        *admitted = memory.admitted_bytes();
        drop(account);
        result
    }

    /// Original physical retained descriptor/index layouts, excluding payloads
    /// that remain owned and admitted by the original answer.
    #[must_use]
    pub fn admitted_bytes(&self) -> usize {
        lock_read_state(&self.account).admitted
    }

    pub(super) fn len(&self) -> usize {
        self.terms.len()
    }

    pub(super) fn resize_workspace(&self, old: usize, new: usize) -> Result<(), StorageError> {
        let mut account = lock_read_state(&self.account);
        let total = account
            .admitted
            .checked_sub(old)
            .and_then(|bytes| bytes.checked_add(new))
            .ok_or(StorageError::SizeOverflow)?;
        account.admission.resize(total)?;
        account.admitted = total;
        drop(account);
        Ok(())
    }
}

pub(super) enum TermBuildError<E> {
    Source(E),
    Storage(StorageError),
}

impl<E> From<StorageError> for TermBuildError<E> {
    fn from(error: StorageError) -> Self {
        Self::Storage(error)
    }
}

fn intern<'a, S: Admission + ?Sized>(
    term: LedgerTerm<'a>,
    terms: &mut Vec<LedgerTerm<'a>>,
    order: &mut Vec<LedgerTermId>,
    memory: &mut Memory<'_, S>,
) -> Result<LedgerTermId, StorageError> {
    let position = match find(term.as_ref(), terms, order) {
        Ok(id) => {
            if let LedgerTerm::DecodedBlank { label, .. } = term {
                memory.release_string(label)?;
            }
            return Ok(id);
        }
        Err(position) => position,
    };
    let id = LedgerTermId(terms.len());
    memory.push(terms, term)?;
    memory.push(order, id)?;
    order[position..].rotate_right(1);
    Ok(id)
}

fn find(
    term: TermRef<'_, LedgerTermId>,
    terms: &[LedgerTerm<'_>],
    order: &[LedgerTermId],
) -> Result<LedgerTermId, usize> {
    order
        .binary_search_by(|id| compare(terms[id.0].as_ref(), term))
        .map(|position| order[position])
}

fn register_composite<S: Admission + ?Sized>(
    lexical: &str,
    datatype: &str,
    terms: &mut Vec<LedgerTerm<'_>>,
    order: &mut Vec<LedgerTermId>,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    let mut blanks = crate::cdt_blank::cdt_embedded_blanks_with_memory(lexical, datatype, memory)?;
    while let Some((label, scope)) = blanks.pop() {
        intern(
            LedgerTerm::DecodedBlank { label, scope },
            terms,
            order,
            memory,
        )?;
    }
    memory.release_vec(blanks)
}

fn compare(left: TermRef<'_, LedgerTermId>, right: TermRef<'_, LedgerTermId>) -> Ordering {
    match (left, right) {
        (TermRef::Iri(a), TermRef::Iri(b)) => a.cmp(b),
        (
            TermRef::Blank {
                label: a,
                scope: sa,
            },
            TermRef::Blank {
                label: b,
                scope: sb,
            },
        ) => (a, sa).cmp(&(b, sb)),
        (
            TermRef::Literal {
                lexical: a,
                datatype: da,
                language: la,
                direction: ra,
            },
            TermRef::Literal {
                lexical: b,
                datatype: db,
                language: lb,
                direction: rb,
            },
        ) => (a, da, la, ra).cmp(&(b, db, lb, rb)),
        (TermRef::Triple { s: a, p: pa, o: oa }, TermRef::Triple { s: b, p: pb, o: ob }) => {
            (a, pa, oa).cmp(&(b, pb, ob))
        }
        (a, b) => tag(a).cmp(&tag(b)),
    }
}

fn tag(term: TermRef<'_, LedgerTermId>) -> u8 {
    match term {
        TermRef::Iri(_) => 0,
        TermRef::Blank { .. } => 1,
        TermRef::Literal { .. } => 2,
        TermRef::Triple { .. } => 3,
    }
}

type Binding = (*const TermValue, LedgerTermId);

fn describe<'a>(
    term: &'a TermValue,
    children: &[Binding],
    mut iri: impl FnMut(&'a str) -> Result<Option<LedgerTermId>, StorageError>,
) -> Result<Option<TermRef<'a, LedgerTermId>>, StorageError> {
    Ok(Some(match term {
        TermValue::Iri(value) => TermRef::Iri(value),
        TermValue::Blank { label, scope } => TermRef::Blank {
            label,
            scope: *scope,
        },
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => {
            let Some(datatype) = iri(datatype)? else {
                return Ok(None);
            };
            TermRef::Literal {
                lexical: lexical_form,
                datatype,
                language: language.as_deref(),
                direction: *direction,
            }
        }
        TermValue::Triple { s, p, o } => {
            let lookup = |value: &TermValue| {
                children
                    .binary_search_by_key(&core::ptr::from_ref(value), |(pointer, _)| *pointer)
                    .ok()
                    .map(|position| children[position].1)
            };
            let (Some(s), Some(p), Some(o)) = (lookup(s), lookup(p), lookup(o)) else {
                return Ok(None);
            };
            TermRef::Triple { s, p, o }
        }
    }))
}

fn walk_ids<'a, S: Admission + ?Sized>(
    root: &'a TermValue,
    memory: &mut Memory<'_, S>,
    mut resolve: impl FnMut(
        &'a TermValue,
        &[Binding],
        &mut Memory<'_, S>,
    ) -> Result<Option<LedgerTermId>, StorageError>,
) -> Result<Option<LedgerTermId>, StorageError> {
    let mut pending = Vec::new();
    let mut bindings = Vec::new();
    let result = (|| {
        let _: ControlFlow<()> = root.visit_terms_with_memory(
            |term, memory| {
                memory.push(&mut pending, term)?;
                Ok(ControlFlow::Continue(()))
            },
            memory,
        )?;
        let mut answer = None;
        while let Some(term) = pending.pop() {
            let Some(id) = resolve(term, &bindings, memory)? else {
                return Ok(None);
            };
            let pointer = core::ptr::from_ref(term);
            let position = bindings
                .binary_search_by_key(&pointer, |(pointer, _)| *pointer)
                .unwrap_or_else(|position| position);
            memory.push(&mut bindings, (pointer, id))?;
            bindings[position..].rotate_right(1);
            answer = Some(id);
        }
        Ok(answer)
    })();
    let pending_release = memory.release_vec(pending);
    let bindings_release = memory.release_vec(bindings);
    result.and_then(|answer| pending_release.and(bindings_release).map(|()| answer))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BlankScope, TermBox};
    use purrdf_lex::allocation::Resident;
    use std::{cell::Cell, rc::Rc};

    fn triple(subject: TermValue) -> TermValue {
        TermValue::Triple {
            s: TermBox::new(subject),
            p: TermBox::new(TermValue::Iri("https://example.org/p".into())),
            o: TermBox::new(TermValue::Literal {
                lexical_form: "original lexical bytes".into(),
                datatype: purrdf_xsd::datatype::XSD_STRING.into(),
                language: None,
                direction: None,
            }),
        }
    }

    #[test]
    fn borrowed_payloads_structural_lookup_and_blank_scope() {
        let first = triple(TermValue::Blank {
            label: "b".into(),
            scope: BlankScope(7),
        });
        let equal = first.clone();
        let foreign = triple(TermValue::Blank {
            label: "b".into(),
            scope: BlankScope(8),
        });
        let dictionary = BorrowedLedgerTerms::new(
            [&first, &equal],
            [purrdf_iri::vocab::rdf::REIFIES],
            Resident,
        )
        .unwrap();
        let first_id = dictionary.lookup(&first).unwrap().unwrap();
        assert_eq!(dictionary.lookup(&equal).unwrap(), Some(first_id));
        assert_eq!(dictionary.lookup(&foreign).unwrap(), None);
        let TermRef::Triple { s, p, o } = dictionary.resolve(first_id).unwrap() else {
            panic!("original triple descriptor");
        };
        assert!(s < first_id && p < first_id && o < first_id);
        let TermRef::Literal {
            lexical, datatype, ..
        } = dictionary.resolve(o).unwrap()
        else {
            panic!("original literal descriptor");
        };
        let TermValue::Triple { o, .. } = &first else {
            unreachable!()
        };
        let TermValue::Literal { lexical_form, .. } = &**o else {
            unreachable!()
        };
        assert_eq!(lexical.as_ptr(), lexical_form.as_ptr());
        assert_eq!(
            dictionary.resolve(datatype),
            Some(TermRef::Iri(purrdf_xsd::datatype::XSD_STRING))
        );
        assert!(dictionary.iri(purrdf_iri::vocab::rdf::REIFIES).is_some());
        assert_eq!(dictionary.terms.len(), 6);
    }

    #[derive(Debug)]
    struct Capacity {
        ceiling: usize,
        live: Rc<Cell<usize>>,
    }

    fn composite() -> TermValue {
        let scoped = crate::blank_label::encode_blank_label(
            "b",
            BlankScope(7),
            crate::blank_label::LabelAlphabet::BlankNodeLabel,
        );
        TermValue::Literal {
            lexical_form: format!("[_:b, '[_:b, _:{scoped}]'^^<{}>]", purrdf_cdt::CDT_LIST),
            datatype: purrdf_cdt::CDT_LIST.into(),
            language: None,
            direction: None,
        }
    }

    #[test]
    fn composite_blanks_resolve_in_the_original_scope_while_guards_remain_borrowed() {
        let root = composite();
        let dictionary = BorrowedLedgerTerms::new([&root, &root], [], Resident).unwrap();
        let local = TermValue::Blank {
            label: "b".into(),
            scope: BlankScope::DEFAULT,
        };
        let scoped = TermValue::Blank {
            label: "b".into(),
            scope: BlankScope(7),
        };
        let foreign = TermValue::Blank {
            label: "b".into(),
            scope: BlankScope(8),
        };
        let local_id = dictionary.lookup(&local).unwrap().unwrap();
        let scoped_id = dictionary.lookup(&scoped).unwrap().unwrap();
        assert_ne!(local_id, scoped_id);
        assert_eq!(dictionary.lookup(&foreign).unwrap(), None);
        assert_eq!(
            dictionary.resolve(local_id),
            Some(TermRef::Blank {
                label: "b",
                scope: BlankScope::DEFAULT
            })
        );
        let guard = dictionary.resolve(scoped_id).unwrap();
        assert_eq!(
            guard,
            TermRef::Blank {
                label: "b",
                scope: BlankScope(7)
            }
        );
        // A borrowed owned-label guard never holds the mutable account lock.
        assert_eq!(dictionary.lookup(&scoped).unwrap(), Some(scoped_id));
        assert_eq!(dictionary.terms.len(), 4);
        assert_eq!(
            dictionary
                .terms
                .iter()
                .filter(|term| matches!(term, LedgerTerm::DecodedBlank { .. }))
                .count(),
            2
        );
        let root_id = dictionary.lookup(&root).unwrap().unwrap();
        let TermRef::Literal { lexical, .. } = dictionary.resolve(root_id).unwrap() else {
            panic!("original composite literal");
        };
        let TermValue::Literal { lexical_form, .. } = &root else {
            unreachable!();
        };
        assert_eq!(lexical.as_ptr(), lexical_form.as_ptr());
    }

    #[derive(Debug)]
    struct RefuseGrowth {
        live: Rc<Cell<usize>>,
        grows: Rc<Cell<usize>>,
        fail: Option<usize>,
    }

    impl Admission for RefuseGrowth {
        fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
            if bytes > self.live.get() {
                let next = self.grows.get() + 1;
                self.grows.set(next);
                if self.fail == Some(next) {
                    return Err(StorageError::AdmissionFailed);
                }
            }
            self.live.set(bytes);
            Ok(())
        }
    }

    #[test]
    fn every_composite_dictionary_growth_refusal_destroys_its_partial_source() {
        let root = composite();
        let grows = Rc::new(Cell::new(0));
        let live = Rc::new(Cell::new(0));
        let dictionary = BorrowedLedgerTerms::new(
            [&root],
            [],
            RefuseGrowth {
                live: live.clone(),
                grows: grows.clone(),
                fail: None,
            },
        )
        .unwrap();
        assert_eq!(live.get(), dictionary.admitted_bytes());
        let count = grows.get();
        assert!(count > 0);
        drop(dictionary);
        for fail in 1..=count {
            let live = Rc::new(Cell::new(0));
            assert!(matches!(
                BorrowedLedgerTerms::new(
                    [&root],
                    [],
                    RefuseGrowth {
                        live: live.clone(),
                        grows: Rc::new(Cell::new(0)),
                        fail: Some(fail),
                    }
                ),
                Err(StorageError::AdmissionFailed)
            ));
            assert_eq!(
                live.get(),
                0,
                "native growth {fail} must refund destroyed construction"
            );
        }
    }

    impl Admission for Capacity {
        fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
            if bytes > self.ceiling {
                return Err(StorageError::AdmissionFailed);
            }
            self.live.set(bytes);
            Ok(())
        }
    }

    #[test]
    fn refused_scratch_is_destroyed_and_original_dictionary_survives() {
        let root = TermValue::Iri("https://example.org/root".into());
        let live = Rc::new(Cell::new(0));
        let dictionary = BorrowedLedgerTerms::new(
            [&root],
            [],
            Capacity {
                ceiling: usize::MAX,
                live: live.clone(),
            },
        )
        .unwrap();
        let baseline = dictionary.admitted_bytes();
        assert_eq!(live.get(), baseline);
        lock_read_state(&dictionary.account).admission.ceiling = baseline;
        let nested = triple(root.clone());
        assert_eq!(
            dictionary.lookup(&nested),
            Err(StorageError::AdmissionFailed)
        );
        assert_eq!(live.get(), baseline);
        assert_eq!(dictionary.admitted_bytes(), baseline);
        assert!(dictionary.iri("https://example.org/root").is_some());
    }

    #[test]
    fn deep_original_triples_have_acyclic_ids_without_resident_ingress_limit() {
        let mut root = TermValue::Iri("https://example.org/root".into());
        for _ in 0..256 {
            root = triple(root);
        }
        let dictionary = BorrowedLedgerTerms::new([&root], [], Resident).unwrap();
        let mut id = dictionary.lookup(&root).unwrap().unwrap();
        let mut depth = 0;
        while let TermRef::Triple { s, p, o } = dictionary.resolve(id).unwrap() {
            assert!(s < id && p < id && o < id);
            id = s;
            depth += 1;
        }
        assert_eq!(depth, 256);
    }
}
