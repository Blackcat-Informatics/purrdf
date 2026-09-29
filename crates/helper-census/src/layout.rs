// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Byte-layout rules: syntactic properties of shipping code that hold framing,
//! fixed-width integer access, layout arithmetic and the word-parallel byte
//! compare to their single homes.
//!
//! Test code is out of scope — `tests/`, `benches/`, `examples/` and
//! `generated/` directories and `#[cfg(test)]`/`#[test]` items: a test that
//! spells a preimage or a wire image out by hand is an independent oracle for
//! the implementation, not a second implementation.
//!
//! * `length-prefix-frame`: a field written after its own length. Two adjacent
//!   statements of one block that write to one sink — the same method
//!   receiver, or the same first argument of a free call — where the first
//!   hands the writer a 64-bit length of `E` (`E.len()` widened by `as u64`,
//!   `u64::try_from` or `u64::from`, under byte conversion and unwrapping, or a
//!   local bound to such a length earlier in the block)
//!   and the second writes `E` itself (`E`, `&E`, `E.as_bytes()`, `E.as_ref()`,
//!   `&E[..]`); or one formatting macro whose arguments are `E.len()` and `E`
//!   (a decimal length prefix). A writer whose name says it writes another
//!   integer encoding — a varint, LEB128 or a CBOR head, or a hasher's
//!   `write_u64` — is a different construction and is not read.
//!   `purrdf_hash::frame::frame_le` and `frame_le_into` are the framing; a
//!   variant with a published byte order or notation of its own is a ledger
//!   row.
//! * `le-slice-int`: a fixed-width integer read or written through a
//!   range-indexed sub-slice — `uN::from_le_bytes(bytes[a..b].try_into()…)`,
//!   or `bytes[a..b].copy_from_slice(&v.to_le_bytes())`. The width is then
//!   stated twice (the range and the type) and checked at run time; a read goes
//!   through `first_chunk`/`split_first_chunk` or `purrdf_core::bytes`, whose
//!   width is the type's.
//! * `align-up-mask`: rounding up to a power of two by hand, `(x + (a - 1)) &
//!   !(a - 1)` or `(x + 7) & !7` — `checked_next_multiple_of` states it and
//!   refuses the overflow the mask wraps through.
//! * `div-ceil-by-hand`: `(n + d - 1) / d`, which overflows where `n + d`
//!   does — `div_ceil` on unsigned types, and on the signed ones (whose
//!   `div_ceil` is not stable) retrieval's `reciprocal_rank::ceil_div`.
//! * `xor-first-mismatch`: the first-mismatch index by hand — an XOR of two
//!   words each loaded by `uN::from_{le,ne,be}_bytes` (in place, or through a
//!   local bound to the load), in a function that also takes a
//!   `trailing_zeros`/`leading_zeros` count. `purrdf_deflate::common_prefix_len`
//!   is the one word-parallel common-prefix length, and its home package holds
//!   the body.

use syn::visit::Visit;

use crate::rules::RuleHit;

/// A field written after its own length.
pub(crate) const LENGTH_PREFIX_FRAME: &str = "rule:length-prefix-frame";
/// A fixed-width integer read or written through a range-indexed sub-slice.
pub(crate) const LE_SLICE_INT: &str = "rule:le-slice-int";
/// Rounding up to a power of two by mask.
pub(crate) const ALIGN_UP_MASK: &str = "rule:align-up-mask";
/// Ceiling division by hand.
pub(crate) const DIV_CEIL_BY_HAND: &str = "rule:div-ceil-by-hand";
/// The first-mismatch index by hand: an XOR of two loaded words and a zero
/// count.
pub(crate) const XOR_FIRST_MISMATCH: &str = "rule:xor-first-mismatch";

/// Every rule this module computes.
pub(crate) const RULES: [&str; 5] = [
    LENGTH_PREFIX_FRAME,
    LE_SLICE_INT,
    ALIGN_UP_MASK,
    DIV_CEIL_BY_HAND,
    XOR_FIRST_MISMATCH,
];

/// The word loads `xor-first-mismatch` reads.
const WORD_LOADS: [&str; 3] = ["from_le_bytes", "from_ne_bytes", "from_be_bytes"];

/// What `xor-first-mismatch` has seen of one function body.
#[derive(Default)]
struct MismatchFrame {
    /// The locals bound to a word load.
    loads: Vec<String>,
    /// The lines of each XOR of two loaded words.
    xors: Vec<usize>,
    /// Whether the body takes a trailing- or leading-zero count.
    counts_zeros: bool,
}

/// The formatting macros whose arguments `length-prefix-frame` reads.
const FORMAT_MACROS: [&str; 4] = ["format", "format_args", "write", "writeln"];

/// A stable spelling of the simple place expressions a frame writes: a path, a
/// field, a no-argument method call on one (`s.as_bytes()`), a literal index,
/// through parentheses. `None` for anything else.
fn place(expr: &syn::Expr) -> Option<String> {
    match expr {
        syn::Expr::Path(path) if path.qself.is_none() => Some(
            path.path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>()
                .join("::"),
        ),
        syn::Expr::Field(field) => {
            let base = place(&field.base)?;
            let member = match &field.member {
                syn::Member::Named(name) => name.to_string(),
                syn::Member::Unnamed(index) => index.index.to_string(),
            };
            Some(format!("{base}.{member}"))
        }
        syn::Expr::MethodCall(call) if call.args.is_empty() && call.turbofish.is_none() => {
            Some(format!("{}.{}()", place(&call.receiver)?, call.method))
        }
        syn::Expr::Index(index) => match index.index.as_ref() {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Int(literal),
                ..
            }) => Some(format!(
                "{}[{}]",
                place(&index.expr)?,
                literal.base10_digits()
            )),
            _ => None,
        },
        syn::Expr::Paren(inner) => place(&inner.expr),
        syn::Expr::Group(inner) => place(&inner.expr),
        _ => None,
    }
}

/// The place an argument writes as bytes: `E`, `&E`, `E.as_bytes()`,
/// `E.as_ref()` or `&E[..]`, spelt as [`place`] spells `E`.
fn written(expr: &syn::Expr) -> Option<String> {
    match expr {
        syn::Expr::Reference(reference) => written(&reference.expr),
        syn::Expr::Paren(inner) => written(&inner.expr),
        syn::Expr::MethodCall(call)
            if call.args.is_empty() && (call.method == "as_bytes" || call.method == "as_ref") =>
        {
            place(&call.receiver)
        }
        syn::Expr::Index(index) if matches!(index.index.as_ref(), syn::Expr::Range(range) if range.start.is_none() && range.end.is_none()) => {
            place(&index.expr)
        }
        other => place(other),
    }
}

/// The place `expr` is the bare length of: `E.len()`.
fn bare_length(expr: &syn::Expr) -> Option<String> {
    match expr {
        syn::Expr::MethodCall(call) if call.method == "len" && call.args.is_empty() => {
            place(&call.receiver)
        }
        syn::Expr::Paren(inner) => bare_length(&inner.expr),
        _ => None,
    }
}

/// The wrappers a length passes through on its way into a writer: byte
/// conversion, a fallible widening's unwrapping, a reference, parentheses.
const LENGTH_WRAPPERS: [&str; 9] = [
    "to_le_bytes",
    "to_be_bytes",
    "expect",
    "unwrap",
    "unwrap_or",
    "unwrap_or_else",
    "map_err",
    "ok_or",
    "ok_or_else",
];

/// The place `expr` is a 64-bit length of — `E.len()` widened by `as u64`,
/// `u64::try_from(E.len())` or `u64::from(E.len())`, under any of
/// [`LENGTH_WRAPPERS`] — or the place a local bound to such a length (in
/// `bound`) stands for. A bare `E.len()` handed to a writer is not read: the
/// writer's width and encoding are out of sight (a capacity check, a varint).
fn length_of(expr: &syn::Expr, bound: &[(String, String)]) -> Option<String> {
    match expr {
        syn::Expr::Reference(inner) => length_of(&inner.expr, bound),
        syn::Expr::Paren(inner) => length_of(&inner.expr, bound),
        syn::Expr::Try(inner) => length_of(&inner.expr, bound),
        syn::Expr::MethodCall(call)
            if LENGTH_WRAPPERS.contains(&call.method.to_string().as_str()) =>
        {
            length_of(&call.receiver, bound)
        }
        syn::Expr::Cast(cast) => match cast.ty.as_ref() {
            syn::Type::Path(ty) if ty.path.is_ident("u64") => bare_length(&cast.expr),
            _ => None,
        },
        syn::Expr::Call(call) => {
            let syn::Expr::Path(function) = call.func.as_ref() else {
                return None;
            };
            let segments: Vec<String> = function
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect();
            match segments.as_slice() {
                [.., ty, method] if ty == "u64" && (method == "try_from" || method == "from") => {
                    bare_length(call.args.first()?)
                }
                _ => None,
            }
        }
        syn::Expr::Path(path) => {
            let ident = path.path.get_ident()?.to_string();
            bound
                .iter()
                .rev()
                .find(|(name, _)| *name == ident)
                .map(|(_, place)| place.clone())
        }
        _ => None,
    }
}

/// Whether a writer's name says it writes a self-delimiting integer encoding
/// (a varint, LEB128, a CBOR head) or a hasher's native integer, not a
/// fixed-width length prefix.
fn is_other_encoding(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    ["varint", "leb", "cbor"]
        .iter()
        .any(|word| name.contains(word))
        || (name.starts_with("write_") && name != "write_all" && name != "write")
}

/// A write statement: the sink it writes to, the writer's name and its
/// arguments.
struct Write<'a> {
    sink: String,
    writer: String,
    arguments: Vec<&'a syn::Expr>,
}

/// The write a statement makes: a method call whose receiver is a place
/// (arguments are the call's), or a free call whose first argument is a place
/// (arguments are the rest). The statement may end in `;` or `?`.
fn write_of(statement: &syn::Stmt) -> Option<Write<'_>> {
    let syn::Stmt::Expr(expr, _) = statement else {
        return None;
    };
    let expr = match expr {
        syn::Expr::Try(inner) => inner.expr.as_ref(),
        other => other,
    };
    match expr {
        syn::Expr::MethodCall(call) => Some(Write {
            sink: place(&call.receiver)?,
            writer: call.method.to_string(),
            arguments: call.args.iter().collect(),
        }),
        syn::Expr::Call(call) => {
            let syn::Expr::Path(function) = call.func.as_ref() else {
                return None;
            };
            let mut arguments = call.args.iter();
            let first = arguments.next()?;
            Some(Write {
                sink: written(first)?,
                writer: function.path.segments.last()?.ident.to_string(),
                arguments: arguments.collect(),
            })
        }
        _ => None,
    }
}

/// Whether two writes reach one sink: the same place, or a place and a field
/// of it (`self.u64(len)` then `self.buf.extend_from_slice(bytes)`).
fn same_sink(first: &str, second: &str) -> bool {
    let nested = |outer: &str, inner: &str| {
        inner
            .strip_prefix(outer)
            .is_some_and(|rest| rest.starts_with('.'))
    };
    first == second || nested(first, second) || nested(second, first)
}

/// A `let` binding one ident to a length of a place (see [`length_of`]): its
/// ident and the place.
fn length_binding(statement: &syn::Stmt, bound: &[(String, String)]) -> Option<(String, String)> {
    let syn::Stmt::Local(local) = statement else {
        return None;
    };
    let syn::Pat::Ident(ident) = &local.pat else {
        return None;
    };
    let place = length_of(&local.init.as_ref()?.expr, bound)?;
    Some((ident.ident.to_string(), place))
}

/// The visitor for the byte-layout rules.
struct Layout<'a> {
    package: &'a str,
    file: &'a str,
    scope: Vec<String>,
    hits: Vec<RuleHit>,
    mismatch: Vec<MismatchFrame>,
}

impl Layout<'_> {
    fn hit(&mut self, rule: &'static str, line: usize, detail: String) {
        let mut symbol = self.file.to_owned();
        for scope in &self.scope {
            symbol.push_str("::");
            symbol.push_str(scope);
        }
        self.hits.push(RuleHit {
            rule,
            package: self.package.to_owned(),
            file: self.file.to_owned(),
            line,
            symbol,
            detail,
            // The framing's home package holds the framings themselves; every
            // other rule's home computes through the standard library and
            // holds no copy either.
            home_exempt: rule == LENGTH_PREFIX_FRAME || rule == XOR_FIRST_MISMATCH,
        });
    }

    fn scoped(&mut self, name: String, walk: impl FnOnce(&mut Self)) {
        self.scope.push(name);
        walk(self);
        self.scope.pop();
    }

    /// Walk one function body with its own `xor-first-mismatch` frame, and
    /// report its XORs of loaded words when the body also counts zeros.
    fn function(&mut self, walk: impl FnOnce(&mut Self)) {
        self.mismatch.push(MismatchFrame::default());
        walk(self);
        let frame = self.mismatch.pop().unwrap_or_default();
        if frame.counts_zeros {
            for line in frame.xors {
                self.hit(
                    XOR_FIRST_MISMATCH,
                    line,
                    "an XOR of two loaded words ended by a zero count: the first-mismatch \
                     index by hand; call purrdf_deflate::common_prefix_len"
                        .to_owned(),
                );
            }
        }
    }

    /// Whether `expr` is a word load: `uN::from_le_bytes(…)` (or `ne`/`be`),
    /// through parentheses, or a local of this function bound to one.
    fn is_word_load(&self, expr: &syn::Expr) -> bool {
        match expr {
            syn::Expr::Paren(inner) => self.is_word_load(&inner.expr),
            syn::Expr::Call(call) => is_word_load_call(call),
            syn::Expr::Path(path) => path.path.get_ident().is_some_and(|ident| {
                self.mismatch
                    .last()
                    .is_some_and(|frame| frame.loads.contains(&ident.to_string()))
            }),
            _ => false,
        }
    }

    /// `length-prefix-frame` over one block's statements.
    fn frames(&mut self, statements: &[syn::Stmt]) {
        let mut bound: Vec<(String, String)> = Vec::new();
        for (index, statement) in statements.iter().enumerate() {
            if let Some(binding) = length_binding(statement, &bound) {
                bound.push(binding);
                continue;
            }
            let Some(first) = write_of(statement) else {
                continue;
            };
            let Some(second) = statements.get(index + 1).and_then(write_of) else {
                continue;
            };
            if !same_sink(&first.sink, &second.sink) || is_other_encoding(&first.writer) {
                continue;
            }
            let prefixed: Vec<String> = first
                .arguments
                .iter()
                .filter_map(|argument| length_of(argument, &bound))
                .collect();
            if let Some(field) = second
                .arguments
                .iter()
                .filter_map(|argument| written(argument))
                .find(|field| prefixed.contains(field))
            {
                let line = statement_line(statement);
                self.hit(
                    LENGTH_PREFIX_FRAME,
                    line,
                    format!(
                        "`{field}` is written after its own length; frame it through \
                         purrdf_hash::frame::frame_le or frame_le_into"
                    ),
                );
            }
        }
    }
}

/// The first line of a statement.
fn statement_line(statement: &syn::Stmt) -> usize {
    use syn::spanned::Spanned as _;
    statement.span().start().line
}

/// Whether `expr` is a range-indexed sub-slice `x[a..b]` (either bound
/// present).
fn is_range_slice(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Index(index) => matches!(
            index.index.as_ref(),
            syn::Expr::Range(range) if range.start.is_some() || range.end.is_some()
        ),
        syn::Expr::Paren(inner) => is_range_slice(&inner.expr),
        syn::Expr::Reference(reference) => is_range_slice(&reference.expr),
        _ => false,
    }
}

/// Whether `expr` converts a range-indexed sub-slice to an array:
/// `x[a..b].try_into()` under any `expect`/`unwrap`/`?`/`map_err` wrapping.
fn slice_array(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::MethodCall(call) if call.method == "try_into" => is_range_slice(&call.receiver),
        syn::Expr::MethodCall(call) => slice_array(&call.receiver),
        syn::Expr::Try(inner) => slice_array(&inner.expr),
        syn::Expr::Paren(inner) => slice_array(&inner.expr),
        syn::Expr::Call(call) => {
            // `<[u8; N]>::try_from(&x[a..b])` spelt as a call.
            let names_try_from = matches!(call.func.as_ref(), syn::Expr::Path(path)
                if path.path.segments.last().is_some_and(|segment| segment.ident == "try_from"));
            names_try_from && call.args.first().is_some_and(is_range_slice)
        }
        _ => false,
    }
}

/// The integer type names a `uN::from_le_bytes`/`to_le_bytes` rule accepts.
const INTEGERS: [&str; 8] = ["u16", "u32", "u64", "u128", "i16", "i32", "i64", "i128"];

impl<'ast> Visit<'ast> for Layout<'_> {
    fn visit_block(&mut self, node: &'ast syn::Block) {
        self.frames(&node.stmts);
        syn::visit::visit_block(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let name = node
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
            .unwrap_or_default();
        if FORMAT_MACROS.contains(&name.as_str())
            && let Ok(arguments) = node.parse_body_with(
                syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated,
            )
        {
            let lengths: Vec<String> = arguments
                .iter()
                .filter_map(|argument| match argument {
                    syn::Expr::MethodCall(call) if call.method == "len" && call.args.is_empty() => {
                        place(&call.receiver)
                    }
                    _ => None,
                })
                .collect();
            if let Some(field) = arguments
                .iter()
                .filter_map(written)
                .find(|field| lengths.contains(field))
            {
                self.hit(
                    LENGTH_PREFIX_FRAME,
                    node.path.segments[0].ident.span().start().line,
                    format!("`{field}` is formatted after its own length: a decimal length prefix"),
                );
            }
        }
        syn::visit::visit_macro(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(function) = node.func.as_ref() {
            let segments: Vec<String> = function
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect();
            if let [.., integer, method] = segments.as_slice()
                && method == "from_le_bytes"
                && INTEGERS.contains(&integer.as_str())
                && node.args.first().is_some_and(slice_array)
            {
                self.hit(
                    LE_SLICE_INT,
                    integer_line(function),
                    format!(
                        "`{integer}::from_le_bytes` over a range-indexed sub-slice; read through \
                         `first_chunk`/`split_first_chunk` or purrdf_core::bytes"
                    ),
                );
            }
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if (node.method == "trailing_zeros" || node.method == "leading_zeros")
            && node.args.is_empty()
            && let Some(frame) = self.mismatch.last_mut()
        {
            frame.counts_zeros = true;
        }
        if node.method == "copy_from_slice"
            && is_range_slice(&node.receiver)
            && node.args.first().is_some_and(|argument| {
                let argument = match argument {
                    syn::Expr::Reference(reference) => reference.expr.as_ref(),
                    other => other,
                };
                matches!(argument, syn::Expr::MethodCall(call) if call.method == "to_le_bytes")
            })
        {
            self.hit(
                LE_SLICE_INT,
                node.method.span().start().line,
                "a little-endian integer written through a range-indexed sub-slice; write through \
                 purrdf_core::bytes or `first_chunk_mut`"
                    .to_owned(),
            );
        }
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        if let syn::BinOp::BitXor(token) = node.op
            && self.is_word_load(&node.left)
            && self.is_word_load(&node.right)
            && let Some(frame) = self.mismatch.last_mut()
        {
            frame.xors.push(token.span.start().line);
        }
        match node.op {
            syn::BinOp::BitAnd(token)
                if is_rounded(&node.left) && is_low_mask_complement(&node.right) =>
            {
                self.hit(
                    ALIGN_UP_MASK,
                    token.span.start().line,
                    "rounding up by mask; use `checked_next_multiple_of`".to_owned(),
                );
            }
            syn::BinOp::Div(token) if is_biased_by(&node.left, &node.right) => {
                self.hit(
                    DIV_CEIL_BY_HAND,
                    token.span.start().line,
                    "ceiling division by hand; use `div_ceil` or a checked ceiling division"
                        .to_owned(),
                );
            }
            _ => {}
        }
        syn::visit::visit_expr_binary(self, node);
    }

    fn visit_local(&mut self, node: &'ast syn::Local) {
        let pattern = match &node.pat {
            syn::Pat::Type(typed) => typed.pat.as_ref(),
            other => other,
        };
        if let syn::Pat::Ident(binding) = pattern
            && let Some(init) = &node.init
            && self.is_word_load(&init.expr)
            && let Some(frame) = self.mismatch.last_mut()
        {
            frame.loads.push(binding.ident.to_string());
        }
        syn::visit::visit_local(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        if crate::source::is_test_only(&node.attrs) {
            return;
        }
        self.scoped(node.sig.ident.to_string(), |this| {
            this.function(|this| syn::visit::visit_item_fn(this, node));
        });
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        if crate::source::is_test_only(&node.attrs) {
            return;
        }
        self.scoped(node.sig.ident.to_string(), |this| {
            this.function(|this| syn::visit::visit_impl_item_fn(this, node));
        });
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if !crate::source::is_test_only(&node.attrs) {
            syn::visit::visit_item_impl(self, node);
        }
    }

    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if crate::source::is_test_only(&node.attrs) {
            return;
        }
        self.scoped(node.ident.to_string(), |this| {
            syn::visit::visit_item_mod(this, node);
        });
    }
}

/// Whether `call` is `uN::from_le_bytes(…)`, `from_ne_bytes` or
/// `from_be_bytes` on an integer type.
fn is_word_load_call(call: &syn::ExprCall) -> bool {
    let syn::Expr::Path(function) = call.func.as_ref() else {
        return false;
    };
    let segments: Vec<String> = function
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect();
    matches!(
        segments.as_slice(),
        [.., integer, method]
            if WORD_LOADS.contains(&method.as_str()) && INTEGERS.contains(&integer.as_str())
    )
}

/// The line of a call's path.
fn integer_line(function: &syn::ExprPath) -> usize {
    function.path.segments[0].ident.span().start().line
}

/// Whether `expr` can be the value a mask rounds: an addition (`x + 7`,
/// `x + a - 1`) or a bound name (`biased`, after a checked addition).
fn is_rounded(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Binary(binary) => match binary.op {
            syn::BinOp::Add(_) => true,
            syn::BinOp::Sub(_) => is_rounded(&binary.left),
            _ => false,
        },
        syn::Expr::Paren(inner) => is_rounded(&inner.expr),
        syn::Expr::Path(path) => path.path.get_ident().is_some(),
        _ => false,
    }
}

/// Whether `expr` is the complement of a low-bit mask: `!(a - 1)`, or `!m`
/// for an integer literal `m` one less than a power of two.
fn is_low_mask_complement(expr: &syn::Expr) -> bool {
    let syn::Expr::Unary(unary) = expr else {
        return matches!(expr, syn::Expr::Paren(inner) if is_low_mask_complement(&inner.expr));
    };
    if !matches!(unary.op, syn::UnOp::Not(_)) {
        return false;
    }
    let mut operand = unary.expr.as_ref();
    while let syn::Expr::Paren(inner) = operand {
        operand = &inner.expr;
    }
    match operand {
        syn::Expr::Binary(binary) => {
            matches!(binary.op, syn::BinOp::Sub(_)) && is_one(&binary.right)
        }
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(literal),
            ..
        }) => literal
            .base10_parse::<u128>()
            .is_ok_and(|mask| mask != 0 && (mask + 1).is_power_of_two()),
        _ => false,
    }
}

/// Whether `numerator` is `(n + d - 1)` for the divisor `divisor` = `d`.
fn is_biased_by(numerator: &syn::Expr, divisor: &syn::Expr) -> bool {
    let syn::Expr::Paren(numerator) = numerator else {
        return false;
    };
    let syn::Expr::Binary(minus) = numerator.expr.as_ref() else {
        return false;
    };
    if !matches!(minus.op, syn::BinOp::Sub(_)) || !is_one(&minus.right) {
        return false;
    }
    let syn::Expr::Binary(plus) = minus.left.as_ref() else {
        return false;
    };
    matches!(plus.op, syn::BinOp::Add(_))
        && place(divisor).is_some()
        && place(&plus.right) == place(divisor)
}

/// Whether `expr` is the integer literal `1`.
fn is_one(expr: &syn::Expr) -> bool {
    matches!(expr, syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(literal), .. }) if literal.base10_digits() == "1")
}

/// Every byte-layout hit in one parsed shipping file.
pub(crate) fn layout_rules(package: &str, file: &str, parsed: &syn::File) -> Vec<RuleHit> {
    let mut visitor = Layout {
        package,
        file,
        scope: Vec::new(),
        hits: Vec::new(),
        mismatch: Vec::new(),
    };
    visitor.visit_file(parsed);
    visitor.hits
}

#[cfg(test)]
mod tests {
    use super::{
        ALIGN_UP_MASK, DIV_CEIL_BY_HAND, LE_SLICE_INT, LENGTH_PREFIX_FRAME, XOR_FIRST_MISMATCH,
        layout_rules,
    };

    fn hits(source: &str) -> Vec<(&'static str, usize)> {
        let parsed = syn::parse_file(source).expect("fixture parses");
        layout_rules("p", "f.rs", &parsed)
            .iter()
            .map(|hit| (hit.rule, hit.line))
            .collect()
    }

    #[test]
    fn a_field_written_after_its_own_length_is_found() {
        let source = "\
fn a(out: &mut Vec<u8>, b: &[u8]) {
    out.extend_from_slice(&(b.len() as u64).to_le_bytes());
    out.extend_from_slice(b);
}
fn b(h: &mut H, f: &[u8]) {
    let n = u64::try_from(f.len()).expect(\"fits\");
    h.update(n.to_be_bytes());
    h.update(f);
}
fn c(out: &mut String, s: &str) { let _ = write!(out, \"{}:{}\", s.len(), s); }
fn d(this: &mut W, v: &[u8]) { this.u64(v.len() as u64); this.buf.extend_from_slice(v); }
";
        assert_eq!(
            hits(source),
            vec![
                (LENGTH_PREFIX_FRAME, 2),
                (LENGTH_PREFIX_FRAME, 7),
                (LENGTH_PREFIX_FRAME, 10),
                (LENGTH_PREFIX_FRAME, 11),
            ]
        );
    }

    #[test]
    fn a_count_a_varint_and_a_hasher_integer_are_not_frames() {
        let source = "\
fn a(out: &mut Vec<u8>, items: &[u8]) {
    out.extend_from_slice(&(items.len() as u64).to_le_bytes());
    for item in items { out.push(*item); }
}
fn b(out: &mut Vec<u8>, b: &[u8]) { write_varint(out, b.len() as u64); out.extend_from_slice(b); }
fn c(h: &mut H, s: &str) { h.write_u64(s.len() as u64); h.write(s.as_bytes()); }
fn d(out: &mut Vec<u8>, a: &[u8], b: &[u8]) {
    out.extend_from_slice(&(a.len() as u64).to_le_bytes());
    out.extend_from_slice(b);
}
#[cfg(test)]
fn oracle(out: &mut Vec<u8>, b: &[u8]) {
    out.extend_from_slice(&(b.len() as u64).to_le_bytes());
    out.extend_from_slice(b);
}
";
        assert_eq!(hits(source), Vec::new());
    }

    #[test]
    fn slice_indexed_integers_and_alignment_masks_are_found_and_nothing_else() {
        let source = "\
fn a(b: &[u8], i: usize) -> u32 { u32::from_le_bytes(b[i..i + 4].try_into().unwrap()) }
fn b(b: &mut [u8], i: usize, v: u64) { b[i..i + 8].copy_from_slice(&v.to_le_bytes()); }
fn c(v: usize) -> usize { (v + 7) & !7 }
fn d(v: u64, a: u64) -> u64 { (v + a - 1) & !(a - 1) }
fn e(b: &[u8], i: usize) -> u32 { u32::from_le_bytes(*b[i..].first_chunk().unwrap()) }
fn f(b: [u8; 4]) -> u32 { u32::from_le_bytes(b) }
fn g(f: u32) -> u32 { f & !(A | B) }
fn h(f: u64, n: u32) -> u64 { f.bits() & !((1 << n) - 1) }
fn i(n: i128, d: i128) -> i128 { (n + d - 1) / d }
fn j(n: i128, d: i128) -> i128 { (n + 7 - 1) / d }
fn k(n: u64, d: u64) -> u64 { n.div_ceil(d) }
";
        assert_eq!(
            hits(source),
            vec![
                (LE_SLICE_INT, 1),
                (LE_SLICE_INT, 2),
                (ALIGN_UP_MASK, 3),
                (ALIGN_UP_MASK, 4),
                (DIV_CEIL_BY_HAND, 9),
            ]
        );
    }

    #[test]
    fn a_first_mismatch_by_hand_is_found_and_its_neighbours_are_not() {
        let source = "\
fn a(x: &[u8; 8], y: &[u8; 8]) -> u32 {
    (u64::from_le_bytes(*x) ^ u64::from_le_bytes(*y)).trailing_zeros() / 8
}
fn b(p: &[u8], q: &[u8]) -> usize {
    let x = u64::from_le_bytes(*p.first_chunk().unwrap());
    let y: u64 = u64::from_ne_bytes(*q.first_chunk().unwrap());
    let diff = x ^ y;
    diff.trailing_zeros() as usize / 8
}
fn c(r: u32, b: &[u8; 4]) -> u32 { (r ^ u32::from_le_bytes(*b)).leading_zeros() }
fn d(x: &[u8; 8], y: &[u8; 8]) -> u64 { u64::from_le_bytes(*x) ^ u64::from_le_bytes(*y) }
fn e(w: &[u8; 8]) -> u32 { u64::from_le_bytes(*w).trailing_zeros() / 8 }
#[cfg(test)]
fn oracle(x: &[u8; 8], y: &[u8; 8]) -> u32 {
    (u64::from_le_bytes(*x) ^ u64::from_le_bytes(*y)).trailing_zeros() / 8
}
";
        assert_eq!(
            hits(source),
            vec![(XOR_FIRST_MISMATCH, 2), (XOR_FIRST_MISMATCH, 7)]
        );
    }
}
