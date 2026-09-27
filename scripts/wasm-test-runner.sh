#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
#
# The cargo runner for wasm32-unknown-unknown test binaries:
#
#     CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=<repo>/scripts/wasm-test-runner.sh \
#         cargo test --target wasm32-unknown-unknown -p <crate> --test <target>
#
# Cargo invokes it as `wasm-test-runner.sh <module.wasm> [test arguments...]`.
# The module is a `harness = false` target on purrdf_testkit::harness. On this
# target the standard library has no command line, environment, console or
# clock, so the harness takes them from a host object that this script's Node
# driver installs:
#
#   1. The module path is made absolute. Cargo may hand over a relative one,
#      and the generated bindings are loaded from another directory.
#   2. The wasm-bindgen CLI generates Node (CommonJS) bindings into a scratch
#      directory under the build tree, never the system temporary directory.
#      The CLI must be exactly the version the root Cargo.toml pins the
#      library to (`wasm-bindgen = "=X.Y.Z"`), and any other version is refused.
#   3. Node runs the driver. It installs the host, loads the bindings (which
#      runs the module's `main`), and exits with the status the harness
#      reported: 0, or 101 when a case failed or the command line was refused.
#      A module that traps or throws exits 101. A module that returns without
#      reporting a status is refused with 101, because a silent module has
#      proved nothing.
#
# The driver also implements the host seal behind
# purrdf_testkit::harness::without_host_clock_or_entropy. For its duration,
# every host clock and entropy source throws an error that names it.
#
# scripts/check-wasm-test-runner.sh observes this runner failing what it must
# fail, beside neighbours that pass.

set -euo pipefail

fail() {
	echo "wasm-test-runner: error: $*" >&2
	exit 101
}

[ "$#" -ge 1 ] || fail "usage: wasm-test-runner.sh <module.wasm> [test arguments...]"
module="$1"
shift
[ -f "$module" ] || fail "no such module: $module"
module_dir="$(cd "$(dirname "$module")" && pwd -P)"
module="$module_dir/$(basename "$module")"

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"

command -v node >/dev/null 2>&1 || fail "node is not on PATH; the module runs in Node"
command -v wasm-bindgen >/dev/null 2>&1 ||
	fail "the wasm-bindgen CLI is not on PATH; install the version root Cargo.toml pins"
pin="$(sed -n 's/^wasm-bindgen = "=\([0-9][0-9.]*\)"$/\1/p' "$root/Cargo.toml")"
[ -n "$pin" ] || fail "no exact wasm-bindgen pin (wasm-bindgen = \"=X.Y.Z\") in $root/Cargo.toml"
found="$(wasm-bindgen --version | sed -n 's/^wasm-bindgen \([0-9][0-9.]*\).*$/\1/p')"
[ "$found" = "$pin" ] ||
	fail "the wasm-bindgen CLI on PATH is ${found:-of unknown version}, but Cargo.toml pins $pin; install wasm-bindgen-cli $pin"

# The scratch directory lives beside the build's own output: the profile
# directory (the parent of `deps/` for a test binary) of the target directory.
profile_dir="$module_dir"
if [ "$(basename "$profile_dir")" = deps ]; then
	profile_dir="$(dirname "$profile_dir")"
fi
scratch_base="$profile_dir/wasm-test-runner"
mkdir -p "$scratch_base"
scratch="$(mktemp -d "$scratch_base/$(basename "$module" .wasm).XXXXXX")"
trap 'rm -rf "$scratch"' EXIT

wasm-bindgen --target nodejs --no-typescript --out-name module --out-dir "$scratch" "$module" ||
	fail "wasm-bindgen could not generate bindings for $module"

cat >"$scratch/driver.cjs" <<'JS'
'use strict';
// Installs the host purrdf_testkit::harness calls on wasm32, loads the module's
// bindings (which runs its `main`), and sets the exit status the harness reported.

const nodeCrypto = require('node:crypto');

const [bindings, ...args] = process.argv.slice(2);

// Must equal MESSAGE_SLOT and ELAPSED_SLOT in crates/testkit/src/host.rs.
const MESSAGE_SLOT = '\0purrdf-testkit-message\0';
const ELAPSED_SLOT = '\0purrdf-testkit-elapsed\0';
const FAILURE_STATUS = 101;

// The harness's own clock, captured before any seal can replace it.
const clock = performance.now.bind(performance);

// ---- The seal --------------------------------------------------------------

const sealedError = (source) =>
  new Error(
    `${source} is sealed: host clocks and entropy sources are withdrawn inside ` +
      'purrdf_testkit::harness::without_host_clock_or_entropy',
  );
const thrower = (source) =>
  function sealed() {
    throw sealedError(source);
  };

const OriginalDate = globalThis.Date;
const sealedDate = new Proxy(OriginalDate, {
  apply() {
    throw sealedError('Date()');
  },
  construct(target, argumentList, newTarget) {
    if (argumentList.length === 0) {
      throw sealedError('new Date()');
    }
    return Reflect.construct(target, argumentList, newTarget);
  },
  get(target, key, receiver) {
    return key === 'now' ? thrower('Date.now') : Reflect.get(target, key, receiver);
  },
});
const sealedHrtime = thrower('process.hrtime');
sealedHrtime.bigint = thrower('process.hrtime.bigint');

// [object, property, replacement] for every clock and entropy source.
const sources = [
  [globalThis, 'Date', sealedDate],
  [performance, 'now', thrower('performance.now')],
  [process, 'hrtime', sealedHrtime],
  [process, 'uptime', thrower('process.uptime')],
  [Math, 'random', thrower('Math.random')],
];
// Web Crypto's methods live on its prototype, which `globalThis.crypto` and
// `node:crypto`'s `webcrypto` share. `node:crypto.getRandomValues` is a
// non-configurable getter over that same method, so sealing the prototype
// covers all three.
const webCrypto = globalThis.crypto || nodeCrypto.webcrypto;
if (webCrypto) {
  const prototype = Object.getPrototypeOf(webCrypto);
  for (const name of ['getRandomValues', 'randomUUID']) {
    sources.push([prototype, name, thrower(`crypto.${name}`)]);
  }
}
for (const name of [
  'randomBytes',
  'randomFill',
  'randomFillSync',
  'randomInt',
  'randomUUID',
  'pseudoRandomBytes',
  'prng',
  'rng',
]) {
  if (name in nodeCrypto) {
    sources.push([nodeCrypto, name, thrower(`node:crypto.${name}`)]);
  }
}

let sealDepth = 0;
let saved = [];

function seal() {
  sealDepth += 1;
  if (sealDepth > 1) {
    return;
  }
  for (const [object, key, replacement] of sources) {
    const descriptor = Object.getOwnPropertyDescriptor(object, key);
    // A property that cannot be replaced would leave a source open while the
    // seal claims it is closed: refuse instead.
    Object.defineProperty(object, key, {
      value: replacement,
      writable: true,
      configurable: true,
      enumerable: descriptor ? descriptor.enumerable : false,
    });
    saved.push([object, key, descriptor]);
  }
}

function unsealAll() {
  for (const [object, key, descriptor] of saved.reverse()) {
    if (descriptor) {
      Object.defineProperty(object, key, descriptor);
    } else {
      delete object[key];
    }
  }
  saved = [];
  sealDepth = 0;
}

function unseal() {
  if (sealDepth === 0) {
    throw new Error('unseal without a matching seal');
  }
  sealDepth -= 1;
  if (sealDepth === 0) {
    unsealAll();
  }
}

// ---- The report a failing case ends the run with ---------------------------

let status = null;
let armed = null;
let reported = false;

function reportArmed(message) {
  if (armed === null || reported) {
    return;
  }
  unsealAll();
  const elapsed = ((clock() - armed.started) / 1000).toFixed(2);
  process.stdout.write(
    armed.report.split(MESSAGE_SLOT).join(message).split(ELAPSED_SLOT).join(elapsed),
  );
  reported = true;
}

const describe = (error) =>
  error instanceof Error ? error.stack || String(error) : String(error);

globalThis.__purrdf_testkit_host = {
  argCount: () => args.length,
  arg: (index) => args[index],
  envVar: (name) => process.env[name],
  writeStdout: (text) => {
    process.stdout.write(text);
  },
  writeStderr: (text) => {
    process.stderr.write(text);
  },
  stdoutIsTerminal: () => Boolean(process.stdout.isTTY),
  nowMillis: () => clock(),
  seal,
  unseal,
  exit: (code) => {
    status = code;
  },
  arm: (name, report, started) => {
    armed = { name, report, started };
  },
  disarm: () => {
    armed = null;
  },
  fail: reportArmed,
};

// ---- The run ---------------------------------------------------------------

let loaded = false;
try {
  require(bindings);
  loaded = true;
} catch (error) {
  unsealAll();
  if (armed !== null && !reported) {
    reportArmed(`thread '${armed.name}' raised a JavaScript exception:\n${describe(error)}\n`);
  }
  if (!reported) {
    process.stderr.write(
      `error: the wasm32 module failed outside any test case:\n${describe(error)}\n`,
    );
  }
  process.exitCode = FAILURE_STATUS;
}
if (loaded) {
  if (status === null) {
    process.stderr.write(
      'error: the wasm32 module returned but never reported an exit status; a test ' +
        'binary run by this runner must end in purrdf_testkit::harness::main\n',
    );
    process.exitCode = FAILURE_STATUS;
  } else {
    process.exitCode = status;
  }
}
JS

set +e
node "$scratch/driver.cjs" "$scratch/module.js" "$@"
status=$?
set -e
exit "$status"
