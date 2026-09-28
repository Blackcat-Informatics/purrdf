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

use crate::binary::{ExternalKind, Reader, put_u32};
use crate::error::LinkError;
use crate::scan::{Scan, Sig};
use crate::template::Op;
use crate::template::{self, Body, Gate, GateVariant, stack_pointer_global};
use crate::{
    ACTIVE_EXPORT, ADD_TO_STACK_POINTER_EXPORT, GateGlobals, IDLE_EXPORT, OUTBOUND_EXPORT,
    PARKED_EXPORT, POISONED_EXPORT, RESERVED_EXPORTS, RUN_EXPORT, Report, STACK_POINTER_EXPORT,
    SUSPEND_NAME, is_release_export, validate,
};
use wasm_encoder::{
    CodeSection, ConstExpr, Encode, ExportKind, ExportSection, FunctionSection, GlobalSection,
    GlobalType, Module, NameMap, NameSection, RawSection, TypeSection, ValType,
};

/// Link `bytes`: the rewritten module and what was found.
pub(crate) fn link(bytes: &[u8]) -> Result<(Vec<u8>, Report), LinkError> {
    let scan = Scan::read(bytes)?;
    let features = scan.features()?;
    validate(bytes, &features, "input")?;
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
    validate(&output, &features, "output")?;
    if rewriter.suspend_sites == 0 {
        return Err(LinkError::Shape(format!(
            "no function calls {SUSPEND_NAME} (function {}); the asynchronous lane was compiled out",
            plan.suspend_import
        )));
    }
    Ok((output, plan.report(&rewriter, &features)))
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

    fn report(&self, rewriter: &Rewriter<'_>, features: &[String]) -> Report {
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

impl Rewriter<'_> {
    fn redirect(&mut self, index: u32, direct: bool) -> Result<u32, LinkError> {
        if index == self.plan.suspend_import {
            if !direct {
                return Err(LinkError::Shape(format!(
                    "{SUSPEND_NAME} (function {index}) is referenced other than by a direct call; an indirect call would bypass $suspend"
                )));
            }
            self.suspend_sites += 1;
        } else if index < self.plan.import_count {
            self.routed_sites += 1;
        }
        Ok(self.plan.redirect(index))
    }
    fn body(&mut self, data: &[u8]) -> Result<Vec<u8>, LinkError> {
        let mut r = Reader::new(data);
        for _ in 0..r.u32()? {
            r.u32()?;
            r.val()?;
        }
        let mut output = data[..r.pos].to_vec();
        let mut copied = r.pos;
        while !r.done() {
            let (op, reference) = r.op()?;
            if let Some((start, end, index)) = reference {
                output.extend_from_slice(&data[copied..start]);
                put_u32(
                    self.redirect(index, matches!(op, Op::Call(_) | Op::ReturnCall(_)))?,
                    &mut output,
                );
                copied = end;
            }
        }
        output.extend_from_slice(&data[copied..]);
        Ok(output)
    }
}
/// Append an encoded section vector to a byte-preserved existing section vector.
fn append_vector(old: Option<&[u8]>, encoded: &[u8]) -> Result<Vec<u8>, LinkError> {
    // wasm-encoder's Section encoding starts with its payload length.
    let mut added = Reader::new(encoded);
    added.u32()?;
    let extra = added.u32()?;
    let mut original = Reader::new(old.unwrap_or(&[0]));
    let count = original.u32()?;
    let mut output = Vec::new();
    put_u32(
        count
            .checked_add(extra)
            .ok_or_else(|| LinkError::Parse("section count overflow".into()))?,
        &mut output,
    );
    output.extend_from_slice(&original.data[original.pos..]);
    output.extend_from_slice(&added.data[added.pos..]);
    Ok(output)
}
fn emit<'p>(
    _bytes: &[u8],
    scan: &Scan<'_>,
    plan: &'p Plan,
) -> Result<(Vec<u8>, Rewriter<'p>), LinkError> {
    let mut rewriter = Rewriter {
        plan,
        suspend_sites: 0,
        routed_sites: 0,
    };
    let mut sections = BTreeMap::new();
    let mut customs = Vec::new();
    for section in &scan.sections {
        if section.id == 0 {
            customs.push(section.data);
            continue;
        }
        if section.id == 10 {
            continue;
        }
        let mut data = Vec::new();
        let mut copied = 0;
        for &(id, start, end, index, direct) in &scan.references {
            if id == section.id {
                data.extend_from_slice(&section.data[copied..start]);
                put_u32(rewriter.redirect(index, direct)?, &mut data);
                copied = end;
            }
        }
        data.extend_from_slice(&section.data[copied..]);
        sections.insert(section.id, data);
    }
    let mut types = TypeSection::new();
    if let Some(sig) = &plan.appended_type {
        types
            .ty()
            .function(sig.params.iter().copied(), sig.results.iter().copied());
    }
    let mut functions = FunctionSection::new();
    let mut code = CodeSection::new();
    for body in &scan.bodies {
        code.raw(&rewriter.body(body.data)?);
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
    let mut globals = GlobalSection::new();
    let counter = GlobalType {
        val_type: ValType::I32,
        mutable: true,
        shared: false,
    };
    for _ in 0..5 {
        globals.global(counter, &ConstExpr::i32_const(0));
    }
    for (id, encoded) in [
        (1, {
            let mut v = Vec::new();
            types.encode(&mut v);
            v
        }),
        (3, {
            let mut v = Vec::new();
            functions.encode(&mut v);
            v
        }),
        (6, {
            let mut v = Vec::new();
            globals.encode(&mut v);
            v
        }),
    ] {
        let payload = append_vector(sections.get(&id).map(Vec::as_slice), &encoded)?;
        sections.insert(id, payload);
    }
    let mut exports = ExportSection::new();
    for export in &scan.exports {
        let index = if export.kind == ExternalKind::Func {
            *plan.wrapper_of.get(&export.index).ok_or_else(|| {
                LinkError::Shape(format!("export {:?} was not planned", export.name))
            })?
        } else {
            export.index
        };
        exports.export(&export.name, export.kind.encoded(), index);
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
    // Tags precede globals; data-count precedes code despite their numeric IDs.
    for id in [1, 2, 3, 4, 5, 13, 6, 7, 8, 9, 12, 10, 11] {
        match id {
            7 => {
                module.section(&exports);
            }
            10 => {
                module.section(&code);
            }
            _ => {
                if let Some(data) = sections.get(&id) {
                    module.section(&RawSection { id, data });
                }
            }
        }
    }
    for data in customs {
        module.section(&RawSection { id: 0, data });
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
