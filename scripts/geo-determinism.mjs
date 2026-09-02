// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

// The wasm32 half of purrdf-geo's cross-target determinism check: instantiate the
// module `scripts/check-geo-determinism.sh` built and print its digest in exactly
// the form the native `geo_digest` example prints, so the shell script compares
// two strings rather than two number types.
//
// # Why the imports are stubbed rather than provided
//
// purrdf-geo depends on purrdf-sparql-eval, which target-gates `js-sys` and
// `wasm-bindgen` on wasm32 to give SPARQL's NOW() and RAND() a browser clock and
// browser entropy. Those are the only host facilities anywhere in the graph, and
// the determinism digest touches neither — it is pure integer arithmetic over a
// constant corpus. So every import is bound to a function that THROWS.
//
// That is the load-bearing choice. Binding them to no-ops, or to a fixed value,
// would let a future change quietly start calling a clock or an entropy source
// and still produce a digest — and a digest that agreed on two targets while one
// of them had consulted a clock would be exactly the false green this whole
// harness exists to prevent. A throwing stub turns "the digest reached a host
// facility" into a loud failure with the import's name in it.

import { readFile } from 'node:fs/promises';

const path = process.argv[2];
if (!path) {
  throw new Error('usage: geo-determinism.mjs <module.wasm>');
}

const bytes = await readFile(path);
const module = await WebAssembly.compile(bytes);

// Build the import object from what the module actually asks for, so a change to
// the dependency graph surfaces as a named throw rather than as an opaque
// LinkError from a hand-maintained list that went stale.
const imports = {};
for (const { module: name, name: field, kind } of WebAssembly.Module.imports(module)) {
  imports[name] ??= {};
  const where = `${name}.${field}`;
  if (kind === 'function') {
    imports[name][field] = () => {
      throw new Error(
        `purrdf-geo's determinism digest called the host import ${where}. ` +
          'The digest must be pure integer arithmetic over a constant corpus; ' +
          'reaching a host facility (a clock, an entropy source, an allocator ' +
          'callback) means the cross-target guarantee no longer holds.',
      );
    };
  } else if (kind === 'global') {
    imports[name][field] = new WebAssembly.Global({ value: 'i32', mutable: false }, 0);
  } else if (kind === 'memory') {
    imports[name][field] = new WebAssembly.Memory({ initial: 1 });
  } else if (kind === 'table') {
    imports[name][field] = new WebAssembly.Table({ initial: 0, element: 'anyfunc' });
  } else {
    throw new Error(`unhandled wasm import kind ${kind} for ${where}`);
  }
}

const instance = await WebAssembly.instantiate(module, imports);

const digest = instance.exports.purrdf_geo_determinism_digest;
const corpusLen = instance.exports.purrdf_geo_determinism_corpus_len;
if (typeof digest !== 'function') {
  throw new Error('the wasm module does not export purrdf_geo_determinism_digest');
}
if (typeof corpusLen !== 'function') {
  throw new Error('the wasm module does not export purrdf_geo_determinism_corpus_len');
}

// A `u64` return reaches JS as a BigInt. Render it in the same zero-padded,
// lower-case, sixteen-hex-digit form the native example prints.
const value = BigInt.asUintN(64, digest());
console.log(`digest=${value.toString(16).padStart(16, '0')}`);
console.log(`corpus_len=${corpusLen()}`);
