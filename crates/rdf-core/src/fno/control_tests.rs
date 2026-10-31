// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original-account lifetime, failure ownership and transactional staging.

use super::{
    FnoCatalog, FnoConstructionFailure, FnoConstructorRefusal, TermValue, tests::sample_catalog,
};
use purrdf_lex::{
    allocation::{Admission, StorageError},
    control::ControlledError,
};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

#[derive(Default)]
struct Ledger {
    live: usize,
    peak: usize,
    calls: usize,
    zeros: usize,
    first_growth: Option<usize>,
}

/// Deliberately has no Drop implementation: Admission does not promise RAII.
struct Account {
    ledger: Arc<Mutex<Ledger>>,
    fail_at: Option<usize>,
}
impl Admission for Account {
    fn resize(&mut self, live: usize) -> Result<(), StorageError> {
        let mut ledger = self.ledger.lock().unwrap();
        ledger.calls += 1;
        if live > ledger.live && ledger.first_growth.is_none() {
            ledger.first_growth = Some(ledger.calls);
        }
        if self.fail_at == Some(ledger.calls) && live != 0 {
            return Err(StorageError::AdmissionFailed);
        }
        if live == 0 {
            ledger.zeros += 1;
        }
        ledger.live = live;
        ledger.peak = ledger.peak.max(live);
        drop(ledger);
        Ok(())
    }
}
fn account(ledger: &Arc<Mutex<Ledger>>, fail_at: Option<usize>) -> Account {
    Account {
        ledger: Arc::clone(ledger),
        fail_at,
    }
}
#[derive(Debug, PartialEq, Eq)]
struct Stop(u8);

#[test]
fn original_account_without_drop_refunds_refusals_and_last_catalog_or_error_owner() {
    let input = sample_catalog().as_input().clone();
    let ledger = Arc::new(Mutex::new(Ledger::default()));
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&calls);
    let catalog =
        FnoCatalog::try_new_with_control(input.clone(), account(&ledger, None), move |_| {
            observed.fetch_add(1, Ordering::Relaxed);
            Ok::<(), Stop>(())
        })
        .unwrap();
    let total = calls.load(Ordering::Relaxed);
    let first_growth = ledger
        .lock()
        .unwrap()
        .first_growth
        .expect("the fixture allocated catalog storage");
    assert!(ledger.lock().unwrap().live > 0);
    let last = catalog.clone();
    drop(catalog);
    assert!(
        ledger.lock().unwrap().live > 0,
        "a catalog clone retains the same original account"
    );
    drop(last);
    assert_eq!(ledger.lock().unwrap().live, 0);

    let ledger = Arc::new(Mutex::new(Ledger::default()));
    let pointer = input.document_iri.as_ptr();
    let expected = input.clone();
    let stopped = FnoCatalog::try_new_with_control(input, account(&ledger, None), |_| Err(Stop(7)))
        .unwrap_err();
    assert!(matches!(stopped.cause, ControlledError::Control(Stop(7))));
    assert_eq!(stopped.input, expected);
    assert_eq!(stopped.input.document_iri.as_ptr(), pointer);
    assert_eq!(ledger.lock().unwrap().live, 0);
    assert!(
        ledger.lock().unwrap().zeros > 0,
        "even a pretripped operation releases its original account"
    );

    let ledger = Arc::new(Mutex::new(Ledger::default()));
    let input = expected.clone();
    let pointer = input.document_iri.as_ptr();
    let mut seen = 0;
    let stopped = FnoCatalog::try_new_with_control(input, account(&ledger, None), |_| {
        seen += 1;
        if seen == total / 2 {
            Err(Stop(19))
        } else {
            Ok(())
        }
    })
    .unwrap_err();
    assert!(matches!(stopped.cause, ControlledError::Control(Stop(19))));
    assert_eq!(stopped.input, expected);
    assert_eq!(stopped.input.document_iri.as_ptr(), pointer);
    assert!(
        ledger.lock().unwrap().peak > 0,
        "this refusal follows actual admitted allocations"
    );
    assert_eq!(ledger.lock().unwrap().live, 0);

    let ledger = Arc::new(Mutex::new(Ledger::default()));
    let refused = FnoCatalog::try_new_with_control(
        expected.clone(),
        account(&ledger, Some(first_growth)),
        |_| Ok::<(), Stop>(()),
    )
    .unwrap_err();
    assert!(matches!(
        refused.cause,
        ControlledError::Native(FnoConstructionFailure::Storage(
            StorageError::AdmissionFailed
        ))
    ));
    assert_eq!(refused.input, expected);
    assert_eq!(ledger.lock().unwrap().live, 0);

    let mut invalid = expected;
    invalid.functions[0]
        .expects
        .push("urn:test:unresolved-parameter".to_owned());
    let original = invalid.clone();
    let pointer = invalid.document_iri.as_ptr();
    let ledger = Arc::new(Mutex::new(Ledger::default()));
    let FnoConstructorRefusal { input, cause } =
        FnoCatalog::try_new_with_control(invalid, account(&ledger, None), |_| Ok::<(), Stop>(()))
            .unwrap_err();
    assert_eq!(input, original);
    assert_eq!(input.document_iri.as_ptr(), pointer);
    let ControlledError::Native(FnoConstructionFailure::Semantic(reason)) = cause else {
        panic!("expected the native unresolved-reference reason")
    };
    let publication = ledger.lock().unwrap().calls;
    assert!(ledger.lock().unwrap().live > 0);
    let last = reason.clone();
    drop(reason);
    assert!(
        ledger.lock().unwrap().live > 0,
        "native error operands keep the original account"
    );
    drop(last);
    assert_eq!(ledger.lock().unwrap().live, 0);

    // Refuse the actual error-publication allocation observed above, without
    // depending on a fixed layout, allocation count or machine word width.
    let ledger = Arc::new(Mutex::new(Ledger::default()));
    let input = original.clone();
    let pointer = input.document_iri.as_ptr();
    let refused =
        FnoCatalog::try_new_with_control(input, account(&ledger, Some(publication)), |_| {
            Ok::<(), Stop>(())
        })
        .unwrap_err();
    assert!(matches!(
        refused.cause,
        ControlledError::Native(FnoConstructionFailure::Storage(
            StorageError::AdmissionFailed
        ))
    ));
    assert_eq!(refused.input, original);
    assert_eq!(refused.input.document_iri.as_ptr(), pointer);
    assert_eq!(ledger.lock().unwrap().live, 0);

    // Exercise the same no-Drop caller account through the production reader.
    let expected = sample_catalog();
    let source = super::read_tests::dataset(&super::to_quads(&expected));
    let ledger = Arc::new(Mutex::new(Ledger::default()));
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&calls);
    let catalog = FnoCatalog::from_dataset_with_control(
        &source,
        &expected.as_input().ontology_iri,
        account(&ledger, None),
        move |_| {
            observed.fetch_add(1, Ordering::Relaxed);
            Ok::<(), Stop>(())
        },
    )
    .unwrap();
    assert_eq!(catalog, expected);
    let total = calls.load(Ordering::Relaxed);
    let last = catalog.clone();
    drop(catalog);
    assert!(ledger.lock().unwrap().live > 0);
    drop(last);
    assert_eq!(ledger.lock().unwrap().live, 0);
    let ledger = Arc::new(Mutex::new(Ledger::default()));
    let mut seen = 0;
    let stopped = FnoCatalog::from_dataset_with_control(
        &source,
        &expected.as_input().ontology_iri,
        account(&ledger, None),
        |_| {
            seen += 1;
            if seen == total / 2 {
                Err(Stop(67))
            } else {
                Ok(())
            }
        },
    );
    assert!(matches!(stopped, Err(ControlledError::Control(Stop(67)))));
    assert!(ledger.lock().unwrap().peak > 0);
    assert_eq!(ledger.lock().unwrap().live, 0);

    let mut rows = super::to_quads(&expected);
    rows.push(crate::RdfQuad::new(
        crate::RdfTerm::iri("https://example.org/outside"),
        "https://example.org/unclaimed",
        crate::RdfTerm::iri("https://example.org/object"),
    ));
    let invalid = super::read_tests::dataset(&rows);
    drop(rows);
    let ledger = Arc::new(Mutex::new(Ledger::default()));
    let failed = FnoCatalog::from_dataset_with_control(
        &invalid,
        &expected.as_input().ontology_iri,
        account(&ledger, None),
        |_| Ok::<(), Stop>(()),
    )
    .unwrap_err();
    drop(invalid);
    let ControlledError::Native(FnoConstructionFailure::Semantic(error)) = failed else {
        panic!("expected the owned original unclaimed statement");
    };
    assert!(matches!(&*error,super::FnoReadError::Statement{row,reason}
        if *reason==super::FnoStructureKind::Unclaimed && row.s==TermValue::iri("https://example.org/outside")));
    let last = error.clone();
    drop(error);
    assert!(ledger.lock().unwrap().live > 0);
    drop(last);
    assert_eq!(ledger.lock().unwrap().live, 0);

    // EOF latches a source failure. A hypothetical next-phase callback would
    // stop, and the account refuses any growth: neither may replace the source.
    use crate::DatasetView;
    let faulty = crate::term_fixture::BudgetedView::new(Arc::clone(&source), 0);
    let after = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&after);
    let ledger = Arc::new(Mutex::new(Ledger::default()));
    let failed = FnoCatalog::from_dataset_with_control(
        &faulty,
        &expected.as_input().ontology_iri,
        account(&ledger, Some(1)),
        |_| {
            if faulty.read_error().is_some() {
                observed.fetch_add(1, Ordering::Relaxed);
                Err(Stop(71))
            } else {
                Ok(())
            }
        },
    )
    .unwrap_err();
    let ControlledError::Native(FnoConstructionFailure::Source(cause)) = failed else {
        panic!("the original provider failure must win without a reporting allocation");
    };
    assert!(matches!(cause, super::FnoReadSourceError::Source(_)));
    assert_eq!(
        purrdf_testkit::error_capture::Capture::read(&cause).code,
        "fno-read-source"
    );
    assert_eq!(after.load(Ordering::Relaxed), 0);
    assert_eq!(ledger.lock().unwrap().peak, 0);
    assert_eq!(ledger.lock().unwrap().live, 0);

    // The same source remains faulted before a new reader starts. Even an
    // immediate callback refusal must not replace that existing provider cause.
    let original = faulty.read_error().unwrap();
    let ledger = Arc::new(Mutex::new(Ledger::default()));
    let mut callbacks = 0;
    let failed = FnoCatalog::from_dataset_with_control(
        &faulty,
        &expected.as_input().ontology_iri,
        account(&ledger, Some(1)),
        |_| {
            callbacks += 1;
            Err(Stop(73))
        },
    )
    .unwrap_err();
    let ControlledError::Native(FnoConstructionFailure::Source(super::FnoReadSourceError::Source(
        cause,
    ))) = failed
    else {
        panic!("the existing provider failure must precede the initial callback");
    };
    assert_eq!(cause, original);
    assert_eq!(callbacks, 0);
    assert_eq!(ledger.lock().unwrap().peak, 0);
    assert_eq!(ledger.lock().unwrap().live, 0);
}

#[test]
fn interrupted_native_canonicalization_only_mutates_the_admitted_staging_copy() {
    let mut original = sample_catalog().as_input().clone();
    original.doc_label = TermValue::Literal {
        lexical_form: "Catalog".to_owned(),
        datatype: purrdf_iri::vocab::rdf::LANG_STRING.to_owned(),
        language: Some("EN-US".to_owned()),
        direction: None,
    };
    let expected = original.clone();
    let pointer = original.document_iri.as_ptr();
    let armed = Arc::new(AtomicBool::new(false));
    let enabled = Arc::clone(&armed);
    let mut steps = 0;
    let ledger = Arc::new(Mutex::new(Ledger::default()));
    let mut context = super::construction::Context::new(account(&ledger, None), move |_| {
        if enabled.load(Ordering::Relaxed) {
            steps += 1;
            if steps == 20 {
                return Err(Stop(41));
            }
        }
        Ok(())
    });
    let shared = super::admission::validate(&mut context, &original).unwrap();
    let mut staged = context
        .copy::<_, super::FnoAdmissionReason>(&original)
        .unwrap();
    armed.store(true, Ordering::Relaxed);
    let result = super::admission::canonicalize(&mut context, &mut staged, &shared);
    assert!(matches!(result, Err(ControlledError::Control(Stop(41)))));
    assert_ne!(
        staged.doc_label, original.doc_label,
        "canonicalization had already changed staging data"
    );
    assert_eq!(original, expected);
    assert_eq!(original.document_iri.as_ptr(), pointer);
    drop((staged, shared));
    drop(context);
    assert_eq!(ledger.lock().unwrap().live, 0);
}
