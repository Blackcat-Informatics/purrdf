// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Fixture modules built with wasm-encoder, in the shape wasm-bindgen and wasm-opt
//! leave the package's module in, and the link and check run over them.

use std::borrow::Cow;

use crate::binary::{ExternalKind, sections};
use wasm_encoder::{
    CodeSection, ConstExpr, CustomSection, ElementSection, Elements, EntityType, ExportKind,
    ExportSection, Function, FunctionSection, GlobalSection, GlobalType, ImportSection,
    Instruction, MemorySection, MemoryType, Module, RefType, TableSection, TableType, TypeSection,
    ValType,
};

use crate::scan::{Scan, Sig};
use crate::template::{self, Body, Gate, GateVariant, Op};
use crate::{
    ADD_TO_STACK_POINTER_EXPORT, LinkError, RESERVED_EXPORTS, RUN_EXPORT, SUSPEND_MODULE,
    SUSPEND_NAME, check, is_release_export, link,
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
    /// A second export name for the release function `__wbg_thing_free`.
    release_alias: Option<&'static str>,
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
            release_alias: None,
        }
    }
}

// Function indices of the fixture: two imports, then six defined functions.
const SUSPEND: u32 = 0;
const DROP_REF: u32 = 1;
const ADD_TO_SP: u32 = 2;
const RUN: u32 = 3;
const NOOP: u32 = 4;
const HELPER: u32 = 5;
/// Exported as `__wbg_thing_free`, a wasm-bindgen object release function.
const RELEASE: u32 = 6;
/// Exported as `pair`: the release function's type under a plain name.
const PAIR: u32 = 7;
/// The first injected function: `$suspend`, then the trampoline, then the run wrapper.
const FIRST_INJECTED: u32 = 8;
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
    types.ty().function([ValType::I32, ValType::I32], []); // 4: release, pair

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
    for ty in [0, 0, 2, 2, 4, 4] {
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
    exports.export("__wbg_thing_free", ExportKind::Func, RELEASE);
    exports.export("pair", ExportKind::Func, PAIR);
    if let Some(alias) = fixture.release_alias {
        exports.export(alias, ExportKind::Func, RELEASE);
    }
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
    // The release function and its plain-named twin: both take (ptr, dealloc) and return
    // nothing, as wasm-bindgen's `__wbg_<type>_free` does.
    code.function(&body(&[Instruction::End]));
    code.function(&body(&[Instruction::End]));

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
    Scan::read(bytes)
        .expect("a parseable module")
        .exports
        .into_iter()
        .map(|entry| (entry.name, entry.kind, entry.index))
        .collect()
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
        report.suspend_function, FIRST_INJECTED,
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
    assert_eq!(
        report.run_wrapper,
        FIRST_INJECTED + 2,
        "after the two trampolines"
    );
    assert_eq!(
        report.wrapped_exports,
        vec![
            ADD_TO_STACK_POINTER_EXPORT.to_owned(),
            RUN_EXPORT.to_owned(),
            "noop".to_owned(),
            "helper".to_owned(),
            "__wbg_thing_free".to_owned(),
            "pair".to_owned(),
        ]
    );
    assert_eq!(report.inert_exports, vec!["__wbg_thing_free".to_owned()]);
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
    assert!(
        report
            .describe()
            .contains("6 exported function(s) behind the poison gate (5 trapping, 1 inert"),
        "{}",
        report.describe()
    );
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

/// Rewrites only the export section while preserving all other bytes.
fn retargeted(linked: &[u8], retargets: &[(&'static str, u32)]) -> Vec<u8> {
    let mut tampered = Module::new();
    for section in sections(linked).expect("sections") {
        if section.id == 7 {
            let mut exports = ExportSection::new();
            for (name, kind, index) in exports_of(linked) {
                let index = retargets
                    .iter()
                    .find(|(key, _)| *key == name)
                    .map_or(index, |(_, target)| *target);
                exports.export(&name, kind.encoded(), index);
            }
            tampered.section(&exports);
        } else {
            tampered.section(&wasm_encoder::RawSection {
                id: section.id,
                data: section.data,
            });
        }
    }
    tampered.finish()
}

#[test]
fn check_refuses_a_linked_module_whose_export_bypasses_the_gate() {
    let input = build(&Fixture::default());
    let (linked, _) = link(&input).expect("the fixture links");
    let tampered = retargeted(&linked, &[("noop", NOOP)]);
    let error = check(&tampered).expect_err("noop no longer stands behind a gate");
    assert!(
        matches!(&error, LinkError::NotLinked(message) if message.contains("\"noop\"") && message.contains("Trapping variant of the poison gate")),
        "{error}"
    );
}

#[test]
fn a_release_export_gets_the_inert_gate_and_every_other_export_the_trapping_one() {
    let input = build(&Fixture::default());
    let (linked, report) = link(&input).expect("the fixture links");
    let scan = Scan::read(&linked).expect("the output scans");
    let gate = Gate {
        sp: SP,
        idle: report.gate_globals.idle,
        poisoned: report.gate_globals.poisoned,
        active: report.gate_globals.active,
        parked: report.gate_globals.parked,
        outbound: report.gate_globals.outbound,
    };
    let sig = Sig {
        params: vec![ValType::I32, ValType::I32],
        results: vec![],
    };
    let release_gate = Body::decode(
        scan.body_of(export_index(&linked, "__wbg_thing_free"))
            .expect("the release gate's body"),
    )
    .expect("decodes");
    assert_eq!(
        release_gate,
        template::gate(&sig, RELEASE, gate, GateVariant::Inert),
        "the release function stands behind the inert gate"
    );
    let pair_gate = Body::decode(
        scan.body_of(export_index(&linked, "pair"))
            .expect("pair's gate body"),
    )
    .expect("decodes");
    assert_eq!(
        pair_gate,
        template::gate(&sig, PAIR, gate, GateVariant::Trapping),
        "the same type under a plain name stands behind the trapping gate"
    );
    assert!(release_gate.ops.contains(&Op::Return) && !release_gate.ops.contains(&Op::Unreachable));
    assert!(pair_gate.ops.contains(&Op::Unreachable) && !pair_gate.ops.contains(&Op::Return));
}

#[test]
fn check_refuses_a_gate_of_the_variant_the_export_name_does_not_select() {
    let input = build(&Fixture::default());
    let (linked, _) = link(&input).expect("the fixture links");
    let release_gate = export_index(&linked, "__wbg_thing_free");
    let pair_gate = export_index(&linked, "pair");
    // Both gates have the same type, so swapping them still validates; the check must
    // see the release name in front of a trapping gate.
    let tampered = retargeted(&linked, &[("__wbg_thing_free", pair_gate)]);
    let error = check(&tampered).expect_err("a release export behind a trapping gate");
    assert!(
        matches!(&error, LinkError::NotLinked(message) if message.contains("\"__wbg_thing_free\"") && message.contains("Inert variant")),
        "{error}"
    );
    let tampered = retargeted(&linked, &[("pair", release_gate)]);
    let error = check(&tampered).expect_err("a plain export behind an inert gate");
    assert!(
        matches!(&error, LinkError::NotLinked(message) if message.contains("\"pair\"") && message.contains("Trapping variant")),
        "{error}"
    );
    // The neighbour: swapping both names onto each other's gate is refused for the same
    // reason, and the untouched module still checks.
    check(&linked).expect("the untampered module checks");
}

#[test]
fn a_function_exported_under_two_names_of_one_variant_shares_a_gate_and_of_two_is_refused() {
    let (linked, report) = link(&build(&Fixture {
        release_alias: Some("__wbg_other_free"),
        ..Fixture::default()
    }))
    .expect("two release names for one function link");
    assert_eq!(
        report.inert_exports,
        vec!["__wbg_thing_free".to_owned(), "__wbg_other_free".to_owned()]
    );
    assert_eq!(
        export_index(&linked, "__wbg_thing_free"),
        export_index(&linked, "__wbg_other_free"),
        "one function, one gate"
    );
    check(&linked).expect("the aliased module checks");

    let error = link(&build(&Fixture {
        release_alias: Some("releaseAlias"),
        ..Fixture::default()
    }))
    .expect_err("one function cannot refuse both ways");
    assert!(
        matches!(&error, LinkError::Shape(message) if message.contains("\"__wbg_thing_free\"") && message.contains("\"releaseAlias\"")),
        "{error}"
    );
}

#[test]
fn only_wasm_bindgen_release_names_select_the_inert_gate() {
    for name in [
        "__wbg_dataset_free",
        "__wbg_thing_free",
        "__wbg_a1_b2_free",
        "__wbg___free",
    ] {
        assert!(is_release_export(name), "{name}");
    }
    for name in [
        "__wbg_free",
        "__wbindgen_free",
        "__wbg_Dataset_free",
        "__wbg_dataset_free_",
        "asyncevidence_freezeMs",
        "wbg_dataset_free",
        "__wbg_dataset_freed",
        "__wbindgen_export4",
        "free",
    ] {
        assert!(!is_release_export(name), "{name}");
    }
}

#[test]
fn declared_target_features_widen_validation_and_an_unknown_one_is_refused() {
    let (_, baseline) = link(&build(&Fixture::default())).expect("the fixture links");
    assert!(baseline.features.contains("simd"), "{}", baseline.features);
    assert!(
        !baseline.features.contains("reference-types"),
        "{}",
        baseline.features
    );
    let input = build(&Fixture {
        target_features: Some(&["mutable-globals", "sign-ext", "reference-types"]),
        ..Fixture::default()
    });
    let (_, report) = link(&input).expect("declared known features link");
    assert!(
        report.features.contains("reference-types"),
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

#[test]
fn check_refuses_an_import_in_a_global_initializer() {
    let (linked, _) = link(&build(&Fixture {
        target_features: Some(&["reference-types"]),
        ..Fixture::default()
    }))
    .expect("fixture links");
    let mut module = Module::new();
    let global = Scan::read(&linked).expect("scan").global_count();
    for section in sections(&linked).expect("sections") {
        if section.id == 6 {
            let mut reader = crate::binary::Reader::new(section.data);
            let count = reader.u32().expect("count");
            let mut data = Vec::new();
            crate::binary::put_u32(count + 1, &mut data);
            data.extend_from_slice(&section.data[reader.pos..]);
            data.extend_from_slice(&[0x70, 0, 0xd2]);
            crate::binary::put_u32(DROP_REF, &mut data);
            data.push(0x0b);
            module.section(&wasm_encoder::RawSection { id: 6, data: &data });
        } else if section.id == 7 {
            let mut exports = ExportSection::new();
            for (name, kind, index) in exports_of(&linked) {
                exports.export(&name, kind.encoded(), index);
            }
            exports.export("raw_import", ExportKind::Global, global);
            module.section(&exports);
        } else {
            module.section(&wasm_encoder::RawSection {
                id: section.id,
                data: section.data,
            });
        }
    }
    let tampered = module.finish();
    let scan = Scan::read(&tampered).expect("scan");
    crate::validate(&tampered, &scan.features().expect("features"), "probe").expect("valid wasm");
    let error = check(&tampered).expect_err("raw imported global reference bypasses trampoline");
    assert!(
        matches!(error,LinkError::NotLinked(message) if message.contains("global initializer"))
    );
}

#[test]
fn instruction_immediates_are_never_interpreted_as_calls() {
    use crate::binary::Reader;
    // f64 payload, SIMD constant and shuffle bytes deliberately contain call opcodes.
    for bytes in [
        &[0x44, 0x10, 0, 0x12, 0, 0xd2, 0, 0x10, 0][..],
        &[
            0xfd, 12, 0x10, 0, 0x12, 0, 0xd2, 0, 0x10, 0, 0x10, 0, 0x12, 0, 0xd2, 0, 0x10, 0,
        ][..],
    ] {
        let mut reader = Reader::new(bytes);
        let (_, reference) = reader.op().expect("one instruction");
        assert!(reference.is_none());
        assert!(reader.done());
    }
    let mut reader = Reader::new(&[0x10, 0x80, 0x01]);
    let (_, reference) = reader.op().expect("call");
    assert_eq!(reference, Some((1, 3, 128)));
    for bytes in [
        &[0xff, 0xff, 0xff, 0xff, 0x10][..],
        &[0x80][..],
        &[0x80, 0x80, 0x80, 0x80, 0x80, 0][..],
    ] {
        assert!(Reader::new(bytes).u32().is_err());
    }
    assert_eq!(
        Reader::new(&[0xff, 0xff, 0xff, 0xff, 0x0f])
            .u32()
            .expect("u32 max"),
        u32::MAX
    );
}

#[test]
fn reference_block_types_and_wide_arithmetic_preserve_instruction_boundaries() {
    use crate::binary::Reader;
    for code in 19..=22 {
        let bytes = [0xfc, code, 0x10, 7];
        let mut reader = Reader::new(&bytes);
        assert!(reader.op().expect("wide arithmetic").1.is_none());
        assert_eq!(reader.op().expect("following call").1, Some((3, 4, 7)));
        assert!(reader.done());
    }
    for nullable in [0x63, 0x64] {
        for heap in [0x69, 0x6e, 0x6f, 0x70, 0x71, 0x73] {
            let bytes = [0x02, nullable, heap, 0x10, 7];
            let mut reader = Reader::new(&bytes);
            assert!(reader.op().expect("reference block type").1.is_none());
            assert_eq!(reader.op().expect("following call").1, Some((4, 5, 7)));
            assert!(reader.done());
            assert!(Reader::new(&[nullable, heap]).val().is_ok());
        }
    }
}

#[test]
fn try_table_reference_results_do_not_swallow_catches_or_following_calls() {
    use crate::binary::Reader;
    for nullable in [0x63, 0x64] {
        // Every catch form: catch(tag,label), catch_ref(tag,label),
        // catch_all(label), catch_all_ref(label). Immediate bytes also equal call.
        let bytes = [
            0x1f, nullable, 0x70, 4, 0, 0x10, 0, 1, 0x10, 0, 2, 0x10, 3, 0x10, 0x10, 7,
        ];
        let mut reader = Reader::new(&bytes);
        assert!(
            reader
                .op()
                .expect("try_table reference block type and catches")
                .1
                .is_none()
        );
        assert_eq!(reader.pos, 14);
        assert_eq!(reader.op().expect("following call").1, Some((15, 16, 7)));
        assert!(reader.done());
    }
}

#[test]
fn frozen_instruction_spans_match_the_independent_decoder() {
    use crate::binary::Reader;
    let fixture = include_str!("../tests/fixtures/instruction-spans.tsv");
    let mut count = 0;
    for line in fixture
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
    {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 3, "invalid frozen fixture row: {line}");
        let encoded = fields[0].as_bytes();
        assert_eq!(encoded.len() % 2, 0, "odd-length hex: {line}");
        let bytes: Vec<_> = encoded
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| {
                let high = char::from(pair[0]).to_digit(16).expect("hex digit");
                let low = char::from(pair[1]).to_digit(16).expect("hex digit");
                ((high << 4) | low) as u8
            })
            .collect();
        let expected_span: usize = fields[1].parse().expect("frozen instruction span");
        let expected_reference: Option<u32> = if fields[2] == "-" {
            None
        } else {
            Some(fields[2].parse().expect("frozen function index"))
        };
        let mut reader = Reader::new(&bytes);
        let (_, reference) = reader
            .op()
            .unwrap_or_else(|error| panic!("row {count}: {line}: {error}"));
        assert_eq!(
            reader.pos, expected_span,
            "instruction boundary in row {count}: {line}"
        );
        assert_eq!(
            reference.map(|(_, _, index)| index),
            expected_reference,
            "function reference in row {count}: {line}"
        );
        if let Some((start, end, index)) = reference {
            assert_eq!(end, expected_span, "function immediate end: {line}");
            assert_eq!(
                Reader::new(&bytes[start..end])
                    .u32()
                    .expect("function immediate"),
                index
            );
        }
        assert_eq!(
            reader.op().expect("sentinel call").0,
            Op::Call(0),
            "following instruction in row {count}: {line}"
        );
        count += 1;
    }
    assert_eq!(count, 2_283, "frozen differential coverage must not shrink");
}
