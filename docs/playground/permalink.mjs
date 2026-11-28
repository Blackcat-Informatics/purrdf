// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
// Why not Rust: URL sharing uses the browser's UTF-8 and compression interfaces.

const b64urlEncode = (bytes) => {
  let bin = "";
  for (const byte of bytes) bin += String.fromCharCode(byte);
  return btoa(bin).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
};

const b64urlDecode = (s) => {
  const pad = s.length % 4 === 0 ? "" : "=".repeat(4 - (s.length % 4));
  const bin = atob(s.replace(/-/g, "+").replace(/_/g, "/") + pad);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return bytes;
};

async function streamThrough(stream, bytes) {
  const writer = stream.writable.getWriter();
  writer.write(bytes);
  writer.close();
  const chunks = [];
  const reader = stream.readable.getReader();
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    chunks.push(value);
  }
  let len = 0;
  for (const c of chunks) len += c.length;
  const out = new Uint8Array(len);
  let off = 0;
  for (const c of chunks) {
    out.set(c, off);
    off += c.length;
  }
  return out;
}

/** Encode state → (gzip when available) → base64url. All client-side, no network. */
export async function encodeState(state) {
  const json = new TextEncoder().encode(JSON.stringify(state));
  if (typeof CompressionStream === "function") {
    const gz = await streamThrough(new CompressionStream("gzip"), json);
    return `g${b64urlEncode(gz)}`;
  }
  // Documented capability fallback: plain base64url of the JSON.
  return `j${b64urlEncode(json)}`;
}

export async function decodeState(fragment) {
  if (!fragment) return null;
  const tag = fragment[0];
  const body = fragment.slice(1);
  const bytes = b64urlDecode(body);
  let jsonBytes = bytes;
  if (tag === "g") {
    if (typeof DecompressionStream !== "function") {
      throw new Error("this permalink is gzip-encoded but DecompressionStream is unavailable");
    }
    jsonBytes = await streamThrough(new DecompressionStream("gzip"), bytes);
  } else if (tag !== "j") {
    throw new Error("unrecognized permalink encoding");
  }
  return JSON.parse(new TextDecoder().decode(jsonBytes));
}
