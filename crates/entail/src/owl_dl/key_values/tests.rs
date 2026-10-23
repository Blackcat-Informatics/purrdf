// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Differential asserted-row semantics and native key index accounting.

use super::{Index, Reads};
use crate::EntailError;
use purrdf_alloc_probe::CurrentThreadWindow;
use purrdf_lex::allocation::{Admission, Memory, Resident, StorageError};
use std::collections::BTreeSet;

fn rows() -> Vec<(u32, u32, u32)> {
    let mut rows = Vec::new();
    for subject in 0..4 {
        for property in 10..13 {
            for value in 20..28 {
                if (subject + property + value) % 3 == 0 {
                    rows.push((subject, property, value));
                }
            }
        }
    }
    rows.extend_from_within(..8);
    rows.push((0, 999, 20));
    rows
}

fn previous(rows: &[(u32, u32, u32)], left: u32, right: u32, property: u32) -> Option<u32> {
    let values = |individual| {
        rows.iter()
            .filter(|&&(s, p, _)| s == individual && p == property)
            .map(|&(_, _, v)| v)
            .collect::<BTreeSet<_>>()
    };
    let a = values(left);
    let b = values(right);
    a.intersection(&b).next().copied()
}

#[test]
fn indexed_common_values_match_set_semantics_and_retain_original_rows_without_lookup_allocations() {
    let rows = rows();
    let mut storage = Resident;
    let mut memory = Memory::new(&mut storage);
    let index = Index::new(&rows, [12, 10, 11, 10], None, &mut memory).unwrap();
    for left in 0..5 {
        for right in 0..5 {
            for property in 10..14 {
                let common = index.common(left, right, property, None).unwrap();
                assert_eq!(
                    common.map(|value| value.value),
                    previous(&rows, left, right, property)
                );
                if let Some(common) = common {
                    assert_eq!(rows[common.left], (left, property, common.value));
                    assert_eq!(rows[common.right], (right, property, common.value));
                    assert_eq!(
                        common.left,
                        rows.iter()
                            .position(|&row| row == rows[common.left])
                            .unwrap()
                    );
                    assert_eq!(
                        common.right,
                        rows.iter()
                            .position(|&row| row == rows[common.right])
                            .unwrap()
                    );
                }
            }
        }
    }
    let window = CurrentThreadWindow::open();
    for _ in 0..1000 {
        std::hint::black_box(index.common(0, 3, 10, None).unwrap());
    }
    assert_eq!(window.close().allocations, 0);
    let window = CurrentThreadWindow::open();
    for _ in 0..1000 {
        std::hint::black_box(previous(&rows, 0, 3, 10));
    }
    assert!(window.close().allocations > 0);
    index.release(&mut memory).unwrap();
    assert_eq!(memory.admitted_bytes(), 0);
}

#[derive(Default)]
struct Account {
    live: usize,
    grows: usize,
    refuse: Option<usize>,
}
impl Admission for Account {
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
        if bytes > self.live {
            self.grows += 1;
            if self.refuse == Some(self.grows) {
                return Err(StorageError::AdmissionFailed);
            }
        }
        self.live = bytes;
        Ok(())
    }
}

#[test]
fn key_value_index_refusals_and_stopping_preserve_the_original_account() {
    let rows = rows();
    let mut healthy = Account::default();
    {
        let mut memory = Memory::new(&mut healthy);
        let window = CurrentThreadWindow::open();
        let index = Index::new(&rows, [10, 11, 12], None, &mut memory).unwrap();
        assert_eq!(
            window.close().retained_bytes,
            i64::try_from(memory.admitted_bytes()).unwrap()
        );
        index.release(&mut memory).unwrap();
    }
    assert_eq!(healthy.live, 0);
    for growth in 1..=healthy.grows {
        let mut account = Account {
            refuse: Some(growth),
            ..Account::default()
        };
        let mut memory = Memory::new(&mut account);
        assert!(matches!(
            Index::new(&rows, [10, 11, 12], None, &mut memory),
            Err(EntailError::Storage(StorageError::AdmissionFailed))
        ));
        assert_eq!(memory.admitted_bytes(), 0);
        assert_eq!(account.live, 0);
    }
    #[derive(Debug)]
    struct Stop;
    impl purrdf_datalog::StopSignal for Stop {
        fn stopped(&self) -> bool {
            true
        }
    }
    let mut storage = Resident;
    let mut memory = Memory::new(&mut storage);
    assert!(matches!(
        Index::new(&[], [], Some(&Stop), &mut memory),
        Err(EntailError::Stopped)
    ));
    let index = Index::new(&rows, [10], None, &mut memory).unwrap();
    assert!(matches!(
        index.common(100, 101, 10, Some(&Stop)),
        Err(EntailError::Stopped)
    ));
    let mut reads = Reads::default();
    reads
        .push(index.common(0, 3, 10, None).unwrap().unwrap())
        .unwrap();
    reads.finish(0).unwrap();
    reads.base = 7;
    assert_eq!(reads.get(7).unwrap().len(), 1);
    assert!(reads.get(6).is_none());
    assert!(reads.get(8).is_none());
    index.release(&mut memory).unwrap();
    assert_eq!(memory.admitted_bytes(), 0);
}
