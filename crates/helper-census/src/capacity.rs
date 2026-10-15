// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Logical table-capacity panic sinks in shipping core and Datalog source.
//!
//! This is a syntactic census, not a Rust type checker. It follows local aliases
//! and the native capacity/mint vocabulary, narrowing count conversions and
//! width assertions. Physical allocation errors are a separate policy. A
//! NonZero representation expect is permitted only after a dominating typed
//! slot check of the same unmodified local and an adequate representation limit.
//! File reachability belongs to the existing source walk, not this visitor.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::{Delimiter, Group, Span, TokenStream, TokenTree};
use syn::spanned::Spanned as _;
use syn::visit::Visit;

use crate::rules::{RuleHit, Scoped as _};

/// The enforced ledger fingerprint for logical table-capacity panic sinks.
pub(crate) const LOGICAL_CAPACITY_PANIC: &str = "rule:logical-capacity-panic";

/// The actual visibility domain of the frozen lexical owner's private fields.
pub(crate) fn frozen_owner_domain(package: &str, module: &str) -> bool {
    package == "purrdf-core"
        && (module == "purrdf_core::ir" || module.starts_with("purrdf_core::ir::"))
}

const COUNT: u8 = 1;
const LOGICAL: u8 = 2;
const LIMIT: u8 = 4;
const FALLIBLE: u8 = 8;
const BOUNDARY: u8 = 16;

/// An individually reviewed reconstruction, bound to its sink and authority source.
#[derive(Clone, Debug)]
pub(crate) struct Proof {
    pub(crate) file: String,
    pub(crate) symbol: String,
    pub(crate) line: usize,
    pub(crate) sink: String,
    pub(crate) class: String,
    pub(crate) authorities: Vec<String>,
    pub(crate) constructors: String,
}

/// Conservatively pin references and opaque expansion sites. This is an audited
/// source ratchet, not a claim to expand macros or type-check Rust.
pub(crate) fn references_constructor(tokens: TokenStream, parsed: &syn::File) -> bool {
    struct Expansion(bool);
    impl<'ast> Visit<'ast> for Expansion {
        fn visit_macro(&mut self, _: &'ast syn::Macro) {
            self.0 = true;
        }
        fn visit_attribute(&mut self, attr: &'ast syn::Attribute) {
            self.0 |= !matches!(
                terminal(attr.path()).as_str(),
                "doc"
                    | "cfg"
                    | "allow"
                    | "expect"
                    | "deny"
                    | "forbid"
                    | "inline"
                    | "cold"
                    | "must_use"
                    | "repr"
                    | "deprecated"
                    | "non_exhaustive"
                    | "path"
                    | "test"
                    | "ignore"
            );
        }
    }
    let mut expansion = Expansion(false);
    expansion.visit_file(parsed);
    expansion.0 || token_words(tokens).iter().any(|word| word == "from_parts")
}

/// Identity of shipping constructor references and the frozen owner's visible module domain.
pub(crate) fn constructor_identity(workspace: &crate::source::Workspace) -> String {
    let records = workspace
        .capacity_constructors
        .iter()
        .filter(|file| file.starts_with("crates/rdf-core/"))
        .map(|file| format!("{file}={}", workspace.capacity_sources[file]))
        .collect::<Vec<_>>()
        .join("\n");
    // Hash through the same literal-preserving source identity home.
    crate::normalize::literal_fingerprint(&TokenStream::from(TokenTree::Literal(
        proc_macro2::Literal::string(&records),
    )))
}

/// Discharge only exact unique sites with unchanged, complete reviewed authorities.
/// Unknown classes, stale rows and any changed source remain gate failures.
pub(crate) fn proofs(
    rows: &[Proof],
    workspace: &crate::source::Workspace,
) -> (BTreeSet<usize>, Vec<String>) {
    if rows.is_empty() {
        return (BTreeSet::new(), Vec::new());
    }
    let mut accepted = BTreeSet::new();
    let mut errors = Vec::new();
    let mut seen = BTreeSet::new();
    let constructors = constructor_identity(workspace);
    for proof in rows {
        let label = format!("capacity proof {} [{}]", proof.symbol, proof.sink);
        let required: &[&str] = match proof.class.as_str() {
            "local-stored-index" | "concrete-frozen-term-range" | "resident-permutation" => &[
                "crates/rdf-core/src/ir/builder.rs",
                "crates/rdf-core/src/ir/dataset.rs",
                "crates/rdf-core/src/ir/frozen_terms.rs",
                "crates/rdf-core/src/ir/term.rs",
                "crates/lex/src/allocation.rs",
            ],
            "global-stored-index" | "resident-global-vec-range" => &[
                "crates/rdf-core/src/ir/global.rs",
                "crates/lex/src/allocation.rs",
            ],
            "segmented-checked-interval" => &[
                "crates/rdf-core/src/ir/segmented/build.rs",
                "crates/rdf-core/src/ir/segmented/mod.rs",
                "crates/rdf-core/src/ir/builder.rs",
                "crates/rdf-core/src/ir/dataset.rs",
                "crates/rdf-core/src/ir/frozen_terms.rs",
                "crates/rdf-core/src/ir/term.rs",
                "crates/rdf-core/src/ir/global.rs",
                "crates/lex/src/allocation.rs",
            ],
            "encoded-shape" => &[
                "crates/rdf-core/src/ir/embedding/wire.rs",
                "crates/rdf-core/src/ir/embedding/mod.rs",
                "crates/rdf-core/src/ir/embedding/contract.rs",
            ],
            _ => {
                errors.push(format!("{label}: unknown proof class"));
                continue;
            }
        };
        let mut authority_files = BTreeSet::new();
        let mut valid = proof.constructors == constructors;
        for authority in &proof.authorities {
            let Some((file, identity)) = authority.split_once('=') else {
                valid = false;
                continue;
            };
            valid &= authority_files.insert(file);
            valid &= workspace
                .capacity_sources
                .get(file)
                .is_some_and(|actual| actual == identity);
        }
        valid &= required.iter().all(|file| authority_files.contains(file));
        valid &= authority_files.contains(proof.file.as_str());
        valid &= seen.insert((&proof.file, &proof.symbol, proof.line, &proof.sink));
        let matching: Vec<_> = workspace
            .rule_hits
            .iter()
            .enumerate()
            .filter(|(_, hit)| {
                hit.rule == LOGICAL_CAPACITY_PANIC
                    && hit.file == proof.file
                    && hit.symbol == proof.symbol
                    && hit.line == proof.line
                    && hit.detail.ends_with(&format!("[{}]", proof.sink))
            })
            .map(|(index, _)| index)
            .collect();
        if valid && let [index] = matching.as_slice() {
            accepted.insert(*index);
        } else {
            errors.push(format!("{label}: stale, duplicate or incomplete source proof ({} matching sinks); review the actual owner and constructor closure", matching.len()));
        }
    }
    (accepted, errors)
}

const MINTS: &[&str] = &[
    "from_index",
    "from_index_with_limit",
    "from_address",
    "from_address_with_limit",
    "intern",
    "try_intern",
    "intern_iri",
    "intern_blank",
    "intern_literal",
    "intern_triple",
    "intern_owned_term",
    "intern_owned_term_scoped",
    "intern_owned_term_bound",
    "intern_literal_parts",
    "intern_literal_bound",
    "intern_text_blank",
    "intern_value",
    "intern_origin",
    "intern_origin_set",
    "register_unit",
    "register_artifact",
    "push_quad",
    "push_reifier",
    "push_annotation",
    "push_reifier_in_graph",
    "push_annotation_in_graph",
    "push_owned_quad",
    "push_owned_quad_scoped",
    "push_owned_reifier",
    "push_owned_reifier_scoped",
    "push_owned_annotation",
    "push_owned_annotation_scoped",
    "push_record",
    "push_record_unchecked",
    "push_quad_with_handle",
    "declare_named_graph",
    "push_dataset",
    "reserve_store_once",
    "store_once",
    "reserve_for_replay",
    "build_permutation",
    "try_permutation",
    "push_arena_str",
    "next_scope",
    "u32_representation",
    "build_page",
    "one_quad",
    "graph_slots",
    "graph_membership_corpus",
    "split_pages",
];

fn is_mint(name: &str) -> bool {
    MINTS.contains(&name.strip_suffix("_with_memory").unwrap_or(name))
}

#[derive(Default)]
struct Names {
    aliases: BTreeMap<String, String>,
    capacity_results: BTreeSet<String>,
    constants: BTreeMap<String, Option<u128>>,
}

impl Names {
    fn name(&self, name: &str) -> String {
        self.aliases
            .get(name)
            .cloned()
            .unwrap_or_else(|| name.to_owned())
    }

    fn import(&mut self, tree: &syn::UseTree, prefix: &str) {
        match tree {
            syn::UseTree::Path(path) => {
                self.import(&path.tree, &format!("{prefix}{}::", path.ident));
            }
            syn::UseTree::Rename(rename) => {
                self.aliases.insert(
                    rename.rename.to_string(),
                    format!("{prefix}{}", rename.ident),
                );
            }
            syn::UseTree::Name(name) => {
                self.aliases
                    .insert(name.ident.to_string(), format!("{prefix}{}", name.ident));
            }
            syn::UseTree::Group(group) => {
                for child in &group.items {
                    self.import(child, prefix);
                }
            }
            syn::UseTree::Glob(_) => {}
        }
    }

    fn signature(&mut self, signature: &syn::Signature) {
        struct CapacityType<'a> {
            names: &'a Names,
            found: bool,
        }
        impl<'ast> Visit<'ast> for CapacityType<'_> {
            fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
                self.found |= path.path.segments.iter().any(|part| {
                    self.names
                        .name(&part.ident.to_string())
                        .split("::")
                        .any(|name| name == "CapacityError")
                });
                syn::visit::visit_type_path(self, path);
            }
        }
        let mut result = CapacityType {
            names: self,
            found: false,
        };
        result.visit_return_type(&signature.output);
        if result.found {
            self.capacity_results.insert(signature.ident.to_string());
        }
    }
}

impl<'ast> Visit<'ast> for Names {
    fn visit_item_const(&mut self, node: &'ast syn::ItemConst) {
        let value = if let syn::Type::Path(ty) = &*node.ty {
            primitive_width(&ty.path)
                .zip(integer(&node.expr))
                .filter(|(width, value)| value <= width)
                .map(|(_, value)| value)
        } else {
            None
        };
        // Ambiguous nested or conditional declarations are not a proof.
        self.constants
            .entry(node.ident.to_string())
            .and_modify(|seen| *seen = None)
            .or_insert(value);
        syn::visit::visit_item_const(self, node);
    }
    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        self.import(&node.tree, "");
    }
    fn visit_signature(&mut self, node: &'ast syn::Signature) {
        self.signature(node);
    }
}

fn terminal(path: &syn::Path) -> String {
    path.segments
        .last()
        .map_or_else(String::new, |part| part.ident.to_string())
}

fn primitive_width(path: &syn::Path) -> Option<u128> {
    if path.segments.len() != 1 {
        return None;
    }
    match terminal(path).as_str() {
        "u32" => Some(u128::from(u32::MAX)),
        "u64" => Some(u128::from(u64::MAX)),
        "u128" => Some(u128::MAX),
        _ => None,
    }
}

fn count_name(name: &str) -> u8 {
    let words: Vec<_> = name.split('_').collect();
    let logical = words.iter().any(|word| {
        matches!(
            *word,
            "term"
                | "terms"
                | "quad"
                | "quads"
                | "row"
                | "rows"
                | "scope"
                | "scopes"
                | "arena"
                | "offset"
                | "postings"
                | "origin"
                | "origins"
                | "artifact"
                | "artifacts"
                | "partition"
                | "partitions"
                | "proof"
                | "proofs"
        )
    });
    let count = logical
        || words.iter().any(|word| {
            matches!(
                *word,
                "count" | "len" | "index" | "next" | "slot" | "ordinal"
            )
        });
    let limit = words
        .iter()
        .any(|word| matches!(*word, "MAX" | "LIMIT" | "limit" | "capacity"));
    (u8::from(count) * COUNT) | (u8::from(logical) * LOGICAL) | (u8::from(limit) * LIMIT)
}

fn expression_key(expr: &syn::Expr) -> Option<String> {
    match expr {
        syn::Expr::Path(path) if path.path.segments.len() == 1 => Some(terminal(&path.path)),
        syn::Expr::Paren(paren) => expression_key(&paren.expr),
        syn::Expr::Group(group) => expression_key(&group.expr),
        syn::Expr::Call(call) if call.args.len() == 1 => {
            if let syn::Expr::Path(path) = &*call.func
                && terminal(&path.path) == "from"
                && path
                    .path
                    .segments
                    .iter()
                    .any(|part| matches!(part.ident.to_string().as_str(), "u64" | "u128"))
            {
                return expression_key(&call.args[0]);
            }
            None
        }
        _ => None,
    }
}

fn integer(expr: &syn::Expr) -> Option<u128> {
    match expr {
        syn::Expr::Lit(lit) => match &lit.lit {
            syn::Lit::Int(value) => value.base10_parse().ok(),
            _ => None,
        },
        syn::Expr::Path(path) => {
            let mut parts = path.path.segments.iter().rev();
            if parts.next()?.ident != "MAX" {
                return None;
            }
            match parts.next()?.ident.to_string().as_str() {
                "u32" => Some(u128::from(u32::MAX)),
                "u64" => Some(u128::from(u64::MAX)),
                "u128" => Some(u128::MAX),
                _ => None,
            }
        }
        syn::Expr::Paren(paren) => integer(&paren.expr),
        syn::Expr::Cast(cast) => integer(&cast.expr),
        syn::Expr::Call(call) if call.args.len() == 1 => {
            if let syn::Expr::Path(path) = &*call.func
                && terminal(&path.path) == "from"
                && path
                    .path
                    .segments
                    .iter()
                    .any(|part| matches!(part.ident.to_string().as_str(), "u32" | "u64" | "u128"))
            {
                return integer(&call.args[0]);
            }
            None
        }
        syn::Expr::Binary(binary) => match binary.op {
            syn::BinOp::Sub(_) => integer(&binary.left)?.checked_sub(integer(&binary.right)?),
            syn::BinOp::Add(_) => integer(&binary.left)?.checked_add(integer(&binary.right)?),
            syn::BinOp::Shl(_) => {
                integer(&binary.left)?.checked_shl(u32::try_from(integer(&binary.right)?).ok()?)
            }
            _ => None,
        },
        _ => None,
    }
}

#[derive(Default, Clone)]
struct Flow {
    locals: BTreeMap<String, u8>,
    calls: BTreeMap<String, Vec<String>>,
    slots: BTreeMap<String, u128>,
    widths: BTreeMap<String, u128>,
    nonzero: BTreeSet<String>,
    lengths: BTreeSet<String>,
    vectors: BTreeSet<String>,
    panic_boundary: bool,
}

struct Capacity<'a> {
    package: &'a str,
    file: &'a str,
    source: &'a str,
    names: Names,
    scope: Vec<String>,
    flow: Flow,
    hits: Vec<RuleHit>,
}

impl crate::rules::Scoped for Capacity<'_> {
    fn scope(&mut self) -> &mut Vec<String> {
        &mut self.scope
    }
}

impl Capacity<'_> {
    fn constant(&self, expression: &syn::Expr) -> Option<u128> {
        integer(expression).or_else(|| {
            let key = expression_key(expression)?;
            if self.flow.locals.contains_key(&key) || self.flow.widths.contains_key(&key) {
                return None;
            }
            self.names.constants.get(&key).copied().flatten()
        })
    }

    fn maximum(&self, expression: &syn::Expr) -> Option<u128> {
        if let Some(value) = self.constant(expression) {
            return Some(value);
        }
        if let Some(key) = expression_key(expression) {
            return self
                .flow
                .slots
                .get(&key)
                .and_then(|limit| limit.checked_sub(1))
                .or_else(|| self.flow.widths.get(&key).copied());
        }
        match expression {
            syn::Expr::Binary(binary)
                if matches!(binary.op, syn::BinOp::Sub(_))
                    && integer(&binary.right) == Some(1)
                    && expression_key(&binary.left)
                        .is_some_and(|key| self.flow.nonzero.contains(&key)) =>
            {
                self.maximum(&binary.left)?.checked_sub(1)
            }
            syn::Expr::MethodCall(call)
                if matches!(call.method.to_string().as_str(), "expect" | "unwrap") =>
            {
                let syn::Expr::Call(convert) = &*call.receiver else {
                    return None;
                };
                let syn::Expr::Path(path) = &*convert.func else {
                    return None;
                };
                let parts = self.path(&path.path);
                if parts.as_slice() == ["u32", "try_from"] && convert.args.len() == 1 {
                    return self
                        .maximum(&convert.args[0])
                        .filter(|value| *value <= u128::from(u32::MAX));
                }
                None
            }
            _ => None,
        }
    }
    fn hit(&mut self, span: Span, detail: &str) {
        let line = span.start().line;
        let symbol = crate::rules::scoped_symbol(self.file, &self.scope);
        if self
            .hits
            .iter()
            .any(|hit| hit.line == line && hit.symbol == symbol && hit.detail == detail)
        {
            return;
        }
        self.hits.push(RuleHit {
            rule: LOGICAL_CAPACITY_PANIC,
            package: self.package.to_owned(),
            file: self.file.to_owned(),
            line,
            symbol,
            detail: format!(
                "{detail} [{}]",
                crate::normalize::literal_fingerprint(&crate::source::relex(
                    self.source,
                    span,
                    false
                ))
            ),
            home_exempt: false,
        });
    }

    fn path(&self, path: &syn::Path) -> Vec<String> {
        path.segments
            .iter()
            .flat_map(|part| {
                self.names
                    .name(&part.ident.to_string())
                    .split("::")
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn call_facts(&self, call: &syn::ExprCall) -> u8 {
        let syn::Expr::Path(path) = &*call.func else {
            return 0;
        };
        let direct = self.path(&path.path);
        let parts = if direct.len() == 1 {
            self.flow.calls.get(&direct[0]).cloned().unwrap_or(direct)
        } else {
            direct
        };
        let Some(last) = parts.last() else { return 0 };
        if (parts.iter().any(|part| part == "CapacityError")
            && matches!(last.as_str(), "slot" | "check"))
            || self.names.capacity_results.contains(last)
        {
            return FALLIBLE | LOGICAL;
        }
        if is_mint(last) {
            // A literal below the smallest native niche ceiling cannot exhaust a table ID.
            if last == "from_index"
                && call
                    .args
                    .first()
                    .and_then(|value| self.maximum(value))
                    .is_some_and(|n| {
                        n < if parts.iter().any(|part| part == "GlobalTermId") {
                            u128::from(u64::MAX)
                        } else {
                            u128::from(u32::MAX)
                        }
                    })
            {
                return 0;
            }
            return FALLIBLE | LOGICAL;
        }
        if last == "try_from" && parts.iter().any(|part| part == "u32") {
            if call
                .args
                .first()
                .and_then(|value| self.maximum(value))
                .is_some_and(|value| value <= u128::from(u32::MAX))
            {
                return 0;
            }
            return FALLIBLE | LOGICAL;
        }
        if last == "new"
            && parts
                .iter()
                .any(|part| matches!(part.as_str(), "NonZeroU32" | "NonZeroU64"))
            && call
                .args
                .first()
                .is_some_and(|arg| matches!(arg, syn::Expr::Binary(_)))
        {
            let arg = &call.args[0];
            let limit = if parts.iter().any(|part| part == "NonZeroU32") {
                u128::from(u32::MAX)
            } else {
                u128::from(u64::MAX)
            };
            let proved = if let syn::Expr::Binary(add) = arg {
                matches!(add.op, syn::BinOp::Add(_))
                    && integer(&add.right) == Some(1)
                    && expression_key(&add.left)
                        .and_then(|key| self.flow.slots.get(&key).copied())
                        .is_some_and(|bound| bound <= limit)
            } else {
                false
            };
            return if proved { 0 } else { FALLIBLE | LOGICAL };
        }
        call.args
            .iter()
            .fold(0, |facts, arg| facts | self.facts(arg))
            & !FALLIBLE
    }

    fn facts(&self, expr: &syn::Expr) -> u8 {
        if integer(expr).is_some_and(|n| n >= u128::from(u32::MAX) - 1) {
            return LIMIT;
        }
        match expr {
            syn::Expr::Path(path) => {
                let name = terminal(&path.path);
                self.flow
                    .locals
                    .get(&name)
                    .copied()
                    .unwrap_or_else(|| count_name(&name))
            }
            syn::Expr::Field(field) => {
                self.facts(&field.base)
                    | match &field.member {
                        syn::Member::Named(name) => count_name(&name.to_string()),
                        syn::Member::Unnamed(_) => 0,
                    }
            }
            syn::Expr::Call(call) => self.call_facts(call),
            syn::Expr::MethodCall(call) => {
                let method = call.method.to_string();
                let receiver = self.facts(&call.receiver);
                if is_mint(&method) {
                    return FALLIBLE | LOGICAL;
                }
                if method == "len" {
                    return receiver | COUNT;
                }
                if matches!(method.as_str(), "checked_add" | "checked_sub")
                    && receiver & LOGICAL != 0
                {
                    return receiver | FALLIBLE;
                }
                if matches!(method.as_str(), "is_ok" | "is_err" | "is_some" | "is_none")
                    && receiver & FALLIBLE != 0
                {
                    return receiver | BOUNDARY;
                }
                if matches!(
                    method.as_str(),
                    "map" | "map_err" | "as_ref" | "copied" | "cloned"
                ) {
                    return receiver;
                }
                0
            }
            syn::Expr::Binary(binary) => {
                let left = self.facts(&binary.left);
                let right = self.facts(&binary.right);
                let comparison = matches!(
                    binary.op,
                    syn::BinOp::Lt(_)
                        | syn::BinOp::Le(_)
                        | syn::BinOp::Gt(_)
                        | syn::BinOp::Ge(_)
                        | syn::BinOp::Eq(_)
                        | syn::BinOp::Ne(_)
                );
                let boundary = comparison
                    && ((left & COUNT != 0 && right & LIMIT != 0)
                        || (right & COUNT != 0 && left & LIMIT != 0));
                left | right | (u8::from(boundary) * BOUNDARY)
            }
            syn::Expr::Lit(_) => {
                u8::from(integer(expr).is_some_and(|n| n >= u128::from(u32::MAX) - 1)) * LIMIT
            }
            syn::Expr::Paren(paren) => self.facts(&paren.expr),
            syn::Expr::Group(group) => self.facts(&group.expr),
            syn::Expr::Cast(cast) => self.facts(&cast.expr),
            syn::Expr::Reference(reference) => self.facts(&reference.expr),
            syn::Expr::Unary(unary) => self.facts(&unary.expr),
            _ => 0,
        }
    }

    fn proof(&mut self, expression: &syn::Expr) {
        if let syn::Expr::If(guard) = expression
            && guard.else_branch.is_none()
            && matches!(
                guard.then_branch.stmts.last(),
                Some(syn::Stmt::Expr(syn::Expr::Return(_), _))
            )
            && let syn::Expr::MethodCall(test) = &*guard.cond
            && test.method == "is_err"
            && test.args.is_empty()
            && let syn::Expr::Call(convert) = &*test.receiver
            && let syn::Expr::Path(path) = &*convert.func
            && self.path(&path.path).as_slice() == ["u32", "try_from"]
            && convert.args.len() == 1
            && let syn::Expr::MethodCall(length) = &convert.args[0]
            && length.method == "len"
            && length.args.is_empty()
            && let Some(key) = expression_key(&length.receiver)
            && self.flow.vectors.contains(&key)
        {
            self.flow.lengths.insert(key);
        }
        let syn::Expr::Try(checked) = expression else {
            return;
        };
        let syn::Expr::Call(call) = &*checked.expr else {
            return;
        };
        let syn::Expr::Path(path) = &*call.func else {
            return;
        };
        let parts = self.path(&path.path);
        if parts.last().is_none_or(|last| last != "slot")
            || !parts.iter().any(|part| part == "CapacityError")
            || call.args.len() != 3
        {
            return;
        }
        let Some(key) = expression_key(&call.args[1]) else {
            return;
        };
        let Some(bound) = integer(&call.args[2]).or_else(|| {
            expression_key(&call.args[2]).and_then(|key| self.flow.widths.get(&key).copied())
        }) else {
            return;
        };
        self.flow.slots.insert(key, bound);
    }

    fn body(&mut self, name: String, walk: impl FnOnce(&mut Self)) {
        let previous = std::mem::take(&mut self.flow);
        self.scoped(name, walk);
        self.flow = previous;
    }

    fn function(&mut self, signature: &syn::Signature, block: &syn::Block) {
        self.body(signature.ident.to_string(), |this| {
            for input in &signature.inputs {
                if let syn::FnArg::Typed(argument) = input
                    && let syn::Pat::Ident(name) = &*argument.pat
                    && let syn::Type::Path(ty) = &*argument.ty
                    && ty.path.segments.len() == 1
                {
                    let width = primitive_width(&ty.path);
                    if let Some(width) = width {
                        this.flow.widths.insert(name.ident.to_string(), width);
                    }
                }
            }
            this.visit_block(block);
        });
    }

    fn branch(&mut self, boundary: bool, walk: impl FnOnce(&mut Self)) {
        let previous = self.flow.clone();
        self.flow.panic_boundary |= boundary;
        walk(self);
        let after = std::mem::replace(&mut self.flow, previous);
        for (name, facts) in after.locals {
            *self.flow.locals.entry(name).or_default() |= facts;
        }
        self.flow
            .slots
            .retain(|key, value| after.slots.get(key) == Some(value));
        self.flow.nonzero.retain(|key| after.nonzero.contains(key));
        self.flow.lengths.retain(|key| after.lengths.contains(key));
    }

    fn binding_scope(&mut self, boundary: bool, walk: impl FnOnce(&mut Self)) {
        self.branch(boundary, |this| {
            this.flow.slots.clear();
            this.flow.widths.clear();
            this.flow.nonzero.clear();
            this.flow.lengths.clear();
            this.flow.vectors.clear();
            walk(this);
        });
    }

    fn macro_groups(&mut self, tokens: TokenStream) {
        for token in tokens {
            let TokenTree::Group(group) = token else {
                continue;
            };
            if group.delimiter() == Delimiter::Brace {
                let normalized = without_dollars(group.stream());
                let mut block = Group::new(Delimiter::Brace, normalized.clone());
                block.set_span(group.span());
                if let Ok(parsed) =
                    syn::parse2::<syn::Block>(TokenStream::from(TokenTree::Group(block)))
                {
                    self.branch(false, |this| this.visit_block(&parsed));
                    continue;
                }
                // Opaque repetitions must not silently hide an apparent logical panic.
                let words = token_words(normalized);
                let panic = words.iter().any(|word| {
                    matches!(
                        word.as_str(),
                        "expect"
                            | "unwrap"
                            | "panic"
                            | "assert"
                            | "assert_eq"
                            | "assert_ne"
                            | "debug_assert"
                            | "debug_assert_eq"
                            | "debug_assert_ne"
                    )
                });
                let logical = words
                    .iter()
                    .any(|word| word == "CapacityError" || is_mint(word))
                    || words.iter().any(|word| word == "u32")
                        && words.iter().any(|word| word == "try_from" || word == "MAX")
                    || words.iter().any(|word| count_name(word) & LOGICAL != 0)
                        && words.iter().any(|word| count_name(word) & LIMIT != 0);
                if panic && logical {
                    self.hit(group.span(), "unanalysed macro transcriber contains a logical-capacity panic; expose the checked operation");
                }
            }
            self.macro_groups(group.stream());
        }
    }
}

fn without_dollars(tokens: TokenStream) -> TokenStream {
    tokens
        .into_iter()
        .filter_map(|token| match token {
            TokenTree::Punct(punct) if punct.as_char() == '$' => None,
            TokenTree::Group(group) => {
                let mut replacement =
                    Group::new(group.delimiter(), without_dollars(group.stream()));
                replacement.set_span(group.span());
                Some(TokenTree::Group(replacement))
            }
            other => Some(other),
        })
        .collect()
}

fn token_words(tokens: TokenStream) -> Vec<String> {
    let mut words = Vec::new();
    for token in tokens {
        match token {
            TokenTree::Ident(ident) => words.push(ident.to_string()),
            TokenTree::Group(group) => words.extend(token_words(group.stream())),
            _ => {}
        }
    }
    words
}

impl<'ast> Visit<'ast> for Capacity<'_> {
    fn visit_block(&mut self, node: &'ast syn::Block) {
        let before = self.flow.clone();
        for statement in &node.stmts {
            self.visit_stmt(statement);
            if let syn::Stmt::Expr(expr, _) = statement {
                self.proof(expr);
            }
        }
        // Block locals cannot export a proof. Retain an outer proof only if
        // every visited mutation left it intact; taint is conservatively merged.
        self.flow
            .slots
            .retain(|key, value| before.slots.get(key) == Some(value));
        self.flow.nonzero.retain(|key| before.nonzero.contains(key));
        self.flow.lengths.retain(|key| before.lengths.contains(key));
        self.flow.widths = before.widths;
        self.flow.vectors = before.vectors;
        for (name, facts) in before.locals {
            *self.flow.locals.entry(name).or_default() |= facts;
        }
    }

    fn visit_local(&mut self, node: &'ast syn::Local) {
        if let Some(init) = &node.init {
            self.visit_expr(&init.expr);
            let pattern = match &node.pat {
                syn::Pat::Type(typed) => &*typed.pat,
                other => other,
            };
            if let syn::Pat::Ident(binding) = pattern {
                let name = binding.ident.to_string();
                self.flow.slots.remove(&name);
                self.flow.widths.remove(&name);
                self.flow.nonzero.remove(&name);
                self.flow.lengths.remove(&name);
                self.flow.vectors.remove(&name);
                self.flow.calls.remove(&name);
                if let syn::Expr::Path(path) = &*init.expr {
                    let direct = self.path(&path.path);
                    let target = if direct.len() == 1 {
                        self.flow.calls.get(&direct[0]).cloned().unwrap_or(direct)
                    } else {
                        direct
                    };
                    self.flow.calls.insert(name.clone(), target);
                }
                let facts = self.facts(&init.expr) | count_name(&name);
                self.flow.locals.insert(name.clone(), facts);
                if let syn::Pat::Type(typed) = &node.pat
                    && let syn::Type::Path(ty) = &*typed.ty
                    && let Some(width) = primitive_width(&ty.path)
                {
                    self.flow.widths.insert(name.clone(), width);
                }
                if let syn::Pat::Type(typed) = &node.pat
                    && let syn::Type::Path(ty) = &*typed.ty
                    && self.path(&ty.path).as_slice() == ["Vec"]
                {
                    self.flow.vectors.insert(name);
                }
            } else {
                // Destructuring can shadow either the checked index or limit.
                self.flow.slots.clear();
                self.flow.widths.clear();
                self.flow.nonzero.clear();
                self.flow.lengths.clear();
                self.flow.vectors.clear();
            }
            if let Some((_, diverge)) = &init.diverge {
                self.branch(self.facts(&init.expr) & FALLIBLE != 0, |this| {
                    this.visit_expr(diverge);
                });
            }
        }
    }

    fn visit_expr_reference(&mut self, node: &'ast syn::ExprReference) {
        if node.mutability.is_some() {
            self.flow.slots.clear();
            self.flow.nonzero.clear();
            self.flow.lengths.clear();
        }
        syn::visit::visit_expr_reference(self, node);
    }

    fn visit_expr_let(&mut self, node: &'ast syn::ExprLet) {
        self.flow.slots.clear();
        self.flow.widths.clear();
        self.flow.nonzero.clear();
        self.flow.lengths.clear();
        self.flow.vectors.clear();
        syn::visit::visit_expr_let(self, node);
    }

    fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
        self.binding_scope(false, |this| {
            syn::visit::visit_expr_closure(this, node);
        });
    }

    fn visit_expr_for_loop(&mut self, node: &'ast syn::ExprForLoop) {
        self.binding_scope(false, |this| {
            syn::visit::visit_expr_for_loop(this, node);
        });
    }

    fn visit_expr_assign(&mut self, node: &'ast syn::ExprAssign) {
        self.flow.slots.clear();
        self.flow.calls.clear();
        self.flow.nonzero.clear();
        self.flow.lengths.clear();
        if let Some(name) = expression_key(&node.left) {
            self.flow.locals.insert(name, self.facts(&node.right));
        }
        syn::visit::visit_expr_assign(self, node);
    }

    fn visit_expr_binary(&mut self, node: &'ast syn::ExprBinary) {
        if matches!(
            node.op,
            syn::BinOp::AddAssign(_)
                | syn::BinOp::SubAssign(_)
                | syn::BinOp::MulAssign(_)
                | syn::BinOp::DivAssign(_)
                | syn::BinOp::RemAssign(_)
                | syn::BinOp::BitOrAssign(_)
                | syn::BinOp::BitAndAssign(_)
                | syn::BinOp::BitXorAssign(_)
                | syn::BinOp::ShlAssign(_)
                | syn::BinOp::ShrAssign(_)
        ) {
            self.flow.slots.clear();
            self.flow.nonzero.clear();
            self.flow.lengths.clear();
        }
        syn::visit::visit_expr_binary(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        // Only a concrete Vec with its unchanged checked length can transfer
        // an ordinal proof through the standard iter().enumerate().map shape.
        if node.method == "map"
            && node.args.len() == 1
            && let syn::Expr::MethodCall(enumerate) = &*node.receiver
            && enumerate.method == "enumerate"
            && enumerate.args.is_empty()
            && let syn::Expr::MethodCall(iter) = &*enumerate.receiver
            && iter.method == "iter"
            && iter.args.is_empty()
            && expression_key(&iter.receiver).is_some_and(|key| self.flow.lengths.contains(&key))
            && let syn::Expr::Closure(closure) = &node.args[0]
            && closure.inputs.len() == 1
            && let syn::Pat::Tuple(bindings) = &closure.inputs[0]
            && let Some(syn::Pat::Ident(index)) = bindings.elems.first()
        {
            self.visit_expr(&node.receiver);
            self.binding_scope(false, |this| {
                this.flow
                    .slots
                    .insert(index.ident.to_string(), u128::from(u32::MAX));
                this.visit_expr(&closure.body);
            });
            return;
        }
        if let Some(key) = expression_key(&node.receiver) {
            // A method can borrow its receiver mutably without an explicit
            // `&mut` expression (for example primitive Clone::clone_from).
            self.flow.slots.remove(&key);
            self.flow.nonzero.remove(&key);
            if !matches!(node.method.to_string().as_str(), "iter" | "len") {
                self.flow.lengths.remove(&key);
            }
        }
        if matches!(
            node.method.to_string().as_str(),
            "expect" | "unwrap" | "unwrap_or_else"
        ) && self.facts(&node.receiver) & FALLIBLE != 0
        {
            let panic = node.method != "unwrap_or_else" || token_free_panics(&node.args);
            if panic {
                self.hit(
                    node.span(),
                    "logical-capacity refusal is converted to panic; propagate the typed error",
                );
            }
        }
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = &*node.func
            && matches!(terminal(&path.path).as_str(), "expect" | "unwrap")
            && node
                .args
                .first()
                .is_some_and(|arg| self.facts(arg) & FALLIBLE != 0)
        {
            self.hit(
                node.span(),
                "logical-capacity refusal is converted to panic; propagate the typed error",
            );
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_if(&mut self, node: &'ast syn::ExprIf) {
        self.visit_expr(&node.cond);
        let boundary = self.facts(&node.cond) & BOUNDARY != 0;
        let nonzero = if let syn::Expr::Binary(condition) = &*node.cond
            && integer(&condition.right) == Some(0)
            && matches!(condition.op, syn::BinOp::Eq(_) | syn::BinOp::Ne(_))
        {
            expression_key(&condition.left)
                .map(|key| (key, matches!(condition.op, syn::BinOp::Ne(_))))
        } else {
            None
        };
        self.branch(boundary, |this| {
            if let Some((key, true)) = &nonzero {
                this.flow.nonzero.insert(key.clone());
            }
            this.visit_block(&node.then_branch);
        });
        if let Some((_, other)) = &node.else_branch {
            self.branch(boundary, |this| {
                if let Some((key, false)) = &nonzero {
                    this.flow.nonzero.insert(key.clone());
                }
                this.visit_expr(other);
            });
        }
        // A branch may mutate a previously checked index; do not export its proof.
        self.flow.slots.clear();
        self.flow.nonzero.clear();
    }

    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        self.visit_expr(&node.expr);
        let boundary = self.facts(&node.expr) & FALLIBLE != 0;
        for arm in &node.arms {
            self.binding_scope(boundary, |this| this.visit_arm(arm));
        }
        self.flow.slots.clear();
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let name = terminal(&node.path);
        let arguments = node.parse_body_with(
            syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated,
        );
        if matches!(
            name.as_str(),
            "assert"
                | "assert_eq"
                | "assert_ne"
                | "debug_assert"
                | "debug_assert_eq"
                | "debug_assert_ne"
        ) && let Ok(args) = &arguments
        {
            let flags = args
                .iter()
                .take(if name.ends_with("_eq") || name.ends_with("_ne") {
                    2
                } else {
                    1
                })
                .fold(0, |flags, arg| flags | self.facts(arg));
            if flags & BOUNDARY != 0 || flags & COUNT != 0 && flags & LIMIT != 0 {
                self.hit(
                    node.span(),
                    "runtime table capacity is asserted; return the named capacity error",
                );
            }
        }
        if matches!(name.as_str(), "panic" | "unreachable") && self.flow.panic_boundary {
            self.hit(
                node.span(),
                "logical-capacity error branch panics instead of returning its typed cause",
            );
        }
        // An opaque macro can mutate a captured binding without spelling an
        // assignment at this call site. No local proof crosses that boundary.
        self.flow.slots.clear();
        self.flow.nonzero.clear();
        self.flow.lengths.clear();
        crate::rules::visit_macro_arguments(self, node);
    }

    fn visit_item_macro(&mut self, node: &'ast syn::ItemMacro) {
        if crate::source::is_test_only(&node.attrs) {
            return;
        }
        if node.mac.path.is_ident("macro_rules") {
            self.scoped(
                node.ident
                    .as_ref()
                    .map_or_else(String::new, ToString::to_string),
                |this| this.macro_groups(node.mac.tokens.clone()),
            );
        } else {
            self.visit_macro(&node.mac);
        }
    }

    crate::rules::shipping_visits!('ast;
        [visit_item_fn(syn::ItemFn), visit_impl_item_fn(syn::ImplItemFn)] => |this, node| {
            this.function(&node.sig, &node.block);
        };
    );
    fn visit_trait_item_fn(&mut self, node: &'ast syn::TraitItemFn) {
        if !crate::source::is_test_only(&node.attrs)
            && let Some(block) = &node.default
        {
            self.function(&node.sig, block);
        }
    }
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if !crate::source::is_test_only(&node.attrs) {
            self.scoped(node.ident.to_string(), |this| {
                syn::visit::visit_item_mod(this, node);
            });
        }
    }
    crate::rules::shipping_visits!('ast;
        visit_item_impl(syn::ItemImpl);
        visit_item_trait(syn::ItemTrait);
    );
    fn visit_item_const(&mut self, node: &'ast syn::ItemConst) {
        if !crate::source::is_test_only(&node.attrs) {
            self.body(node.ident.to_string(), |this| this.visit_expr(&node.expr));
        }
    }
}

fn token_free_panics(arguments: &syn::punctuated::Punctuated<syn::Expr, syn::Token![,]>) -> bool {
    struct Panics(bool);
    impl<'ast> Visit<'ast> for Panics {
        fn visit_macro(&mut self, node: &'ast syn::Macro) {
            self.0 |= matches!(terminal(&node.path).as_str(), "panic" | "unreachable");
            crate::rules::visit_macro_arguments(self, node);
        }
    }
    let mut visitor = Panics(false);
    for argument in arguments {
        visitor.visit_expr(argument);
    }
    visitor.0
}

/// Read an already parsed file reached from a shipping crate root.
pub(crate) fn check(package: &str, file: &str, source: &str, parsed: &syn::File) -> Vec<RuleHit> {
    if !matches!(package, "purrdf-core" | "purrdf-datalog") {
        return Vec::new();
    }
    let mut names = Names::default();
    names.visit_file(parsed);
    let mut visitor = Capacity {
        package,
        file,
        source,
        names,
        scope: Vec::new(),
        flow: Flow::default(),
        hits: Vec::new(),
    };
    visitor.visit_file(parsed);
    visitor.hits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{Memory, Workspace};
    use std::fmt::Write as _;

    fn hits(source: &str) -> Vec<RuleHit> {
        check(
            "purrdf-core",
            "crates/rdf-core/src/lib.rs",
            source,
            &syn::parse_file(source).unwrap(),
        )
    }

    #[test]
    fn fixed_constants_and_nonzero_width_proofs_keep_invalid_neighbours() {
        for source in [
            "const SIZE:u64=40; fn f(){u32::try_from(SIZE).expect(\"fixed\");}",
            "fn f(graph:u64){if graph==0 {None} else {Some(GlobalTermId::from_index(graph-1).unwrap())};}",
            "fn f(){let graph:u64=read()?;if graph!=0 {GlobalTermId::from_index(graph-1).unwrap();}}",
        ] {
            assert_eq!(hits(source), [], "{source}");
        }
        for source in [
            "const SIZE:u64=4294967296; fn f(){u32::try_from(SIZE).expect(\"fixed\");}",
            "const SIZE:u64=40; fn f(){const SIZE:u64=4294967296;u32::try_from(SIZE).unwrap();}",
            "fn f(graph:u64){GlobalTermId::from_index(graph-1).unwrap();}",
            "fn f(graph:u128){if graph!=0 {GlobalTermId::from_index(graph-1).unwrap();}}",
            "fn f(mut graph:u64){if graph!=0 {graph=0;GlobalTermId::from_index(graph-1).unwrap();}}",
            "fn f(graph:u64){if graph!=0 {let graph=0;GlobalTermId::from_index(graph-1).unwrap();}}",
            "fn f(graph:u64){if graph!=0 {(||{let graph=0;GlobalTermId::from_index(graph-1).unwrap();})();}}",
            "fn f(graph:u128){{let graph:u64=read();}if graph!=0{GlobalTermId::from_index(graph-1).unwrap();}}",
        ] {
            assert_ne!(hits(source), [], "{source}");
        }
    }

    #[test]
    fn checked_concrete_vector_enumeration_does_not_prove_foreign_counts() {
        let template = "fn f()->Result<(),Error>{let ids:Vec<Id>=input();if u32::try_from(ids.len()).is_err(){return Err(error());}REPLACEMENT ids.iter().enumerate().map(|(local,id)|TermId::from_index(u32::try_from(local).expect(\"width\")).expect(\"niche\")).collect();Ok(())}";
        assert_eq!(hits(&template.replace("REPLACEMENT", "")), []);
        for mutation in [
            "ids.push(id);",
            "let ids=foreign_view();",
            "mutate!();",
            "mutate(&mut ids);",
        ] {
            assert_ne!(
                hits(&template.replace("REPLACEMENT", mutation)),
                [],
                "{mutation}"
            );
        }
        assert_ne!(
            hits(
                &template
                    .replace("let ids:Vec<Id>=input();", "let ids:DatasetView=input();")
                    .replace("REPLACEMENT", "")
            ),
            []
        );
        assert_ne!(
            hits(
                &template
                    .replace("REPLACEMENT", "")
                    .replace(
                        "|(local,id)|TermId",
                        "|(local,id)|{let local=u32::MAX;TermId"
                    )
                    .replace(
                        ".expect(\"niche\")).collect();",
                        ".expect(\"niche\")}).collect();"
                    )
            ),
            []
        );
    }

    #[test]
    fn reconstruction_proofs_fail_closed_on_source_and_constructor_changes() {
        let tree = Memory { files: [
            ("crates/rdf-core/Cargo.toml", "[package]\nname=\"purrdf-core\"\n"),
            ("crates/rdf-core/src/lib.rs", "mod ir;"),
            ("crates/rdf-core/src/ir/mod.rs", "mod builder;mod dataset;mod term;mod frozen_terms;mod sibling;#[path=\"../mounted_owner.rs\"] mod mounted_owner;"),
            ("crates/rdf-core/src/ir/builder.rs", "fn mint()->Result<(),Error>{CapacityError::slot(space,count,u32::MAX)?;RdfDataset::from_parts(terms);Ok(())}"),
            ("crates/rdf-core/src/ir/dataset.rs", "struct RdfDataset {terms:Vec<Term>} impl RdfDataset {pub(crate) fn from_parts(terms:Vec<Term>)->Self{Self{terms}} fn read(&self){TermId::from_index(index).expect(\"stored\");}}"),
            ("crates/rdf-core/src/ir/term.rs", "struct TermId(u32);"),
            ("crates/rdf-core/src/ir/frozen_terms.rs", "pub(super) struct FrozenTerms {pub(super) index:Vec<u32>}"),
            ("crates/rdf-core/src/ir/sibling.rs", "fn untouched(){}"),
            ("crates/rdf-core/src/mounted_owner.rs", "fn untouched(){}"),
            ("crates/lex/Cargo.toml", "[package]\nname=\"purrdf-lex\"\n"),
            ("crates/lex/src/lib.rs", "pub mod allocation;"),
            ("crates/lex/src/allocation.rs", "pub struct CapacityError;impl CapacityError{pub fn slot(){}}"),
        ].into_iter().map(|(path, text)| (path.to_owned(), text.to_owned())).collect() };
        let workspace = Workspace::walk(&tree);
        let (index, hit) = workspace
            .rule_hits
            .iter()
            .enumerate()
            .find(|(_, hit)| hit.rule == LOGICAL_CAPACITY_PANIC)
            .unwrap();
        let row = Proof {
            file: hit.file.clone(),
            symbol: hit.symbol.clone(),
            line: hit.line,
            sink: hit
                .detail
                .rsplit_once('[')
                .unwrap()
                .1
                .trim_end_matches(']')
                .to_owned(),
            class: "concrete-frozen-term-range".to_owned(),
            authorities: [
                "crates/rdf-core/src/ir/builder.rs",
                "crates/rdf-core/src/ir/dataset.rs",
                "crates/rdf-core/src/ir/frozen_terms.rs",
                "crates/rdf-core/src/ir/term.rs",
                "crates/lex/src/allocation.rs",
            ]
            .into_iter()
            .map(|path| format!("{path}={}", workspace.capacity_sources[path]))
            .collect(),
            constructors: constructor_identity(&workspace),
        };
        assert_eq!(
            proofs(std::slice::from_ref(&row), &workspace),
            (BTreeSet::from([index]), Vec::new())
        );
        let mut missing_owner = row.clone();
        missing_owner
            .authorities
            .retain(|authority| !authority.starts_with("crates/rdf-core/src/ir/frozen_terms.rs="));
        assert_ne!(
            proofs(&[missing_owner], &workspace).1,
            [] as [String; 0],
            "the constructor aggregate alone cannot replace the explicit frozen owner authority"
        );
        for file in [
            "crates/rdf-core/src/ir/sibling.rs",
            "crates/rdf-core/src/mounted_owner.rs",
        ] {
            assert!(workspace.capacity_constructors.contains(file));
            let mut changed = Memory {
                files: tree.files.clone(),
            };
            // No from_parts token or opaque expansion: the actual module
            // visibility alone permits this unvalidated writer.
            changed.files.insert(
                file.to_owned(),
                "fn mutate(owner:&mut super::frozen_terms::FrozenTerms){owner.index.push(u32::MAX);}"
                    .to_owned(),
            );
            assert_ne!(
                proofs(std::slice::from_ref(&row), &Workspace::walk(&changed)).1,
                [] as [String; 0],
                "unreviewed visible owner writer: {file}"
            );
        }
        for (mount, file) in [
            ("../../../../outside_owner.rs", "outside_owner.rs"),
            ("../../tests/owner.rs", "crates/rdf-core/tests/owner.rs"),
            (
                "../../generated/owner.rs",
                "crates/rdf-core/generated/owner.rs",
            ),
        ] {
            for test_only in [false, true] {
                let mut changed = Memory {
                    files: tree.files.clone(),
                };
                let cfg = if test_only { "#[cfg(test)]" } else { "" };
                write!(
                    changed
                        .files
                        .get_mut("crates/rdf-core/src/ir/mod.rs")
                        .unwrap(),
                    "{cfg} #[path=\"{mount}\"] mod escaped_owner;"
                )
                .unwrap();
                changed.files.insert(
                    file.to_owned(),
                    "fn mutate(owner:&mut super::frozen_terms::FrozenTerms){owner.index.push(u32::MAX);}"
                        .to_owned(),
                );
                let changed = Workspace::walk(&changed);
                assert_eq!(
                    changed.errors.iter().any(|error| error == &format!(
                        "{file}: shipping frozen-owner module purrdf_core::ir::escaped_owner is outside the audited source domain"
                    )),
                    !test_only,
                    "mounted owner path {mount}, test-only={test_only}"
                );
            }
        }
        let mut duplicate = vec![row.clone(), row.clone()];
        assert_ne!(proofs(&duplicate, &workspace).1, [] as [String; 0]);
        duplicate.truncate(1);
        duplicate[0].authorities.clear();
        assert_ne!(proofs(&duplicate, &workspace).1, [] as [String; 0]);
        let ledger = crate::ledger::Ledger {
            jobs: vec![crate::ledger::Job {
                id: "logical-capacity-panic".to_owned(),
                home: "purrdf_lex::allocation::CapacityError::slot".to_owned(),
                entry_points: Vec::new(),
                enforced: true,
                forbidden: crate::ledger::Forbidden {
                    fingerprints: vec![LOGICAL_CAPACITY_PANIC.to_owned()],
                    ..Default::default()
                },
                variants: Vec::new(),
                capacity_proofs: duplicate,
            }],
            distinct: Vec::new(),
            document: BTreeMap::new(),
        };
        assert_ne!(
            crate::census::check_findings(&workspace, &ledger),
            [] as [String; 0]
        );
        assert_ne!(
            crate::census::index(&workspace, &ledger)["errors"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
        for (path, from, to) in [
            ("crates/rdf-core/src/ir/builder.rs", "u32::MAX", "u64::MAX"),
            (
                "crates/rdf-core/src/ir/builder.rs",
                "Ok(())",
                "unchecked_mutation();Ok(())",
            ),
            (
                "crates/rdf-core/src/ir/dataset.rs",
                "terms:Vec<Term>",
                "pub(crate) terms:Vec<Term>",
            ),
            ("crates/rdf-core/src/ir/dataset.rs", "fn read", "fn moved"),
            ("crates/rdf-core/src/ir/dataset.rs", "fn read", "\nfn read"),
            (
                "crates/rdf-core/src/ir/dataset.rs",
                "TermId::from_index(index).expect(\"stored\");",
                "",
            ),
            (
                "crates/rdf-core/src/ir/dataset.rs",
                "TermId::from_index(index).expect(\"stored\");",
                "TermId::from_index(index).expect(\"stored\");TermId::from_index(index).expect(\"stored\");",
            ),
        ] {
            let mut changed = Memory {
                files: tree.files.clone(),
            };
            changed
                .files
                .insert(path.to_owned(), changed.files[path].replacen(from, to, 1));
            assert_ne!(
                proofs(std::slice::from_ref(&row), &Workspace::walk(&changed)).1,
                [] as [String; 0],
                "{path}: {to}"
            );
        }
        for source in [
            "fn new_ingress(){RdfDataset::from_parts(unvalidated());}",
            "use crate::ir::dataset::RdfDataset::from_parts as ctor;fn new_ingress(){ctor(unvalidated());}",
            "macro_rules! construct {()=>{RdfDataset::from_parts(unvalidated())};}",
            "fn new_ingress(){opaque_constructor!();}",
            "#[external_constructor] struct Expansion;",
        ] {
            let mut changed = Memory {
                files: tree.files.clone(),
            };
            changed
                .files
                .get_mut("crates/rdf-core/src/lib.rs")
                .unwrap()
                .push_str("mod new_ingress;");
            changed.files.insert(
                "crates/rdf-core/src/new_ingress.rs".to_owned(),
                source.to_owned(),
            );
            assert_ne!(
                proofs(std::slice::from_ref(&row), &Workspace::walk(&changed)).1,
                [] as [String; 0],
                "{source}"
            );
        }
        let mut foreign = tree;
        foreign
            .files
            .get_mut("crates/rdf-core/src/lib.rs")
            .unwrap()
            .push_str("mod foreign;");
        foreign.files.insert("crates/rdf-core/src/foreign.rs".to_owned(), "fn read(view:&dyn DatasetView){for index in 0..view.term_count(){TermId::from_index(index).expect(\"stored\");}}".to_owned());
        let foreign = Workspace::walk(&foreign);
        let (accepted, errors) = proofs(&[row], &foreign);
        assert_eq!(errors, [] as [String; 0]);
        assert!(
            foreign
                .rule_hits
                .iter()
                .enumerate()
                .any(|(index, hit)| hit.rule == LOGICAL_CAPACITY_PANIC
                    && !accepted.contains(&index)
                    && hit.file.ends_with("foreign.rs"))
        );
    }

    #[test]
    fn original_producer_shapes_and_csr_are_refused_without_message_matching() {
        for body in [
            "TermId::from_index(index).expect(\"x\")",
            "u32::try_from(arena.len()).unwrap()",
            "assert!(quads.len() < u32::MAX as usize)",
            "assert!(u32::MAX as usize >= annotations.len())",
            "debug_assert!(reifiers.len() <= 4294967295)",
            "assert!(quads.len() < (1_u64 << 32))",
            "u32::try_from(permutation.len()).expect(\"invariant\")",
            "delta.intern(term).unwrap()",
            "next_merge_scope.checked_add(1).unwrap()",
            "next_blank_scope.checked_add(1).expect(\"safe\")",
            "intern_origin(x).unwrap()",
            "register_unit(x).unwrap()",
            "register_artifact(x).unwrap()",
            "intern_value_with_memory(builder,value,memory).expect(\"fixture\")",
            "builder.push_owned_quad_scoped_with_memory(quad,scope,memory).unwrap()",
            "graph_membership_corpus(tag,graphs).expect(\"authored\")",
            "Id::from_index(index).unwrap()",
            "u32::try_from(postings.len()).expect(\"checked\")",
        ] {
            assert!(!hits(&format!("fn f() {{ {body}; }}")).is_empty(), "{body}");
        }
    }

    #[test]
    fn typed_aliases_locals_error_branches_and_macros_are_seen() {
        for source in [
            "use purrdf_lex::allocation::CapacityError as C; fn f(){ let answer=C::slot(s,n,l); let second=answer; second.expect(\"x\"); }",
            "use purrdf_lex::allocation::CapacityError::slot as reserve; fn f(){reserve(s,n,u32::MAX).expect(\"x\");}",
            "use purrdf_lex::{allocation::{CapacityError::{slot as reserve}}}; fn f(){reserve(s,n,u32::MAX).unwrap();}",
            "use purrdf_lex::allocation::CapacityError::slot; fn f(){slot(s,n,u32::MAX).unwrap();}",
            "fn mint()->Result<u32,CapacityError>{todo!()} fn f(){mint().map_err(convert).unwrap();}",
            "use purrdf_lex::allocation::CapacityError as C; fn mint()->Result<u32,C>{todo!()} fn f(){mint().unwrap();}",
            "fn f(){ let answer=CapacityError::check(s,n,l); Result::unwrap(answer); }",
            "fn f(){ let mint=CapacityError::slot; let alias=mint; alias(s,n,l).unwrap(); }",
            "fn f(){ let mut answer=Ok(()); if flag {answer=CapacityError::check(s,n,l);} answer.unwrap(); }",
            "fn f(){ if rows.len()>u32::MAX as usize {panic!(\"x\");} }",
            "fn f(){match CapacityError::slot(s,n,l){Ok(n)=>n,Err(_)=>panic!(\"x\")};}",
            "fn f(){ CapacityError::slot(s,n,l).unwrap_or_else(|_| panic!(\"x\")); }",
            "fn f(){ let limit=u32::MAX as usize; let n=quads.len(); std::assert!(n<=limit); }",
            "fn f(){ let limit=2; let n=quads.len(); std::assert!(n<=limit); }",
            "macro_rules! mint { ($n:expr) => {{ TermId::from_index($n).unwrap() }}; }",
            "macro_rules! mint { ($($n:expr),*) => {$(TermId::from_index($n).unwrap();)*}; }",
            "trait T { fn f(){u32::try_from(rows.len()).unwrap();} }",
        ] {
            assert!(!hits(source).is_empty(), "{source}");
        }
    }

    #[test]
    fn dominating_slot_proof_is_operand_and_order_sensitive() {
        let safe = "fn f(index:u32)->Result<(),CapacityError>{ CapacityError::slot(s,u64::from(index),u64::from(u32::MAX))?; NonZeroU32::new(index+1).expect(\"representation\"); Ok(()) }";
        assert_eq!(hits(safe), []);
        let typed_limit = "fn f(index:u64,limit:u64)->Result<(),CapacityError>{CapacityError::slot(s,index,limit)?;NonZeroU64::new(index+1).expect(\"representation\");Ok(())}";
        assert_eq!(hits(typed_limit), []);
        assert_ne!(hits(&typed_limit.replace("limit:u64", "limit:u128")), []);
        assert_ne!(hits(&typed_limit.replace("limit:u64", "limit:Unknown")), []);
        assert_ne!(
            hits(&typed_limit.replace(
                "CapacityError::slot",
                "let limit=dynamic_u128;CapacityError::slot"
            )),
            []
        );
        for operation in ["+=", "-=", "*=", "/=", "%=", "|=", "&=", "^=", "<<=", ">>="] {
            let changed = safe.replace(
                "NonZeroU32::new",
                &format!("index {operation} 1; NonZeroU32::new"),
            );
            assert!(!hits(&changed).is_empty(), "{changed}");
        }
        for source in [
            safe.replace("u64::from(u32::MAX)", "unknown_limit"),
            safe.replace("u64::from(u32::MAX)", "u64::MAX"),
            safe.replace("u64::from(u32::MAX)", "u128::MAX")
                .replace("NonZeroU32", "NonZeroU64"),
            safe.replace("u64::from(u32::MAX)", "dynamic_u128")
                .replace("NonZeroU32", "NonZeroU64"),
            safe.replace(
                "CapacityError::slot(s,u64::from(index),u64::from(u32::MAX))?;",
                "",
            ),
            safe.replace("u64::from(index)", "u64::from(other)"),
            safe.replace("NonZeroU32::new", "index += 1; NonZeroU32::new"),
            safe.replace("NonZeroU32::new", "change(&mut index); NonZeroU32::new"),
            safe.replace("index:u32", "mut index:u32").replace(
                "NonZeroU32::new",
                "index.clone_from(&u32::MAX); NonZeroU32::new",
            ),
            "fn f(mut graph:u64){if graph!=0{graph.clone_from(&0);GlobalTermId::from_index(graph-1).expect(\"checked nonzero\");}}".to_owned(),
            safe.replace("u64::from(index)", "(index as u8) as u64"),
            safe.replace(
                "CapacityError::slot(s,u64::from(index),u64::from(u32::MAX))?;",
                "if flag { CapacityError::slot(s,u64::from(index),u64::from(u32::MAX))?; }",
            ),
        ] {
            assert!(!hits(&source).is_empty(), "{source}");
        }
    }

    #[test]
    fn physical_and_fixed_invariants_and_typed_propagation_are_not_capacity_panics() {
        let source = r#"
fn checked(index:u64)->Result<(),CapacityError>{CapacityError::slot(s,index,u64::MAX)?;NonZeroU64::new(index+1).expect("nonzero");Ok(())}
fn physical(len:usize,additional:usize){len.checked_add(additional).expect("capacity overflow");table.try_reserve(1).expect("capacity allocation");}
fn invariant(){assert!(size_of::<TermId>()==4);ids.map(|id|id.expect("populated"));u64::try_from(items.len()).expect("resident length");}
fn typed()->Result<(),CapacityError>{CapacityError::check(s,n,l)?;Ok(())}
use unrelated::slot as reserve;
fn unrelated_slot(){reserve(s,n,l).unwrap();}
fn ordinary(){assert!(valid);TermId::from_index(0).expect("zero");let _="quads.len().unwrap()";}
#[cfg(test)] mod tests{fn hidden(){TermId::from_index(n).unwrap();}}
"#;
        assert_eq!(hits(source), []);
    }

    #[test]
    fn checked_index_proofs_do_not_certify_shadowing_bindings() {
        for body in [
            "let f=|index:u32| NonZeroU32::new(index+1).unwrap();",
            "for index in values {NonZeroU32::new(index+1).unwrap();}",
            "if let Some(index)=value {NonZeroU32::new(index+1).unwrap();}",
            "match value {index=>{NonZeroU32::new(index+1).unwrap();}}",
            "let (index,)=value; NonZeroU32::new(index+1).unwrap();",
        ] {
            let source = format!(
                "fn f(index:u32)->Result<(),CapacityError>{{CapacityError::slot(s,u64::from(index),u64::from(u32::MAX))?;{body}Ok(())}}"
            );
            assert!(!hits(&source).is_empty(), "{source}");
        }
        let source = "fn f(limit:u64){let g=|limit:u128| {CapacityError::slot(s,index,limit)?;NonZeroU64::new(index+1).unwrap();};}";
        assert_ne!(hits(source), []);
    }

    #[test]
    fn shipping_function_and_impl_declarations_preserve_cfg_reachability() {
        let source = "\
#[cfg(test)] fn hidden(){TermId::from_index(n).unwrap();}
#[cfg(test)] impl Test {fn hidden(){TermId::from_index(n).unwrap();}}
impl Production {#[cfg(test)] fn hidden(){TermId::from_index(n).unwrap();}}
#[cfg(any(test,target_arch=\"wasm32\"))] impl Portable {fn exposed(){TermId::from_index(n).unwrap();}}
fn exposed(){TermId::from_index(n).unwrap();}
";
        let found = hits(source);
        assert_eq!(found.len(), 2);
        assert!(found.iter().all(|hit| hit.symbol.ends_with("::exposed")));
    }

    #[test]
    fn shipping_reachability_and_gate_projection_are_enforced() {
        let mut tree = Memory::default();
        for (path, text) in [
            ("crates/lex/Cargo.toml", "[package]\nname=\"purrdf-lex\"\n"),
            (
                "crates/lex/src/lib.rs",
                "pub mod allocation { pub struct CapacityError; impl CapacityError { /// Typed home.\n pub fn check() {} } }",
            ),
            (
                "crates/rdf-core/Cargo.toml",
                "[package]\nname=\"purrdf-core\"\n",
            ),
            (
                "crates/rdf-core/src/lib.rs",
                "#[cfg(test)] mod internal; #[cfg(any(test,target_arch=\"wasm32\"))] mod portable; fn bad(){TermId::from_index(n).unwrap();}",
            ),
            (
                "crates/rdf-core/src/internal.rs",
                "fn test_helper(){TermId::from_index(n).unwrap();}",
            ),
            (
                "crates/rdf-core/src/portable.rs",
                "fn shipping(){assert!(rows.len()<=u32::MAX as usize);}",
            ),
            (
                "crates/rdf-core/tests/fixture.rs",
                "fn support(){TermId::from_index(n).unwrap();}",
            ),
        ] {
            tree.files.insert(path.to_owned(), text.to_owned());
        }
        let workspace = Workspace::walk(&tree);
        assert_eq!(workspace.errors, [] as [String; 0]);
        let found: Vec<_> = workspace
            .rule_hits
            .iter()
            .filter(|hit| hit.rule == LOGICAL_CAPACITY_PANIC)
            .collect();
        assert_eq!(found.len(), 2);
        assert!(
            found
                .iter()
                .all(|hit| !hit.file.ends_with("internal.rs") && !hit.file.contains("/tests/"))
        );
        let ledger = crate::ledger::parse(
            r#"
[[job]]
id="logical-capacity-panic"
summary="Logical table capacity returns a typed cause"
home="purrdf_lex::allocation::CapacityError::check"
entry_points=[]
spec="No logical table-capacity panic in core or Datalog"
vectors=[]
bench=[]
sites=[]
replaces_external=[]
enforced=true
[job.forbidden]
constants=[]
fingerprints=["rule:logical-capacity-panic"]
names=[]
"#,
        )
        .unwrap();
        let job = ledger.job("logical-capacity-panic").unwrap();
        assert_eq!(
            crate::census::copies(&crate::census::matches(job, &workspace, "purrdf-lex")),
            2
        );
        assert!(
            crate::census::check_findings(&workspace, &ledger)
                .iter()
                .any(|finding| finding.contains("logical-capacity-panic"))
        );
        let index = crate::census::index(&workspace, &ledger);
        assert_eq!(
            index["jobs"]["logical-capacity-panic"]["copies"].as_u64(),
            Some(2)
        );
        tree.files.insert("crates/rdf-core/src/lib.rs".to_owned(), "#[cfg(test)] mod internal; mod portable; fn good()->Result<(),CapacityError>{CapacityError::check(s,n,l)?;Ok(())}".to_owned());
        tree.files.insert(
            "crates/rdf-core/src/portable.rs".to_owned(),
            "fn good()->Result<(),CapacityError>{CapacityError::slot(s,n,l)?;Ok(())}".to_owned(),
        );
        let corrected = Workspace::walk(&tree);
        assert!(
            corrected
                .rule_hits
                .iter()
                .all(|hit| hit.rule != LOGICAL_CAPACITY_PANIC)
        );
        assert_eq!(
            crate::census::index(&corrected, &ledger)["jobs"]["logical-capacity-panic"]["copies"]
                .as_u64(),
            Some(0)
        );
        tree.files.insert(
            "crates/rdf-core/src/portable.rs".to_owned(),
            "fn broken(".to_owned(),
        );
        let malformed = Workspace::walk(&tree);
        assert_ne!(malformed.errors, [] as [String; 0]);
        assert_ne!(
            crate::census::check_findings(&malformed, &ledger),
            [] as [String; 0]
        );
    }
}
