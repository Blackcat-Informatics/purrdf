<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Upstream license declaration

The pinned OBO Graphs 0.3.2 repository does not contain a standalone license
file or per-schema copyright header. Its Maven POM declares:

```xml
<license>
    <name>The BSD 3-Clause License</name>
    <url>https://opensource.org/licenses/BSD-3-Clause</url>
    <distribution>repo</distribution>
</license>
```

The complete, byte-identical upstream POM is retained as `pom.upstream.xml` so
the declaration travels with the vendored schema rather than depending on a
mutable web page. The standard license text is retained at
`LICENSES/BSD-3-Clause.txt`, and `REUSE.toml` applies that identifier to every
byte-identical upstream file. The schema files remain under those upstream BSD
3-Clause terms. This explanatory file and `UPSTREAM.md` are licensed CC-BY-4.0
under their SPDX headers.
