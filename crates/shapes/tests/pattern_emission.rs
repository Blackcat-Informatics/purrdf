// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Production SHACL emission checked by the Unicode ECMAScript engine.
//! Node is a native test tool, never a library dependency.

#![cfg(not(target_arch = "wasm32"))]

use std::io::Write;
use std::process::{Command, Stdio};

use purrdf_core::xsd_regex;
use purrdf_shapes::json_schema::{
    CompiledSchema, Namespaces, SchemaCompileError, SchemaCompileRequest, SchemaSurfaceMode,
    compile, compile_schema,
};
use purrdf_shapes::shapes::from_dataset;
use purrdf_shapes::text_ingest::parse_turtle_to_dataset;
use serde_json::{Value, json};

fn compile_pattern(pattern: &str, flags: &str) -> Result<CompiledSchema, SchemaCompileError> {
    let turtle = format!(
        "@prefix sh: <http://www.w3.org/ns/shacl#> .
         @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
         @prefix ex: <https://example.org/> .
         ex:Shape a sh:NodeShape ; sh:targetClass ex:Probe ;
           sh:property [ sh:path ex:code ; sh:maxCount 1 ; sh:datatype xsd:string ;
             sh:pattern {} ; sh:flags {} ] .",
        serde_json::to_string(pattern).expect("pattern string"),
        serde_json::to_string(flags).expect("flags string"),
    );
    let dataset = parse_turtle_to_dataset(&turtle, None).expect("fixture Turtle");
    let shapes = from_dataset(&dataset).expect("fixture shapes");
    let namespaces = Namespaces::new(
        "ex",
        &[("ex".to_owned(), "https://example.org/".to_owned())],
    )
    .expect("fixture namespaces");
    let compiled = compile(&shapes, &namespaces);
    for mode in [
        SchemaSurfaceMode::ShapedOnly,
        SchemaSurfaceMode::OntologyComplete,
    ] {
        let result = compile_schema(&SchemaCompileRequest::new(
            &shapes,
            &namespaces,
            &dataset,
            mode,
        ));
        match (&compiled, result) {
            (Ok(expected), Ok(actual)) => {
                assert_eq!(actual.compiled.schema_json, expected.schema_json);
            }
            (Err(expected), Err(actual)) => assert_eq!(actual.to_string(), expected.to_string()),
            (expected, actual) => panic!("entry points disagree: {expected:?}, {actual:?}"),
        }
    }
    compiled
}

fn emitted(pattern: &str, flags: &str) -> String {
    let compiled = compile_pattern(pattern, flags).expect("faithful compilation");
    assert!(
        compiled.losses.is_empty(),
        "{}",
        compiled.losses.render_json()
    );
    let schema: Value = serde_json::from_str(&compiled.schema_json).expect("JSON Schema");
    let openapi: Value = serde_json::from_str(&compiled.openapi_json).expect("OpenAPI");
    let pattern = &schema["$defs"]["Probe"]["properties"]["ex:code"]["pattern"];
    assert_eq!(
        pattern,
        &openapi["components"]["schemas"]["Probe"]["properties"]["ex:code"]["pattern"]
    );
    pattern.as_str().expect("emitted pattern").to_owned()
}

#[test]
fn emitted_patterns_preserve_their_languages_in_unicode_ecmascript() {
    let cases = [
        (r"^\i\c*$", "", "Éclair-9", true),
        (r"^\i\c*$", "", "9Éclair", false),
        (r"^\i$", "", "𐀀", true),
        (r"^\I$", "", "9", true),
        (r"^\c$", "", "·", true),
        (r"^\C$", "", " ", true),
        (r"^\d+$", "", "١２", true),
        (r"^\D$", "", "١", false),
        (r"^\s+$", "", "\t\n\r ", true),
        (r"^\s$", "", "\u{a0}", false),
        (r"^\S$", "", "\u{a0}", true),
        (r"^\w$", "", "🦀", true),
        (r"^\w$", "", "_", false),
        (r"^\W$", "", "_", true),
        (r"^\p{L}+$", "", "Ω𐐀", true),
        (r"^\P{L}$", "", "7", true),
        (r"^\p{IsBasicLatin}+$", "", "ASCII", true),
        (r"^\p{IsBasicLatin}+$", "", "é", false),
        (r"^\P{IsGreekandCoptic}$", "", "z", true),
        (r"^[a-z-[aeiou]]+$", "", "bcdf", true),
        (r"^[a-z-[aeiou]]+$", "", "face", false),
        (r"^[a-z-[aeiou-[e]]]$", "", "e", true),
        (r"^[a-z-[aeiou-[e]]]$", "", "a", false),
        (r"^[\i-[A-Z]]$", "", "a", true),
        (r"^[\i-[A-Z]]$", "", "A", false),
        (r"^.$", "", "\u{2028}", true),
        (r"^.$", "", "\u{2029}", true),
        (r"^.$", "", "🦀", true),
        (r"^.$", "", "\r", false),
        (r"^.$", "", "\n", false),
        (r"^.$", "s", "\n", true),
        (r"^ a . b $", "smx", "a\nb", true),
        (r"^ . $", "qsmx", "^ . $", true),
        (r"^a$", "", "a\n", false),
        (r"^a$", "m", "x\na\ny", true),
        (r"^a$", "m", "x\ra\ry", false),
        (r"^ a b # $", "x", "ab#", true),
        (r"[.]", "q", "[.]", true),
        (r"[.]", "q", "a", false),
        (r"a b", "qx", "a b", true),
        (r"^(ab|🦀){2,3}?$", "", "ab🦀ab", true),
        (r"^[a&&b]$", "", "&", true),
        (r"^[a~~b]$", "", "~", true),
        (r"^[a-[a]]$", "", "a", false),
        (r"^.$", "s", "\0", true),
    ];
    let mut requests = Vec::new();
    for (source, flags, input, expected) in cases {
        let pattern = emitted(source, flags);
        let validator = xsd_regex::compile(source, flags).expect("source validator");
        assert_eq!(
            validator.as_regex().is_match(input),
            expected,
            "{source:?}/{flags:?} on {input:?}"
        );
        if !flags.contains('m') {
            let imported = xsd_regex::from_ecma_262(&pattern).expect("shared pattern imports");
            let imported = xsd_regex::compile(&imported, "").expect("imported XSD source");
            assert_eq!(imported.as_regex().is_match(input), expected);
        }
        requests.push(json!({"pattern": pattern, "input": input, "expected": expected}));
    }
    // The emitter expresses XPath's exact multiline anchor rule; the existing
    // Rust validator retains its separately documented final-newline divergence.
    for (source, input, positions) in [
        ("^", "a\n", vec![0]),
        ("$", "a\n", vec![1]),
        ("^", "a\nb", vec![0, 2]),
        ("$", "a\nb", vec![1, 3]),
        ("^", "", vec![0]),
        ("$", "", vec![0]),
        ("^", "\n", vec![0]),
        ("$", "\n", vec![0]),
    ] {
        requests
            .push(json!({"pattern": emitted(source, "m"), "input": input, "positions": positions}));
    }
    run_ecmascript_oracle(&requests);
}

/// Run every `{pattern, input, expected | positions}` row through Node's
/// Unicode `RegExp` (the flags JSON Schema 2020-12 Core §6.4 recommends) and
/// fail on the first row whose outcome differs.
fn run_ecmascript_oracle(requests: &[Value]) {
    let script = r"
        const fs = require('node:fs');
        for (const row of JSON.parse(fs.readFileSync(0, 'utf8'))) {
            const re = new RegExp(row.pattern, row.positions ? 'gu' : 'u');
            const actual = row.positions
                ? Array.from(row.input.matchAll(re), match => match.index)
                : re.test(row.input);
            const expected = row.positions ?? row.expected;
            if (JSON.stringify(actual) !== JSON.stringify(expected)) {
                throw new Error(JSON.stringify({row, actual}));
            }
        }
    ";
    let mut child = Command::new("node")
        .args(["--eval", script])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Node is required for the Unicode ECMAScript conformance oracle");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(&serde_json::to_vec(requests).expect("oracle input"))
        .expect("write oracle input");
    let output = child.wait_with_output().expect("ECMAScript oracle");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Every flag has an exact ECMA-262 rewrite, `i` included: XPath F&O 3.1
/// §5.6.2 defines it by case variants (`fn:lower-case(C1) eq
/// fn:lower-case(C2) or fn:upper-case(C1) eq fn:upper-case(C2)`), which the
/// emitter writes into the source, so the pattern is emitted with no loss and
/// the Unicode ECMAScript engine accepts exactly the XPath language — the
/// specification's own examples, and the two places the relation differs from
/// the simple case folding the Rust validator applies (dotless `ı`, and
/// `\p{Lu}`, which `i` leaves unaffected).
#[test]
fn i_flag_patterns_carry_xpath_case_variants_in_unicode_ecmascript() {
    // (source, flags, input, XPath outcome, whether the Rust validator agrees)
    let cases = [
        ("^z$", "i", "Z", true, true),
        ("^[A-Z]+$", "i", "Kelvin\u{212a}", true, true),
        ("^[A-Z-[IO]]$", "i", "b", true, true),
        ("^[A-Z-[IO]]$", "i", "i", false, true),
        ("^[A-Z-[IO]]$", "i", "O", false, true),
        ("^[^Q]$", "i", "q", false, true),
        ("^[^Q]$", "i", "r", true, true),
        ("^i$", "i", "ı", true, false),
        ("^i$", "i", "İ", false, true),
        ("^\\p{Lu}$", "i", "a", false, false),
        ("^[\\i-[ı]]$", "i", "i", false, false),
        ("^[I-[\\i-[ı]]]$", "i", "I", true, false),
        ("literal", "i", "LITERAL", true, true),
        ("a.C", "qi", "xA.cx", true, true),
        ("a.C", "qi", "xAbcx", false, true),
        ("^ a b $", "ix", "AB", true, true),
        ("^A.B$", "is", "a\nb", true, true),
        ("^b$", "im", "a\nB\nc", true, true),
    ];
    let mut requests = Vec::new();
    for (source, flags, input, expected, validator_agrees) in cases {
        let pattern = emitted(source, flags);
        let validator = xsd_regex::compile(source, flags).expect("source validator");
        assert_eq!(
            validator.as_regex().is_match(input) == expected,
            validator_agrees,
            "{source:?}/{flags:?} on {input:?}"
        );
        requests.push(json!({"pattern": pattern, "input": input, "expected": expected}));
    }
    run_ecmascript_oracle(&requests);
}
