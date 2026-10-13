# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: Python type checkers require stubs for the native Python classes.

from __future__ import annotations

class GeoProfile:
    """Immutable strict native profile; omitted input selects standard CRS84."""
    def __init__(self, profile: str | None = ...) -> None: ...
    def to_json(self) -> str: ...

class GeoSession:
    """One Rust session over the strict version-one geographic records."""
    def __init__(self, geo: GeoProfile | GeoSession | str | None = ...) -> None: ...
    def call(self, request: str) -> str:
        """Return canonical semantic outcomes; raise ValueError on operational refusal."""
        ...
    def point_index(self, request: str) -> GeoPointIndex: ...
    def profile(self) -> GeoProfile: ...

class GeoPointIndex:
    """Immutable ordered point buckets, constructed by GeoSession.point_index."""
    def call(self, request: str) -> str:
        """Search through the same native comparison law and refusal channel."""
        ...
