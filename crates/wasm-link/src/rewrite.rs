// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The link: plan the injected indices from the scan, then re-encode the module with
//! the redirects applied and the injected items appended.
//!
//! Nothing that exists is renumbered. Injected functions take the indices after the
//! last defined function, injected globals the indices after the last global, and an
//! injected type (the run wrapper's, when the module has no such type) the index after
//! the last type. Existing indices in every body, table and export therefore stay valid,
//! and the rewrite touches exactly the references it redirects.

use std::collections::BTreeMap;

use wasm_encoder::reencode::{Error as ReencodeError, Reencode, utils};
use wasm_encoder::{
    CodeSection, ConstExpr, CustomSection, DataCountSection, DataSection, ElementSection,
    ExportKind, ExportSection, FunctionSection, GlobalSection, GlobalType, ImportSection,
    Instruction, MemorySection, Module, NameMap, NameSection, StartSection, TableSection,
    TagSection, TypeSection, ValType,
};
use wasmparser::{ExternalKind, Operator, Parser, Payload, WasmFeatures};

use crate::error::LinkError;
use crate::scan::{Scan, Sig};
use crate::template::{self, Body, Gate, GateVariant, stack_pointer_global};
use crate::{
    ACTIVE_EXPORT, ADD_TO_STACK_POINTER_EXPORT, GateGlobals, IDLE_EXPORT, OUTBOUND_EXPORT,
    PARKED_EXPORT, POISONED_EXPORT, RESERVED_EXPORTS, RUN_EXPORT, Report, STACK_POINTER_EXPORT,
    SUSPEND_NAME, is_release_export, validate,
};

/// Link `bytes`: the rewritten module and what was found.
pub(crate) fn link(bytes: &[u8]) -> Result<(Vec<u8>, Report), LinkError> {
    let scan = Scan::read(bytes)?;
    let features = scan.features()?;
    validate(bytes, features, "input")?;
    for export in &scan.exports {
        if RESERVED_EXPORTS.contains(&export.name.as_str()) {
            return Err(LinkError::AlreadyLinked(format!(
                "export {:?} is already present, so the module has been through this tool",
                export.name
            )));
        }
    }
    let plan = Plan::new(&scan)?;
    let (output, rewriter) = emit(bytes, &scan, &plan)?;
    validate(&output, features, "output")?;
    if rewriter.suspend_sites == 0 {
        return Err(LinkError::Shape(format!(
            "no function calls {SUSPEND_NAME} (function {}); the asynchronous lane was compiled out",
            plan.suspend_import
        )));
    }
    Ok((output, plan.report(&rewriter, features)))
}

/// One gate wrapper: the function it guards and the export names it stands behind.
#[derive(Debug)]
struct GateEntry {
    /// The function the gate calls: the run wrapper for the run export, the trampoline
    /// for a re-exported import, the exported function itself otherwise.
    inner: u32,
    /// The wrapper's own index.
    wrapper: u32,
    /// The wrapper's signature (the guarded function's).
    sig: Sig,
    /// The wrapper's type index.
    type_index: u32,
    /// The first export name that reaches this wrapper.
    name: String,
    /// How the wrapper refuses, decided by the export name.
    variant: GateVariant,
}

/// Every index the rewrite introduces, decided before a byte is emitted.
#[derive(Debug)]
struct Plan {
    /// The shadow-stack pointer global.
    sp_global: u32,
    /// The suspending import's function index.
    suspend_import: u32,
    /// The number of imported functions.
    import_count: u32,
    /// The first injected function index: import `k`'s trampoline (or `$suspend`) is
    /// `new_base + k`.
    new_base: u32,
    /// The original run function.
    run_function: u32,
    /// Its signature.
    run_sig: Sig,
    /// The run wrapper's index.
    run_wrapper: u32,
    /// The run wrapper's type index.
    run_wrapper_type: u32,
    /// The run wrapper's type, when the module has no such type and one is appended.
    appended_type: Option<Sig>,
    /// The injected globals.
    gate: Gate,
    /// The gate wrappers, in order of first export.
    gates: Vec<GateEntry>,
    /// Exported function index → its gate wrapper.
    wrapper_of: BTreeMap<u32, u32>,
    /// Every function export name, in export order.
    wrapped_exports: Vec<String>,
    /// The function export names whose gate is inert, in export order.
    inert_exports: Vec<String>,
}

impl Plan {
    fn new(scan: &Scan<'_>) -> Result<Self, LinkError> {
        let sp_global = locate_stack_pointer(scan)?;
        let suspend_import = scan.suspend_import()?;
        let import_count = scan.import_count();
        let new_base = import_count + scan.defined_count();
        let run_function = scan.exported_function(RUN_EXPORT)?;
        if run_function < import_count {
            return Err(LinkError::Shape(format!(
                "{RUN_EXPORT} is an import (function {run_function}), not a defined function"
            )));
        }
        let run_sig = scan.sig_of(run_function)?.clone();
        let wrapper_sig = template::run_wrapper_sig(&run_sig);
        let (run_wrapper_type, appended_type) =
            match scan.types.iter().position(|sig| *sig == wrapper_sig) {
                Some(index) => (index as u32, None),
                None => (scan.types.len() as u32, Some(wrapper_sig.clone())),
            };
        let run_wrapper = new_base + import_count;
        let base = scan.global_count();
        let gate = Gate {
            sp: sp_global,
            idle: base,
            poisoned: base + 1,
            active: base + 2,
            parked: base + 3,
            outbound: base + 4,
        };
        let mut plan = Self {
            sp_global,
            suspend_import,
            import_count,
            new_base,
            run_function,
            run_sig,
            run_wrapper,
            run_wrapper_type,
            appended_type,
            gate,
            gates: Vec::new(),
            wrapper_of: BTreeMap::new(),
            wrapped_exports: Vec::new(),
            inert_exports: Vec::new(),
        };
        for export in &scan.exports {
            if export.kind != ExternalKind::Func {
                continue;
            }
            plan.wrapped_exports.push(export.name.clone());
            let variant = if is_release_export(&export.name) {
                plan.inert_exports.push(export.name.clone());
                GateVariant::Inert
            } else {
                GateVariant::Trapping
            };
            if let Some(wrapper) = plan.wrapper_of.get(&export.index) {
                // One function exported under several names shares one wrapper, so the
                // names must agree on how it refuses.
                let existing = plan
                    .gates
                    .iter()
                    .find(|entry| entry.wrapper == *wrapper)
                    .ok_or_else(|| {
                        LinkError::Shape(format!(
                            "export {:?} names a wrapper that was not planned",
                            export.name
                        ))
                    })?;
                if existing.variant != variant {
                    return Err(LinkError::Shape(format!(
                        "function {} is exported as {:?} ({:?} gate) and as {:?} ({variant:?} gate); one \
                         function cannot refuse both ways",
                        export.index, existing.name, existing.variant, export.name
                    )));
                }
                continue;
            }
            let wrapper = run_wrapper + 1 + plan.gates.len() as u32;
            let (inner, sig, type_index) = if export.index == run_function {
                (run_wrapper, wrapper_sig.clone(), run_wrapper_type)
            } else {
                (
                    plan.redirect(export.index),
                    scan.sig_of(export.index)?.clone(),
                    scan.type_index_of(export.index)?,
                )
            };
            plan.gates.push(GateEntry {
                inner,
                wrapper,
                sig,
                type_index,
                name: export.name.clone(),
                variant,
            });
            plan.wrapper_of.insert(export.index, wrapper);
        }
        Ok(plan)
    }

    /// Where a reference to function `func` now points: an import's trampoline (or
    /// `$suspend`), or the function itself.
    fn redirect(&self, func: u32) -> u32 {
        if func < self.import_count {
            self.new_base + func
        } else {
            func
        }
    }

    fn report(&self, rewriter: &Rewriter<'_>, features: WasmFeatures) -> Report {
        Report {
            stack_pointer_global: self.sp_global,
            suspend_import: self.suspend_import,
            suspend_function: self.redirect(self.suspend_import),
            suspend_call_sites: rewriter.suspend_sites,
            trampolines: (self.import_count - 1) as usize,
            trampoline_references: rewriter.routed_sites,
            run_wrapper: self.run_wrapper,
            wrapped_exports: self.wrapped_exports.clone(),
            inert_exports: self.inert_exports.clone(),
            gate_globals: GateGlobals {
                idle: self.gate.idle,
                poisoned: self.gate.poisoned,
                active: self.gate.active,
                parked: self.gate.parked,
                outbound: self.gate.outbound,
            },
            features: format!("{features:?}"),
        }
    }
}

/// The shadow-stack pointer: the global `__wbindgen_add_to_stack_pointer` sets, when
/// that function's body is exactly the expected shape.
fn locate_stack_pointer(scan: &Scan<'_>) -> Result<u32, LinkError> {
    let function = scan.exported_function(ADD_TO_STACK_POINTER_EXPORT)?;
    let expected = Sig {
        params: vec![ValType::I32],
        results: vec![ValType::I32],
    };
    let sig = scan.sig_of(function)?;
    if *sig != expected {
        return Err(LinkError::Shape(format!(
            "{ADD_TO_STACK_POINTER_EXPORT} (function {function}) has type {sig:?}, not (i32) -> i32"
        )));
    }
    let body = Body::decode(scan.body_of(function)?)?;
    let global = stack_pointer_global(&body).ok_or_else(|| {
        LinkError::Shape(format!(
            "{ADD_TO_STACK_POINTER_EXPORT} (function {function}) has the body {:?} with locals {:?}; expected \
             `local.get 0; global.get G; i32.add; global.set G; global.get G` (either operand order) and no locals",
            body.ops, body.locals
        ))
    })?;
    match scan.globals.get(global as usize) {
        Some(entry) if entry.defined && entry.mutable && entry.val_type == ValType::I32 => {
            Ok(global)
        }
        Some(entry) => Err(LinkError::Shape(format!(
            "{ADD_TO_STACK_POINTER_EXPORT} sets global {global}, which is {entry:?} rather than a defined mutable i32"
        ))),
        None => Err(LinkError::Shape(format!(
            "{ADD_TO_STACK_POINTER_EXPORT} sets global {global}, which does not exist"
        ))),
    }
}

/// The re-encoder: every section is translated as it is, except that function
/// references to imports are redirected and a reference to the suspending import other
/// than a direct call is refused.
struct Rewriter<'p> {
    plan: &'p Plan,
    /// Direct calls of the suspending import, now calls of `$suspend`.
    suspend_sites: usize,
    /// References to other imports, now references to their trampolines.
    routed_sites: usize,
}

impl Reencode for Rewriter<'_> {
    type Error = LinkError;

    fn function_index(&mut self, func: u32) -> Result<u32, ReencodeError<LinkError>> {
        if func == self.plan.suspend_import {
            return Err(ReencodeError::UserError(LinkError::Shape(format!(
                "{SUSPEND_NAME} (function {func}) is referenced other than by a direct call (a table \
                 element, a start section or ref.func); an indirect call would bypass $suspend"
            ))));
        }
        if func < self.plan.import_count {
            self.routed_sites += 1;
        }
        Ok(self.plan.redirect(func))
    }

    fn instruction<'a>(
        &mut self,
        operator: Operator<'a>,
    ) -> Result<Instruction<'a>, ReencodeError<LinkError>> {
        match operator {
            Operator::Call { function_index } if function_index == self.plan.suspend_import => {
                self.suspend_sites += 1;
                Ok(Instruction::Call(self.plan.redirect(function_index)))
            }
            Operator::ReturnCall { function_index }
                if function_index == self.plan.suspend_import =>
            {
                self.suspend_sites += 1;
                Ok(Instruction::ReturnCall(self.plan.redirect(function_index)))
            }
            other => utils::instruction(self, other),
        }
    }
}

/// The rewritten module bytes and the rewriter that counted the redirects.
fn emit<'p>(
    bytes: &[u8],
    scan: &Scan<'_>,
    plan: &'p Plan,
) -> Result<(Vec<u8>, Rewriter<'p>), LinkError> {
    let mut rewriter = Rewriter {
        plan,
        suspend_sites: 0,
        routed_sites: 0,
    };
    let mut types = TypeSection::new();
    let mut imports = ImportSection::new();
    let mut functions = FunctionSection::new();
    let mut tables = TableSection::new();
    let mut memories = MemorySection::new();
    let mut tags = TagSection::new();
    let mut globals = GlobalSection::new();
    let mut exports = ExportSection::new();
    let mut start = None;
    let mut elements = ElementSection::new();
    let mut data_count = None;
    let mut code = CodeSection::new();
    let mut data = DataSection::new();
    let mut customs: Vec<CustomSection<'_>> = Vec::new();
    let (mut has_tables, mut has_memories, mut has_tags, mut has_elements, mut has_data) =
        (false, false, false, false, false);

    for payload in Parser::new(0).parse_all(bytes) {
        match payload? {
            Payload::Version { .. }
            | Payload::End(_)
            | Payload::CodeSectionStart { .. }
            | Payload::ExportSection(_) => {}
            Payload::TypeSection(section) => rewriter.parse_type_section(&mut types, section)?,
            Payload::ImportSection(section) => {
                rewriter.parse_import_section(&mut imports, section)?;
            }
            Payload::FunctionSection(section) => {
                rewriter.parse_function_section(&mut functions, section)?;
            }
            Payload::TableSection(section) => {
                has_tables = true;
                rewriter.parse_table_section(&mut tables, section)?;
            }
            Payload::MemorySection(section) => {
                has_memories = true;
                rewriter.parse_memory_section(&mut memories, section)?;
            }
            Payload::TagSection(section) => {
                has_tags = true;
                rewriter.parse_tag_section(&mut tags, section)?;
            }
            Payload::GlobalSection(section) => {
                rewriter.parse_global_section(&mut globals, section)?;
            }
            Payload::StartSection { func, .. } => start = Some(rewriter.function_index(func)?),
            Payload::ElementSection(section) => {
                has_elements = true;
                rewriter.parse_element_section(&mut elements, section)?;
            }
            Payload::DataCountSection { count, .. } => {
                data_count = Some(rewriter.data_count(count)?);
            }
            Payload::DataSection(section) => {
                has_data = true;
                rewriter.parse_data_section(&mut data, section)?;
            }
            Payload::CodeSectionEntry(body) => rewriter.parse_function_body(&mut code, body)?,
            Payload::CustomSection(section) => customs.push(rewriter.custom_section(section)?),
            other => {
                return Err(LinkError::Shape(format!(
                    "unexpected section in a core module: {other:?}"
                )));
            }
        }
    }

    // The appended type, functions, globals and bodies, in one fixed order: the
    // trampolines (import order, `$suspend` at the suspending import's position), the
    // run wrapper, then the gate wrappers.
    if let Some(sig) = &plan.appended_type {
        types
            .ty()
            .function(sig.params.iter().copied(), sig.results.iter().copied());
    }
    let gate = plan.gate;
    for (index, import) in scan.imports.iter().enumerate() {
        functions.function(import.ty);
        let sig = scan.sig_of(index as u32)?;
        let body = if index as u32 == plan.suspend_import {
            template::suspend(sig, index as u32, gate)
        } else {
            template::trampoline(sig, index as u32, gate)
        };
        code.function(&body.encode());
    }
    functions.function(plan.run_wrapper_type);
    code.function(&template::run_wrapper(&plan.run_sig, plan.run_function, gate).encode());
    for entry in &plan.gates {
        functions.function(entry.type_index);
        code.function(&template::gate(&entry.sig, entry.inner, gate, entry.variant).encode());
    }
    let counter = GlobalType {
        val_type: ValType::I32,
        mutable: true,
        shared: false,
    };
    for _ in [
        gate.idle,
        gate.poisoned,
        gate.active,
        gate.parked,
        gate.outbound,
    ] {
        globals.global(counter, &ConstExpr::i32_const(0));
    }
    for export in &scan.exports {
        let index = if export.kind == ExternalKind::Func {
            *plan.wrapper_of.get(&export.index).ok_or_else(|| {
                LinkError::Shape(format!("export {:?} was not planned", export.name))
            })?
        } else {
            export.index
        };
        exports.export(&export.name, rewriter.export_kind(export.kind)?, index);
    }
    for (name, global) in [
        (STACK_POINTER_EXPORT, gate.sp),
        (IDLE_EXPORT, gate.idle),
        (POISONED_EXPORT, gate.poisoned),
        (ACTIVE_EXPORT, gate.active),
        (PARKED_EXPORT, gate.parked),
        (OUTBOUND_EXPORT, gate.outbound),
    ] {
        exports.export(name, ExportKind::Global, global);
    }

    let mut module = Module::new();
    module.section(&types);
    module.section(&imports);
    module.section(&functions);
    if has_tables {
        module.section(&tables);
    }
    if has_memories {
        module.section(&memories);
    }
    if has_tags {
        module.section(&tags);
    }
    module.section(&globals);
    module.section(&exports);
    if let Some(function_index) = start {
        module.section(&StartSection { function_index });
    }
    if has_elements {
        module.section(&elements);
    }
    if let Some(count) = data_count {
        module.section(&DataCountSection { count });
    }
    module.section(&code);
    if has_data {
        module.section(&data);
    }
    for custom in &customs {
        module.section(custom);
    }
    if !scan.has_name_section {
        module.section(&names(scan, plan));
    }
    Ok((module.finish(), rewriter))
}

/// Names for the injected functions and globals, so a trap's stack trace and a
/// disassembly show `purrdf_gate:<export>` rather than a bare index.
fn names(scan: &Scan<'_>, plan: &Plan) -> NameSection {
    let mut functions = NameMap::new();
    for (index, import) in scan.imports.iter().enumerate() {
        let name = if index as u32 == plan.suspend_import {
            "purrdf_suspend".to_owned()
        } else {
            format!("purrdf_out:{}", import.name)
        };
        functions.append(plan.new_base + index as u32, &name);
    }
    functions.append(plan.run_wrapper, "purrdf_run");
    for entry in &plan.gates {
        let kind = match entry.variant {
            GateVariant::Trapping => "purrdf_gate",
            GateVariant::Inert => "purrdf_inert_gate",
        };
        functions.append(entry.wrapper, &format!("{kind}:{}", entry.name));
    }
    let mut globals = NameMap::new();
    let gate = plan.gate;
    for (index, name) in [
        (gate.sp, STACK_POINTER_EXPORT),
        (gate.idle, IDLE_EXPORT),
        (gate.poisoned, POISONED_EXPORT),
        (gate.active, ACTIVE_EXPORT),
        (gate.parked, PARKED_EXPORT),
        (gate.outbound, OUTBOUND_EXPORT),
    ] {
        globals.append(index, name);
    }
    let mut names = NameSection::new();
    names.functions(&functions);
    names.globals(&globals);
    names
}
