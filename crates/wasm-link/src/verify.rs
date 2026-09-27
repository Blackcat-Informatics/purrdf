// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `--check`: prove an already-linked module still has every property the link
//! establishes, by regenerating each injected body from the indices the module names
//! and comparing.

use wasm_encoder::ValType;
use wasmparser::ExternalKind;

use crate::error::LinkError;
use crate::scan::{Scan, Sig};
use crate::template::{self, Body, Gate, callee_of, stack_pointer_global};
use crate::{
    ACTIVE_EXPORT, ADD_TO_STACK_POINTER_EXPORT, GateGlobals, IDLE_EXPORT, OUTBOUND_EXPORT,
    PARKED_EXPORT, POISONED_EXPORT, RUN_EXPORT, Report, STACK_POINTER_EXPORT, validate,
};

/// Check `bytes`: what a linked module was found to contain.
pub(crate) fn check(bytes: &[u8]) -> Result<Report, LinkError> {
    let scan = Scan::read(bytes)?;
    let features = scan.features()?;
    validate(bytes, features, "input")?;
    let gate = gate_globals(&scan)?;
    let import_count = scan.import_count();
    let bodies = scan
        .bodies
        .iter()
        .map(Body::decode)
        .collect::<Result<Vec<Body>, LinkError>>()?;
    let body_of = |func: u32| -> Result<&Body, LinkError> {
        if func < import_count {
            return Err(LinkError::NotLinked(format!(
                "function {func} is an import where a defined function was expected"
            )));
        }
        bodies
            .get((func - import_count) as usize)
            .ok_or_else(|| LinkError::NotLinked(format!("function {func} does not exist")))
    };

    // Every exported function is a gate wrapper; the run export's gate guards the run
    // wrapper, which guards a function of the run's own type.
    let mut wrapped_exports = Vec::new();
    let mut run_wrapper = None;
    let mut add_to_stack_pointer = None;
    for export in &scan.exports {
        if export.kind != ExternalKind::Func {
            continue;
        }
        let sig = scan.sig_of(export.index)?;
        let inner = callee_of(body_of(export.index)?, |inner| {
            template::gate(sig, inner, gate)
        })
        .ok_or_else(|| {
            LinkError::NotLinked(format!(
                "export {:?} (function {}) is not behind the poison gate",
                export.name, export.index
            ))
        })?;
        if inner < import_count {
            return Err(LinkError::NotLinked(format!(
                "export {:?} gates import {inner} directly instead of its trampoline",
                export.name
            )));
        }
        if export.name == RUN_EXPORT {
            run_wrapper = Some(check_run_wrapper(&scan, &body_of, inner, gate)?);
        }
        if export.name == ADD_TO_STACK_POINTER_EXPORT {
            add_to_stack_pointer = Some(inner);
        }
        wrapped_exports.push(export.name.clone());
    }
    let run_wrapper = run_wrapper
        .ok_or_else(|| LinkError::NotLinked(format!("no function export named {RUN_EXPORT:?}")))?;
    let add_to_stack_pointer = add_to_stack_pointer.ok_or_else(|| {
        LinkError::NotLinked(format!(
            "no function export named {ADD_TO_STACK_POINTER_EXPORT:?}"
        ))
    })?;
    let found = stack_pointer_global(body_of(add_to_stack_pointer)?).ok_or_else(|| {
        LinkError::NotLinked(format!(
            "{ADD_TO_STACK_POINTER_EXPORT} guards function {add_to_stack_pointer}, whose body is not the stack-pointer shape"
        ))
    })?;
    if found != gate.sp {
        return Err(LinkError::NotLinked(format!(
            "{STACK_POINTER_EXPORT} exports global {} but {ADD_TO_STACK_POINTER_EXPORT} sets global {found}",
            gate.sp
        )));
    }

    // Every import is reached through exactly one function, its trampoline (`$suspend`
    // for the suspending import), and never from a table.
    let suspend_import = scan.suspend_import()?;
    for target in &scan.element_functions {
        if *target < import_count {
            return Err(LinkError::NotLinked(format!(
                "import {target} is referenced from an element segment; only its trampoline may reach it"
            )));
        }
    }
    let mut referrers: Vec<Vec<u32>> = vec![Vec::new(); import_count as usize];
    for (offset, body) in bodies.iter().enumerate() {
        let func = import_count + offset as u32;
        for target in body.referenced_functions() {
            if target < import_count && !referrers[target as usize].contains(&func) {
                referrers[target as usize].push(func);
            }
        }
    }
    let mut suspend_function = None;
    let mut trampolines = Vec::new();
    for (index, referrers) in referrers.iter().enumerate() {
        let import = index as u32;
        let described = format!(
            "import {import} ({:?} {:?})",
            scan.imports[index].module, scan.imports[index].name
        );
        let [caller] = referrers.as_slice() else {
            return Err(LinkError::NotLinked(format!(
                "{described} is referenced from {} functions ({referrers:?}); exactly one, its trampoline, may reach it",
                referrers.len()
            )));
        };
        let sig = scan.sig_of(import)?;
        let expected = if import == suspend_import {
            template::suspend(sig, import, gate)
        } else {
            template::trampoline(sig, import, gate)
        };
        if *body_of(*caller)? != expected {
            return Err(LinkError::NotLinked(format!(
                "{described} is called from function {caller}, whose body is not its trampoline"
            )));
        }
        if import == suspend_import {
            suspend_function = Some(*caller);
        } else {
            trampolines.push(*caller);
        }
    }
    let suspend_function = suspend_function.ok_or_else(|| {
        LinkError::NotLinked(format!("import {suspend_import} has no $suspend function"))
    })?;

    let references_to = |targets: &[u32]| -> usize {
        bodies
            .iter()
            .enumerate()
            .filter(|(offset, _)| {
                let func = import_count + *offset as u32;
                !targets.contains(&func) && func != suspend_function
            })
            .flat_map(|(_, body)| body.referenced_functions())
            .filter(|target| targets.contains(target))
            .count()
            + scan
                .element_functions
                .iter()
                .filter(|target| targets.contains(target))
                .count()
    };

    Ok(Report {
        stack_pointer_global: gate.sp,
        suspend_import,
        suspend_function,
        suspend_call_sites: references_to(&[suspend_function]),
        trampolines: trampolines.len(),
        trampoline_references: references_to(&trampolines),
        run_wrapper,
        wrapped_exports,
        gate_globals: GateGlobals {
            idle: gate.idle,
            poisoned: gate.poisoned,
            active: gate.active,
            parked: gate.parked,
            outbound: gate.outbound,
        },
        features: format!("{features:?}"),
    })
}

/// The six exported globals, each a distinct defined mutable `i32`.
fn gate_globals(scan: &Scan<'_>) -> Result<Gate, LinkError> {
    let global = |name: &str| -> Result<u32, LinkError> {
        let index = match scan.export(name) {
            Some(export) if export.kind == ExternalKind::Global => export.index,
            Some(export) => {
                return Err(LinkError::NotLinked(format!(
                    "export {name:?} is a {:?}, not a global",
                    export.kind
                )));
            }
            None => {
                return Err(LinkError::NotLinked(format!(
                    "no export named {name:?}; the module has not been through this tool"
                )));
            }
        };
        match scan.globals.get(index as usize) {
            Some(entry) if entry.defined && entry.mutable && entry.val_type == ValType::I32 => {
                Ok(index)
            }
            Some(entry) => Err(LinkError::NotLinked(format!(
                "export {name:?} is global {index}, which is {entry:?} rather than a defined mutable i32"
            ))),
            None => Err(LinkError::NotLinked(format!(
                "export {name:?} names global {index}, which does not exist"
            ))),
        }
    };
    let gate = Gate {
        sp: global(STACK_POINTER_EXPORT)?,
        idle: global(IDLE_EXPORT)?,
        poisoned: global(POISONED_EXPORT)?,
        active: global(ACTIVE_EXPORT)?,
        parked: global(PARKED_EXPORT)?,
        outbound: global(OUTBOUND_EXPORT)?,
    };
    let mut indices = [
        gate.sp,
        gate.idle,
        gate.poisoned,
        gate.active,
        gate.parked,
        gate.outbound,
    ];
    indices.sort_unstable();
    if indices.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(LinkError::NotLinked(format!(
            "the exported gate globals are not distinct: {gate:?}"
        )));
    }
    Ok(gate)
}

/// The run wrapper behind the run export's gate: its type is the guarded run's with one
/// more `i32`, and its body is the run-wrapper template around a function of the run's
/// type. Returns the wrapper's index.
fn check_run_wrapper<'b>(
    scan: &Scan<'_>,
    body_of: &impl Fn(u32) -> Result<&'b Body, LinkError>,
    wrapper: u32,
    gate: Gate,
) -> Result<u32, LinkError> {
    let wrapper_sig = scan.sig_of(wrapper)?;
    let Some((ValType::I32, params)) = wrapper_sig.params.split_last() else {
        return Err(LinkError::NotLinked(format!(
            "{RUN_EXPORT} guards function {wrapper} of type {wrapper_sig:?}, which has no trailing i32 region-top parameter"
        )));
    };
    let run_sig = Sig {
        params: params.to_vec(),
        results: wrapper_sig.results.clone(),
    };
    let original = callee_of(body_of(wrapper)?, |original| {
        template::run_wrapper(&run_sig, original, gate)
    })
    .ok_or_else(|| {
        LinkError::NotLinked(format!(
            "{RUN_EXPORT} guards function {wrapper}, whose body is not the run wrapper"
        ))
    })?;
    if *scan.sig_of(original)? != run_sig {
        return Err(LinkError::NotLinked(format!(
            "the run wrapper (function {wrapper}) calls function {original}, whose type is not the run's"
        )));
    }
    Ok(wrapper)
}
