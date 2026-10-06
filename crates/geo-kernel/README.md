<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# purrdf-geo-kernel

The shared geometry engine for PurRDF. Exact rational coordinates, carrier codecs,
topology and reusable spatial indexes live here independently of SPARQL execution.
The `purrdf-geo` facade retains established Rust import paths; evaluator adapters
live in `purrdf-sparql-eval`.

`make geo-determinism` executes frozen planar carrier, integer cell hierarchy,
and completed geodesy, operation, metric, buffer, cover and index bytes on
native, portable wasm and SIMD wasm. The numerical replay exercises every
available endpoint-product backend. Missing execution prerequisites refuse the
gate; compilation alone does not establish cross-target answers.

Geographic relation matrices partition the complete selected physical set into
its actual strata. Isolated retained nodes have point interior; retained open
curves have curve interior and a mod-two endpoint boundary after duplicate
physical fragments are removed. Ordinary areal faces retain their physical
interior and boundary. Adding a redundant point or curve to an areal boundary
does not change that set's topology. Ambient closed-region membership and
complete box separation remain distinct proof labels used by area and covers.

## License

Licensed under any one of the following, at your option:

- [MIT license](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MIT)
- [Apache License, Version 2.0](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-APACHE)
- [Mulan Permissive Software License, Version 2 (MulanPSL-2.0)](https://github.com/Blackcat-Informatics/purrdf/blob/main/LICENSE-MULAN)
