// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Original scanner bytes, physical before-growth refusal, and retained token storage.

use purrdf_alloc_probe::{CountingAllocator, CurrentThreadWindow};
use purrdf_lex::allocation::{Admission, Memory, StorageError};
use purrdf_sparql_algebra::error::ParseError;
use purrdf_sparql_algebra::lexer::{LexerOptions, tokenize, tokenize_with_memory};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

struct Account {
    window: CurrentThreadWindow,
    live: usize,
    peak: usize,
    ceiling: usize,
}

impl Admission for Account {
    fn resize(&mut self, bytes: usize) -> Result<(), StorageError> {
        let measured = self.window.sample();
        assert!(measured.retained_bytes <= i64::try_from(self.live).unwrap());
        if bytes > self.ceiling {
            return Err(StorageError::AdmissionFailed);
        }
        self.live = bytes;
        self.peak = self.peak.max(bytes);
        Ok(())
    }
}

#[test]
fn escaped_tokens_keep_exact_original_storage_and_scanner_values() {
    let input = format!(
        "SELECT ?x {{ {} }}",
        "<http://example.org/a\\u0041> p:a\\~b \"c\\nlong\" . ".repeat(1024)
    );
    let expected = tokenize(&input).unwrap();
    let mut account = Account {
        window: CurrentThreadWindow::open(),
        live: 0,
        peak: 0,
        ceiling: usize::MAX,
    };
    let mut memory = Memory::new(&mut account);
    let tokens = tokenize_with_memory(&input, LexerOptions::default(), &mut memory).unwrap();
    assert_eq!(tokens, expected);
    let live = memory.admitted_bytes();
    drop(tokens);
    memory.release_bytes(live).unwrap();
    let measured = account.window.close();
    assert!(measured.peak_working_bytes <= i64::try_from(account.peak).unwrap());
    assert_eq!(measured.retained_bytes, 0);
    assert_eq!(account.live, 0);
}

#[test]
fn first_token_capacity_refuses_before_allocating() {
    let mut account = Account {
        window: CurrentThreadWindow::open(),
        live: 0,
        peak: 0,
        ceiling: 0,
    };
    let mut memory = Memory::new(&mut account);
    let error = tokenize_with_memory("SELECT", LexerOptions::default(), &mut memory).unwrap_err();
    assert_eq!(error, ParseError::Storage(StorageError::AdmissionFailed));
    drop(error);
    let measured = account.window.close();
    assert_eq!(measured.allocations, 0);
    assert_eq!(measured.retained_bytes, 0);
}

#[test]
fn lexical_failure_releases_token_and_decoding_scratch_before_publication() {
    let input = format!("{} \"bad\\z\"", "?x ".repeat(1024));
    let expected = tokenize(&input).unwrap_err();
    let mut account = Account {
        window: CurrentThreadWindow::open(),
        live: 0,
        peak: 0,
        ceiling: usize::MAX,
    };
    let mut memory = Memory::new(&mut account);
    let error = tokenize_with_memory(&input, LexerOptions::default(), &mut memory).unwrap_err();
    assert_eq!(error, expected);
    let ParseError::Lex { reason, .. } = &error else {
        panic!("lexical error")
    };
    assert_eq!(memory.admitted_bytes(), reason.capacity());
    let live = memory.admitted_bytes();
    drop(error);
    memory.release_bytes(live).unwrap();
    let measured = account.window.close();
    assert!(measured.peak_working_bytes <= i64::try_from(account.peak).unwrap());
    assert_eq!(measured.retained_bytes, 0);
    assert_eq!(account.live, 0);
}
