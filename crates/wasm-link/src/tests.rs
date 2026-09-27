// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fixture modules built with wasm-encoder, in the shape wasm-bindgen and wasm-opt
//! leave the package's module in, and the link and check run over them.

use std::borrow::Cow;
use std::convert::Infallible;

use wasm_encoder::reencode::{Error as ReencodeError, Reencode};
use wasm_encoder::{
    CodeSection, ConstExpr, CustomSection, ElementSection, Elements, EntityType, ExportKind,
    ExportSection, Function, FunctionSection, GlobalSection, GlobalType, ImportSection,
    Instruction, MemorySection, MemoryType, Module, RefType, TableSection, TableType, TypeSection,
    ValType,
};
use wasmparser::{ExportSectionReader, ExternalKind, Parser};

use crate::scan::{Scan, Sig};
use crate::{
    ADD_TO_STACK_POINTER_EXPORT, LinkError, RESERVED_EXPORTS, RUN_EXPORT, SUSPEND_MODULE,
    SUSPEND_NAME, check, link,
};

/// The body of the fixture's `__wbindgen_add_to_stack_pointer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpShape {
    /// `local.get 0; global.get 1; i32.add; global.set 1; global.get 1` (wasm-opt's order).
    Artifact,
    /// `global.get 1; local.get 0; i32.add; global.set 1; global.get 1` (wasm-bindgen's).
    Swapped,
    /// A body that does not add to a global.
    Wrong,
}

/// The knobs a fixture is built from.
#[derive(Debug, Clone)]
struct Fixture {
    /// Whether the run function calls the suspending import.
    call_import: bool,
    /// Whether `noop` takes a `ref.func` of the suspending import (declared in an
    /// element segment so the module validates).
    ref_func_import: bool,
    /// Whether the function table's active segment names the suspending import.
    elem_import: bool,
    /// The stack-pointer function's body.
    sp_shape: SpShape,
    /// The suspending import's field name.
    suspend_name: &'static str,
    /// The run export's name.
    run_export: &'static str,
    /// A `target_features` custom section's `+` entries.
    target_features: Option<&'static [&'static str]>,
}

impl Default for Fixture {
    fn default() -> Self {
        Self {
            call_import: true,
            ref_func_import: false,
            elem_import: false,
            sp_shape: SpShape::Artifact,
            suspend_name: SUSPEND_NAME,
            run_export: RUN_EXPORT,
            target_features: None,
        }
    }
}

// Function indices of the fixture: two imports, then four defined functions.
const SUSPEND: u32 = 0;
const DROP_REF: u32 = 1;
const ADD_TO_SP: u32 = 2;
const RUN: u32 = 3;
const NOOP: u32 = 4;
const HELPER: u32 = 5;
/// The stack-pointer global (global 0 is an immutable data-end marker).
const SP: u32 = 1;

fn body(instructions: &[Instruction<'_>]) -> Function {
    let mut function = Function::new([]);
    for instruction in instructions {
        function.instruction(instruction);
    }
    function
}

fn target_features(names: &[&str]) -> Vec<u8> {
    let mut data = vec![names.len() as u8];
    for name in names {
        data.push(b'+');
        data.push(name.len() as u8);
        data.extend_from_slice(name.as_bytes());
    }
    data
}

/// A module in the package's shape: the suspending import and one wasm-bindgen import,
/// the stack-pointer function, a run function, a trivial export and a helper that calls
/// the other import, a function table, a memory, the stack-pointer global behind an
/// immutable one, and exports for all of it.
fn build(fixture: &Fixture) -> Vec<u8> {
    let mut types = TypeSection::new();
    types.ty().function([ValType::I32], [ValType::I32]); // 0: add_to_sp, run
    types
        .ty()
        .function([ValType::I32, ValType::I32, ValType::I32], [ValType::I32]); // 1: suspend
    types.ty().function([], []); // 2: noop, helper
    types.ty().function([ValType::I32], []); // 3: drop_ref

    let mut imports = ImportSection::new();
    imports.import(
        SUSPEND_MODULE,
        fixture.suspend_name,
        EntityType::Function(1),
    );
    imports.import(
        "./purrdf_wasm_bg.js",
        "__wbindgen_object_drop_ref",
        EntityType::Function(3),
    );

    let mut functions = FunctionSection::new();
    for ty in [0, 0, 2, 2] {
        functions.function(ty);
    }

    let mut tables = TableSection::new();
    tables.table(TableType {
        element_type: RefType::FUNCREF,
        table64: false,
        minimum: 2,
        maximum: Some(2),
        shared: false,
    });

    let mut memories = MemorySection::new();
    memories.memory(MemoryType {
        minimum: 1,
        maximum: None,
        memory64: false,
        shared: false,
        page_size_log2: None,
    });

    let mut globals = GlobalSection::new();
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: false,
            shared: false,
        },
        &ConstExpr::i32_const(65_536),
    );
    globals.global(
        GlobalType {
            val_type: ValType::I32,
            mutable: true,
            shared: false,
        },
        &ConstExpr::i32_const(1_048_576),
    );

    let mut exports = ExportSection::new();
    exports.export("memory", ExportKind::Memory, 0);
    exports.export(ADD_TO_STACK_POINTER_EXPORT, ExportKind::Func, ADD_TO_SP);
    exports.export(fixture.run_export, ExportKind::Func, RUN);
    exports.export("noop", ExportKind::Func, NOOP);
    exports.export("helper", ExportKind::Func, HELPER);
    exports.export("__data_end", ExportKind::Global, 0);

    let mut elements = ElementSection::new();
    let table_entries: [u32; 2] = if fixture.elem_import {
        [SUSPEND, HELPER]
    } else {
        [NOOP, HELPER]
    };
    elements.active(
        None,
        &ConstExpr::i32_const(0),
        Elements::Functions(Cow::Borrowed(&table_entries)),
    );
    if fixture.ref_func_import {
        elements.declared(Elements::Functions(Cow::Borrowed(&[SUSPEND])));
    }

    let mut code = CodeSection::new();
    code.function(&body(&match fixture.sp_shape {
        SpShape::Artifact => vec![
            Instruction::LocalGet(0),
            Instruction::GlobalGet(SP),
            Instruction::I32Add,
            Instruction::GlobalSet(SP),
            Instruction::GlobalGet(SP),
            Instruction::End,
        ],
        SpShape::Swapped => vec![
            Instruction::GlobalGet(SP),
            Instruction::LocalGet(0),
            Instruction::I32Add,
            Instruction::GlobalSet(SP),
            Instruction::GlobalGet(SP),
            Instruction::End,
        ],
        SpShape::Wrong => vec![
            Instruction::LocalGet(0),
            Instruction::GlobalGet(SP),
            Instruction::I32Sub,
            Instruction::GlobalSet(SP),
            Instruction::GlobalGet(SP),
            Instruction::End,
        ],
    }));
    code.function(&body(&if fixture.call_import {
        vec![
            Instruction::LocalGet(0),
            Instruction::I32Const(0),
            Instruction::I32Const(0),
            Instruction::Call(SUSPEND),
            Instruction::Drop,
            Instruction::LocalGet(0),
            Instruction::End,
        ]
    } else {
        vec![Instruction::LocalGet(0), Instruction::End]
    }));
    code.function(&body(&if fixture.ref_func_import {
        vec![
            Instruction::RefFunc(SUSPEND),
            Instruction::Drop,
            Instruction::End,
        ]
    } else {
        vec![Instruction::End]
    }));
    code.function(&body(&[
        Instruction::I32Const(7),
        Instruction::Call(DROP_REF),
        Instruction::End,
    ]));

    let mut module = Module::new();
    module.section(&types);
    module.section(&imports);
    module.section(&functions);
    module.section(&tables);
    module.section(&memories);
    module.section(&globals);
    module.section(&exports);
    module.section(&elements);
    module.section(&code);
    if let Some(names) = fixture.target_features {
        let data = target_features(names);
        module.section(&CustomSection {
            name: Cow::Borrowed("target_features"),
            data: Cow::Borrowed(&data),
        });
    }
    module.finish()
}

/// `(name, kind, index)` for every export of `bytes`.
fn exports_of(bytes: &[u8]) -> Vec<(String, ExternalKind, u32)> {
    let mut found = Vec::new();
    for payload in Parser::new(0).parse_all(bytes) {
        if let wasmparser::Payload::ExportSection(reader) = payload.expect("a parseable module") {
            for export in reader {
                let export = export.expect("a parseable export");
                found.push((export.name.to_owned(), export.kind, export.index));
            }
        }
    }
    found
}

fn export_index(bytes: &[u8], name: &str) -> u32 {
    exports_of(bytes)
        .into_iter()
        .find(|(found, _, _)| found == name)
        .map_or_else(|| panic!("no export named {name}"), |(_, _, index)| index)
}

#[test]
fn links_the_fixture_reports_what_it_found_and_the_output_checks() {
    let input = build(&Fixture::default());
    let (linked, report) = link(&input).expect("the fixture links");
    assert_eq!(report.stack_pointer_global, SP);
    assert_eq!(report.suspend_import, SUSPEND);
    assert_eq!(
        report.suspend_function, 6,
        "the first injected function is import 0's $suspend"
    );
    assert_eq!(
        report.suspend_call_sites, 1,
        "run's one call of the import is redirected"
    );
    assert_eq!(report.trampolines, 1, "the other import has a trampoline");
    assert_eq!(
        report.trampoline_references, 1,
        "helper's one call of the other import is routed"
    );
    assert_eq!(report.run_wrapper, 8, "after the two trampolines");
    assert_eq!(
        report.wrapped_exports,
        vec![
            ADD_TO_STACK_POINTER_EXPORT.to_owned(),
            RUN_EXPORT.to_owned(),
            "noop".to_owned(),
            "helper".to_owned()
        ]
    );
    assert_eq!(
        (
            report.gate_globals.idle,
            report.gate_globals.poisoned,
            report.gate_globals.active,
            report.gate_globals.parked,
            report.gate_globals.outbound
        ),
        (2, 3, 4, 5, 6)
    );

    let exports = exports_of(&linked);
    for name in RESERVED_EXPORTS {
        assert!(
            exports
                .iter()
                .any(|(found, kind, _)| found == name && *kind == ExternalKind::Global),
            "{name} is exported as a global"
        );
    }
    assert_eq!(export_index(&linked, crate::STACK_POINTER_EXPORT), SP);
    // Every function export now points past the original functions and the trampolines.
    for (name, kind, index) in &exports {
        if *kind == ExternalKind::Func {
            assert!(
                *index > report.run_wrapper,
                "{name} (function {index}) is a gate wrapper"
            );
        }
    }
    let scan = Scan::read(&linked).expect("the output scans");
    assert_eq!(
        scan.sig_of(export_index(&linked, RUN_EXPORT))
            .expect("the run gate's type"),
        &Sig {
            params: vec![ValType::I32, ValType::I32],
            results: vec![ValType::I32],
        },
        "the run export takes the region top as one more i32"
    );

    let checked = check(&linked).expect("the linked module checks");
    assert_eq!(checked, report);
    assert!(report.describe().contains("1 call site(s)"));
}

#[test]
fn both_operand_orders_of_the_stack_pointer_body_are_accepted_and_another_body_is_refused() {
    for shape in [SpShape::Artifact, SpShape::Swapped] {
        let input = build(&Fixture {
            sp_shape: shape,
            ..Fixture::default()
        });
        let (_, report) = link(&input).expect("both operand orders link");
        assert_eq!(report.stack_pointer_global, SP);
    }
    let input = build(&Fixture {
        sp_shape: SpShape::Wrong,
        ..Fixture::default()
    });
    let error = link(&input).expect_err("a subtracting body is not the stack-pointer function");
    match error {
        LinkError::Shape(message) => {
            assert!(message.contains(ADD_TO_STACK_POINTER_EXPORT), "{message}");
            assert!(
                message.contains("I32Sub"),
                "the refusal names the body it found: {message}"
            );
        }
        other => panic!("expected a shape refusal, got {other}"),
    }
}

#[test]
fn a_ref_func_of_the_import_is_refused() {
    // `ref.func` needs the reference-types proposal, which the fixture declares so the
    // input validates and the refusal is the linker's, not the validator's.
    let input = build(&Fixture {
        ref_func_import: true,
        target_features: Some(&["reference-types"]),
        ..Fixture::default()
    });
    let error = link(&input).expect_err("ref.func of the import bypasses $suspend");
    assert!(
        matches!(&error, LinkError::Shape(message) if message.contains("other than by a direct call")),
        "{error}"
    );
}

#[test]
fn a_table_element_naming_the_import_is_refused() {
    let input = build(&Fixture {
        elem_import: true,
        ..Fixture::default()
    });
    let error = link(&input).expect_err("a table element naming the import bypasses $suspend");
    assert!(
        matches!(&error, LinkError::Shape(message) if message.contains("other than by a direct call")),
        "{error}"
    );
}

#[test]
fn a_module_in_which_nothing_calls_the_import_is_refused() {
    let input = build(&Fixture {
        call_import: false,
        ..Fixture::default()
    });
    let error = link(&input).expect_err("the lane was compiled out");
    assert!(
        matches!(&error, LinkError::Shape(message) if message.contains("no function calls")),
        "{error}"
    );
}

#[test]
fn a_missing_import_or_run_export_is_refused_by_name() {
    let input = build(&Fixture {
        suspend_name: "purrdf_jspi_other",
        ..Fixture::default()
    });
    let error = link(&input).expect_err("the suspending import is absent");
    assert!(
        matches!(&error, LinkError::Shape(message) if message.contains("found 0")),
        "{error}"
    );
    let input = build(&Fixture {
        run_export: "purrdf_jspi_other",
        ..Fixture::default()
    });
    let error = link(&input).expect_err("the run export is absent");
    assert!(
        matches!(&error, LinkError::Shape(message) if message.contains(RUN_EXPORT)),
        "{error}"
    );
}

#[test]
fn linking_twice_is_refused() {
    let input = build(&Fixture::default());
    let (linked, _) = link(&input).expect("the fixture links");
    let error = link(&linked).expect_err("a linked module is not linked again");
    assert!(matches!(error, LinkError::AlreadyLinked(_)), "{error}");
}

#[test]
fn check_refuses_an_unlinked_module() {
    let input = build(&Fixture::default());
    let error = check(&input).expect_err("the input is not linked");
    assert!(
        matches!(&error, LinkError::NotLinked(message) if message.contains(crate::STACK_POINTER_EXPORT)),
        "{error}"
    );
}

/// Re-encodes a linked module with one export pointed back at the function it guarded.
struct Bypass {
    export: &'static str,
    target: u32,
}

impl Reencode for Bypass {
    type Error = Infallible;

    fn parse_export_section(
        &mut self,
        exports: &mut ExportSection,
        section: ExportSectionReader<'_>,
    ) -> Result<(), ReencodeError<Infallible>> {
        for export in section {
            let export = export?;
            let index = if export.name == self.export {
                self.target
            } else {
                export.index
            };
            exports.export(export.name, self.export_kind(export.kind)?, index);
        }
        Ok(())
    }
}

#[test]
fn check_refuses_a_linked_module_whose_export_bypasses_the_gate() {
    let input = build(&Fixture::default());
    let (linked, _) = link(&input).expect("the fixture links");
    let mut tampered = Module::new();
    Bypass {
        export: "noop",
        target: NOOP,
    }
    .parse_core_module(&mut tampered, Parser::new(0), &linked)
    .expect("the linked module re-encodes");
    let tampered = tampered.finish();
    let error = check(&tampered).expect_err("noop no longer stands behind a gate");
    assert!(
        matches!(&error, LinkError::NotLinked(message) if message.contains("\"noop\"") && message.contains("poison gate")),
        "{error}"
    );
}

#[test]
fn declared_target_features_widen_validation_and_an_unknown_one_is_refused() {
    let (_, baseline) = link(&build(&Fixture::default())).expect("the fixture links");
    assert!(baseline.features.contains("SIMD"), "{}", baseline.features);
    assert!(
        !baseline.features.contains("REFERENCE_TYPES"),
        "{}",
        baseline.features
    );
    let input = build(&Fixture {
        target_features: Some(&["mutable-globals", "sign-ext", "reference-types"]),
        ..Fixture::default()
    });
    let (_, report) = link(&input).expect("declared known features link");
    assert!(
        report.features.contains("REFERENCE_TYPES"),
        "{}",
        report.features
    );
    let input = build(&Fixture {
        target_features: Some(&["fp16"]),
        ..Fixture::default()
    });
    let error = link(&input).expect_err("an unknown feature name has no validator flag");
    assert!(
        matches!(&error, LinkError::Shape(message) if message.contains("\"fp16\"")),
        "{error}"
    );
}
