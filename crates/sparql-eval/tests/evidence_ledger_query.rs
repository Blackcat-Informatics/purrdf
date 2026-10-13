// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Native engine reads the lazy evidence graph, including privacy projections.

use purrdf_core::evidence::{
    AnswerLedger, EvidenceRecord, Frame, FrameVocabulary, InformationMap, LedgerDataset,
    LedgerError, LedgerNodeKey, LedgerNodes, LedgerRow, LedgerSource, LedgerStatement,
    LedgerVocabulary, ReasonTable, ReasonVocabulary, RetainedReason, RowVocabulary,
    SearchCompleteness,
};
use purrdf_core::{DatasetView, FallibleDatasetView, SparqlRequest, TermBox, TermValue};
use purrdf_lex::allocation::{Admission, Memory, Resident, StorageError};
use purrdf_sparql_eval::{FallibleSparqlError, NativeSparqlEngine, QueryOptions};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

const REASON: &str = "https://example.org/reason";
const CAUSE: &str = "https://example.org/cause";
const COMPLETE: &str = "https://example.org/completeness";

#[derive(Debug)]
struct ContentNodes {
    keys: [LedgerNodeKey; 2],
    nodes: [TermValue; 2],
}

impl LedgerNodes for ContentNodes {
    fn node(&self, key: LedgerNodeKey) -> Option<&TermValue> {
        if key == self.keys[0] {
            Some(&self.nodes[0])
        } else if key == self.keys[1] {
            Some(&self.nodes[1])
        } else {
            None
        }
    }
}

fn vocabulary<'a>(row: &'a [TermValue; 22], extra: &'a [TermValue; 14]) -> LedgerVocabulary<'a> {
    LedgerVocabulary {
        row: RowVocabulary {
            completeness: &row[0],
            information: &row[1],
            information_context: &row[2],
            standpoint: &row[3],
            frame: &row[4],
            reason: &row[5],
            accepted_reason: &row[6],
            aggregate: &row[7],
            column: &row[8],
            bound: &row[9],
            unscoped: &row[10],
            information_values: [&row[11], &row[12], &row[13], &row[14]],
            completeness_values: [&row[15], &row[16], &row[17]],
            bound_values: [&row[18], &row[19], &row[20], &row[21]],
        },
        reason: ReasonVocabulary {
            cause: &extra[0],
            constraint: &extra[1],
            vault: &extra[2],
            watermark: &extra[3],
        },
        frame: FrameVocabulary {
            mode: &extra[4],
            alternative: &extra[5],
            components: [&extra[6], &extra[7], &extra[8], &extra[9], &extra[10]],
            actual: &extra[11],
            conjunction: &extra[12],
            disjunction: &extra[13],
        },
    }
}

#[derive(Debug)]
struct OriginalLedger<'a> {
    rows: &'a [TermValue],
    reason: &'a TermValue,
    cause: &'a TermValue,
    concern: &'a TermValue,
    complete: &'a TermValue,
}

impl<'a> LedgerSource<'a> for OriginalLedger<'a> {
    fn statements(&self) -> impl Iterator<Item = Result<LedgerStatement<'a>, LedgerError>> {
        let reason = self.reason;
        let complete = self.complete;
        self.rows
            .iter()
            .flat_map(move |row| {
                [
                    LedgerStatement {
                        subject: row,
                        predicate: REASON,
                        object: reason,
                    },
                    LedgerStatement {
                        subject: row,
                        predicate: COMPLETE,
                        object: complete,
                    },
                ]
            })
            .chain([
                LedgerStatement {
                    subject: reason,
                    predicate: CAUSE,
                    object: self.cause,
                },
                LedgerStatement {
                    subject: reason,
                    predicate: purrdf_iri::vocab::rdf::REIFIES,
                    object: self.concern,
                },
            ])
            .map(Ok)
    }
}

fn request(query: &str) -> SparqlRequest<'_> {
    SparqlRequest {
        query,
        base_iri: None,
        substitutions: &[],
    }
}

#[test]
fn shared_reason_group_count_and_private_construct_use_the_native_engine() {
    let rows: Vec<_> = (0..2279)
        .map(|index| TermValue::Iri(format!("https://example.org/row/{index}")))
        .collect();
    let reason = TermValue::Iri("https://example.org/reason-node".into());
    let cause = TermValue::Iri("https://example.org/redacted-data".into());
    let complete = TermValue::Iri("https://example.org/undetermined".into());
    let concern = TermValue::Triple {
        s: TermBox::new(TermValue::Iri("https://example.org/private-subject".into())),
        p: TermBox::new(TermValue::Iri(
            "https://example.org/private-predicate".into(),
        )),
        o: TermBox::new(TermValue::Iri("https://example.org/private-object".into())),
    };
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let original =
        RetainedReason::from_parts(&cause, &concern, None, None, None, &mut memory).unwrap();
    let mut reasons = ReasonTable::with_admission(Resident);
    let reason_id = reasons.intern(original).unwrap();
    let record = EvidenceRecord::from_parts(
        &InformationMap::default(),
        SearchCompleteness::Undetermined,
        &Frame::actual(),
        &[reason_id],
        &[],
        &mut memory,
    )
    .unwrap();
    let nodes = ContentNodes {
        keys: [
            LedgerNodeKey::Reason(reason_id),
            LedgerNodeKey::Frame(record.frame().identity()),
        ],
        nodes: [
            reason,
            TermValue::Iri("https://example.org/actual-frame".into()),
        ],
    };
    let mut row_v =
        core::array::from_fn(|index| TermValue::Iri(format!("https://example.org/row-v/{index}")));
    row_v[0] = TermValue::Iri(COMPLETE.into());
    row_v[5] = TermValue::Iri(REASON.into());
    row_v[16] = complete;
    let mut extra = core::array::from_fn(|index| {
        TermValue::Iri(format!("https://example.org/extra-v/{index}"))
    });
    extra[0] = TermValue::Iri(CAUSE.into());
    let source = AnswerLedger::new(
        rows.iter().map(|node| LedgerRow {
            node,
            solution: &concern,
            record: &record,
            accepted: &[],
        }),
        &reasons,
        &nodes,
        vocabulary(&row_v, &extra),
        Resident,
    )
    .unwrap();
    assert_eq!(source.dictionary_len(), 2);
    let ledger = LedgerDataset::new(source, Resident).expect("original ledger admitted");
    assert_eq!(ledger.quads().count(), 5 * 2279 + 3);
    assert_queries_and_private_projection(&ledger, &cause);
    assert_eq!(ledger.read_error(), None);
}

fn assert_queries_and_private_projection<D: FallibleDatasetView + Sync>(
    ledger: &D,
    cause: &TermValue,
) {
    let engine = NativeSparqlEngine::new();
    let grouped = engine
        .query_fallible_view(
            ledger,
            request(
                "SELECT ?cause (COUNT(?row) AS ?n) WHERE {
           ?row <https://example.org/reason> ?reason .
           ?reason <https://example.org/cause> ?cause
         } GROUP BY ?cause",
            ),
            QueryOptions::default(),
        )
        .expect("complete group query");
    let (variables, grouped) = grouped
        .result
        .solutions()
        .expect("native grouped solutions");
    assert_eq!(variables, ["cause", "n"]);
    assert_eq!(grouped.len(), 1);
    assert_eq!(grouped[0][0], Some(cause.clone()));
    let Some(TermValue::Literal { lexical_form, .. }) = &grouped[0][1] else {
        panic!("native count literal");
    };
    assert_eq!(lexical_form, "2279");

    let projected = engine
        .query_fallible_view(
            ledger,
            request(
                "CONSTRUCT {
           ?row <https://example.org/completeness> ?complete .
           ?reason <https://example.org/cause> ?cause
         } WHERE {
           ?row <https://example.org/reason> ?reason .
           ?row <https://example.org/completeness> ?complete .
           ?reason <https://example.org/cause> ?cause
         }",
            ),
            QueryOptions::default(),
        )
        .expect("complete private projection");
    let graph = projected.result.graph().expect("native CONSTRUCT graph");
    assert_eq!(graph.quad_count(), 2280);
    let predicates: Vec<_> = graph
        .quad_refs()
        .map(|quad| {
            let purrdf_core::TermRef::Iri(iri) = quad.p else {
                panic!("IRI predicate");
            };
            iri.to_owned()
        })
        .collect();
    assert_eq!(
        predicates
            .iter()
            .filter(|iri| iri.as_str() == COMPLETE)
            .count(),
        2279
    );
    assert_eq!(
        predicates
            .iter()
            .filter(|iri| iri.as_str() == CAUSE)
            .count(),
        1
    );
    assert!(
        !predicates
            .iter()
            .any(|iri| iri == purrdf_iri::vocab::rdf::REIFIES)
    );
}

#[derive(Debug)]
struct OriginalCapacity(Arc<AtomicUsize>);

#[test]
fn composite_literal_blanks_have_native_ids_without_changing_literal_bytes() {
    let reason = TermValue::Iri("https://example.org/reason-node".into());
    let complete = TermValue::Iri("https://example.org/complete".into());
    let scope = purrdf_core::BlankScope(7);
    let token = purrdf_core::blank_label::encode_blank_label(
        "b",
        scope,
        purrdf_core::blank_label::LabelAlphabet::BlankNodeLabel,
    );
    let cause = TermValue::Literal {
        lexical_form: format!("[_:{token}, '[_:{token}]'^^<{}>]", purrdf_cdt::CDT_LIST),
        datatype: purrdf_cdt::CDT_LIST.into(),
        language: None,
        direction: None,
    };
    let concern = TermValue::Triple {
        s: TermBox::new(reason.clone()),
        p: TermBox::new(TermValue::Iri(
            "https://example.org/private-predicate".into(),
        )),
        o: TermBox::new(cause.clone()),
    };
    let ledger = LedgerDataset::new(
        OriginalLedger {
            rows: &[],
            reason: &reason,
            cause: &cause,
            concern: &concern,
            complete: &complete,
        },
        Resident,
    )
    .unwrap();
    let blank = TermValue::Blank {
        label: "b".into(),
        scope,
    };
    let id = ledger
        .term_id_by_value(&blank)
        .unwrap()
        .expect("embedded blank registered");
    let guard = ledger.resolve(id).unwrap();
    assert_eq!(guard, purrdf_core::TermRef::Blank { label: "b", scope });
    assert_eq!(ledger.term_id_by_value(&blank).unwrap(), Some(id));
    assert_eq!(
        ledger
            .term_id_by_value(&TermValue::Blank {
                label: "b".into(),
                scope: purrdf_core::BlankScope(8),
            })
            .unwrap(),
        None
    );
    let result = NativeSparqlEngine::new()
        .query_fallible_view(
            &ledger,
            request("SELECT ?cause WHERE { ?reason <https://example.org/cause> ?cause }"),
            QueryOptions::default(),
        )
        .expect("native composite literal result");
    let (_, rows) = result.result.solutions().expect("native solution");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], Some(cause.clone()));
    assert_eq!(ledger.read_error(), None);
}

impl Admission for OriginalCapacity {
    fn resize(&mut self, live_bytes: usize) -> Result<(), StorageError> {
        (live_bytes <= self.0.load(Ordering::Relaxed))
            .then_some(())
            .ok_or(StorageError::AdmissionFailed)
    }
}

#[test]
fn native_ledger_refusal_is_an_operational_error_without_partial_answers() {
    use purrdf_core::evidence::LedgerReadError;
    use purrdf_core::{FallibleDatasetView, ViewOperationStatus};

    let rows = [TermValue::Iri("https://example.org/row".into())];
    let reason = TermValue::Iri("https://example.org/reason-node".into());
    let cause = TermValue::Iri("https://example.org/redacted-data".into());
    let complete = TermValue::Iri("https://example.org/complete".into());
    let concern = TermValue::Triple {
        s: TermBox::new(rows[0].clone()),
        p: TermBox::new(TermValue::Iri(
            "https://example.org/private-predicate".into(),
        )),
        o: TermBox::new(reason.clone()),
    };
    let capacity = Arc::new(AtomicUsize::new(usize::MAX));
    let ledger = LedgerDataset::new(
        OriginalLedger {
            rows: &rows,
            reason: &reason,
            cause: &cause,
            concern: &concern,
            complete: &complete,
        },
        OriginalCapacity(capacity.clone()),
    )
    .expect("original dictionary admitted");
    let original_bytes = *ledger.operation_status().evidence();
    capacity.store(original_bytes, Ordering::Relaxed);
    let error = NativeSparqlEngine::new()
        .query_fallible_view(
            &ledger,
            request("SELECT ?row WHERE { ?row <https://example.org/reason> ?reason }"),
            QueryOptions::default(),
        )
        .expect_err("reporting/operator growth exceeds original grant");
    assert!(matches!(
        error,
        FallibleSparqlError::Operational {
            error: LedgerReadError::Storage(StorageError::AdmissionFailed),
            ..
        }
    ));
    assert_eq!(
        ledger.read_error(),
        Some(LedgerReadError::Storage(StorageError::AdmissionFailed))
    );
    assert_eq!(
        ledger.checked_read(|view| view.quads().count()),
        Err(LedgerReadError::Storage(StorageError::AdmissionFailed))
    );
    assert!(matches!(
        ledger.operation_status(),
        ViewOperationStatus::Failed { .. }
    ));
}
