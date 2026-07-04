# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Version invariant: the compat shim's ``__version__`` tracks the locked oracle."""

from __future__ import annotations

import purrdf.compat.rdflib


def test_compat_version_matches_locked_rdflib(locked_rdflib_version: str) -> None:
    """``purrdf.compat.rdflib.__version__`` equals the locked rdflib version."""
    assert purrdf.compat.rdflib.__version__ == locked_rdflib_version
