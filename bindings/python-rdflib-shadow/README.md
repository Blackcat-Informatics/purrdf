<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# purrdf-rdflib — the opt-in `import rdflib` shadow

This is a thin, pure-Python distribution that claims the top-level `rdflib`
import name and re-exports [purrdf](https://github.com/Blackcat-Informatics/purrdf)'s
`purrdf.compat.rdflib` surface. With it installed, third-party code doing a
literal `import rdflib` transparently runs on purrdf — no source changes.

```bash
pip install purrdf[rdflib]
```

```python
import rdflib                       # resolves to purrdf's compat surface
g = rdflib.Graph()
g.parse(data="<http://example.org/s> <http://example.org/p> <http://example.org/o> .", format="nt")
print(g.serialize(format="turtle"))

from rdflib.namespace import RDF    # -> purrdf.compat.rdflib.namespace
from rdflib import URIRef, Literal  # -> purrdf.compat.rdflib.term
import rdflib.plugins.sparql        # -> purrdf.compat.rdflib.plugins.sparql
```

## Collision caveat — do not co-install with real rdflib

This shadow and the genuine [`rdflib`](https://pypi.org/project/rdflib/) both own
the `rdflib` import name and **must never co-inhabit one environment**. Whichever
is resolvable on `sys.path` wins; installing both is unsupported and undefined.
That is why this shadow ships as a **separate** distribution rather than inside
the main `purrdf` wheel: environments that need the real rdflib (for example
purrdf's own differential test oracle) simply do not install `purrdf-rdflib`.
