// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Cargo-feature and dead Python-extension test policies, over Rust tokens.
//!
//! PyO3's extension-module leaves CPython symbols for the interpreter to
//! resolve at dlopen. Its `test = false` library cannot run Rust tests: Python
//! surface assertions belong in bindings/python/tests, and engine assertions
//! in the Rust crate that owns the engine. Cargo's sole allowed first-party
//! feature is the empty cargo-c marker purrdf-capi:capi; it gates no code.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::{Command, ExitCode};

use proc_macro2::{Delimiter, TokenStream, TokenTree};
use purrdf_lex::json::{self, Value};
use purrdf_lex::walk::WorkList;
use syn::parse::discouraged::Speculative;
use syn::parse::{ParseStream, Parser};
use syn::{Expr, Token};

const PYTHON_SOURCE: &str = "bindings/python/src";
const CAPI: &str = "purrdf-capi";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Rule {
    Feature,
    CfgTest,
    TestAttribute,
}

impl Rule {
    const fn message(self) -> &'static str {
        match self {
            Self::Feature => "[cfg-feature] Rust cfg(feature = ...) use is forbidden",
            Self::CfgTest => "[cfg-test] a `test` predicate in a cfg invocation",
            Self::TestAttribute => "[test-attr] a `#[test]` attribute",
        }
    }
}

fn ident(token: Option<&TokenTree>, name: &str) -> bool {
    matches!(token, Some(TokenTree::Ident(value)) if value.to_string().trim_start_matches("r#") == name)
}

fn punct(token: Option<&TokenTree>, value: char) -> bool {
    matches!(token, Some(TokenTree::Punct(found)) if found.as_char() == value)
}

/// Find the first actual predicate token in a cfg body. Literals and comments
/// are never Rust predicates; qualified paths and named values are not `test`.
fn predicate(body: TokenStream, feature: bool) -> Option<(usize, usize)> {
    let mut pending: WorkList<TokenStream, 16> = WorkList::with(body);
    let mut found = BTreeSet::new();
    while let Some(body) = pending.pop() {
        let tokens: Vec<_> = body.into_iter().collect();
        for (at, token) in tokens.iter().enumerate() {
            let name = if feature { "feature" } else { "test" };
            if ident(Some(token), name)
                && if feature {
                    punct(tokens.get(at + 1), '=')
                } else {
                    !punct(tokens.get(at + 1), '=')
                        && !punct(tokens.get(at + 1), ':')
                        && !at
                            .checked_sub(1)
                            .is_some_and(|before| punct(tokens.get(before), ':'))
                }
            {
                let start = token.span().start();
                found.insert((start.line, start.column));
            }
            if let TokenTree::Group(group) = token {
                pending.push(group.stream());
            }
        }
    }
    found.into_iter().next()
}

/// Only a cfg_select arm's header is a predicate. Its payload is Rust code,
/// where a variable named `test` or an assignment to `feature` is ordinary data.
fn cfg_select_headers(body: TokenStream) -> Result<Vec<TokenStream>, syn::Error> {
    (|input: ParseStream<'_>| {
        let mut headers = Vec::new();
        while !input.is_empty() {
            let mut header = TokenStream::new();
            while !input.is_empty() && !input.peek(Token![=>]) {
                header.extend([input.parse::<TokenTree>()?]);
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![=>]>()?;
            headers.push(header);
            if input.peek(syn::token::Brace) {
                input.parse::<TokenTree>()?;
            } else {
                let expression = input.fork();
                if expression.parse::<Expr>().is_ok() {
                    input.advance_to(&expression);
                } else {
                    // A macro transcriber can carry `$expression` rather than
                    // a concrete Expr. Balanced token trees keep its commas
                    // separate from the comma between cfg_select arms.
                    while !input.is_empty() && !input.peek(Token![,]) {
                        input.parse::<TokenTree>()?;
                    }
                }
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(headers)
    })
    .parse2(body)
}

/// Attributes and cfg! macros can occur inside macro bodies, so scan their
/// token trees rather than only the item AST. This also handles inner attrs.
fn token_findings(source: &str, feature: bool) -> Result<Vec<(usize, Rule)>, String> {
    let stream: TokenStream = source
        .parse()
        .map_err(|error| format!("Rust tokens: {error}"))?;
    let mut pending: WorkList<TokenStream, 16> = WorkList::with(stream);
    let mut found = BTreeSet::new();
    while let Some(stream) = pending.pop() {
        let tokens: Vec<_> = stream.into_iter().collect();
        for (at, token) in tokens.iter().enumerate() {
            if punct(Some(token), '#') {
                let attr_at = at + 1 + usize::from(punct(tokens.get(at + 1), '!'));
                if let Some(TokenTree::Group(attr)) = tokens.get(attr_at)
                    && attr.delimiter() == Delimiter::Bracket
                {
                    let attr: Vec<_> = attr.stream().into_iter().collect();
                    if !feature && attr.len() == 1 && ident(attr.first(), "test") {
                        let start = token.span().start();
                        found.insert((start.line, start.column, Rule::TestAttribute));
                    }
                    if (ident(attr.first(), "cfg") || ident(attr.first(), "cfg_attr"))
                        && let Some(TokenTree::Group(body)) = attr.get(1)
                        && body.delimiter() == Delimiter::Parenthesis
                        && let Some((line, column)) = predicate(body.stream(), feature)
                    {
                        found.insert((
                            line,
                            column,
                            if feature {
                                Rule::Feature
                            } else {
                                Rule::CfgTest
                            },
                        ));
                    }
                }
            }
            if ident(Some(token), "cfg")
                && punct(tokens.get(at + 1), '!')
                && let Some(TokenTree::Group(body)) = tokens.get(at + 2)
                && let Some((line, column)) = predicate(body.stream(), feature)
            {
                found.insert((
                    line,
                    column,
                    if feature {
                        Rule::Feature
                    } else {
                        Rule::CfgTest
                    },
                ));
            }
            if ident(Some(token), "cfg_select")
                && punct(tokens.get(at + 1), '!')
                && let Some(TokenTree::Group(body)) = tokens.get(at + 2)
            {
                for header in cfg_select_headers(body.stream())
                    .map_err(|error| format!("cfg_select tokens: {error}"))?
                {
                    if let Some((line, column)) = predicate(header, feature) {
                        found.insert((
                            line,
                            column,
                            if feature {
                                Rule::Feature
                            } else {
                                Rule::CfgTest
                            },
                        ));
                    }
                }
            }
            if let TokenTree::Group(group) = token {
                pending.push(group.stream());
            }
        }
    }
    Ok(found
        .into_iter()
        .map(|(line, _, rule)| (line, rule))
        .collect())
}

fn source_findings(
    root: &Path,
    directory: &Path,
    feature: bool,
) -> Result<(Vec<String>, usize), String> {
    if !directory.is_dir() {
        return Err(format!(
            "{} is missing: cannot verify its Rust sources",
            directory.display()
        ));
    }
    let mut sources = Vec::new();
    purrdf_testkit::paths::collect_rs_filtered(directory, &mut sources, &|path| {
        !path.is_symlink()
            && (!feature
                || path
                    .file_name()
                    .is_none_or(|name| name != ".git" && name != ".worktrees" && name != "target"))
    });
    let mut findings = Vec::new();
    for path in &sources {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let relative = path
            .strip_prefix(root)
            .map_err(|error| error.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        for (line, rule) in
            token_findings(&text, feature).map_err(|error| format!("{relative}: {error}"))?
        {
            findings.push(format!("{relative}:{line}: {}", rule.message()));
        }
    }
    Ok((findings, sources.len()))
}

fn feature_maps(metadata: &Value) -> Result<Vec<String>, String> {
    let members = metadata["workspace_members"]
        .as_array()
        .ok_or("metadata has no workspace_members array")?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or("metadata has no packages array")?;
    let mut findings = Vec::new();
    let mut saw_capi = false;
    for member in members {
        let id = member.as_str().ok_or("workspace member is not an id")?;
        let package = packages
            .iter()
            .find(|package| package["id"].as_str() == Some(id))
            .ok_or_else(|| format!("workspace member {id} has no package"))?;
        let name = package["name"].as_str().ok_or("package has no name")?;
        let features = package["features"]
            .as_object()
            .ok_or("package has no features map")?;
        let mut declared = BTreeMap::new();
        for (key, value) in features.iter() {
            let values = value
                .as_array()
                .ok_or("feature expansion is not an array")?
                .iter()
                .map(|value| {
                    value
                        .as_str()
                        .map(str::to_owned)
                        .ok_or("feature expansion is not a string")
                })
                .collect::<Result<Vec<_>, _>>()?;
            declared.insert(key.to_owned(), values);
        }
        let mut expected = BTreeMap::new();
        if name == CAPI {
            saw_capi = true;
            expected.insert("capi".to_owned(), Vec::<String>::new());
        }
        if declared != expected {
            let render = |map: BTreeMap<String, Vec<String>>| {
                json::write_compact(&Value::object(
                    map.into_iter()
                        .map(|(key, values)| (key, Value::from(values))),
                ))
            };
            findings.push(format!(
                "{name}: declared {}; expected {}",
                render(declared),
                render(expected)
            ));
        }
    }
    if !saw_capi {
        findings.push(format!("{CAPI}: package missing from workspace"));
    }
    Ok(findings)
}

pub(crate) fn no_features(root: &Path) -> Result<ExitCode, String> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1", "--locked"])
        .current_dir(root)
        .output()
        .map_err(|error| format!("cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let text = std::str::from_utf8(&output.stdout)
        .map_err(|error| format!("cargo metadata UTF-8: {error}"))?;
    let metadata = json::read(text).map_err(|error| format!("cargo metadata JSON: {error}"))?;
    let mut findings = feature_maps(&metadata)?;
    findings.extend(source_findings(root, root, true)?.0);
    Ok(report(
        &findings,
        "First-party workspace crates must declare no Cargo features except the exact empty purrdf-capi:capi cargo-c marker.",
    ))
}

pub(crate) fn python_binding_tests(root: &Path) -> Result<ExitCode, String> {
    let (findings, count) = source_findings(root, &root.join(PYTHON_SOURCE), false)?;
    if findings.is_empty() {
        println!("OK: no Rust test module under {PYTHON_SOURCE} ({count} sources scanned)");
    }
    Ok(report(
        &findings,
        "The Python extension crate must carry no Rust test module: its test=false extension-module cannot link a test executable. Move Python-visible assertions to bindings/python/tests (make pytest against the built extension), and engine assertions to the Rust crate that owns them. Do not add Cargo features or link libpython into the portable cdylib.",
    ))
}

fn report(findings: &[String], meaning: &str) -> ExitCode {
    if findings.is_empty() {
        ExitCode::SUCCESS
    } else {
        eprintln!("{meaning}\n{}", findings.join("\n"));
        ExitCode::FAILURE
    }
}

const FEATURE_CASES: &[(&str, &str, bool)] = &[
    ("feature cfg", "#[cfg(feature = \"x\")] fn f() {}", true),
    (
        "feature cfg valid neighbor",
        "#[cfg(target_os = \"linux\")] fn f() {}",
        false,
    ),
    (
        "nested feature cfg_attr",
        "#![cfg_attr(unix, cfg(any(feature = \"x\", windows)))]",
        true,
    ),
    (
        "cfg_attr valid neighbor",
        "#![cfg_attr(unix, doc = \"feature = x\")]",
        false,
    ),
    (
        "raw feature macro",
        "let x = cfg!(not(r#feature = \"x\"));",
        true,
    ),
    (
        "macro valid neighbor",
        "let x = cfg!(not(feature_name = \"x\"));",
        false,
    ),
    (
        "braced feature macro",
        "let x = cfg!{feature = \"x\"};",
        true,
    ),
    ("braced macro valid neighbor", "let x = cfg!{unix};", false),
    (
        "bracketed feature macro",
        "let x = cfg![feature = \"x\"];",
        true,
    ),
    (
        "bracketed macro valid neighbor",
        "let x = cfg![unix];",
        false,
    ),
    (
        "cfg_select feature condition",
        "let x = cfg_select! { unix => false, feature = \"x\" => true, _ => false };",
        true,
    ),
    (
        "cfg_select payload is not a feature condition",
        "let x = cfg_select! { unix => { feature = \"x\"; }, _ => feature = \"y\" };",
        false,
    ),
    (
        "cfg_select macro transcriber feature condition",
        "macro_rules! x { ($value:expr) => { cfg_select! { unix => $value, r#feature = \"x\" => {$value} _ => false } }; }",
        true,
    ),
    (
        "cfg_select macro transcriber valid neighbor",
        "macro_rules! x { ($value:expr) => { cfg_select! { unix => $value, target_os = \"linux\" => {$value} _ => false } }; }",
        false,
    ),
    (
        "feature in macro body",
        "macro_rules! x { () => { #[cfg(feature = \"x\")] fn f() {} }; }",
        true,
    ),
    (
        "feature prose is data",
        "/* #[cfg(feature = \"x\")] */ const X: &str = r###\"cfg!(feature = \"x\")\"###;",
        false,
    ),
];

const TEST_CASES: &[(&str, &str, bool)] = &[
    ("test cfg", "#[cfg(test)] mod tests {}", true),
    (
        "test cfg valid neighbor",
        "#[cfg(test_helpers)] mod helpers {}",
        false,
    ),
    (
        "nested inner test cfg_attr",
        "#![cfg_attr(unix, cfg(all(not(test), windows)))]",
        true,
    ),
    (
        "cfg_attr test valid neighbor",
        "#[cfg_attr(unix, doc = \"test\")] fn f() {}",
        false,
    ),
    (
        "raw test cfg macro",
        "let x = cfg!(any(r#test, unix));",
        true,
    ),
    (
        "test macro valid neighbor",
        "let x = cfg!(feature = \"test\");",
        false,
    ),
    ("braced test macro", "let x = cfg!{test};", true),
    (
        "braced test macro valid neighbor",
        "let x = cfg!{testing};",
        false,
    ),
    ("bracketed test macro", "let x = cfg![test];", true),
    (
        "bracketed test macro valid neighbor",
        "let x = cfg![testing];",
        false,
    ),
    (
        "cfg_select nested test condition",
        "let x = cfg_select! { unix => false, any(r#test, windows) => true, _ => false };",
        true,
    ),
    (
        "cfg_select payload is not a test condition",
        "let x = cfg_select! { unix => if x { test() } else { test }, _ => test() };",
        false,
    ),
    (
        "cfg_select bracketed test condition",
        "let x = cfg_select![test => true, _ => false];",
        true,
    ),
    (
        "cfg_select parenthesized valid neighbor",
        "let x = cfg_select!(feature = \"test\" => test(), _ => \"test\");",
        false,
    ),
    ("bare test attribute", "#[test] fn f() {}", true),
    (
        "test attribute valid neighbor",
        "#[test_case] fn f() {}",
        false,
    ),
    (
        "cfg_attr emits test",
        "#[cfg_attr(unix, test)] fn f() {}",
        true,
    ),
    (
        "named test value is not predicate",
        "#[cfg(test = \"x\")] fn f() {}",
        false,
    ),
    (
        "test in macro body",
        "macro_rules! x { () => { #[test] fn f() {} }; }",
        true,
    ),
    (
        "test prose is data",
        "//! #[cfg(test)]\n/* nested /* #[test] */ comment */ const X: &str = \"#[test]\";",
        false,
    ),
];

fn metadata_with(features: &str, ordinary: &str) -> Value {
    json::read(&format!("{{\"workspace_members\":[\"capi\",\"ordinary\"],\"packages\":[{{\"id\":\"capi\",\"name\":\"purrdf-capi\",\"features\":{features}}},{{\"id\":\"ordinary\",\"name\":\"ordinary\",\"features\":{ordinary}}}]}}" )).expect("fixture JSON")
}

pub(crate) fn self_test_cases() -> Vec<(&'static str, bool)> {
    let mut cases = Vec::new();
    for (feature, fixtures) in [(true, FEATURE_CASES), (false, TEST_CASES)] {
        for &(name, source, refuses) in fixtures {
            cases.push((
                name,
                token_findings(source, feature).is_ok_and(|hits| hits.is_empty() != refuses),
            ));
        }
    }
    for (name, capi, ordinary, refuses) in [
        ("exact empty capi marker", "{\"capi\":[]}", "{}", false),
        ("missing capi marker", "{}", "{}", true),
        ("expanded capi marker", "{\"capi\":[\"other\"]}", "{}", true),
        (
            "extra capi feature",
            "{\"capi\":[],\"other\":[]}",
            "{}",
            true,
        ),
        (
            "ordinary Cargo feature",
            "{\"capi\":[]}",
            "{\"default\":[]}",
            true,
        ),
        (
            "optional dependency implicit feature",
            "{\"capi\":[]}",
            "{\"dep\":[\"dep:dep\"]}",
            true,
        ),
    ] {
        cases.push((
            name,
            feature_maps(&metadata_with(capi, ordinary))
                .is_ok_and(|hits| hits.is_empty() != refuses),
        ));
    }
    cases.push((
        "missing capi package",
        feature_maps(&json::read("{\"workspace_members\":[],\"packages\":[]}").expect("fixture"))
            .is_ok_and(|hits| hits == ["purrdf-capi: package missing from workspace"]),
    ));
    cases.push((
        "invalid metadata is refused",
        feature_maps(&Value::Null).is_err(),
    ));
    cases.push((
        "unbalanced Rust tokens are refused",
        token_findings("#[cfg(test)", false).is_err(),
    ));
    cases
}

#[cfg(test)]
mod tests {
    use super::{
        Rule, feature_maps, json, metadata_with, python_binding_tests, self_test_cases,
        source_findings, token_findings,
    };

    #[test]
    fn all_policy_negative_vectors_and_valid_neighbors_hold() {
        // These compile as cfg! invocations, independently of our token reader.
        const {
            assert!(cfg![test]);
            assert!(cfg! { test });
        }
        for (name, held) in self_test_cases() {
            assert!(held, "{name}");
        }
    }

    #[test]
    fn lines_and_rules_match_the_former_dead_test_guard() {
        assert_eq!(token_findings("\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn f() {}\n}\n\n#[cfg(all(test, unix))]\nconst X: &str = \"x\";\n\n#[cfg_attr(test, derive(Debug))]\nstruct S;", false).expect("tokens"), [(2, Rule::CfgTest), (4, Rule::TestAttribute), (8, Rule::CfgTest), (11, Rule::CfgTest)]);
    }

    #[test]
    fn missing_governed_directory_fails_and_empty_directory_passes() {
        let dir = purrdf_testkit::TempDir::for_unit_test().expect("temp dir");
        assert!(python_binding_tests(dir.path()).is_err());
        std::fs::create_dir_all(dir.path().join("bindings/python/src")).expect("governed");
        assert_eq!(
            python_binding_tests(dir.path()).expect("scan"),
            std::process::ExitCode::SUCCESS
        );
    }

    #[test]
    fn the_source_walk_excludes_build_and_sibling_trees() {
        let dir = purrdf_testkit::TempDir::for_unit_test().expect("temp dir");
        for path in ["target/a.rs", ".worktrees/b.rs", ".git/c.rs"] {
            let path = dir.path().join(path);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
            std::fs::write(path, "#[cfg(feature = \"x\")] fn f() {}").expect("source");
        }
        std::fs::write(dir.path().join("lib.rs"), "fn f() {}").expect("valid source");
        assert_eq!(
            source_findings(dir.path(), dir.path(), true).expect("scan"),
            (vec![], 1)
        );
    }

    #[test]
    fn feature_maps_ignore_packages_outside_the_workspace() {
        let mut metadata = metadata_with("{\"capi\":[]}", "{}");
        let external =
            json::read("{\"id\":\"external\",\"name\":\"external\",\"features\":{\"x\":[]}}")
                .expect("external metadata");
        metadata["packages"]
            .as_array_mut()
            .expect("packages")
            .push(external);
        assert_eq!(feature_maps(&metadata).expect("maps"), Vec::<String>::new());
    }

    #[test]
    fn python_sources_have_no_build_directory_exemption() {
        let dir = purrdf_testkit::TempDir::for_unit_test().expect("temp dir");
        let source = dir.path().join("bindings/python/src/target/hidden.rs");
        std::fs::create_dir_all(source.parent().expect("parent")).expect("dirs");
        std::fs::write(&source, "#[test] fn f() {}").expect("source");
        assert_eq!(
            source_findings(dir.path(), &dir.path().join("bindings/python/src"), false)
                .expect("scan")
                .0,
            ["bindings/python/src/target/hidden.rs:1: [test-attr] a `#[test]` attribute"]
        );
        std::fs::write(source, "fn f() {}").expect("valid neighbor");
        assert_eq!(
            python_binding_tests(dir.path()).expect("scan"),
            std::process::ExitCode::SUCCESS
        );
    }
}
