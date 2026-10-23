# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: drives the shacl_profile keyword of the installed CPython extension's shapes.validate and Shapes entry points

"""The dated SHACL law a caller selects with ``shacl_profile``.

The keyword is decoded once, in Rust, through ``purrdf_validate::complete_profile``. A
keyword that was read but never threaded through would be invisible: the call still
succeeds and the compatibility law answers instead. So the law is pinned by two vendored
SHACL 1.2 suite entries whose answers separate the laws, each refusal beside its valid
neighbour:

* ``sparql/pre-binding/pre-binding-009`` (a ``VALUES`` over a variable that is not
  pre-bound): the compatibility law refuses the shapes graph at load, the editor's-draft
  law (``shacl-ed-2026-10-16``) validates it.
* ``sparql/node/prefixes-003`` (a prefix declared through a version-IRI path the
  2026-09-18 working draft does not define): ``shacl-wd-2026-09-18`` refuses it at load,
  the editor's-draft law validates it.
* ``shacl-ed-2026-10-17`` names no law: ``ValueError`` carrying ``unsupported-profile``.
"""

from __future__ import annotations

import pathlib

import pytest

import purrdf

_SUITE = pathlib.Path(__file__).resolve().parents[3] / "vectors/shacl12/tests"
ED = "shacl-ed-2026-10-16"
WD = "shacl-wd-2026-09-18"
REC = "shacl-rec-2017-07-20"
XPATH20 = "xpath-2.0-2010-12-14"
XPATH31 = "xpath-3.1-2017-03-21"


def _entry(relative: str) -> tuple[str, str, str]:
    """The suite file's shapes graph (Turtle), its IRI, and its data graph (N-Triples)."""
    path = _SUITE / relative
    turtle = path.read_text()
    base = path.as_uri()
    store = purrdf.Store()
    store.load(turtle, purrdf.RdfFormat.TURTLE, base=base)
    return turtle, base, store.dump(format=purrdf.RdfFormat.N_TRIPLES).decode()


def test_pre_binding_009_is_refused_by_the_default_law_and_validated_by_the_editors_draft() -> None:
    shapes_ttl, base, data_nt = _entry("sparql/pre-binding/pre-binding-009.ttl")
    with pytest.raises(ValueError, match="VALUES is not allowed"):
        purrdf.shapes.validate(shapes_ttl, data_nt, shapes_base=base)
    with pytest.raises(ValueError, match="VALUES is not allowed"):
        purrdf.shapes.validate(shapes_ttl, data_nt, shapes_base=base, shacl_profile="legacy")

    report = purrdf.shapes.validate(shapes_ttl, data_nt, shapes_base=base, shacl_profile=ED)
    assert report["conforms"] is False
    assert len(report["results"]) == 1
    assert report["results"][0]["focus"].endswith("InvalidResource>")


def test_prefixes_003_is_refused_by_the_working_draft_and_validated_by_the_editors_draft() -> None:
    shapes_ttl, base, data_nt = _entry("sparql/node/prefixes-003.ttl")
    with pytest.raises(ValueError, match="undeclared prefix"):
        purrdf.shapes.validate(shapes_ttl, data_nt, shapes_base=base, shacl_profile=WD)

    report = purrdf.shapes.validate(shapes_ttl, data_nt, shapes_base=base, shacl_profile=ED)
    assert report["conforms"] is False
    assert len(report["results"]) == 1


def test_the_shapes_constructor_carries_the_law_into_every_validation() -> None:
    shapes_ttl, base, data_nt = _entry("sparql/pre-binding/pre-binding-009.ttl")
    with pytest.raises(ValueError, match="VALUES is not allowed"):
        purrdf.shapes.Shapes(shapes_ttl, base=base)
    shapes = purrdf.shapes.Shapes(shapes_ttl, base=base, shacl_profile=ED)
    assert len(shapes.validate_nt(data_nt).results) == 1
    # The prefix neighbour, the other way round.
    prefixes_ttl, prefixes_base, _ = _entry("sparql/node/prefixes-003.ttl")
    with pytest.raises(ValueError, match="undeclared prefix"):
        purrdf.shapes.Shapes(prefixes_ttl, base=prefixes_base, shacl_profile=WD)
    assert purrdf.shapes.Shapes(prefixes_ttl, base=prefixes_base, shacl_profile=ED)


@pytest.mark.parametrize("unknown", ["shacl-ed-2026-10-17", "SHACL-ED-2026-10-16", "shacl-ed", ""])
def test_an_unknown_profile_id_is_the_unsupported_profile_status(unknown: str) -> None:
    shapes_ttl, base, data_nt = _entry("sparql/node/prefixes-003.ttl")
    with pytest.raises(ValueError, match="unsupported-profile") as validate_refusal:
        purrdf.shapes.validate(shapes_ttl, data_nt, shapes_base=base, shacl_profile=unknown)
    assert repr(unknown) in str(validate_refusal.value).replace("'", '"') or unknown in str(
        validate_refusal.value
    )
    with pytest.raises(ValueError, match="unsupported-profile"):
        purrdf.shapes.Shapes(shapes_ttl, base=base, shacl_profile=unknown)
    # The exact id beside the refused ones is accepted.
    assert purrdf.shapes.validate(shapes_ttl, data_nt, shapes_base=base, shacl_profile=ED)


def test_a_dated_law_refuses_an_xpath_override_of_another_law_and_accepts_its_own() -> None:
    shapes_ttl, base, data_nt = _entry("sparql/node/prefixes-003.ttl")
    with pytest.raises(ValueError, match="requires") as conflict:
        purrdf.shapes.validate(
            shapes_ttl, data_nt, shapes_base=base, shacl_profile=ED, xpath_regex=XPATH20
        )
    assert ED in str(conflict.value)
    assert XPATH20 in str(conflict.value)
    same = purrdf.shapes.validate(
        shapes_ttl, data_nt, shapes_base=base, shacl_profile=ED, xpath_regex=XPATH31
    )
    assert len(same["results"]) == 1
    # The 2017 Recommendation requires XPath 2.0: the pair the other way round.
    with pytest.raises(ValueError, match="requires"):
        purrdf.shapes.validate(
            shapes_ttl, data_nt, shapes_base=base, shacl_profile=REC, xpath_regex=XPATH31
        )
    # The compatibility law leaves the pattern law to the caller.
    legacy = purrdf.shapes.validate(shapes_ttl, data_nt, shapes_base=base, xpath_regex=XPATH20)
    assert len(legacy["results"]) == 1


def test_a_shapes_built_under_a_dated_law_refuses_a_conflicting_xpath_override_later() -> None:
    shapes_ttl, base, data_nt = _entry("sparql/node/prefixes-003.ttl")
    shapes = purrdf.shapes.Shapes(shapes_ttl, base=base, shacl_profile=ED)
    # The same Shapes validates with no override and with its own XPath law...
    assert len(shapes.validate_nt(data_nt).results) == 1
    assert len(shapes.validate_nt(data_nt, xpath_regex=XPATH31).results) == 1
    # ...and refuses the other law rather than substituting it.
    with pytest.raises(ValueError, match="requires"):
        shapes.validate_nt(data_nt, xpath_regex=XPATH20)
