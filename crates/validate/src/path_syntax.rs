// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Render a SHACL [`Path`] as SPARQL property-path syntax.
//!
//! A SHACL result path can be a complex structure (inverse, sequence,
//! alternative, transitive closures). The RDF report graph emits it as nested
//! blank nodes; for a SARIF `logicalLocation` we want a single readable string,
//! so this walks the same [`Path`] enum the report serializer walks and renders
//! the equivalent SPARQL property path (`^`, `/`, `|`, `*`, `+`, `?`), reusing
//! the model instead of re-deriving path structure.

use purrdf_shapes::shapes::Path;

/// Render `path` as a SPARQL property path. Composite sub-paths are
/// parenthesized so precedence is unambiguous (`^ex:a/ex:b` vs `^(ex:a/ex:b)`).
#[must_use]
pub fn render_path(path: &Path) -> String {
    match path {
        Path::Predicate(p) => format!("<{}>", p.as_str()),
        Path::Inverse(inner) => format!("^{}", grouped(inner)),
        Path::Sequence(parts) => join(parts, "/"),
        Path::Alternative(parts) => join(parts, "|"),
        Path::ZeroOrMore(inner) => format!("{}*", grouped(inner)),
        Path::OneOrMore(inner) => format!("{}+", grouped(inner)),
        Path::ZeroOrOne(inner) => format!("{}?", grouped(inner)),
    }
}

/// Render a sub-path, wrapping composite forms in parentheses so an enclosing
/// operator binds correctly. A bare predicate needs no parentheses.
fn grouped(path: &Path) -> String {
    match path {
        Path::Predicate(_) => render_path(path),
        _ => format!("({})", render_path(path)),
    }
}

fn join(parts: &[Path], sep: &str) -> String {
    parts.iter().map(grouped).collect::<Vec<_>>().join(sep)
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_shapes::term::NamedNode;

    fn pred(iri: &str) -> Path {
        Path::Predicate(NamedNode::new_unchecked(iri))
    }

    #[test]
    fn plain_predicate() {
        assert_eq!(
            render_path(&pred("http://example.org/p")),
            "<http://example.org/p>"
        );
    }

    #[test]
    fn inverse_and_sequence() {
        let seq = Path::Sequence(vec![pred("http://ex/a"), pred("http://ex/b")]);
        assert_eq!(render_path(&seq), "<http://ex/a>/<http://ex/b>");
        let inv = Path::Inverse(Box::new(pred("http://ex/parent")));
        assert_eq!(render_path(&inv), "^<http://ex/parent>");
    }

    #[test]
    fn closures_and_grouping() {
        let star = Path::ZeroOrMore(Box::new(pred("http://ex/next")));
        assert_eq!(render_path(&star), "<http://ex/next>*");
        // A closure over a sequence must parenthesize the sequence.
        let grouped = Path::OneOrMore(Box::new(Path::Sequence(vec![
            pred("http://ex/a"),
            pred("http://ex/b"),
        ])));
        assert_eq!(render_path(&grouped), "(<http://ex/a>/<http://ex/b>)+");
    }

    #[test]
    fn alternative() {
        let alt = Path::Alternative(vec![pred("http://ex/a"), pred("http://ex/b")]);
        assert_eq!(render_path(&alt), "<http://ex/a>|<http://ex/b>");
    }
}
