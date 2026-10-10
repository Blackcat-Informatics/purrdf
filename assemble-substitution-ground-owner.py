# Stage-only diff assembly; never writes shipping paths.
from pathlib import Path
import difflib

root = Path(__file__).resolve().parents[2]
stage = Path(__file__).resolve().parent
out = stage / "substitution-ground-owner-postimages"
out.mkdir(exist_ok=True)
changes = {}

def snippet(name):
    return (stage / name).read_text()

def put(path, before, after):
    assert before != after, path
    changes[path] = (before, after)
    target = out / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(after)

path = "crates/sparql-eval/src/substitute.rs"
before = (root / path).read_text()
start = before.index("/// Convert a dataset-independent [`TermValue`] to the algebra's [`GroundTerm`].")
end = before.index("#[cfg(test)]\nmod tests {", start)
replacement = snippet("substitution-ground-resident-wrappers.rs") + "\n" + snippet("substitution-ground-owner-draft.rs") + "\n"
put(path, before, before[:start] + replacement + before[end:])

path = "crates/sparql-eval/src/workspace.rs"
before = (root / path).read_text()
marker = "    /// Transfer the native surviving layout's original grant with its payload."
assert before.count(marker) == 1
after = before.replace(marker, snippet("native-text-publication-draft.rs") + "\n" + marker, 1)
last = after.rfind("\n}")
assert after[last + 2:].strip() == "", "regression module is no longer final"
assert after.rfind("mod owned_row_regressions") < last
after = after[:last] + "\n" + snippet("substitution-ground-owner-fixtures.rs") + after[last:]
put(path, before, after)

path = "crates/sparql-algebra/src/error.rs"
before = (root / path).read_text()
marker = "impl fmt::Display for ParseError {"
assert before.count(marker) == 1
after = before.replace(marker, snippet("iri-term-display-draft.rs") + "\n" + marker, 1)
old = '''            Self::Iri { lexical, reason } => {
                write!(f, "invalid IRI {lexical:?} in term position: {reason}")
            }'''
new = '''            Self::Iri { lexical, reason } => {
                fmt::Display::fmt(&IriTermDisplay::new(lexical, reason), f)
            }'''
assert after.count(old) == 1
put(path, before, after.replace(old, new, 1))

path = "crates/sparql-algebra/src/tree.rs"
before = (root / path).read_text()
old = '''child_node!(
    crate::GraphPattern,
    crate::Expression,
    crate::PropertyPathExpression,
    crate::TriplePattern,
    crate::GroundTriple,
);'''
new = '''child_node!(crate::GraphPattern, crate::Expression, crate::PropertyPathExpression);

''' + snippet("native-quoted-triple-drop-draft.rs")
assert before.count(old) == 1
after = before.replace(old, new, 1)
old = '''//! times over and abort the test process'''
after = after.replace(
    "//! children, so no drop recurses more than a fixed number of frames, however deep\n//! the tree.",
    "//! children, so no drop recurses more than a fixed number of frames, however deep\n//! the tree. Quoted ground and pattern triples instead rotate their original child\n//! slots through the shared allocation-free dismantling body; their drop needs no\n//! work-list allocation, including after admission failure.", 1,
)
after = after.replace(
    "/// node to [`Subtree`]'s iterative release (see the [module docs](self)).",
    "/// node to its shared iterative dismantling law (see the [module docs](self)).", 1,
)
put(path, before, after)

path = "crates/sparql-algebra/tests/deep_trees.rs"
before = (root / path).read_text()
put(path, before, before.rstrip() + "\n\n" + snippet("native-quoted-triple-drop-fixture.rs"))

def emit():
    patch = "".join(
        "".join(difflib.unified_diff(
            before.splitlines(keepends=True), (out / path).read_text().splitlines(keepends=True),
            fromfile="a/" + path, tofile="b/" + path,
        )) for path, (before, _) in changes.items()
    )
    (stage / "substitution-ground-owner-draft.patch").write_text(patch)

emit()
print("Wrote standard Stage patch for", len(changes), "shipping homes; no shipping writes.")
