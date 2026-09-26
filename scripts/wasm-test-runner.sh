#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
#
# Cargo's runner for wasm32-unknown-unknown test binaries: RUN them, in Node.
#
#   CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=scripts/wasm-test-runner.sh \
#       cargo test --locked --target wasm32-unknown-unknown -p <crate> --test <target>
#
# Cargo invokes this script as `wasm-test-runner.sh <module.wasm> <test args…>`.
# Every test target it runs is a `harness = false` binary whose `main` hands its
# cases to `purrdf_testkit::harness`, the same runner the target uses natively, so
# the two targets run the same named cases and print the same libtest console
# lines. The script:
#
#   1. generates the module's Node bindings with the `wasm-bindgen` CLI, at the
#      exact version the root Cargo.toml pins the `wasm-bindgen` library to (the
#      CLI refuses a module built against any other, and a mismatch is reported
#      here by name first);
#   2. loads them in Node with `process.argv` set to the module path followed by
#      the test arguments, which is what the harness reads as its command line.
#      Loading runs the module's start function, which calls `main`;
#   3. exits with the status the harness handed to `process.exitCode`.
#
# A computation a test wraps in `purrdf_testkit::harness::without_host_clock_or_entropy`
# runs with every host clock and entropy source — `Date.now`, `new Date()`,
# `Date()`, `performance.now`, `Math.random`, `crypto.getRandomValues` and
# `crypto.randomUUID` — replaced by a function that throws, naming itself. The
# launcher provides that seal as `globalThis.purrdfWasmTestRunner`. A
# cross-target determinism digest is a claim that an answer is a function of its
# inputs alone, and one that consulted a clock or a random source would agree
# across targets only by accident, so reaching one fails the run loudly. The
# harness times itself with `process.uptime`, which is never sealed.
#
# A panic aborts a wasm32 module, since the target has no unwinding. The harness's
# panic hook prints the failing case, its message and the `FAILED` tally before
# that happens; the module then traps, and this script reports the trap and exits
# 101. A module that returns without setting `process.exitCode` never ran the
# harness — a test file with no `main`, for instance — and that is a failure too,
# never a pass with zero tests.
#
# The bindings are written next to the module, under the cargo target directory.
# Set only by the Makefile's wasm test targets and CI's wasm job; nothing else
# runs wasm32 test binaries.

set -euo pipefail

fail() {
	echo "wasm-test-runner: FAIL: $*" >&2
	exit 101
}

[ "$#" -ge 1 ] || fail "usage: wasm-test-runner.sh <module.wasm> [test arguments...]"
module="$1"
shift
[ -f "$module" ] || fail "no module at $module"
case "$module" in
*.wasm) ;;
*) fail "$module is not a .wasm module" ;;
esac

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
pin="$(sed -n 's/^wasm-bindgen = "=\([0-9][0-9.]*\)"$/\1/p' "$root/Cargo.toml")"
[ -n "$pin" ] || fail "no exact wasm-bindgen pin found in $root/Cargo.toml"

command -v wasm-bindgen >/dev/null 2>&1 ||
	fail "the wasm-bindgen CLI is not on PATH; install wasm-bindgen-cli $pin ('make doctor')"
command -v node >/dev/null 2>&1 || fail "node is not on PATH; the test binary runs in Node ('make doctor')"
found="$(wasm-bindgen --version | sed -n 's/^wasm-bindgen \([0-9][0-9.]*\).*$/\1/p')"
[ "$found" = "$pin" ] ||
	fail "the wasm-bindgen CLI is ${found:-of unknown version}, but Cargo.toml pins the library to $pin; install wasm-bindgen-cli $pin"

stem="$(basename "$module" .wasm)"
bindings="${module%.wasm}.bindgen"
rm -rf "$bindings"
wasm-bindgen --target nodejs --out-dir "$bindings" "$module" ||
	fail "wasm-bindgen could not generate bindings for $module"
[ -f "$bindings/$stem.js" ] || fail "wasm-bindgen wrote no $bindings/$stem.js"

# The launcher. `process.argv` becomes [node, module, ...test arguments]: the
# harness skips the first two entries, as a native binary skips its own name.
# shellcheck disable=SC2016 # the launcher is JavaScript, expanded by Node
launcher='
const [bindings, module, ...args] = process.argv.slice(1);
process.argv = [process.argv[0], module, ...args];
process.exitCode = undefined;

// While a case has sealed the host, every host clock and entropy source throws,
// naming itself. purrdf_testkit::harness::without_host_clock_or_entropy seals
// around a computation whose answer must be a function of its inputs alone (a
// cross-target determinism digest): an answer that consulted a clock or a random
// source agrees across targets only by accident. The harness times itself with
// process.uptime, which is never sealed.
const sealed = (name) => function () {
  throw new Error(`the code under test called ${name} inside without_host_clock_or_entropy; that computation must answer from its inputs alone`);
};
const sources = [
  [Date, "now", "Date.now"],
  [Math, "random", "Math.random"],
  [performance, "now", "performance.now"],
  [globalThis.crypto, "getRandomValues", "crypto.getRandomValues"],
  [globalThis.crypto, "randomUUID", "crypto.randomUUID"],
];
const HostDate = globalThis.Date;
const SealedDate = new Proxy(HostDate, {
  apply: sealed("Date()"),
  construct: sealed("new Date()"),
  get: (target, key) => (key === "now" ? sealed("Date.now") : Reflect.get(target, key)),
});
let saved = null;
globalThis.purrdfWasmTestRunner = Object.freeze({
  sealHost() {
    if (saved !== null) throw new Error("the host is already sealed");
    saved = sources.map(([owner, key]) => Object.getOwnPropertyDescriptor(owner, key));
    for (const [owner, key, name] of sources) {
      Object.defineProperty(owner, key, { value: sealed(name), writable: true, configurable: true });
    }
    globalThis.Date = SealedDate;
  },
  unsealHost() {
    if (saved === null) throw new Error("the host is not sealed");
    sources.forEach(([owner, key], index) => {
      if (saved[index] === undefined) delete owner[key];
      else Object.defineProperty(owner, key, saved[index]);
    });
    globalThis.Date = HostDate;
    saved = null;
  },
});

let loaded = false;
try {
  require(bindings);
  loaded = true;
} catch (error) {
  console.error(`wasm-test-runner: the module trapped or threw: ${error && error.stack ? error.stack : error}`);
}
if (loaded && typeof process.exitCode !== "number") {
  console.error("wasm-test-runner: the module returned without an exit status, so it never ran the purrdf_testkit harness; a wasm32 test target must be harness = false with a main that calls purrdf_testkit::harness::main");
}
// Set, never process.exit(): Node then drains the console before it exits.
if (!loaded || typeof process.exitCode !== "number") {
  process.exitCode = 101;
}
'
exec node -e "$launcher" "$bindings/$stem.js" "$module" "$@"
