# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: These cases exercise Python keyword extraction, exceptions and retained extension objects across the actual PyO3 boundary.
"""Recursion request transport through the Python SHACL entry points."""

import pytest

import purrdf

SHAPES = """@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
ex:S a sh:NodeShape ; sh:targetNode ex:a ; sh:property ex:P .
ex:P sh:path ex:p ; sh:property ex:P .
"""
DATA = '<http://example.org/a> <http://example.org/p> <http://example.org/a> .\n'


def request(mode, **overrides):
    return dict(recursion=mode, recursion_max_pairs=2**64 - 1,
                recursion_max_iterations=2**64 - 1) | overrides


@pytest.mark.parametrize("mode,expected", [("lfp", False), ("gfp", True)])
def test_requests_survive_direct_prepared_xpath_and_incremental_execution(mode, expected):
    options = request(mode)
    direct = purrdf.shapes.validate(SHAPES, DATA, **options)
    assert direct["conforms"] is expected
    shapes = purrdf.shapes.Shapes(SHAPES, **options)
    prepared = shapes.prepare()
    for law in (None, *purrdf.XPATH_REGEX_PROFILES):
        for executable in (shapes, prepared):
            report = executable.validate_nt(DATA, xpath_regex=law)
            assert report.conforms is expected
            assert report.results == []
        store = purrdf.Store()
        store.checkpoint()
        store.load(DATA, purrdf.RdfFormat.N_TRIPLES)
        assert shapes.validate_store(store, xpath_regex=law).conforms is expected
        changed = prepared.validate_store_changes(store, xpath_regex=law)
        assert changed.report.conforms is expected


@pytest.mark.parametrize("mode,expected", [("lfp", False), ("gfp", True)])
def test_products_require_current_request_on_every_restore(mode, expected):
    options = request(mode)
    packed = purrdf.shapes.pack_product(SHAPES, **options, canonical_search="1000000").value
    assert packed == purrdf.shapes.Shapes(SHAPES, **options).prepare().to_product(canonical_search="1000000").value
    product = purrdf.shapes.ShapesProduct.open(packed)
    for name in ("admit", "admit_expecting", "rebuild", "rebuild_expecting"):
        args = (product.identity_digest(),) if name.endswith("expecting") else ()
        restore = getattr(product, name)
        with pytest.raises(purrdf.shapes.ShapesProductError) as refused:
            restore(*args)
        assert refused.value.dimension == "malformed"
        assert restore(*args, **options).validate_nt(DATA).conforms is expected


@pytest.mark.parametrize("options", [
    {"recursion": "unknown", "recursion_max_pairs": 1, "recursion_max_iterations": 1},
    {"recursion": "lfp"},
    {"recursion": "gfp", "recursion_max_pairs": 1},
    {"recursion": "lfp", "recursion_max_iterations": 1},
    {"recursion_max_pairs": 1},
    {"recursion_max_iterations": 1},
])
def test_partial_or_unknown_request_fails_before_semantic_shape_loading(options):
    for load in (
        lambda: purrdf.shapes.validate(SHAPES, DATA, **options),
        lambda: purrdf.shapes.Shapes(SHAPES, **options),
        lambda: purrdf.shapes.pack_product(SHAPES, **options, canonical_search="1000000").value,
    ):
        with pytest.raises(ValueError, match="recursion"):
            load()
    assert purrdf.shapes.validate(SHAPES, DATA, **request("gfp"))["conforms"] is True


@pytest.mark.parametrize("limit", ["recursion_max_pairs", "recursion_max_iterations"])
def test_zero_limits_reach_native_refusal_without_partial_report(limit):
    options = request("gfp", **{limit: 0})
    with pytest.raises(ValueError):
        purrdf.shapes.validate(SHAPES, DATA, **options)
    with pytest.raises(ValueError):
        purrdf.shapes.Shapes(SHAPES, **options).prepare().validate_nt(DATA)
    assert purrdf.shapes.validate(SHAPES, DATA, **request("gfp"))["conforms"] is True


def test_omission_keeps_default_cycle_refusal_and_acyclic_behavior():
    with pytest.raises(ValueError, match="cyclic SHACL"):
        purrdf.shapes.Shapes(SHAPES)
    acyclic = SHAPES.replace(" ; sh:property ex:P .\n", " .\n", 1).replace(
        " ; sh:property ex:P", ""
    )
    assert purrdf.shapes.validate(acyclic, DATA)["conforms"] is True
    assert purrdf.shapes.Shapes(acyclic).prepare().validate_nt(DATA).conforms is True
