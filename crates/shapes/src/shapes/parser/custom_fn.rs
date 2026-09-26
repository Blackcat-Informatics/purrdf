// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Declaration parsing for the CUSTOM node-expression functions of
//! SHACL 1.2 Node Expressions §6 "Custom Node Expressions".
//!
//! Two declaring classes, one intermediate representation
//! ([`crate::expression::CustomFunction`]):
//!
//! * `sh:ListParameterExpressionFunction` (§6.2) — the function's own IRI is its
//!   list parameter property, its call site is `[ ex:f ( arg0 arg1 ) ]`, and its
//!   body reads arguments by zero-based index (`[ shnex:arg 0 ]`). This is also the
//!   class SHACL 1.2 SPARQL Extensions §7.3 asks a SPARQL engine to register a
//!   callable function for.
//! * `sh:NamedParameterExpressionFunction` (§6.1) — arguments are supplied under
//!   the parameters' own `sh:path` IRIs, its call site is `[ ex:average <expr> ]`,
//!   and its body reads them by IRI (`[ shnex:arg ex:average ]`). At least one
//!   parameter must carry `sh:keyParameter true`, and key parameters must be
//!   disjoint across functions — that is what makes a call site recognisable at all.
//!
//! # Two passes, because a body may call a function
//!
//! Declarations are DISCOVERED first ([`Parser::discover_custom_functions`]) and
//! their bodies parsed afterwards ([`Parser::parse_custom_function_bodies`]), then
//! installed into the declarations by [`crate::shapes::link::link_shapes`].
//! A body is a node expression that may call any declared function, including
//! itself, so inlining it during discovery would not terminate; interning the
//! declaration first means every call site — in a shape, in another function's body,
//! or in the function's own body — resolves to the same `Arc`.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, OnceLock};

use crate::expression::{ArgKey, CustomFnKind, CustomFunction};
use crate::model::{rdf, sh, shnex};
use crate::shapes::{InFlight, Parser};
use crate::spec::FunctionClass;
use crate::term::Term;

/// Every custom node-expression function a shapes graph declares, plus the
/// key-parameter index that makes a named-parameter call site recognisable.
///
/// Both maps are `BTreeMap`s so iteration order is the IRI order, which is what
/// makes body installation and SPARQL registration deterministic.
#[derive(Debug, Default, Clone)]
pub(crate) struct CustomFnIndex {
    /// Function IRI → the declaration.
    by_iri: BTreeMap<String, Arc<CustomFunction>>,
    /// A `sh:keyParameter true` parameter's `sh:path` IRI → the function IRI it
    /// identifies (SHACL 1.2 Node Expressions §6.1).
    by_key_param: BTreeMap<String, String>,
}

impl CustomFnIndex {
    /// The declaration for `iri`, if the graph declares one.
    pub(crate) fn get(&self, iri: &str) -> Option<&Arc<CustomFunction>> {
        self.by_iri.get(iri)
    }

    /// The custom NAMED parameter function whose key parameter is `path`, if any —
    /// the lookup that recognises a `[ ex:average <expr> ]` call site.
    pub(crate) fn by_key_parameter(&self, path: &str) -> Option<&Arc<CustomFunction>> {
        self.by_key_param
            .get(path)
            .and_then(|iri| self.by_iri.get(iri))
    }

    /// Every declaration, in IRI order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &Arc<CustomFunction>> {
        self.by_iri.values()
    }

    /// Every key parameter with the function it identifies, in path order.
    pub(crate) fn key_parameters(&self) -> Vec<(String, String)> {
        self.by_key_param
            .iter()
            .map(|(path, function)| (path.clone(), function.clone()))
            .collect()
    }

    /// Whether the graph declares no custom node-expression function at all (the
    /// overwhelmingly common case).
    pub(crate) fn is_empty(&self) -> bool {
        self.by_iri.is_empty()
    }
}

/// What linking a shapes graph's function declarations produced: the custom
/// functions, and the built-in LIST-parameter functions the graph declares.
///
/// The built-ins are kept apart on purpose — a built-in's declaration adds nothing
/// to [`CustomFnIndex`]. They are recorded only because SHACL 1.2 SPARQL Extensions
/// §7.3 asks an engine to register a SPARQL function for every declared instance of
/// `sh:ListParameterExpressionFunction`, and a built-in is registered with its
/// native implementation.
#[derive(Debug, Default, Clone)]
pub(crate) struct LinkedFunctions {
    /// The custom declarations.
    pub(crate) custom: CustomFnIndex,
    /// The IRIs of the built-in list-parameter functions the graph declares, in
    /// IRI order.
    pub(crate) native_list: BTreeSet<String>,
}

/// Refuse a custom function whose call key is node-expression vocabulary.
///
/// SHACL 1.2 Node Expressions §6.1: "The key parameters of all node expression
/// functions (including the built-in ones from the shnex: namespace) must be
/// disjoint." A custom key that is a built-in's key — or an AF spelling of one, or
/// any other structural node-expression term — would never reach the custom
/// function: the call site dispatches to the built-in first.
fn refuse_reserved_key(function: &str, key: &str) -> Result<(), String> {
    match crate::spec::reserved_key(key) {
        Some(why) => Err(format!(
            "custom node-expression function <{function}> is keyed by <{key}>, which is {why}; \
             SHACL 1.2 Node Expressions §6.1: \"The key parameters of all node expression \
             functions (including the built-in ones from the shnex: namespace) must be \
             disjoint.\""
        )),
        None => Ok(()),
    }
}

/// A parameter as authored on a declaring node, before it is reduced to an
/// [`ArgKey`].
struct RawParam {
    /// The parameter's `sh:path` IRI.
    path: String,
    /// Whether `sh:optional true` is declared.
    optional: bool,
    /// Whether `sh:keyParameter true` is declared.
    key: bool,
}

impl Parser<'_> {
    /// Link every node-expression function declaration: bind each BUILT-IN's
    /// declaration to its native implementation (see [`crate::spec`]), and discover
    /// every CUSTOM declaration WITHOUT parsing any body (see the module doc for why
    /// the two passes are separate).
    ///
    /// # Errors
    ///
    /// Hard-fails on a malformed declaration: a non-IRI declaring node, a parameter
    /// without an IRI `sh:path`, a list-parameter function whose parameter paths are
    /// not the contiguous `shnex:arg0 … shnex:arg{n-1}` block §6.2 defines, a
    /// required parameter after an optional one, a named-parameter function with no
    /// `sh:keyParameter true`, or a key parameter already claimed by another
    /// function. Hard-fails too on a built-in's declaration that supplies an
    /// implementation (duplicate definition), names another declaring class or a
    /// foreign parameter (kind or signature mismatch), and on a custom function
    /// keyed by built-in node-expression vocabulary (key clash).
    pub(crate) fn discover_custom_functions(&self) -> Result<LinkedFunctions, String> {
        let mut linked = LinkedFunctions::default();
        for class in [
            FunctionClass::ListParameter,
            FunctionClass::NamedParameter,
            FunctionClass::Plain,
        ] {
            // A graph that never mentions the class IRI declares nothing under it;
            // answering that from the term table costs no allocation, which is what
            // keeps this pass free for the overwhelmingly common graph that declares
            // no function at all.
            if self.data.term_id_by_iri(class.iri()).is_none() {
                continue;
            }
            let mut ids: Vec<Term> = self
                .quads_with(None, Some(rdf::TYPE), Some(class.iri()))
                .into_iter()
                .map(|(subject, _, _)| subject)
                .collect();
            crate::term::sort_terms_canonical(&mut ids);
            ids.dedup();
            for id in ids {
                let kind = match class {
                    FunctionClass::ListParameter => Some(CustomFnKind::ListParameter),
                    FunctionClass::NamedParameter => Some(CustomFnKind::NamedParameter),
                    FunctionClass::Plain => None,
                };
                // §6.1/§6.2 both say "an IRI in a shapes graph that is a SHACL
                // instance of …": a blank-node declaration names nothing a call site
                // could reference, so it is a malformed declaration rather than a
                // function nobody can call. `sh:NodeExpressionFunction` itself has no
                // call form, so a node typed with it alone declares nothing callable.
                let Term::NamedNode(iri) = &id else {
                    if kind.is_none() {
                        continue;
                    }
                    return Err(format!(
                        "<{}> declaration {id} is not an IRI; a custom node-expression \
                         function must be named by an IRI so a call site can reference it",
                        class.iri()
                    ));
                };
                // The linker (`crate::spec`). A declaration of a built-in is its
                // SIGNATURE: it binds natively — checked against the native class
                // and signature, and refused if it tries to supply a second
                // implementation — and adds nothing to the custom index.
                crate::spec::refuse_function_declared_component(iri.as_str(), class)?;
                if let Some(native) = crate::spec::native_function(iri.as_str()) {
                    crate::spec::bind_native_function(self.data, iri.as_str(), class, native)?;
                    if class == FunctionClass::ListParameter {
                        linked.native_list.insert(iri.as_str().to_owned());
                    }
                    continue;
                }
                let Some(kind) = kind else {
                    continue;
                };
                let index = &mut linked.custom;
                if index.by_iri.contains_key(iri.as_str()) {
                    return Err(format!(
                        "<{}> is declared both a sh:ListParameterExpressionFunction and a \
                         sh:NamedParameterExpressionFunction; it can be only one",
                        iri.as_str()
                    ));
                }
                let raw = self.custom_fn_params(&id)?;
                let (params, required) = match kind {
                    CustomFnKind::ListParameter => {
                        // A list-parameter function's own IRI IS its call key
                        // (`[ ex:f ( … ) ]`), so it must not be one the built-ins
                        // already claim — the call site would dispatch to the
                        // built-in and the declaration would be unreachable.
                        refuse_reserved_key(iri.as_str(), iri.as_str())?;
                        Self::list_parameter_keys(iri.as_str(), &raw)?
                    }
                    CustomFnKind::NamedParameter => Self::named_parameter_keys(iri.as_str(), &raw)?,
                };
                if matches!(kind, CustomFnKind::NamedParameter) {
                    for param in raw.iter().filter(|p| p.key) {
                        refuse_reserved_key(iri.as_str(), &param.path)?;
                        if let Some(other) = index
                            .by_key_param
                            .insert(param.path.clone(), iri.as_str().to_owned())
                            && other != iri.as_str()
                        {
                            return Err(format!(
                                "sh:keyParameter <{}> is claimed by both <{other}> and <{}>; SHACL \
                                 1.2 Node Expressions §6.1 requires key parameters to be disjoint \
                                 across functions",
                                param.path,
                                iri.as_str()
                            ));
                        }
                    }
                }
                index.by_iri.insert(
                    iri.as_str().to_owned(),
                    Arc::new(CustomFunction {
                        iri: iri.clone(),
                        kind,
                        params,
                        required,
                        body: OnceLock::new(),
                    }),
                );
            }
        }
        Ok(linked)
    }

    /// Parse every `sh:parameter` of a declaring node into a [`RawParam`], ordered
    /// by `sh:path` IRI so the result is independent of graph iteration order.
    fn custom_fn_params(&self, id: &Term) -> Result<Vec<RawParam>, String> {
        let mut raw: Vec<RawParam> = Vec::new();
        for p_node in self.objects_of(id, sh::PARAMETER_PROPERTY) {
            let path = self
                .first_object_of(&p_node, sh::PATH)
                .and_then(|t| match t {
                    Term::NamedNode(n) => Some(n.as_str().to_owned()),
                    _ => None,
                })
                .ok_or_else(|| {
                    format!(
                        "custom node-expression function <{id}> has a sh:parameter without an IRI \
                         sh:path"
                    )
                })?;
            raw.push(RawParam {
                path,
                optional: self.custom_fn_flag(&p_node, sh::OPTIONAL, id)?,
                key: self.custom_fn_flag(&p_node, sh::KEY_PARAMETER, id)?,
            });
        }
        raw.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(raw)
    }

    /// An `xsd:boolean` flag on a parameter node, defaulting to `false` when absent
    /// and hard-failing on a value that is not a boolean.
    fn custom_fn_flag(&self, p_node: &Term, predicate: &str, id: &Term) -> Result<bool, String> {
        match self.first_object_of(p_node, predicate) {
            None => Ok(false),
            Some(Term::Literal(lit)) => {
                match purrdf_xsd::parse_by_iri(lit.value(), lit.datatype_str()) {
                    Ok(Some(purrdf_xsd::XsdValue::Boolean(b))) => Ok(b),
                    _ => Err(format!(
                        "custom node-expression function <{id}> has a sh:parameter whose \
                         <{predicate}> is not an xsd:boolean: {}",
                        lit.value()
                    )),
                }
            }
            Some(other) => Err(format!(
                "custom node-expression function <{id}> has a sh:parameter whose <{predicate}> is \
                 not a literal: {other}"
            )),
        }
    }

    /// Reduce a list-parameter function's declared parameters to indexed argument
    /// keys (SHACL 1.2 Node Expressions §6.2).
    ///
    /// §6.2 spells those parameters `sh:path shnex:arg0`, `shnex:arg1`, …, so the
    /// declared block must be exactly `shnex:arg0 … shnex:arg{n-1}`. A gap, a
    /// duplicate, a non-`shnex:arg` path or a non-numeric suffix would leave the
    /// call's arity undefined, so each is a load failure rather than a call-time
    /// surprise.
    fn list_parameter_keys(iri: &str, raw: &[RawParam]) -> Result<(Vec<ArgKey>, usize), String> {
        let mut indexed: Vec<(u64, bool)> = Vec::with_capacity(raw.len());
        for param in raw {
            let suffix = param.path.strip_prefix(shnex::ARG).ok_or_else(|| {
                format!(
                    "sh:ListParameterExpressionFunction <{iri}> has a sh:parameter whose sh:path \
                     <{}> is not one of the shnex:arg0, shnex:arg1, … argument parameters SHACL \
                     1.2 Node Expressions §6.2 defines",
                    param.path
                )
            })?;
            let position: u64 = suffix.parse().map_err(|_| {
                format!(
                    "sh:ListParameterExpressionFunction <{iri}> has a sh:parameter whose sh:path \
                     <{}> does not end in a zero-based argument index",
                    param.path
                )
            })?;
            indexed.push((position, param.optional));
        }
        indexed.sort_by_key(|&(position, _)| position);
        for (expected, &(position, _)) in indexed.iter().enumerate() {
            let expected = u64::try_from(expected)
                .map_err(|e| format!("argument index is not representable: {e}"))?;
            if position != expected {
                return Err(format!(
                    "sh:ListParameterExpressionFunction <{iri}> declares shnex:arg{position} where \
                     shnex:arg{expected} was expected; the argument parameters must be the \
                     contiguous block shnex:arg0 … shnex:arg{}",
                    indexed.len().saturating_sub(1)
                ));
            }
        }
        let mut seen_optional = false;
        for &(position, optional) in &indexed {
            if optional {
                seen_optional = true;
            } else if seen_optional {
                return Err(format!(
                    "sh:ListParameterExpressionFunction <{iri}> declares the required parameter \
                     shnex:arg{position} after an optional one, which leaves its arity ambiguous"
                ));
            }
        }
        let required = indexed.iter().filter(|&&(_, optional)| !optional).count();
        let params = indexed
            .into_iter()
            .map(|(position, _)| ArgKey::Index(position))
            .collect();
        Ok((params, required))
    }

    /// Reduce a named-parameter function's declared parameters to IRI argument keys
    /// (SHACL 1.2 Node Expressions §6.1).
    fn named_parameter_keys(iri: &str, raw: &[RawParam]) -> Result<(Vec<ArgKey>, usize), String> {
        if raw.is_empty() {
            return Err(format!(
                "sh:NamedParameterExpressionFunction <{iri}> declares no sh:parameter; SHACL 1.2 \
                 Node Expressions §6.1 requires one or more"
            ));
        }
        if !raw.iter().any(|p| p.key) {
            return Err(format!(
                "sh:NamedParameterExpressionFunction <{iri}> has no parameter marked \
                 sh:keyParameter true; SHACL 1.2 Node Expressions §6.1 requires at least one, and \
                 without it no call site could be recognised"
            ));
        }
        let required = raw.iter().filter(|p| !p.optional).count();
        let params = raw.iter().map(|p| ArgKey::Named(p.path.clone())).collect();
        Ok((params, required))
    }

    /// Parse every declared function's `sh:bodyExpression`, keyed by function IRI.
    ///
    /// Runs after discovery, so a body that calls a function — itself included —
    /// resolves against the interned declarations.
    ///
    /// The bodies are RETURNED rather than installed: writing one into a
    /// declaration's `OnceLock` is a linking step, and every linking step belongs to
    /// [`crate::shapes::link::link_shapes`], which is where the same installation is
    /// verified rather than assumed. The map is a `BTreeMap`, so it carries the
    /// declaration table's own IRI order through to the installer.
    ///
    /// # Errors
    ///
    /// Hard-fails when a declaration carries no `sh:bodyExpression` or more than
    /// one, when the body does not parse as a node expression, or when the body
    /// references an argument key the declaration does not have.
    pub(crate) fn parse_custom_function_bodies(
        &mut self,
        index: &CustomFnIndex,
    ) -> Result<BTreeMap<String, crate::expression::NodeExpr>, String> {
        let mut parsed_bodies = BTreeMap::new();
        for func in index.iter() {
            let id = Term::NamedNode(func.iri.clone());
            let bodies = self.objects_of(&id, sh::BODY_EXPRESSION);
            let [body_node] = bodies.as_slice() else {
                return Err(format!(
                    "custom node-expression function <{}> declares {} sh:bodyExpression values; \
                     SHACL 1.2 Node Expressions §6.1/§6.2 require exactly one",
                    func.iri.as_str(),
                    bodies.len()
                ));
            };
            // A body is parsed with a FRESH in-flight set: the declaration is a
            // top-level parse of its own, not a continuation of whatever shape
            // happened to reach it first, and sharing the caller's stack would make
            // a legitimate shared sub-expression look like a cycle.
            let saved = std::mem::take(&mut self.in_flight);
            self.in_flight.insert(InFlight::NodeExpr(id));
            let parsed = self.parse_node_expr(body_node);
            self.in_flight = saved;
            let body = parsed.map_err(|e| {
                format!(
                    "custom node-expression function <{}> has an unusable sh:bodyExpression: {e}",
                    func.iri.as_str()
                )
            })?;
            check_body_args(func, &body)?;
            if parsed_bodies
                .insert(func.iri.as_str().to_owned(), body)
                .is_some()
            {
                return Err(format!(
                    "internal error: custom node-expression function <{}> already had a body \
                     parsed",
                    func.iri.as_str()
                ));
            }
        }
        Ok(parsed_bodies)
    }
}

/// Refuse a body that reads an argument key its own declaration does not have.
///
/// An unknown key evaluates to the empty list (SHACL 1.2 Node Expressions §6.3's
/// second case), so without this check a typo — `[ shnex:arg 2 ]` in a two-argument
/// function, `[ shnex:arg ex:avarage ]` against `ex:average` — would load green and
/// silently contribute nothing at every call. The declaration and the body are both
/// in hand at load, so the mismatch is decided there.
fn check_body_args(
    func: &CustomFunction,
    body: &crate::expression::NodeExpr,
) -> Result<(), String> {
    let mut used: Vec<ArgKey> = Vec::new();
    collect_arg_keys(body, &mut used);
    for key in used {
        if !func.params.contains(&key) {
            return Err(format!(
                "custom node-expression function <{}> reads shnex:arg {key}, which is not one of \
                 its declared parameters ({})",
                func.iri.as_str(),
                render_params(&func.params)
            ));
        }
    }
    Ok(())
}

/// A human-readable rendering of a declared parameter list, for the arity/key
/// mismatch diagnostic.
fn render_params(params: &[ArgKey]) -> String {
    if params.is_empty() {
        return "none".to_owned();
    }
    params
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Every `shnex:arg` key `expr` reads, EXCLUDING those inside a nested custom
/// function call.
///
/// The exclusion is the point: a nested call replaces the argument scope outright
/// (`evalExpr(body, focusGraph, focusNode, argScope)`), so the keys its own body
/// reads are that callee's business and are checked against that callee's
/// declaration. The nested call's ARGUMENT expressions, however, are evaluated in
/// the caller's scope, so they are walked.
fn collect_arg_keys(expr: &crate::expression::NodeExpr, out: &mut Vec<ArgKey>) {
    use crate::expression::{FnCall, NodeExpr};
    match expr {
        NodeExpr::Arg(key) => out.push(key.clone()),
        NodeExpr::CustomCall { args, .. } => {
            for (_, arg) in args {
                collect_arg_keys(arg, out);
            }
        }
        NodeExpr::Union(items) | NodeExpr::Intersection(items) | NodeExpr::Concat(items) => {
            for item in items {
                collect_arg_keys(item, out);
            }
        }
        NodeExpr::Call(
            FnCall::Builtin { args, .. }
            | FnCall::UserDefined { args, .. }
            | FnCall::Sparql { args, .. },
        ) => {
            for arg in args {
                collect_arg_keys(arg, out);
            }
        }
        NodeExpr::If { cond, then, els } => {
            collect_arg_keys(cond, out);
            collect_arg_keys(then, out);
            collect_arg_keys(els, out);
        }
        NodeExpr::Remove { nodes, remove } => {
            collect_arg_keys(nodes, out);
            collect_arg_keys(remove, out);
        }
        NodeExpr::FlatMap { nodes, map } => {
            collect_arg_keys(nodes, out);
            collect_arg_keys(map, out);
        }
        NodeExpr::OrderBy { of, key, .. } => {
            collect_arg_keys(of, out);
            collect_arg_keys(key, out);
        }
        NodeExpr::Filter { nodes, .. }
        | NodeExpr::FindFirst { nodes, .. }
        | NodeExpr::MatchAll { nodes, .. } => collect_arg_keys(nodes, out),
        NodeExpr::ConformsToShape { node, shape } => {
            collect_arg_keys(node, out);
            // A COMPUTED shape argument is a node expression evaluated in the same
            // scope as the node argument (§4.5.3), so an argument it reads is read
            // by this body.
            if let crate::expression::ShapeArg::Computed { expr, .. } = shape {
                collect_arg_keys(expr, out);
            }
        }
        NodeExpr::PathValues { focus, .. } => collect_arg_keys(focus, out),
        NodeExpr::Count { of, .. } | NodeExpr::Limit { of, .. } | NodeExpr::Offset { of, .. } => {
            collect_arg_keys(of, out);
        }
        NodeExpr::Distinct(inner)
        | NodeExpr::InstancesOf(inner)
        | NodeExpr::Min(inner)
        | NodeExpr::Max(inner)
        | NodeExpr::Sum(inner)
        | NodeExpr::Exists(inner) => collect_arg_keys(inner, out),
        // Leaves: they carry no sub-expression, so there is nothing to walk.
        NodeExpr::Constant(_)
        | NodeExpr::This
        | NodeExpr::Path(_)
        | NodeExpr::Empty
        | NodeExpr::Var(_)
        | NodeExpr::List(_)
        | NodeExpr::NodesMatching(_)
        | NodeExpr::Select { .. } => {}
    }
}
