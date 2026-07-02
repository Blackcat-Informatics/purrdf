# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Top-level ``rdflib`` shadow — an OPT-IN drop-in that resolves to purrdf.

Installing the ``purrdf-rdflib`` distribution (``pip install purrdf[rdflib]``)
adds this package to the environment, which claims the literal ``rdflib`` import
name and re-exports :mod:`purrdf.compat.rdflib`. Third-party code doing a plain
``import rdflib`` then transparently runs on purrdf, with zero source changes.

Single source of truth: this module does **not** duplicate any code. It imports
each ``purrdf.compat.rdflib`` submodule object and registers it in
:data:`sys.modules` under the shadow's dotted name (``rdflib.<sub>``), plus
aliases it as an attribute. So ``from rdflib import Graph``,
``from rdflib.namespace import RDF``, and ``import rdflib.plugins.sparql`` all
resolve to the exact same objects as their ``purrdf.compat.rdflib.*``
counterparts.

Collision caveat: this shadow and the real ``rdflib`` both own the ``rdflib``
import name and MUST NEVER co-inhabit one environment. It is packaged as a
separate distribution (never bundled into the main ``purrdf`` wheel) precisely
so environments that need the genuine rdflib — e.g. purrdf's own differential
test oracle — simply do not install it.
"""

from __future__ import annotations

import importlib
import sys

import purrdf.compat.rdflib as _compat

# Re-export the public surface (`from rdflib import Graph, URIRef, RDF, ...`).
for _name in _compat.__all__:
    globals()[_name] = getattr(_compat, _name)

# Submodules real rdflib code reaches for. Register each purrdf compat submodule
# under the shadow's dotted name so both `import rdflib.<sub>` and
# `from rdflib.<sub> import X` resolve to the single source of truth. Order
# matters: a parent package must be registered before its child (``plugins``
# before ``plugins.sparql``) so the child can be attached to the parent object.
_SUBMODULES: tuple[str, ...] = (
    "term",
    "namespace",
    "graph",
    "query",
    "plugin",
    "parser",
    "serializer",
    "store",
    "collection",
    "compare",
    "util",
    "paths",
    "resource",
    "plugins",
    "plugins.sparql",
)

for _sub in _SUBMODULES:
    _module = importlib.import_module(f"purrdf.compat.rdflib.{_sub}")
    sys.modules[f"{__name__}.{_sub}"] = _module
    _head, _, _tail = _sub.rpartition(".")
    _parent = sys.modules[__name__] if not _head else sys.modules[f"{__name__}.{_head}"]
    setattr(_parent, _tail, _module)

__all__ = [*_compat.__all__, *(_s for _s in _SUBMODULES if "." not in _s)]
