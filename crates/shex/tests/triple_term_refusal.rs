//! A ShExC schema has no triple-term value form, so `<< s p o >>` and
//! `<<( s p o )>>` are refused — with a diagnostic that says so, not a generic
//! IRI-reference lex error. Valid neighbours must still parse.

use purrdf_shex::parse_shexc;

const PREFIX: &str = "PREFIX ex: <http://example.org/>\n";

fn refusal(body: &str, offset: usize) -> String {
    let error = parse_shexc(&format!("{PREFIX}{body}"), None)
        .expect_err("a triple-term value is not ShExC")
        .to_string();
    assert!(
        error.contains("a ShExC schema has no triple-term value"),
        "diagnostic must name the real cause: {error}"
    );
    assert!(
        error.contains(&format!("at byte {offset}:")),
        "byte offset of the `<<` is reported: {error}"
    );
    assert!(
        !error.contains("not allowed in an IRI reference"),
        "no misleading IRI diagnostic: {error}"
    );
    error
}

#[test]
fn reifier_syntax_in_a_value_set_is_refused_precisely() {
    refusal(
        "ex:S { ex:p [ << ex:a ex:b ex:c >> ] }\n",
        PREFIX.len() + 14,
    );
}

#[test]
fn triple_term_syntax_in_a_value_set_is_refused_precisely() {
    refusal(
        "ex:S { ex:p [ <<( ex:a ex:b ex:c )>> ] }\n",
        PREFIX.len() + 14,
    );
}

#[test]
fn triple_term_syntax_as_a_node_constraint_is_refused_precisely() {
    refusal("ex:S { ex:p << ex:a ex:b ex:c >> }\n", PREFIX.len() + 12);
}

#[test]
fn valid_neighbours_still_parse() {
    for body in [
        "ex:S { ex:p [ ex:a ] }\n",
        "<http://example.org/S> { <http://example.org/p> [ <http://example.org/a> ] }\n",
        "<S> { <p> [ <a> ] }\n",
    ] {
        parse_shexc(&format!("{PREFIX}{body}"), Some("http://example.org/base/"))
            .unwrap_or_else(|e| panic!("{body:?} must parse: {e}"));
    }
}
