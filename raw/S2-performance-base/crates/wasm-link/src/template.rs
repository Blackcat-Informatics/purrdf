// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The injected function bodies, written once as instruction templates.
//!
//! The link emits a template; the check decodes a body back into the same [`Op`]
//! vocabulary and compares it with the template regenerated from the indices the body
//! names. One definition therefore describes both what is written and what is accepted.

use crate::binary::{FunctionBody, Reader};
use wasm_encoder::{BlockType, Function, Instruction, ValType};

use crate::error::LinkError;
use crate::scan::Sig;

/// The instruction vocabulary the templates are written in. Any other operator decodes
/// to [`Op::Other`], which no template contains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Op {
    /// `local.get`.
    LocalGet(u32),
    /// `local.set`.
    LocalSet(u32),
    /// `global.get`.
    GlobalGet(u32),
    /// `global.set`.
    GlobalSet(u32),
    /// `i32.const`.
    I32Const(i32),
    /// `i32.add`.
    I32Add,
    /// `i32.sub`.
    I32Sub,
    /// `i32.ne`.
    I32Ne,
    /// `if` with an empty block type.
    If,
    /// `unreachable`.
    Unreachable,
    /// `end`.
    End,
    /// `call`.
    Call(u32),
    /// `return_call`.
    ReturnCall(u32),
    /// `ref.func`.
    RefFunc(u32),
    /// `drop`.
    Drop,
    /// `return`.
    Return,
    /// Any operator outside this vocabulary.
    Other,
}

impl Op {
    /// The encoder instruction for a template entry.
    fn instruction(self) -> Instruction<'static> {
        match self {
            Self::LocalGet(index) => Instruction::LocalGet(index),
            Self::LocalSet(index) => Instruction::LocalSet(index),
            Self::GlobalGet(index) => Instruction::GlobalGet(index),
            Self::GlobalSet(index) => Instruction::GlobalSet(index),
            Self::I32Const(value) => Instruction::I32Const(value),
            Self::I32Add => Instruction::I32Add,
            Self::I32Sub => Instruction::I32Sub,
            Self::I32Ne => Instruction::I32Ne,
            Self::If => Instruction::If(BlockType::Empty),
            Self::Unreachable => Instruction::Unreachable,
            Self::End => Instruction::End,
            Self::Call(index) => Instruction::Call(index),
            Self::ReturnCall(index) => Instruction::ReturnCall(index),
            Self::RefFunc(index) => Instruction::RefFunc(index),
            Self::Drop => Instruction::Drop,
            Self::Return => Instruction::Return,
            Self::Other => {
                unreachable!("a template never contains an operator outside its vocabulary")
            }
        }
    }

    /// The function a call, tail call or function reference names.
    pub(crate) fn referenced_function(self) -> Option<u32> {
        match self {
            Self::Call(index) | Self::ReturnCall(index) | Self::RefFunc(index) => Some(index),
            _ => None,
        }
    }
}

/// A function body: its declared locals (one entry per local, after the parameters)
/// and its instructions, the trailing `end` included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Body {
    /// The declared locals' types.
    pub(crate) locals: Vec<ValType>,
    /// The instructions, in order.
    pub(crate) ops: Vec<Op>,
}

impl Body {
    /// The encoded function.
    pub(crate) fn encode(&self) -> Function {
        let mut function = Function::new_with_locals_types(self.locals.iter().copied());
        for op in &self.ops {
            function.instruction(&op.instruction());
        }
        function
    }

    /// A body decoded into the template vocabulary.
    pub(crate) fn decode(body: &FunctionBody<'_>) -> Result<Self, LinkError> {
        let mut reader = Reader::new(body.data);
        let mut locals = Vec::new();
        for _ in 0..reader.u32()? {
            let count = reader.u32()? as usize;
            let ty = reader.val()?;
            if count > 1_000_000 - locals.len() {
                return Err(LinkError::Parse("too many locals".into()));
            }
            locals.extend(std::iter::repeat_n(ty, count));
        }
        let mut ops = Vec::new();
        while !reader.done() {
            ops.push(reader.op()?.0);
        }
        Ok(Self { locals, ops })
    }

    /// The functions this body names by call, tail call or `ref.func`.
    pub(crate) fn referenced_functions(&self) -> impl Iterator<Item = u32> + '_ {
        self.ops.iter().filter_map(|op| op.referenced_function())
    }
}

/// The globals the injected code reads and writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Gate {
    /// The shadow-stack pointer.
    pub(crate) sp: u32,
    /// The stack pointer of the JavaScript context that runs while wasm is suspended
    /// or returned.
    pub(crate) idle: u32,
    /// Non-zero once the instance may not be entered again.
    pub(crate) poisoned: u32,
    /// Gated export entries that have not returned.
    pub(crate) active: u32,
    /// `$suspend` calls waiting for the suspending import to return.
    pub(crate) parked: u32,
    /// Import calls in progress.
    pub(crate) outbound: u32,
}

/// The two operand orders of `__wbindgen_add_to_stack_pointer`'s body: wasm-bindgen
/// writes `global.get; local.get; i32.add`, and `wasm-opt` re-emits it as `local.get;
/// global.get; i32.add`. Both then `global.set` and re-read the global.
pub(crate) fn stack_pointer_shapes(global: u32) -> [Vec<Op>; 2] {
    [
        vec![
            Op::LocalGet(0),
            Op::GlobalGet(global),
            Op::I32Add,
            Op::GlobalSet(global),
            Op::GlobalGet(global),
            Op::End,
        ],
        vec![
            Op::GlobalGet(global),
            Op::LocalGet(0),
            Op::I32Add,
            Op::GlobalSet(global),
            Op::GlobalGet(global),
            Op::End,
        ],
    ]
}

/// The stack-pointer global a `__wbindgen_add_to_stack_pointer` body sets, when the body
/// has exactly one of the accepted shapes and no declared locals.
pub(crate) fn stack_pointer_global(body: &Body) -> Option<u32> {
    if !body.locals.is_empty() {
        return None;
    }
    let global = body.ops.iter().find_map(|op| match op {
        Op::GlobalSet(global) => Some(*global),
        _ => None,
    })?;
    stack_pointer_shapes(global)
        .contains(&body.ops)
        .then_some(global)
}

/// A body that forwards its first `forwarded` parameters to `callee`, running `before`
/// first and `after` once the callee has returned, and then returns the callee's
/// results. The results wait in locals across `after`; `extra_locals` follow them.
struct Forward<'a> {
    params: &'a [ValType],
    forwarded: u32,
    results: &'a [ValType],
    callee: u32,
    extra_locals: &'a [ValType],
    before: &'a [Op],
    after: &'a [Op],
}

impl Forward<'_> {
    fn body(&self) -> Body {
        let first_local = self.params.len() as u32;
        let result_count = self.results.len() as u32;
        let mut locals: Vec<ValType> = self.results.to_vec();
        locals.extend_from_slice(self.extra_locals);
        let mut ops: Vec<Op> = self.before.to_vec();
        ops.extend((0..self.forwarded).map(Op::LocalGet));
        ops.push(Op::Call(self.callee));
        ops.extend(
            (0..result_count)
                .rev()
                .map(|i| Op::LocalSet(first_local + i)),
        );
        ops.extend_from_slice(self.after);
        ops.extend((0..result_count).map(|i| Op::LocalGet(first_local + i)));
        ops.push(Op::End);
        Body { locals, ops }
    }
}

/// `$suspend`: the one function that calls the suspending import. It parks the job's
/// stack pointer in a local, switches to the idle context, counts itself as parked,
/// calls the import, and on resumption counts itself back in, records the resumer's
/// context as the new idle pointer, and restores its own stack pointer before
/// returning the import's result.
pub(crate) fn suspend(sig: &Sig, import: u32, gate: Gate) -> Body {
    let saved = sig.params.len() as u32 + sig.results.len() as u32;
    let before = [
        Op::GlobalGet(gate.sp),
        Op::LocalSet(saved),
        Op::GlobalGet(gate.idle),
        Op::GlobalSet(gate.sp),
        Op::GlobalGet(gate.parked),
        Op::I32Const(1),
        Op::I32Add,
        Op::GlobalSet(gate.parked),
    ];
    let after = [
        Op::GlobalGet(gate.parked),
        Op::I32Const(1),
        Op::I32Sub,
        Op::GlobalSet(gate.parked),
        Op::GlobalGet(gate.sp),
        Op::GlobalSet(gate.idle),
        Op::LocalGet(saved),
        Op::GlobalSet(gate.sp),
    ];
    Forward {
        params: &sig.params,
        forwarded: sig.params.len() as u32,
        results: &sig.results,
        callee: import,
        extra_locals: &[ValType::I32],
        before: &before,
        after: &after,
    }
    .body()
}

/// An import trampoline: every call of an import other than the suspending one goes
/// through it. It counts the call as outbound around the import and puts the idle
/// pointer back to what it was before JavaScript ran, so a job the callback started
/// cannot leave its own context's pointer behind as the caller's.
pub(crate) fn trampoline(sig: &Sig, import: u32, gate: Gate) -> Body {
    let saved_idle = sig.params.len() as u32 + sig.results.len() as u32;
    let before = [
        Op::GlobalGet(gate.idle),
        Op::LocalSet(saved_idle),
        Op::GlobalGet(gate.outbound),
        Op::I32Const(1),
        Op::I32Add,
        Op::GlobalSet(gate.outbound),
    ];
    let after = [
        Op::GlobalGet(gate.outbound),
        Op::I32Const(1),
        Op::I32Sub,
        Op::GlobalSet(gate.outbound),
        Op::LocalGet(saved_idle),
        Op::GlobalSet(gate.idle),
    ];
    Forward {
        params: &sig.params,
        forwarded: sig.params.len() as u32,
        results: &sig.results,
        callee: import,
        extra_locals: &[ValType::I32],
        before: &before,
        after: &after,
    }
    .body()
}

/// The signature of the run wrapper: the original run's parameters, then the region
/// top as one more `i32`.
pub(crate) fn run_wrapper_sig(run: &Sig) -> Sig {
    let mut params = run.params.clone();
    params.push(ValType::I32);
    Sig {
        params,
        results: run.results.clone(),
    }
}

/// The run wrapper: records the caller's stack pointer as idle, moves the stack
/// pointer to the region top the caller passed, runs the original, and puts the idle
/// pointer back before returning.
pub(crate) fn run_wrapper(run: &Sig, inner: u32, gate: Gate) -> Body {
    let wrapper = run_wrapper_sig(run);
    let top = run.params.len() as u32;
    let before = [
        Op::GlobalGet(gate.sp),
        Op::GlobalSet(gate.idle),
        Op::LocalGet(top),
        Op::GlobalSet(gate.sp),
    ];
    let after = [Op::GlobalGet(gate.idle), Op::GlobalSet(gate.sp)];
    Forward {
        params: &wrapper.params,
        forwarded: top,
        results: &wrapper.results,
        callee: inner,
        extra_locals: &[],
        before: &before,
        after: &after,
    }
    .body()
}

/// How a gate answers a call it must not admit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GateVariant {
    /// Trap: the call fails with a `RuntimeError` the JavaScript runtime maps to the
    /// poison error. Every export takes this variant unless its name is one of
    /// wasm-bindgen's object release functions.
    Trapping,
    /// Return without entering the guarded function, with zero results if it has any.
    /// The variant of `__wbg_<type>_free`: the glue calls those from `free()`,
    /// `[Symbol.dispose]()` and a `FinalizationRegistry` callback, and a finalizer has no
    /// caller to report a trap to, so a dead instance's objects are left unreleased
    /// (the instance's whole memory is abandoned with it) rather than turning garbage
    /// collection into uncaught errors.
    Inert,
}

/// The poison gate around one exported function. On entry it refuses when the instance
/// is poisoned, poisons and refuses when an earlier entry never returned, and otherwise
/// counts itself active; on the one exit it counts itself back out. `variant` decides
/// what a refusal is: a trap, or a return with zero results.
pub(crate) fn gate(sig: &Sig, inner: u32, gate: Gate, variant: GateVariant) -> Body {
    let first_result_local = sig.params.len() as u32;
    let refuse: Vec<Op> = match variant {
        GateVariant::Trapping => vec![Op::Unreachable],
        GateVariant::Inert => (0..sig.results.len() as u32)
            .map(|i| Op::LocalGet(first_result_local + i))
            .chain(std::iter::once(Op::Return))
            .collect(),
    };
    let mut before = vec![Op::GlobalGet(gate.poisoned), Op::If];
    before.extend_from_slice(&refuse);
    before.extend([
        Op::End,
        Op::GlobalGet(gate.active),
        Op::GlobalGet(gate.parked),
        Op::I32Sub,
        Op::GlobalGet(gate.outbound),
        Op::I32Ne,
        Op::If,
        Op::I32Const(1),
        Op::GlobalSet(gate.poisoned),
    ]);
    before.extend_from_slice(&refuse);
    before.extend([
        Op::End,
        Op::GlobalGet(gate.active),
        Op::I32Const(1),
        Op::I32Add,
        Op::GlobalSet(gate.active),
    ]);
    let after = [
        Op::GlobalGet(gate.active),
        Op::I32Const(1),
        Op::I32Sub,
        Op::GlobalSet(gate.active),
    ];
    Forward {
        params: &sig.params,
        forwarded: sig.params.len() as u32,
        results: &sig.results,
        callee: inner,
        extra_locals: &[],
        before: &before,
        after: &after,
    }
    .body()
}

/// The callee of `body` when `body` is exactly what `template` produces for that callee.
pub(crate) fn callee_of(body: &Body, template: impl Fn(u32) -> Body) -> Option<u32> {
    let callee = body.ops.iter().find_map(|op| match op {
        Op::Call(callee) => Some(*callee),
        _ => None,
    })?;
    (template(callee) == *body).then_some(callee)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gate_globals() -> Gate {
        Gate {
            sp: 1,
            idle: 3,
            poisoned: 4,
            active: 5,
            parked: 6,
            outbound: 7,
        }
    }

    #[test]
    fn a_forwarding_body_keeps_results_in_locals_across_the_epilogue() {
        let sig = Sig {
            params: vec![ValType::I32, ValType::I64],
            results: vec![ValType::I32],
        };
        let body = trampoline(&sig, 9, gate_globals());
        assert_eq!(body.locals, vec![ValType::I32, ValType::I32]);
        let call = body
            .ops
            .iter()
            .position(|op| *op == Op::Call(9))
            .expect("the call");
        assert_eq!(body.ops[call - 2..call], [Op::LocalGet(0), Op::LocalGet(1)]);
        assert_eq!(body.ops[call + 1], Op::LocalSet(2));
        assert_eq!(body.ops[body.ops.len() - 2..], [Op::LocalGet(2), Op::End]);
        assert_eq!(
            callee_of(&body, |callee| trampoline(&sig, callee, gate_globals())),
            Some(9)
        );
        assert_eq!(
            callee_of(&body, |callee| suspend(&sig, callee, gate_globals())),
            None
        );
    }

    #[test]
    fn the_run_wrapper_forwards_every_parameter_but_the_region_top() {
        let run = Sig {
            params: vec![ValType::I32],
            results: vec![ValType::I32],
        };
        let body = run_wrapper(&run, 4, gate_globals());
        assert_eq!(
            run_wrapper_sig(&run).params,
            vec![ValType::I32, ValType::I32]
        );
        assert!(body.ops.starts_with(&[
            Op::GlobalGet(1),
            Op::GlobalSet(3),
            Op::LocalGet(1),
            Op::GlobalSet(1),
            Op::LocalGet(0),
            Op::Call(4),
            Op::LocalSet(2),
        ]));
    }

    #[test]
    fn the_two_gate_variants_differ_only_in_how_they_refuse() {
        let sig = Sig {
            params: vec![ValType::I32, ValType::I32],
            results: vec![],
        };
        let trapping = gate(&sig, 9, gate_globals(), GateVariant::Trapping);
        let inert = gate(&sig, 9, gate_globals(), GateVariant::Inert);
        assert_eq!(trapping.ops.len(), inert.ops.len());
        let differences: Vec<(Op, Op)> = trapping
            .ops
            .iter()
            .zip(&inert.ops)
            .filter(|(a, b)| a != b)
            .map(|(a, b)| (*a, *b))
            .collect();
        assert_eq!(
            differences,
            vec![(Op::Unreachable, Op::Return), (Op::Unreachable, Op::Return)]
        );
        assert_eq!(
            callee_of(&inert, |callee| gate(
                &sig,
                callee,
                gate_globals(),
                GateVariant::Inert
            )),
            Some(9)
        );
        assert_eq!(
            callee_of(&inert, |callee| gate(
                &sig,
                callee,
                gate_globals(),
                GateVariant::Trapping
            )),
            None,
            "a body of one variant is not accepted as the other"
        );
        // With results, the inert refusal returns the zero-initialized result locals.
        let with_result = Sig {
            params: vec![ValType::I32],
            results: vec![ValType::I32],
        };
        let inert = gate(&with_result, 9, gate_globals(), GateVariant::Inert);
        assert!(inert.ops.starts_with(&[
            Op::GlobalGet(4),
            Op::If,
            Op::LocalGet(1),
            Op::Return,
            Op::End,
        ]));
    }

    #[test]
    fn both_stack_pointer_operand_orders_name_the_global_and_nothing_else_does() {
        for shape in stack_pointer_shapes(1) {
            let body = Body {
                locals: vec![],
                ops: shape,
            };
            assert_eq!(stack_pointer_global(&body), Some(1));
        }
        let with_local = Body {
            locals: vec![ValType::I32],
            ops: stack_pointer_shapes(1)[0].clone(),
        };
        assert_eq!(stack_pointer_global(&with_local), None);
        let subtracting = Body {
            locals: vec![],
            ops: vec![
                Op::LocalGet(0),
                Op::GlobalGet(1),
                Op::I32Sub,
                Op::GlobalSet(1),
                Op::GlobalGet(1),
                Op::End,
            ],
        };
        assert_eq!(stack_pointer_global(&subtracting), None);
    }
}
