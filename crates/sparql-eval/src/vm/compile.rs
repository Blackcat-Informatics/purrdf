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
//!   `STRENDS`, `REGEX`, `LANGMATCHES`) as a lexical form and a language tag. A string
//!   constant is read as written and never interned, and `STR(x)` / `LANG(x)` in this
//!   position read `x`'s lexical form or tag straight off the term rather than minting
//!   the string `STR`/`LANG` would return.
//! * [`Mode::StrLexical`] — `STR(x)`'s operand under [`Mode::StringArg`]: an IRI or
//!   literal constant is its own lexical form, uninterned.
//! * [`Mode::LangLexical`] — `LANG(x)`'s operand under [`Mode::StringArg`]: a literal
//!   constant's tag, uninterned.
//!
//! Every other position evaluates in [`Mode::Term`], and an effective boolean value is
//! the term's, read by [`Op::EbvOf`] after the term is built — so a logical operator's
//! boolean result is interned exactly where the tree walk interned it.
//!
//! Operands are emitted left to right, and a conditional's untaken branches are jumped
//! over, so the program evaluates exactly the operands the tree walk evaluated, in the
//! same order. The walk over the expression is a work list: an expression of any depth
//! compiles on heap, never on the machine stack.

use purrdf_core::TermValue;
use purrdf_sparql_algebra::{
    ArithmeticOperator, Expression, Function, Literal, NamedNode, Variable,
};

use crate::DetHashMap;

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
const RDF_DIR_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString";

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
    /// Pop a term; push it as a string argument (lexical form and tag).
    ToStrArg,
    /// Pop a term; push its `STR` lexical form as a string argument.
    ToStrLexical,
    /// Pop a term; push its `LANG` tag as a string argument.
    ToLangLexical,
    /// Pop the needle and haystack string arguments; push the predicate.
    StrPred(StrPred),
    /// Pop flags, pattern and text string arguments; push `REGEX`. Regex slot `k`
    /// holds the link-time pattern when pattern and flags are constants.
    Regex(u32),
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
    /// The constant as the term value the evaluator interns.
    pub(crate) fn value(&self) -> TermValue {
        match self {
            Self::Iri(node) => TermValue::Iri(node.as_str().to_owned()),
            Self::Literal(literal) => crate::convert::literal_to_value(literal),
        }
    }
}

/// A compiled expression: a flat instruction array and the tables it indexes. Holds no
/// reference into the expression it was compiled from and no evaluation state, so one
/// program is shared by every evaluation of its site.
#[derive(Debug, Default)]
pub(crate) struct ExprProgram {
    /// The instructions.
    pub(super) ops: Vec<Op>,
    /// The variables the program reads, one slot each.
    pub(super) vars: Vec<Variable>,
    /// The term constants, one per occurrence.
    pub(super) consts: Vec<Constant>,
    /// The string-argument constants.
    pub(super) strs: Vec<(String, Option<String>)>,
    /// The functions called.
    pub(super) calls: Vec<Function>,
    /// For each regex slot, the constant `(pattern, flags)` it links, when both are
    /// constants.
    pub(super) regexes: Vec<Option<(String, String)>>,
    /// How many `EXISTS` the expression holds.
    pub(super) exists: u32,
}

/// How an operand is read. See the module docs.
#[derive(Debug, Clone, Copy)]
enum Mode {
    Term,
    StringArg,
    StrLexical,
    LangLexical,
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
type Tasks<'x> = smallvec::SmallVec<[Task<'x>; 16]>;

/// How many variables a program finds by scanning its variable table before the
/// compiler indexes them.
const SCANNED_VARIABLES: usize = 8;

/// The compiler's state.
struct Compiler {
    program: ExprProgram,
    /// Each variable's slot, once the program reads more than [`SCANNED_VARIABLES`];
    /// until then a slot is found by scanning [`ExprProgram::vars`].
    slots: DetHashMap<Variable, u32>,
    labels: smallvec::SmallVec<[u32; 8]>,
}

impl ExprProgram {
    /// Compile `expr`, evaluated as a term.
    pub(crate) fn compile(expr: &Expression) -> Self {
        let mut program = Self::default();
        program.recompile(expr);
        program
    }

    /// Compile `expr` into this program, replacing everything it held. The program's
    /// tables keep their capacity, so compiling an expression of the shape this one was
    /// compiled from allocates nothing for them.
    pub(crate) fn recompile(&mut self, expr: &Expression) {
        let mut program = std::mem::take(self);
        let Self {
            ops,
            vars,
            consts,
            strs,
            calls,
            regexes,
            exists,
        } = &mut program;
        ops.clear();
        vars.clear();
        consts.clear();
        strs.clear();
        calls.clear();
        regexes.clear();
        *exists = 0;
        let mut compiler = Compiler {
            program,
            slots: DetHashMap::default(),
            labels: smallvec::SmallVec::new(),
        };
        let mut work: Tasks<'_> = smallvec::smallvec![Task::Compile(expr, Mode::Term)];
        let mut next: Tasks<'_> = smallvec::SmallVec::new();
        while let Some(task) = work.pop() {
            match task {
                Task::Compile(expr, mode) => {
                    compiler.expand(expr, mode, &mut next);
                    // `next` is in evaluation order and the work list pops from its end,
                    // so moving `next` over last-first leaves its first task on top.
                    while let Some(task) = next.pop() {
                        work.push(task);
                    }
                }
                Task::AbsentStringArg => compiler.program.ops.push(Op::StrNone),
                Task::Emit(op) => compiler.program.ops.push(op),
                Task::Label(label) => {
                    compiler.labels[label as usize] = compiler.program.ops.len() as u32;
                }
                Task::Skip(args) => compiler.program.exists += count_exists(args),
            }
        }
        compiler.resolve_labels();
        *self = compiler.program;
    }

    /// Whether the program reaches an `EXISTS`.
    pub(crate) const fn has_exists(&self) -> bool {
        self.exists > 0
    }
}

impl Compiler {
    /// A fresh label.
    fn label(&mut self) -> u32 {
        self.labels.push(u32::MAX);
        (self.labels.len() - 1) as u32
    }

    /// The slot of `var`: the position of its first occurrence among the variables the
    /// program reads.
    fn slot(&mut self, var: &Variable) -> u32 {
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
            return slot;
        }
        let slot = self.program.vars.len() as u32;
        self.program.vars.push(var.clone());
        if !self.slots.is_empty() {
            self.slots.insert(var.clone(), slot);
        } else if self.program.vars.len() > SCANNED_VARIABLES {
            for (index, known) in self.program.vars.iter().enumerate() {
                self.slots.insert(known.clone(), index as u32);
            }
        }
        slot
    }

    /// A new term constant.
    fn constant(&mut self, value: Constant) -> Op {
        self.program.consts.push(value);
        Op::Const((self.program.consts.len() - 1) as u32)
    }

    /// A new string constant.
    fn string(&mut self, lexical: String, language: Option<String>) -> Op {
        self.program.strs.push((lexical, language));
        Op::StrConst((self.program.strs.len() - 1) as u32)
    }

    /// A new call-table entry.
    fn call(&mut self, function: &Function) -> u32 {
        self.program.calls.push(function.clone());
        (self.program.calls.len() - 1) as u32
    }

    /// A new regex slot.
    fn regex(&mut self, constant: Option<(String, String)>) -> u32 {
        self.program.regexes.push(constant);
        (self.program.regexes.len() - 1) as u32
    }

    /// Lay out `expr` read in `mode` as tasks, in evaluation order. Leaves are emitted
    /// here, which is where the pre-order visits them; composite nodes become their
    /// operands' tasks followed by their own instruction.
    fn expand<'x>(&mut self, expr: &'x Expression, mode: Mode, out: &mut Tasks<'x>) {
        match mode {
            Mode::Term => self.expand_term(expr, out),
            Mode::StringArg => match expr {
                Expression::Literal(lit)
                    if lit.datatype().as_str() == XSD_STRING
                        || lit.datatype().as_str() == RDF_LANG_STRING =>
                {
                    let op = self.string(
                        lit.value().to_owned(),
                        lit.language().map(str::to_ascii_lowercase),
                    );
                    out.push(Task::Emit(op));
                }
                Expression::FunctionCall(Function::Str, inner) if inner.len() == 1 => {
                    out.push(Task::Compile(&inner[0], Mode::StrLexical));
                }
                Expression::FunctionCall(Function::Lang, inner) if inner.len() == 1 => {
                    out.push(Task::Compile(&inner[0], Mode::LangLexical));
                }
                _ => {
                    out.push(Task::Compile(expr, Mode::Term));
                    out.push(Task::Emit(Op::ToStrArg));
                }
            },
            Mode::StrLexical => match expr {
                Expression::NamedNode(node) => {
                    let op = self.string(node.as_str().to_owned(), None);
                    out.push(Task::Emit(op));
                }
                Expression::Literal(lit) => {
                    let op = self.string(lit.value().to_owned(), None);
                    out.push(Task::Emit(op));
                }
                _ => {
                    out.push(Task::Compile(expr, Mode::Term));
                    out.push(Task::Emit(Op::ToStrLexical));
                }
            },
            Mode::LangLexical => match expr {
                Expression::Literal(lit) => {
                    let op =
                        self.string(lit.language().map_or_default(str::to_ascii_lowercase), None);
                    out.push(Task::Emit(op));
                }
                _ => {
                    out.push(Task::Compile(expr, Mode::Term));
                    out.push(Task::Emit(Op::ToLangLexical));
                }
            },
        }
    }

    /// [`Self::expand`] for [`Mode::Term`].
    fn expand_term<'x>(&mut self, expr: &'x Expression, out: &mut Tasks<'x>) {
        match expr {
            Expression::NamedNode(node) => {
                let op = self.constant(Constant::Iri(node.clone()));
                out.push(Task::Emit(op));
            }
            Expression::Literal(lit) => {
                let op = self.constant(Constant::Literal(lit.clone()));
                out.push(Task::Emit(op));
            }
            Expression::Variable(var) => {
                let slot = self.slot(var);
                out.push(Task::Emit(Op::Var(slot)));
            }
            Expression::Bound(var) => {
                let slot = self.slot(var);
                out.push(Task::Emit(Op::Bound(slot)));
            }
            Expression::Or(operands) | Expression::And(operands) => {
                for operand in operands {
                    out.push(Task::Compile(operand, Mode::Term));
                    out.push(Task::Emit(Op::EbvOf));
                }
                out.push(Task::Emit(Op::Kleene {
                    or: matches!(expr, Expression::Or(_)),
                    n: operands.len() as u32,
                }));
            }
            Expression::Not(operand) => {
                out.push(Task::Compile(operand, Mode::Term));
                out.push(Task::Emit(Op::EbvOf));
                out.push(Task::Emit(Op::Not));
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
                out.push(Task::Compile(a, Mode::Term));
                out.push(Task::Compile(b, Mode::Term));
                out.push(Task::Emit(op));
            }
            Expression::If(condition, then, otherwise) => {
                let on_false = self.label();
                let end = self.label();
                out.push(Task::Compile(condition, Mode::Term));
                out.push(Task::Emit(Op::EbvOf));
                out.push(Task::Emit(Op::Branch { on_false, end }));
                out.push(Task::Compile(then, Mode::Term));
                out.push(Task::Emit(Op::Jmp(end)));
                out.push(Task::Label(on_false));
                out.push(Task::Compile(otherwise, Mode::Term));
                out.push(Task::Label(end));
            }
            Expression::Coalesce(items) => {
                let Some((last, init)) = items.split_last() else {
                    out.push(Task::Emit(Op::PushUnbound));
                    return;
                };
                let end = self.label();
                for item in init {
                    out.push(Task::Compile(item, Mode::Term));
                    out.push(Task::Emit(Op::CoalesceNext(end)));
                }
                out.push(Task::Compile(last, Mode::Term));
                out.push(Task::Label(end));
            }
            Expression::In(needle, haystack) => {
                let end = self.label();
                out.push(Task::Compile(needle, Mode::Term));
                out.push(Task::Emit(Op::InNeedle(end)));
                for item in haystack {
                    out.push(Task::Compile(item, Mode::Term));
                    out.push(Task::Emit(Op::InItem(end)));
                }
                out.push(Task::Emit(Op::InEnd));
                out.push(Task::Label(end));
            }
            Expression::Exists(_) => {
                let site = self.program.exists;
                self.program.exists += 1;
                out.push(Task::Emit(Op::Exists(site)));
            }
            Expression::Arithmetic(first, steps) => {
                out.push(Task::Compile(first, Mode::Term));
                for (op, operand) in steps {
                    out.push(Task::Compile(operand, Mode::Term));
                    out.push(Task::Emit(Op::Arith(*op)));
                }
            }
            Expression::UnaryPlus(operand) => {
                out.push(Task::Compile(operand, Mode::Term));
                out.push(Task::Emit(Op::UnaryPlus));
            }
            Expression::UnaryMinus(operand) => {
                out.push(Task::Compile(operand, Mode::Term));
                out.push(Task::Emit(Op::UnaryMinus));
            }
            Expression::FunctionCall(function, args) => self.expand_call(function, args, out),
        }
    }

    /// A function call in [`Mode::Term`]. The string predicates read their arguments
    /// as string arguments and ignore any past the ones they take; every other call
    /// evaluates every argument as a term, then calls.
    fn expand_call<'x>(
        &mut self,
        function: &'x Function,
        args: &'x [Expression],
        out: &mut Tasks<'x>,
    ) {
        let string_args = |count: usize, out: &mut Tasks<'x>| {
            for index in 0..count {
                out.push(args.get(index).map_or(Task::AbsentStringArg, |arg| {
                    Task::Compile(arg, Mode::StringArg)
                }));
            }
            if args.len() > count {
                out.push(Task::Skip(&args[count..]));
            }
        };
        match function {
            Function::Contains | Function::StrStarts | Function::StrEnds => {
                string_args(2, out);
                out.push(Task::Emit(Op::StrPred(match function {
                    Function::Contains => StrPred::Contains,
                    Function::StrStarts => StrPred::StrStarts,
                    _ => StrPred::StrEnds,
                })));
            }
            Function::Regex => {
                let constant = constant_string_arg(args.get(1)).and_then(|pattern| {
                    let flags = match args.get(2) {
                        None => String::new(),
                        Some(flags) => constant_string_arg(Some(flags))?,
                    };
                    Some((pattern, flags))
                });
                let slot = self.regex(constant);
                string_args(3, out);
                out.push(Task::Emit(Op::Regex(slot)));
            }
            Function::LangMatches => {
                string_args(2, out);
                out.push(Task::Emit(Op::LangMatches));
            }
            Function::Custom(_) => {
                for arg in args {
                    out.push(Task::Compile(arg, Mode::Term));
                }
                let call = self.call(function);
                out.push(Task::Emit(Op::CallCustom {
                    call,
                    argc: args.len() as u32,
                }));
            }
            _ => {
                let regex = if matches!(function, Function::Replace) {
                    replace_constant(args).map(|constant| self.regex(Some(constant)))
                } else {
                    None
                };
                for arg in args {
                    out.push(Task::Compile(arg, Mode::Term));
                }
                let call = self.call(function);
                out.push(Task::Emit(Op::Call {
                    call,
                    argc: args.len() as u32,
                    regex,
                }));
            }
        }
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

/// The string a [`Mode::StringArg`] operand reads without evaluating anything, when it
/// is one: a string literal, or `STR`/`LANG` of a constant.
fn constant_string_arg(expr: Option<&Expression>) -> Option<String> {
    match expr? {
        Expression::Literal(lit)
            if lit.datatype().as_str() == XSD_STRING
                || lit.datatype().as_str() == RDF_LANG_STRING =>
        {
            Some(lit.value().to_owned())
        }
        Expression::FunctionCall(Function::Str, inner) if inner.len() == 1 => match &inner[0] {
            Expression::NamedNode(node) => Some(node.as_str().to_owned()),
            Expression::Literal(lit) => Some(lit.value().to_owned()),
            _ => None,
        },
        Expression::FunctionCall(Function::Lang, inner) if inner.len() == 1 => match &inner[0] {
            Expression::Literal(lit) => {
                Some(lit.language().map_or_default(str::to_ascii_lowercase))
            }
            _ => None,
        },
        _ => None,
    }
}

/// `REPLACE`'s pattern and flags, when both are string literals (or the flags are
/// absent): the lexical forms its string-argument reading takes from them.
fn replace_constant(args: &[Expression]) -> Option<(String, String)> {
    let string_literal = |expr: &Expression| match expr {
        Expression::Literal(lit)
            if lit.datatype().as_str() == XSD_STRING
                || lit.datatype().as_str() == RDF_LANG_STRING
                || lit.datatype().as_str() == RDF_DIR_LANG_STRING =>
        {
            Some(lit.value().to_owned())
        }
        _ => None,
    };
    let pattern = string_literal(args.get(1)?)?;
    let flags = match args.get(3) {
        None => String::new(),
        Some(flags) => string_literal(flags)?,
    };
    Some((pattern, flags))
}

/// The `EXISTS` nodes in `exprs`, counted over a work list.
fn count_exists(exprs: &[Expression]) -> u32 {
    let mut count = 0;
    let mut pending: Vec<&Expression> = exprs.iter().collect();
    while let Some(expr) = pending.pop() {
        if matches!(expr, Expression::Exists(_)) {
            count += 1;
        }
        push_operands(expr, &mut pending);
    }
    count
}

/// Push `expr`'s operands onto `pending` — every sub-expression, in reverse so a stack
/// pops them left to right.
pub(super) fn push_operands<'x>(expr: &'x Expression, pending: &mut Vec<&'x Expression>) {
    let start = pending.len();
    match expr {
        Expression::NamedNode(_)
        | Expression::Literal(_)
        | Expression::Variable(_)
        | Expression::Bound(_)
        | Expression::Exists(_) => {}
        Expression::Or(operands) | Expression::And(operands) => pending.extend(operands.iter()),
        Expression::Equal(a, b)
        | Expression::SameTerm(a, b)
        | Expression::Greater(a, b)
        | Expression::GreaterOrEqual(a, b)
        | Expression::Less(a, b)
        | Expression::LessOrEqual(a, b) => pending.extend([&**a, &**b]),
        Expression::Not(a) | Expression::UnaryPlus(a) | Expression::UnaryMinus(a) => {
            pending.push(a);
        }
        Expression::If(a, b, c) => pending.extend([&**a, &**b, &**c]),
        Expression::Coalesce(items) => pending.extend(items.iter()),
        Expression::In(needle, haystack) => {
            pending.push(needle);
            pending.extend(haystack.iter());
        }
        Expression::Arithmetic(first, steps) => {
            pending.push(first);
            pending.extend(steps.iter().map(|(_, operand)| operand));
        }
        Expression::FunctionCall(_, args) => pending.extend(args.iter()),
    }
    pending[start..].reverse();
}
