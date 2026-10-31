// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc.
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Exercise the borrowed native literal validator and its original stop error.

use purrdf_core::ir::validate::validate_literal_with_control;
use purrdf_lex::allocation::{Memory, Resident};
use purrdf_lex::control::ControlledError;

fn main() {
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let mut work = 0u64;
    validate_literal_with_control(
        "[_:a, _:a]",
        purrdf_cdt::CDT_LIST,
        None,
        None,
        &mut memory,
        &mut |units| {
            work += units;
            Ok::<_, u8>(())
        },
    )
    .expect("valid native literal");
    assert_eq!(memory.admitted_bytes(), 0);
    let result = validate_literal_with_control(
        "[]",
        purrdf_cdt::CDT_LIST,
        None,
        None,
        &mut memory,
        &mut |_| Err(7u8),
    );
    assert!(matches!(result, Err(ControlledError::Control(7))));
    assert_eq!(memory.admitted_bytes(), 0);
    assert!(work > 0);
}
