// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The expression compiler: one [`Expression`] to one flat [`ExprProgram`].
//!
//! The compiler is **mode-directed**. The evaluator reads an operand in one of four
//! ways, and each way interns a different set of values, so each is its own mode:
//!
//! * [`Mode::Term`] — the operand's value as a solution term. A constant is interned
//!   the first time it is read; a computed value is interned where it is computed.
//! * [`Mode::StringArg`] — a string built-in's argument (`CONTAINS`, `STRSTARTS`,
//!   `STRENDS`, `REGEX`, `LANGMATCHES`) as a lexical form, a language tag and a base
//!   direction. A string
//!   constant is read as written and never interned, and `STR(x)` / `LANG(x)` in this
//!   position read `x`'s lexical form or tag straight off the term rather than minting
//!   the string `STR`/`LANG` would return.
//! * [`Mode::StrLexical`] — `STR(x)`'s operand under [`Mode::StringArg`]: an IRI or
//!   literal constant is its own lexical form, uninterned.
//! * [`Mode::LangLexical`] — `LANG(x)`'s operand under [`Mode::StringArg`]: a literal
//!   constant's tag, uninterned.
//! * [`Mode::TripleObject`] — a triple term constructor (`TRIPLE(s, p, o)`, `<<( s p o
//!   )>>`) in the object position of another: the value it builds, uninterned, moved
//!   into the term that encloses it. A chain of `d` nested constructors interns its
//!   outermost term alone, rather than every level's whole term — `d` terms of up to `d`
//!   levels each, stored and hashed once each, which is the square of the depth in time
//!   and memory. The value, and every value it is unbound for, is the one the
//!   level-by-level interning answered: each level's subject and predicate are judged
//!   where they are built, and the one language-tag check at the outermost level reads
//!   every level's tags.
//!
//! Every other position evaluates in [`Mode::Term`], and an effective boolean value is
//! the term's, read by [`Op::EbvOf`] after the term is built — so a logical operator's
//! boolean result is interned exactly where the tree walk interned it.
//!
//! Operands are emitted left to right, and a conditional's untaken branches are jumped
//! over, so the program evaluates exactly the operands the tree walk evaluated, in the
//! same order. The walk over the expression is a work list: an expression of any depth
//! compiles on heap, never on the machine stack.

use purrdf_sparql_algebra::{
    ArithmeticOperator, Expression, Function, Literal, NamedNode, Variable,
};

use purrdf_core::FastMap;
use purrdf_lex::allocation::{Admission, Memory, Resident, StorageError};

use purrdf_iri::vocab::rdf::LANG_STRING as RDF_LANG_STRING;
use purrdf_xsd::datatype::XSD_STRING;

/// One of the four ordering comparisons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cmp {
    /// `>`.
    Greater,
    /// `>=`.
    GreaterOrEqual,
    /// `<`.
    Less,
    /// `<=`.
    LessOrEqual,
}

/// One of the three substring predicates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StrPred {
    /// `CONTAINS`.
    Contains,
    /// `STRSTARTS`.
    StrStarts,
    /// `STRENDS`.
    StrEnds,
}

/// One instruction. Jump targets are instruction indices.
///
/// The value stack holds four kinds of entry: a term (possibly unbound), an effective
/// boolean value (possibly an error), a string argument (possibly absent), and an
/// `IN` in progress. Each instruction's doc names what it pops and pushes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Op {
    /// Push constant `k`'s term, interning it the first time this link reads it.
    Const(u32),
    /// Push variable slot `k`'s binding in the current row.
    Var(u32),
    /// Push `BOUND` of variable slot `k`, as an `xsd:boolean` term.
    Bound(u32),
    /// Pop a term; push its effective boolean value.
    EbvOf,
    /// Pop `n` effective boolean values; push their Kleene disjunction (`or`) or
    /// conjunction as an `xsd:boolean` term.
    Kleene {
        /// Disjunction when set, conjunction otherwise.
        or: bool,
        /// The operand count.
        n: u32,
    },
    /// Pop an effective boolean value; push its negation as a term.
    Not,
    /// Pop two terms; push `=`.
    Equal,
    /// Pop two terms; push `sameTerm`.
    SameTerm,
    /// Pop two terms; push the ordering comparison.
    Cmp(Cmp),
    /// Pop an effective boolean value: `true` falls through, `false` jumps to
    /// `on_false`, an error pushes an unbound term and jumps to `end`.
    Branch {
        /// The else branch.
        on_false: u32,
        /// Past the conditional.
        end: u32,
    },
    /// Jump.
    Jmp(u32),
    /// `COALESCE` between two items: a bound term on top is the answer (jump to the
    /// target, keeping it); an unbound one is popped and the next item runs.
    CoalesceNext(u32),
    /// Push an unbound term.
    PushUnbound,
    /// Pop the `IN` needle: unbound pushes an unbound term and jumps to the target;
    /// bound opens an `IN` entry.
    InNeedle(u32),
    /// Pop one `IN` candidate and fold it into the open `IN` entry; a match closes the
    /// entry, pushes `true` and jumps to the target.
    InItem(u32),
    /// Close the open `IN` entry: push `false`, or an unbound term when a candidate
    /// raised.
    InEnd,
    /// Pop the right and left terms; push one arithmetic step.
    Arith(ArithmeticOperator),
    /// Pop a term; push its unary plus.
    UnaryPlus,
    /// Pop a term; push its unary minus.
    UnaryMinus,
    /// Push `EXISTS` of site `k` (the `k`-th `EXISTS` in the expression's pre-order) as
    /// an `xsd:boolean` term.
    Exists(u32),
    /// Pop `argc` terms; push built-in call `call` over their values. `regex` names the
    /// link-time pattern of a `REPLACE` whose pattern and flags are constants.
    Call {
        /// Index into the program's call table.
        call: u32,
        /// The argument count.
        argc: u32,
        /// The constant pattern's regex slot, if any.
        regex: Option<u32>,
    },
    /// Pop `argc` terms; push the caller-registered (or XSD-cast) function `call`.
    CallCustom {
        /// Index into the program's call table.
        call: u32,
        /// The argument count.
        argc: u32,
    },
    /// Push string constant `k`.
    StrConst(u32),
    /// Push an absent string argument.
    StrNone,
    /// Pop a term; push it as a string argument (lexical form, tag and direction).
    ToStrArg,
    /// Pop a term; push its `STR` lexical form as a string argument.
    ToStrLexical,
    /// Pop a term; push its `LANG` tag as a string argument.
    ToLangLexical,
    /// Pop the needle and haystack string arguments; push the predicate.
    StrPred(StrPred),
    /// Pop flags (when the call supplies them), pattern and text string arguments;
    /// push `REGEX`. Regex slot `slot` holds the link-time pattern when pattern and
    /// flags are constants.
    Regex {
        /// The call's regex slot.
        slot: u32,
        /// Whether the call supplies a flags argument. Omitted flags are no flags; a
        /// supplied flags argument must be a bound simple literal, so an unbound one
        /// is an error rather than an omission.
        flags: bool,
    },
    /// Pop the object — a term, or a value a nested constructor built — then the
    /// predicate and subject terms; build the triple term `TRIPLE` builds. `intern`
    /// pushes it interned as a term; otherwise it is pushed as the value, for the
    /// constructor it is the object of.
    Triple {
        /// Whether the triple term is interned, rather than left a value.
        intern: bool,
    },
    /// Pop range and tag string arguments; push `LANGMATCHES`.
    LangMatches,
}

/// A term constant of a program: the expression's own leaf, shared rather than
/// converted, so compiling costs no term value per constant. [`Self::value`] is the
/// term value a read of it interns.
#[derive(Debug, Clone)]
pub(crate) enum Constant {
    /// An IRI constant.
    Iri(NamedNode),
    /// A literal constant.
    Literal(Literal),
}

impl Constant {
    /// Copy the authored constant only after its actual facets are admitted.
    pub(crate) fn value_admitted(
        &self,
        workspace: &crate::WorkspaceCapability,
    ) -> Result<crate::WorkspaceTerm, crate::EvalError> {
        match self {
            Self::Iri(node) => crate::convert::named_node_to_workspace_value(node, workspace),
            Self::Literal(literal) => {
                crate::convert::literal_to_workspace_value(literal, workspace)
            }
        }
    }
}

/// A compiled expression: a flat instruction array and the tables it indexes. Holds no
/// reference into the expression it was compiled from and no evaluation state, so one
/// program is shared by every evaluation of its site.
#[derive(Debug, Default)]
pub(crate) struct ExprProgram {
    /// The instructions, inline up to a small expression's length.
    pub(super) ops: purrdf_core::SmallVec<[Op; 8]>,
    /// The variables the program reads, one slot each, inline up to two.
    pub(super) vars: purrdf_core::SmallVec<[Variable; 2]>,
    /// The term constants, one per occurrence, inline up to two.
    pub(super) consts: purrdf_core::SmallVec<[Constant; 2]>,
    /// The string-argument constants.
    pub(super) strs: Vec<crate::expr::StringArg>,
    /// The functions called, inline up to two.
    pub(super) calls: purrdf_core::SmallVec<[Function; 2]>,
    /// For each regex slot, the constant `(pattern, flags)` it links, when both are
    /// constants.
    pub(super) regexes: Vec<Option<(String, String)>>,
    /// How many `EXISTS` the expression holds.
    pub(super) exists: u32,
    owner: Option<crate::workspace::LexicalFrame>,
}

/// How an operand is read. See the module docs.
#[derive(Debug, Clone, Copy)]
enum Mode {
    Term,
    StringArg,
    StrLexical,
    LangLexical,
    TripleObject,
}

/// One unit of pending compilation.
enum Task<'x> {
    /// Compile `expr` in `mode`.
    Compile(&'x Expression, Mode),
    /// A string argument position the call does not fill.
    AbsentStringArg,
    /// Emit an instruction whose jump fields hold label ids.
    Emit(Op),
    /// Bind a label to the next instruction.
    Label(u32),
    /// Arguments the evaluator never reads: count their `EXISTS` so the site numbering
    /// stays the expression's pre-order.
    Skip(&'x [Expression]),
}

/// A list of pending compilation units, inline until an expression is wide or deep
/// enough to need more.
type Tasks<'x> = purrdf_core::SmallVec<[Task<'x>; 16]>;

/// How many variables a program finds by scanning its variable table before the
/// compiler indexes them.
const SCANNED_VARIABLES: usize = 8;

/// The compiler's state.
struct Compiler<'m, 's, S: Admission + ?Sized> {
    memory: &'m mut Memory<'s, S>,
    program: ExprProgram,
    /// Each variable's slot, once the program reads more than [`SCANNED_VARIABLES`];
    /// until then a slot is found by scanning [`ExprProgram::vars`].
    slots: FastMap<Variable, u32>,
    labels: purrdf_core::SmallVec<[u32; 8]>,
}

impl ExprProgram {
    /// Compile through the original native instruction body.
    pub(crate) fn compile(expr: &Expression) -> Self {
        let mut resident = Resident;
        Self::default()
            .compile_with_memory(expr, &mut Memory::new(&mut resident))
            .expect("resident expression compilation")
    }
    pub(crate) fn compile_admitted(
        expr: &Expression,
        workspace: &crate::WorkspaceCapability,
    ) -> Result<Self, crate::EvalError> {
        let mut frame = crate::workspace::LexicalFrame::new(workspace);
        let result = Memory::new(&mut frame)
            .scope(|memory| Self::default().compile_with_memory(expr, memory));
        let mut program =
            result.map_err(|error| frame.storage_error(error, "expression compilation"))?;
        program.owner = Some(frame);
        Ok(program)
    }
    pub(crate) fn recompile(&mut self, expr: &Expression) {
        let mut resident = Resident;
        *self = std::mem::take(self)
            .compile_with_memory(expr, &mut Memory::new(&mut resident))
            .expect("resident expression recompilation");
    }
    fn compile_with_memory<S: Admission + ?Sized>(
        mut self,
        expr: &Expression,
        memory: &mut Memory<'_, S>,
    ) -> Result<Self, StorageError> {
        self.ops.clear();
        self.vars.clear();
        self.consts.clear();
        self.strs.clear();
        self.calls.clear();
        self.regexes.clear();
        self.exists = 0;
        let mut compiler = Compiler {
            program: self,
            slots: FastMap::default(),
            labels: purrdf_core::SmallVec::new(),
            memory,
        };
        let mut work: Tasks<'_> = purrdf_core::SmallVec::new();
        work.push_with_memory(Task::Compile(expr, Mode::Term), compiler.memory)?;
        let mut next: Tasks<'_> = purrdf_core::SmallVec::new();
        while let Some(task) = work.pop() {
            match task {
                Task::Compile(expr, mode) => {
                    compiler.expand(expr, mode, &mut next)?;
                    while let Some(task) = next.pop() {
                        work.push_with_memory(task, compiler.memory)?;
                    }
                }
                Task::AbsentStringArg => compiler
                    .program
                    .ops
                    .push_with_memory(Op::StrNone, compiler.memory)?,
                Task::Emit(op) => compiler.program.ops.push_with_memory(op, compiler.memory)?,
                Task::Label(label) => {
                    compiler.labels[label as usize] = compiler.program.ops.len() as u32;
                }
                Task::Skip(args) => compiler.program.exists += count_exists(args, compiler.memory)?,
            }
        }
        compiler.resolve_labels();
        work.release_with_memory(compiler.memory)?;
        next.release_with_memory(compiler.memory)?;
        compiler.labels.release_with_memory(compiler.memory)?;
        let bytes = purrdf_core::hash::hash_table_allocation_bound::<(Variable, u32)>(
            compiler.slots.capacity(),
        )
        .ok_or(StorageError::SizeOverflow)?;
        drop(compiler.slots);
        compiler.memory.release_bytes(bytes)?;
        Ok(compiler.program)
    }
    /// Whether the program reaches an `EXISTS`.
    pub(crate) const fn has_exists(&self) -> bool {
        self.exists > 0
    }
}

impl<S: Admission + ?Sized> Compiler<'_, '_, S> {
    /// A fresh label.
    fn label(&mut self) -> Result<u32, StorageError> {
        self.labels.push_with_memory(u32::MAX, self.memory)?;
        Ok((self.labels.len() - 1) as u32)
    }

    /// The slot of `var`: the position of its first occurrence among the variables the
    /// program reads.
    fn slot(&mut self, var: &Variable) -> Result<u32, StorageError> {
        let found = if self.slots.is_empty() {
            self.program
                .vars
                .iter()
                .position(|known| known == var)
                .map(|index| index as u32)
        } else {
            self.slots.get(var).copied()
        };
        if let Some(slot) = found {
            return Ok(slot);
        }
        let slot = self.program.vars.len() as u32;
        self.program
            .vars
            .push_with_memory(var.clone(), self.memory)?;
        if !self.slots.is_empty() {
            let required = self.slots.len() + 1;
            purrdf_core::hash::reserve_map_with_memory(&mut self.slots, required, self.memory)?;
            self.slots.insert(var.clone(), slot);
        } else if self.program.vars.len() > SCANNED_VARIABLES {
            for (index, known) in self.program.vars.iter().enumerate() {
                let required = self.slots.len() + 1;
                purrdf_core::hash::reserve_map_with_memory(&mut self.slots, required, self.memory)?;
                self.slots.insert(known.clone(), index as u32);
            }
        }
        Ok(slot)
    }

    /// A new term constant.
    fn constant(&mut self, value: Constant) -> Result<Op, StorageError> {
        self.program.consts.push_with_memory(value, self.memory)?;
        Ok(Op::Const((self.program.consts.len() - 1) as u32))
    }

    /// A new string constant.
    fn string(&mut self, lexical: String, language: Option<String>) -> Result<Op, StorageError> {
        // The constants read in place are the untagged and `rdf:langString` ones; a
        // directional literal is read through its interned term.
        self.memory
            .push(&mut self.program.strs, (lexical, language, None))?;
        Ok(Op::StrConst((self.program.strs.len() - 1) as u32))
    }

    /// A new call-table entry.
    fn call(&mut self, function: &Function) -> Result<u32, StorageError> {
        let function = function.clone_with_memory(self.memory)?;
        self.program.calls.push_with_memory(function, self.memory)?;
        Ok((self.program.calls.len() - 1) as u32)
    }

    /// A new regex slot.
    fn regex(&mut self, constant: Option<(String, String)>) -> Result<u32, StorageError> {
        self.memory.push(&mut self.program.regexes, constant)?;
        Ok((self.program.regexes.len() - 1) as u32)
    }

    /// Lay out `expr` read in `mode` as tasks, in evaluation order. Leaves are emitted
    /// here, which is where the pre-order visits them; composite nodes become their
    /// operands' tasks followed by their own instruction.
    fn expand<'x>(
        &mut self,
        expr: &'x Expression,
        mode: Mode,
        out: &mut Tasks<'x>,
    ) -> Result<(), StorageError> {
        match mode {
            Mode::Term => self.expand_term(expr, out)?,
            Mode::TripleObject => match expr {
                Expression::FunctionCall(Function::Triple, args) if args.len() == 3 => {
                    triple_operands(args, out, self.memory)?;
                    out.push_with_memory(Task::Emit(Op::Triple { intern: false }), self.memory)?;
                }
                _ => out.push_with_memory(Task::Compile(expr, Mode::Term), self.memory)?,
            },
            Mode::StringArg => match expr {
                Expression::Literal(lit)
                    if lit.datatype().as_str() == XSD_STRING
                        || lit.datatype().as_str() == RDF_LANG_STRING =>
                {
                    let lexical = self.memory.string(lit.value())?;
                    let language = lit
                        .language()
                        .map(|tag| purrdf_iri::langtag::identity_fold_with_memory(tag, self.memory))
                        .transpose()?;
                    let op = self.string(lexical, language)?;
                    out.push_with_memory(Task::Emit(op), self.memory)?;
                }
                Expression::FunctionCall(Function::Str, inner) if inner.len() == 1 => {
                    out.push_with_memory(Task::Compile(&inner[0], Mode::StrLexical), self.memory)?;
                }
                Expression::FunctionCall(Function::Lang, inner) if inner.len() == 1 => {
                    out.push_with_memory(Task::Compile(&inner[0], Mode::LangLexical), self.memory)?;
                }
                _ => {
                    out.push_with_memory(Task::Compile(expr, Mode::Term), self.memory)?;
                    out.push_with_memory(Task::Emit(Op::ToStrArg), self.memory)?;
                }
            },
            Mode::StrLexical => match expr {
                Expression::NamedNode(node) => {
                    let lexical = self.memory.string(node.as_str())?;
                    let op = self.string(lexical, None)?;
                    out.push_with_memory(Task::Emit(op), self.memory)?;
                }
                Expression::Literal(lit) => {
                    let lexical = self.memory.string(lit.value())?;
                    let op = self.string(lexical, None)?;
                    out.push_with_memory(Task::Emit(op), self.memory)?;
                }
                _ => {
                    out.push_with_memory(Task::Compile(expr, Mode::Term), self.memory)?;
                    out.push_with_memory(Task::Emit(Op::ToStrLexical), self.memory)?;
                }
            },
            Mode::LangLexical => match expr {
                Expression::Literal(lit) => {
                    let language = purrdf_iri::langtag::identity_fold_with_memory(
                        lit.language().unwrap_or_default(),
                        self.memory,
                    )?;
                    let op = self.string(language, None)?;
                    out.push_with_memory(Task::Emit(op), self.memory)?;
                }
                _ => {
                    out.push_with_memory(Task::Compile(expr, Mode::Term), self.memory)?;
                    out.push_with_memory(Task::Emit(Op::ToLangLexical), self.memory)?;
                }
            },
        }
        Ok(())
    }

    /// [`Self::expand`] for [`Mode::Term`].
    fn expand_term<'x>(
        &mut self,
        expr: &'x Expression,
        out: &mut Tasks<'x>,
    ) -> Result<(), StorageError> {
        match expr {
            Expression::NamedNode(node) => {
                let op = self.constant(Constant::Iri(node.clone()))?;
                out.push_with_memory(Task::Emit(op), self.memory)?;
            }
            Expression::Literal(lit) => {
                let op = self.constant(Constant::Literal(lit.clone()))?;
                out.push_with_memory(Task::Emit(op), self.memory)?;
            }
            Expression::Variable(var) => {
                let slot = self.slot(var)?;
                out.push_with_memory(Task::Emit(Op::Var(slot)), self.memory)?;
            }
            Expression::Bound(var) => {
                let slot = self.slot(var)?;
                out.push_with_memory(Task::Emit(Op::Bound(slot)), self.memory)?;
            }
            Expression::Or(operands) | Expression::And(operands) => {
                for operand in operands {
                    out.push_with_memory(Task::Compile(operand, Mode::Term), self.memory)?;
                    out.push_with_memory(Task::Emit(Op::EbvOf), self.memory)?;
                }
                out.push_with_memory(
                    Task::Emit(Op::Kleene {
                        or: matches!(expr, Expression::Or(_)),
                        n: operands.len() as u32,
                    }),
                    self.memory,
                )?;
            }
            Expression::Not(operand) => {
                out.push_with_memory(Task::Compile(operand, Mode::Term), self.memory)?;
                out.push_with_memory(Task::Emit(Op::EbvOf), self.memory)?;
                out.push_with_memory(Task::Emit(Op::Not), self.memory)?;
            }
            Expression::Equal(a, b)
            | Expression::SameTerm(a, b)
            | Expression::Greater(a, b)
            | Expression::GreaterOrEqual(a, b)
            | Expression::Less(a, b)
            | Expression::LessOrEqual(a, b) => {
                let op = match expr {
                    Expression::Equal(..) => Op::Equal,
                    Expression::SameTerm(..) => Op::SameTerm,
                    Expression::Greater(..) => Op::Cmp(Cmp::Greater),
                    Expression::GreaterOrEqual(..) => Op::Cmp(Cmp::GreaterOrEqual),
                    Expression::Less(..) => Op::Cmp(Cmp::Less),
                    _ => Op::Cmp(Cmp::LessOrEqual),
                };
                out.push_with_memory(Task::Compile(a, Mode::Term), self.memory)?;
                out.push_with_memory(Task::Compile(b, Mode::Term), self.memory)?;
                out.push_with_memory(Task::Emit(op), self.memory)?;
            }
            Expression::If(condition, then, otherwise) => {
                let on_false = self.label()?;
                let end = self.label()?;
                out.push_with_memory(Task::Compile(condition, Mode::Term), self.memory)?;
                out.push_with_memory(Task::Emit(Op::EbvOf), self.memory)?;
                out.push_with_memory(Task::Emit(Op::Branch { on_false, end }), self.memory)?;
                out.push_with_memory(Task::Compile(then, Mode::Term), self.memory)?;
                out.push_with_memory(Task::Emit(Op::Jmp(end)), self.memory)?;
                out.push_with_memory(Task::Label(on_false), self.memory)?;
                out.push_with_memory(Task::Compile(otherwise, Mode::Term), self.memory)?;
                out.push_with_memory(Task::Label(end), self.memory)?;
            }
            Expression::Coalesce(items) => {
                let Some((last, init)) = items.split_last() else {
                    out.push_with_memory(Task::Emit(Op::PushUnbound), self.memory)?;
                    return Ok(());
                };
                let end = self.label()?;
                for item in init {
                    out.push_with_memory(Task::Compile(item, Mode::Term), self.memory)?;
                    out.push_with_memory(Task::Emit(Op::CoalesceNext(end)), self.memory)?;
                }
                out.push_with_memory(Task::Compile(last, Mode::Term), self.memory)?;
                out.push_with_memory(Task::Label(end), self.memory)?;
            }
            Expression::In(needle, haystack) => {
                let end = self.label()?;
                out.push_with_memory(Task::Compile(needle, Mode::Term), self.memory)?;
                out.push_with_memory(Task::Emit(Op::InNeedle(end)), self.memory)?;
                for item in haystack {
                    out.push_with_memory(Task::Compile(item, Mode::Term), self.memory)?;
                    out.push_with_memory(Task::Emit(Op::InItem(end)), self.memory)?;
                }
                out.push_with_memory(Task::Emit(Op::InEnd), self.memory)?;
                out.push_with_memory(Task::Label(end), self.memory)?;
            }
            Expression::Exists(_) => {
                let site = self.program.exists;
                self.program.exists += 1;
                out.push_with_memory(Task::Emit(Op::Exists(site)), self.memory)?;
            }
            Expression::Arithmetic(first, steps) => {
                out.push_with_memory(Task::Compile(first, Mode::Term), self.memory)?;
                for (op, operand) in steps {
                    out.push_with_memory(Task::Compile(operand, Mode::Term), self.memory)?;
                    out.push_with_memory(Task::Emit(Op::Arith(*op)), self.memory)?;
                }
            }
            Expression::UnaryPlus(operand) => {
                out.push_with_memory(Task::Compile(operand, Mode::Term), self.memory)?;
                out.push_with_memory(Task::Emit(Op::UnaryPlus), self.memory)?;
            }
            Expression::UnaryMinus(operand) => {
                out.push_with_memory(Task::Compile(operand, Mode::Term), self.memory)?;
                out.push_with_memory(Task::Emit(Op::UnaryMinus), self.memory)?;
            }
            Expression::FunctionCall(function, args) => self.expand_call(function, args, out)?,
        }
        Ok(())
    }

    /// A function call in [`Mode::Term`]. The string predicates read their arguments
    /// as string arguments and ignore any past the ones they take; every other call
    /// evaluates every argument as a term, then calls.
    fn expand_call<'x>(
        &mut self,
        function: &'x Function,
        args: &'x [Expression],
        out: &mut Tasks<'x>,
    ) -> Result<(), StorageError> {
        let string_args = |count: usize,
                           out: &mut Tasks<'x>,
                           memory: &mut Memory<'_, S>|
         -> Result<(), StorageError> {
            for index in 0..count {
                out.push_with_memory(
                    args.get(index).map_or(Task::AbsentStringArg, |arg| {
                        Task::Compile(arg, Mode::StringArg)
                    }),
                    memory,
                )?;
            }
            if args.len() > count {
                out.push_with_memory(Task::Skip(&args[count..]), memory)?;
            }
            Ok(())
        };
        match function {
            Function::Contains | Function::StrStarts | Function::StrEnds => {
                string_args(2, out, self.memory)?;
                out.push_with_memory(
                    Task::Emit(Op::StrPred(match function {
                        Function::Contains => StrPred::Contains,
                        Function::StrStarts => StrPred::StrStarts,
                        _ => StrPred::StrEnds,
                    })),
                    self.memory,
                )?;
            }
            Function::Regex => {
                let constant = match constant_string_arg(args.get(1), self.memory)? {
                    Some(pattern) => match args.get(2) {
                        None => Some((pattern, String::new())),
                        Some(flags) => match constant_string_arg(Some(flags), self.memory)? {
                            Some(flags) => Some((pattern, flags)),
                            None => {
                                self.memory.release_string(pattern)?;
                                None
                            }
                        },
                    },
                    None => None,
                };
                let slot = self.regex(constant)?;
                let flags = args.len() > 2;
                string_args(if flags { 3 } else { 2 }, out, self.memory)?;
                out.push_with_memory(Task::Emit(Op::Regex { slot, flags }), self.memory)?;
            }
            Function::LangMatches => {
                string_args(2, out, self.memory)?;
                out.push_with_memory(Task::Emit(Op::LangMatches), self.memory)?;
            }
            // A constructor whose object is another constructor: the chain below it is
            // built as one value and interned here, once.
            Function::Triple if args.len() == 3 && is_triple_constructor(&args[2]) => {
                triple_operands(args, out, self.memory)?;
                out.push_with_memory(Task::Emit(Op::Triple { intern: true }), self.memory)?;
            }
            Function::Custom(_) => {
                for arg in args {
                    out.push_with_memory(Task::Compile(arg, Mode::Term), self.memory)?;
                }
                let call = self.call(function)?;
                out.push_with_memory(
                    Task::Emit(Op::CallCustom {
                        call,
                        argc: args.len() as u32,
                    }),
                    self.memory,
                )?;
            }
            _ => {
                let regex = if matches!(function, Function::Replace) {
                    match replace_constant(args, self.memory)? {
                        Some(constant) => Some(self.regex(Some(constant))?),
                        None => None,
                    }
                } else {
                    None
                };
                for arg in args {
                    out.push_with_memory(Task::Compile(arg, Mode::Term), self.memory)?;
                }
                let call = self.call(function)?;
                out.push_with_memory(
                    Task::Emit(Op::Call {
                        call,
                        argc: args.len() as u32,
                        regex,
                    }),
                    self.memory,
                )?;
            }
        }
        Ok(())
    }

    /// Rewrite every jump's label id to the instruction index the label was bound to.
    fn resolve_labels(&mut self) {
        let labels = &self.labels;
        let at = |label: u32| labels[label as usize];
        for op in &mut self.program.ops {
            match op {
                Op::Branch { on_false, end } => {
                    *on_false = at(*on_false);
                    *end = at(*end);
                }
                Op::Jmp(target)
                | Op::CoalesceNext(target)
                | Op::InNeedle(target)
                | Op::InItem(target) => *target = at(*target),
                _ => {}
            }
        }
    }
}

/// Whether `expr` is a three-argument triple term constructor.
pub(super) fn is_triple_constructor(expr: &Expression) -> bool {
    matches!(expr, Expression::FunctionCall(Function::Triple, args) if args.len() == 3)
}

/// A triple term constructor's three operands, in order: the subject and predicate as
/// terms, and the object as a value when it is itself a constructor.
fn triple_operands<'x, S: Admission + ?Sized>(
    args: &'x [Expression],
    out: &mut Tasks<'x>,
    memory: &mut Memory<'_, S>,
) -> Result<(), StorageError> {
    out.push_with_memory(Task::Compile(&args[0], Mode::Term), memory)?;
    out.push_with_memory(Task::Compile(&args[1], Mode::Term), memory)?;
    out.push_with_memory(
        Task::Compile(
            &args[2],
            if is_triple_constructor(&args[2]) {
                Mode::TripleObject
            } else {
                Mode::Term
            },
        ),
        memory,
    )?;
    Ok(())
}

/// The string a [`Mode::StringArg`] operand reads without evaluating anything, when it
/// is one: a string literal, or `STR`/`LANG` of a constant.
fn constant_string_arg<S: Admission + ?Sized>(
    expr: Option<&Expression>,
    memory: &mut Memory<'_, S>,
) -> Result<Option<String>, StorageError> {
    let Some(expr) = expr else {
        return Ok(None);
    };
    let text = match expr {
        Expression::Literal(lit)
            if lit.datatype().as_str() == XSD_STRING
                || lit.datatype().as_str() == RDF_LANG_STRING =>
        {
            lit.value()
        }
        Expression::FunctionCall(Function::Str, inner) if inner.len() == 1 => match &inner[0] {
            Expression::NamedNode(node) => node.as_str(),
            Expression::Literal(lit) => lit.value(),
            _ => return Ok(None),
        },
        Expression::FunctionCall(Function::Lang, inner) if inner.len() == 1 => match &inner[0] {
            Expression::Literal(lit) => {
                return purrdf_iri::langtag::identity_fold_with_memory(
                    lit.language().unwrap_or_default(),
                    memory,
                )
                .map(Some);
            }
            _ => return Ok(None),
        },
        _ => return Ok(None),
    };
    memory.string(text).map(Some)
}

/// `REPLACE`'s pattern and flags, when both are simple literals (or the flags are
/// absent): the lexical forms its string-argument reading takes from them. A tagged
/// pattern or flags argument is an error the call reports per row, so it links none.
fn replace_constant<S: Admission + ?Sized>(
    args: &[Expression],
    memory: &mut Memory<'_, S>,
) -> Result<Option<(String, String)>, StorageError> {
    fn string_literal(expr: &Expression) -> Option<&str> {
        match expr {
            Expression::Literal(lit) if lit.datatype().as_str() == XSD_STRING => Some(lit.value()),
            _ => None,
        }
    }
    let Some(pattern) = args.get(1).and_then(string_literal) else {
        return Ok(None);
    };
    let flags = match args.get(3) {
        None => "",
        Some(flags) => {
            let Some(flags) = string_literal(flags) else {
                return Ok(None);
            };
            flags
        }
    };
    Ok(Some((memory.string(pattern)?, memory.string(flags)?)))
}

/// The `EXISTS` nodes in `exprs`, counted over a work list.
fn count_exists<S: Admission + ?Sized>(
    exprs: &[Expression],
    memory: &mut Memory<'_, S>,
) -> Result<u32, StorageError> {
    let mut count = 0u32;
    let mut pending = Vec::new();
    for expr in exprs {
        memory.push(&mut pending, expr)?;
    }
    while let Some(expr) = pending.pop() {
        if matches!(expr, Expression::Exists(_)) {
            count = count.checked_add(1).ok_or(StorageError::SizeOverflow)?;
        }
        let mut result = Ok(());
        visit_operands(expr, |operand| {
            if result.is_ok() {
                result = memory.push(&mut pending, operand);
            }
        });
        result?;
    }
    memory.release_vec(pending)?;
    Ok(count)
}

/// The one operand-order traversal used by resident and admitted walks.
pub(super) fn visit_operands<'x>(expr: &'x Expression, mut visit: impl FnMut(&'x Expression)) {
    match expr {
        Expression::NamedNode(_)
        | Expression::Literal(_)
        | Expression::Variable(_)
        | Expression::Bound(_)
        | Expression::Exists(_) => {}
        Expression::Or(operands) | Expression::And(operands) => {
            for operand in operands {
                visit(operand);
            }
        }
        Expression::Equal(a, b)
        | Expression::SameTerm(a, b)
        | Expression::Greater(a, b)
        | Expression::GreaterOrEqual(a, b)
        | Expression::Less(a, b)
        | Expression::LessOrEqual(a, b) => {
            visit(a);
            visit(b);
        }
        Expression::Not(a) | Expression::UnaryPlus(a) | Expression::UnaryMinus(a) => {
            visit(a);
        }
        Expression::If(a, b, c) => {
            visit(a);
            visit(b);
            visit(c);
        }
        Expression::Coalesce(items) => {
            for operand in items {
                visit(operand);
            }
        }
        Expression::In(needle, haystack) => {
            visit(needle);
            for operand in haystack {
                visit(operand);
            }
        }
        Expression::Arithmetic(first, steps) => {
            visit(first);
            for (_, operand) in steps {
                visit(operand);
            }
        }
        Expression::FunctionCall(_, args) => {
            for operand in args {
                visit(operand);
            }
        }
    }
}
