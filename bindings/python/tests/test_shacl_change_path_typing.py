# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""The incremental SHACL change lane, EXERCISED through its declarations.

``test_shacl_change_path.py`` proves the lane's behaviour. This proves the other half
of shipping it: that a typed consumer can write the call at all.

This package ships ``py.typed``, so ``purrdf/__init__.pyi`` is the compiled
extension's only declaration — a type checker reads it and never the ``.so``. A
binding method the stub does not declare is live at runtime and invisible to every
checked caller, which is a failure mode with no runtime symptom whatsoever: the
tests pass, the code runs, and the user's own type checker tells them their correct
call is wrong. That is exactly how ``Store.checkpoint``,
``Store.change_size``, ``PreparedShapes.validate_store_changes`` and the whole
``ChangeValidation`` type shipped undeclared.

So :func:`summarize_change` below is written the way a downstream consumer writes it
— every value pinned to the type the stub promises — and two tests hold it to that:

* one RUNS it, on both the bounded and the unbounded arm, so the annotations are
  checked against what the extension actually hands back;
* one runs ``mypy --strict`` over this very module, so the annotations are checked
  against the shipped stub. An undeclared member fails that run with
  ``[attr-defined]``, and a member declared with the wrong type fails it with
  ``[assignment]``.

Declaring a member is only half of it: the name has to be legal where a consumer WRITES
it. Every step of the chain — ``Shapes``, ``PreparedShapes``, ``ChangeValidation``,
``ValidationReport`` — is therefore annotated in PARAMETER position here, because each
is reachable only as ``purrdf.shapes.X`` and a namespace re-export spelled as a plain
assignment is a variable to a type checker, rejected in annotation position and taking
every method call on it down with a second ``[attr-defined]``. Covering one name and
not its siblings leaves the hole open on the siblings, which is why all four cross a
function boundary below rather than only the outcome type.

``scripts/check-python-stub-parity.py`` is the gate that makes an undeclared member
impossible to commit; this is the test that makes a WRONGLY declared one impossible
to ship, because parity is about names and these assignments are about types.

Fixtures use ``example.org``.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import purrdf

# Core constraints only — `ex:age` must be present and an integer — which is what
# makes this shapes graph's change footprint boundable.
_SHAPES = """@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:age ; sh:datatype xsd:integer ; sh:minCount 1 ] .
"""

# The same obligation, read through SPARQL query text. No bounded footprint exists for
# such a graph, so this is the fallback arm: `bounded` is False, `focus_nodes` is None
# and `reason` names the construct responsible.
_SPARQL_SHAPES = """@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .

ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:sparql [
        a sh:SPARQLConstraint ;
        sh:message "every person needs an integer age" ;
        sh:select \"\"\"SELECT $this WHERE {
            $this a <http://example.org/Person> .
            FILTER NOT EXISTS {
                $this <http://example.org/age> ?a .
                FILTER(datatype(?a) = <http://www.w3.org/2001/XMLSchema#integer>)
            }
        }\"\"\"
    ] .
"""

_BASE_NT = (
    "<http://example.org/bob> "
    "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> "
    "<http://example.org/Person> .\n"
    '<http://example.org/bob> <http://example.org/age> "42"'
    "^^<http://www.w3.org/2001/XMLSchema#integer> .\n"
)

_RDF_TYPE = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type"


class Summary:
    """What one typed caller reads off a change validation.

    A plain class rather than a tuple so each field carries the type the stub promises
    for the member it came from, and a drift between the two is a type error here.
    """

    def __init__(
        self,
        *,
        added: int,
        removed: int,
        bounded: bool,
        focus_nodes: int | None,
        reason: str | None,
        conforms: bool,
    ) -> None:
        self.added = added
        self.removed = removed
        self.bounded = bounded
        self.focus_nodes = focus_nodes
        self.reason = reason
        self.conforms = conforms


def read_scope(outcome: purrdf.shapes.ChangeValidation) -> str:
    """The scope a validation answered for, rendered.

    `ChangeValidation` in ANNOTATION position, which is how a consumer factors a
    handler out of the call site — and which only works because the stub spells the
    namespace member as a `TypeAlias` rather than a plain assignment.
    """
    if outcome.bounded:
        return f"bounded({outcome.focus_nodes})"
    reason = outcome.reason
    assert reason is not None, "the unbounded arm always names its cause"
    return f"everything({reason})"


# The three helpers below take the REST of the SHACL chain in parameter position, for
# the same reason and against the same hole. `ChangeValidation` alone is the outcome
# type; a consumer that factors the lane into functions — loads shapes once here, hands
# the prepared object to a handler there, reads the report somewhere else — has to
# write `Shapes`, `PreparedShapes` and `ValidationReport` in signatures too, and each
# is reachable ONLY as `purrdf.shapes.X` (there is no `purrdf.PreparedShapes`, and
# `__all__` carries none of them). A plain `X = _X` for any one of them is that
# consumer's `Variable "..." is not valid as a type`, followed by the same
# `[attr-defined]` on every method they then call. Annotating one member and not its
# siblings leaves the hole open on the siblings, so all four are exercised here.


def prepare_once(shapes: purrdf.shapes.Shapes) -> purrdf.shapes.PreparedShapes:
    """`Shapes` in, `PreparedShapes` out — the step a caller hoists out of a loop."""
    return shapes.prepare()


def validate_changes(
    prepared: purrdf.shapes.PreparedShapes, store: purrdf.Store
) -> purrdf.shapes.ChangeValidation:
    """`PreparedShapes` in parameter position, and the change call made ON it.

    The `[attr-defined]` this catches is the exact one the stub work was raised to fix:
    a plain assignment makes `prepared` an untyped variable, and
    `prepared.validate_store_changes(store)` is then an attribute of nothing.
    """
    return prepared.validate_store_changes(store)


def render_report(report: purrdf.shapes.ValidationReport) -> str:
    """`ValidationReport` in parameter position, and its members read off it."""
    if report.conforms:
        return "conforms"
    return f"violations({len(report.results)})"


def summarize_change(shapes_ttl: str, base_nt: str, quads: list[purrdf.Quad]) -> Summary:
    """Load, checkpoint, mutate, and validate only what the mutation can move."""
    store = purrdf.Store()
    store.load(base_nt, purrdf.RdfFormat.N_TRIPLES)
    store.checkpoint()

    settled: tuple[int, int] = store.change_size()
    assert settled == (0, 0), "a checkpoint leaves the pending change empty"

    for quad in quads:
        store.add(quad)
    added, removed = store.change_size()

    prepared = purrdf.shapes.Shapes(shapes_ttl).prepare()
    outcome = prepared.validate_store_changes(store)

    bounded: bool = outcome.bounded
    focus_nodes: int | None = outcome.focus_nodes
    reason: str | None = outcome.reason
    conforms: bool = outcome.report.conforms
    return Summary(
        added=added,
        removed=removed,
        bounded=bounded,
        focus_nodes=focus_nodes,
        reason=reason,
        conforms=conforms,
    )


def _iri(value: str) -> purrdf.NamedNode:
    return purrdf.NamedNode(value)


def _new_person(subject: str) -> list[purrdf.Quad]:
    """A person with no age at all — a violation of both shapes graphs above."""
    return [purrdf.Quad(_iri(subject), _iri(_RDF_TYPE), _iri("http://example.org/Person"))]


def test_the_typed_lane_runs_on_the_bounded_arm() -> None:
    """Every annotation in `summarize_change` is checked against a real answer."""
    summary = summarize_change(_SHAPES, _BASE_NT, _new_person("http://example.org/alice"))

    assert (summary.added, summary.removed) == (1, 0)
    assert summary.bounded is True
    assert summary.focus_nodes == 1
    assert summary.reason is None
    assert summary.conforms is False, "the new person has no age"


def test_the_typed_lane_runs_on_the_unbounded_arm() -> None:
    """The arm where `focus_nodes` is `None` and `reason` is a `str`.

    The neighbouring case to the one above, and the reason the two members are
    declared optional: a stub that typed `focus_nodes` as a plain `int` would type
    check every line of `summarize_change` and be wrong here, where the honest answer
    is "every focus node" rather than a number.
    """
    summary = summarize_change(
        _SPARQL_SHAPES, _BASE_NT, _new_person("http://example.org/alice")
    )

    assert summary.bounded is False
    assert summary.focus_nodes is None
    assert isinstance(summary.reason, str) and summary.reason
    assert summary.conforms is False


def test_read_scope_renders_both_arms() -> None:
    """`ChangeValidation` used in annotation position, on both arms."""
    store = purrdf.Store()
    store.load(_BASE_NT, purrdf.RdfFormat.N_TRIPLES)
    store.checkpoint()
    for quad in _new_person("http://example.org/alice"):
        store.add(quad)

    bounded = purrdf.shapes.Shapes(_SHAPES).prepare().validate_store_changes(store)
    unbounded = purrdf.shapes.Shapes(_SPARQL_SHAPES).prepare().validate_store_changes(store)

    assert read_scope(bounded) == "bounded(1)"
    assert read_scope(unbounded).startswith("everything(")


def test_the_whole_chain_runs_through_its_annotations() -> None:
    """`Shapes` → `PreparedShapes` → `ChangeValidation` → `ValidationReport`, each
    crossing a function boundary under the name a consumer must write.

    The companion to the `mypy` test below: that one proves the four names are legal in
    annotation position, this one proves the values really are what those annotations
    claim, on both a conforming store and a violating one.
    """
    store = purrdf.Store()
    store.load(_BASE_NT, purrdf.RdfFormat.N_TRIPLES)
    store.checkpoint()

    prepared = prepare_once(purrdf.shapes.Shapes(_SHAPES))

    settled = validate_changes(prepared, store)
    assert read_scope(settled) == "bounded(0)", "a checkpoint leaves nothing to revisit"
    assert render_report(settled.report) == "conforms"

    for quad in _new_person("http://example.org/alice"):
        store.add(quad)

    moved = validate_changes(prepared, store)
    assert read_scope(moved) == "bounded(1)"
    assert render_report(moved.report) == "violations(1)", "the new person has no age"


def test_mypy_accepts_this_module_against_the_shipped_stub(tmp_path: Path) -> None:
    """The declarations, checked by the tool a consumer actually uses.

    Over THIS file, because the repository's own `mypy` target checks
    `python/src/purrdf` — the shim and the compat layer — and an omission in the stub
    is invisible from inside the package that ships it. A consumer's file is the only
    vantage point from which `"Store" has no attribute "checkpoint"` is observable,
    which is precisely why the omission survived to release.

    `mypy` is a declared dev dependency, so a missing one is a failure and never a
    skip: a check that skips by default is a check nobody is running.
    """
    here = Path(__file__).resolve()
    config = here.parent.parent / "pyproject.toml"
    proc = subprocess.run(
        [
            sys.executable,
            "-m",
            "mypy",
            "--strict",
            "--no-incremental",
            f"--cache-dir={tmp_path / 'mypy-cache'}",
            f"--config-file={config}",
            str(here),
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    assert "No module named mypy" not in proc.stderr, (
        "mypy is a declared dev dependency of this project and is not installed in the "
        f"environment running the tests: {proc.stderr}"
    )
    assert proc.returncode == 0, (
        "mypy rejects a correct use of the change lane, which means the shipped "
        f"declarations are wrong or missing:\n{proc.stdout}\n{proc.stderr}"
    )
