// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: JavaScript owns typed-array brands, buffer views and intrinsic
// copies; Rust admits native storage before asking this module to fill it.

const Bytes = Uint8Array;
const apply = Reflect.apply;
const descriptor = Object.getOwnPropertyDescriptor;
const typedArray = Object.getPrototypeOf(Bytes.prototype);
const tag = descriptor(typedArray, Symbol.toStringTag).get;
const length = descriptor(typedArray, "byteLength").get;
const offset = descriptor(typedArray, "byteOffset").get;
const buffer = descriptor(typedArray, "buffer").get;
const arrayBufferLength = descriptor(ArrayBuffer.prototype, "byteLength").get;
const set = Bytes.prototype.set;
const empty = new Bytes(0);

// The returned fixed-length view retains the buffer without copying its bytes.
export function captureBytes(value) {
  if (apply(tag, value, []) !== "Uint8Array") {
    throw new TypeError("YAML source buffers must be Uint8Array values");
  }
  const backing = apply(buffer, value, []);
  // This intrinsic accepts ArrayBuffer only, explicitly refusing shared storage.
  try {
    apply(arrayBufferLength, backing, []);
  } catch (cause) {
    throw new TypeError("YAML source buffers require non-shared ArrayBuffer storage", { cause });
  }
  // Validate detached or out-of-bounds views without changing any caller bytes.
  apply(set, value, [empty]);
  return new Bytes(backing, apply(offset, value, []), apply(length, value, []));
}

// Return a host number intact; Rust checks its portable range before conversion.
export function byteLength(value) {
  return apply(length, value, []);
}

export function copyBytes(source, destination) {
  if (apply(length, source, []) !== apply(length, destination, [])) {
    throw new TypeError("YAML source buffer changed before its admitted copy");
  }
  apply(set, destination, [source]);
}

export function ownedBytes(source) {
  const output = new Bytes(apply(length, source, []));
  apply(set, output, [source]);
  return output;
}
