# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Compatibility shim: purrdf → purrdf_native.rdf.

Single-cdylib unification: all five native extensions now live in one
`purrdf_native` cdylib. This shim swaps itself for the real submodule so the
legacy `import purrdf` returns the exact submodule object — same pyclasses.

It also mirrors the Rust `purrdf` umbrella crate's module layout — the RDF surface
at the root, and every other engine under a stable top-level submodule
(`purrdf.shapes`, `purrdf.shex`, `purrdf.slice`, `purrdf.gts`) — so **no caller
ever reaches into `purrdf_native`**. Each submodule is both attached as an
attribute (`purrdf.shapes.validate(...)`) and registered in `sys.modules` (so
`import purrdf.shapes` resolves too).

The hand-written `__init__.pyi` stub + PEP 561 `py.typed` marker beside this file
keep mypy type-checking every `purrdf` call site (the native oxigraph
Store/SPARQL/parse/canonicalize surface).
"""

import sys
from types import ModuleType

from .purrdf_native import rdf as _module
from .purrdf_native import shacl as _shacl
from .purrdf_native import shex as _shex
from .purrdf_native import slice as _slice

# PyO3 submodules carry no `__file__`. The legacy top-level name is expected to be
# locatable (CI imports it and reads `__file__`, and tooling/tracebacks expect it),
# so point the submodule at this shim before swapping it in.
_module.__file__ = __file__

# Make the swapped native module a *package* for the import system: pure-Python
# subpackages of `purrdf` (e.g. `purrdf.compat.rdflib`, the purrdf P0 shim)
# live beside this file on disk. After the `sys.modules` swap below the path-based
# finder resolves `purrdf.<subpkg>` against `__path__`; without this the native
# module object carries no `__path__` and `import purrdf.compat.rdflib` would
# fail. The native term/store names (`purrdf.NamedNode`, …) stay served by the
# swapped module object, so both halves coexist.
_module.__path__ = __path__
_module.__package__ = __name__

# ── Top-level submodules mirroring the Rust umbrella crate ───────────────────────
#
# The Rust `purrdf` crate carries SHACL as the `shapes` module, ShEx as `shex`,
# slice tooling as `slice`, and the GTS container engine as `gts`. Present the
# same shape here. `shapes` is the canonical name (Rust parity); `shacl` is kept
# as a back-compat alias for the native submodule's own name.
_gts = ModuleType(f"{__name__}.gts")
_gts.__doc__ = (
    "The GTS container surface (also available at the purrdf root), grouped to "
    "mirror the Rust umbrella crate's `purrdf::gts` module."
)
# The GTS entry points are registered natively onto the root `rdf` module; gather
# them under `purrdf.gts` for discoverability. Names absent at runtime are skipped.
_GTS_EXPORTS = (
    "gts_from_quads",
    "gts_from_rdf12_bytes",
    "compile_gts_native",
    "snapshot_content_id_native",
    "feedback_bundle_native",
    "to_json_ld",
    "from_json_ld",
    "to_rdf_xml",
    "from_rdf_xml",
    "RdfDataset",
    "GtsFoldViewNative",
    "gts_relational_rows_from_bytes",
    "gts_to_sqlite",
    "gts_to_duckdb",
    "gts_to_parquet",
)
for _name in _GTS_EXPORTS:
    _value = getattr(_module, _name, None)
    if _value is not None:
        setattr(_gts, _name, _value)
setattr(_gts, "__all__", [n for n in _GTS_EXPORTS if hasattr(_gts, n)])

# Attach every engine to the swapped module by attribute access …
_module.shapes = _shacl
_module.shacl = _shacl  # back-compat alias for the native submodule name
_module.shex = _shex
_module.slice = _slice
_module.gts = _gts

# … and swap the package in, then register each submodule in `sys.modules` so a
# real `import purrdf.<engine>` resolves (attribute access alone is not enough).
sys.modules[__name__] = _module
sys.modules[f"{__name__}.shapes"] = _shacl
sys.modules[f"{__name__}.shacl"] = _shacl
sys.modules[f"{__name__}.shex"] = _shex
sys.modules[f"{__name__}.slice"] = _slice
sys.modules[f"{__name__}.gts"] = _gts
