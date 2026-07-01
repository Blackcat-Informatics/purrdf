# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Compatibility shims layered over the native ``purrdf`` surface.

``purrdf.compat.rdflib`` is the purrdf P0 subset of the eventual P9 public
rdflib drop-in: a pure-Python facade (terms, namespaces, ``Graph``, SPARQL
results, ``Collection``, comparison, format detection) so the internal toolchain
runs with no ``rdflib`` dependency on the default path.
"""
