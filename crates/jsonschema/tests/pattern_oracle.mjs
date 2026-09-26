// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

/**
 * The ECMA-262 oracle for purrdf-jsonschema's `pattern` translation.
 *
 * JavaScript's own `RegExp`, with the `u` flag JSON Schema 2020-12 patterns
 * are read under, answers every (pattern, string) pair of:
 *
 *   - the official suite's pattern cases: every `pattern` value and
 *     `patternProperties` key in the vendored draft 2020-12 tests, paired
 *     with every string (and object key) of every test instance;
 *   - a seeded corpus of 5,000 generated pairs over the constructs the
 *     translation runs — literals, escapes, classes, the `\d \w \s` sets,
 *     `.`, anchors, word boundaries, property escapes, groups, alternation
 *     and quantifiers — with a tenth of its patterns deliberately malformed;
 *   - a fixed syntax list, including the constructs the translation refuses
 *     (lookaround, backreferences, modifiers), whose only answer is whether
 *     the pattern is ECMA-262 at all.
 *
 * The answers are frozen in `pattern_differential_vectors.txt` in the
 * workspace's self-hashing vector format, and `pattern_differential.rs`
 * replays them against the Rust translation.
 *
 *   node crates/jsonschema/tests/pattern_oracle.mjs          # re-ask Node; fail on drift
 *   node crates/jsonschema/tests/pattern_oracle.mjs --write  # rewrite the vectors
 */

import { createHash } from "node:crypto";
import { readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HERE = path.dirname(fileURLToPath(import.meta.url));
const SUITE = path.join(HERE, "suite", "tests", "draft2020-12");
const VECTORS = path.join(HERE, "pattern_differential_vectors.txt");
const SEED = 0x5eed2020;
const CORPUS_PAIRS = 5000;
const STRINGS_PER_PATTERN = 5;

// ---------------------------------------------------------------- the suite

function jsonFiles(directory) {
  const found = [];
  for (const name of readdirSync(directory).sort()) {
    const full = path.join(directory, name);
    if (statSync(full).isDirectory()) {
      found.push(...jsonFiles(full));
    } else if (name.endsWith(".json")) {
      found.push(full);
    }
  }
  return found;
}

function collectPatterns(value, into) {
  if (Array.isArray(value)) {
    for (const item of value) collectPatterns(item, into);
  } else if (value !== null && typeof value === "object") {
    for (const [key, member] of Object.entries(value)) {
      if (key === "pattern" && typeof member === "string") into.push(member);
      if (key === "patternProperties" && member !== null && typeof member === "object") {
        into.push(...Object.keys(member));
      }
      collectPatterns(member, into);
    }
  }
}

function collectStrings(value, into) {
  if (typeof value === "string") {
    into.push(value);
  } else if (Array.isArray(value)) {
    for (const item of value) collectStrings(item, into);
  } else if (value !== null && typeof value === "object") {
    for (const [key, member] of Object.entries(value)) {
      into.push(key);
      collectStrings(member, into);
    }
  }
}

function suitePairs() {
  const pairs = [];
  for (const file of jsonFiles(SUITE)) {
    for (const group of JSON.parse(readFileSync(file, "utf8"))) {
      const patterns = [];
      collectPatterns(group.schema, patterns);
      if (patterns.length === 0) continue;
      const strings = [];
      for (const test of group.tests) collectStrings(test.data, strings);
      for (const pattern of patterns) {
        for (const input of strings) pairs.push([pattern, input]);
      }
    }
  }
  return pairs;
}

// ------------------------------------------------------ the seeded corpus

function mulberry32(seed) {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const random = mulberry32(SEED);
const below = (n) => Math.floor(random() * n);
const pick = (items) => items[below(items.length)];

// Every character here is assigned in Unicode 15.0 or earlier, so the
// answer cannot depend on which Unicode version either engine carries.
const ALPHABET = [
  "a", "b", "c", "x", "Z", "0", "1", "7", "9", "_", "-", ".", " ", "/",
  "\t", "\n", "\r", "\u000b", "\u000c", "\u00a0", "\u1680", "\u180e",
  "\u2003", "\u200b", "\u2028", "\u2029", "\u202f", "\u3000", "\ufeff",
  "\u00e9", "\u00df", "\u00b5", "\u00aa", "\u0301", "\u200d", "\u212a",
  "\u017f", "\u0130", "\u03b1", "\u03a9", "\u0436", "\u05d0", "\u0628",
  "\u0663", "\u0915", "\u096f", "\u09ea", "\uff19", "\u4e2d", "\u3042",
  "\u{1f432}", "\u{1f600}", "\u{10400}",
];

const PROPERTIES = [
  "L", "Lu", "Ll", "Lt", "Lo", "N", "Nd", "P", "S", "Z", "Zs", "M", "Mn",
  "Letter", "Uppercase_Letter", "digit", "punct", "Cased_Letter",
  "General_Category=Nd", "gc=Lu", "Script=Greek", "sc=Latn", "Script=Han",
  "scx=Hira", "Script_Extensions=Cyrillic", "sc=Arab", "White_Space",
  "Alphabetic", "ASCII", "Any", "Assigned", "Emoji", "ID_Start",
  "ID_Continue", "Lowercase", "Uppercase", "Hex_Digit", "Dash",
];

const SYNTAX = new Set(["^", "$", "\\", ".", "*", "+", "?", "(", ")", "[", "]", "{", "}", "|", "/"]);

function hex(code, width) {
  return code.toString(16).padStart(width, "0");
}

function literal(ch) {
  if (SYNTAX.has(ch)) return `\\${ch}`;
  const code = ch.codePointAt(0);
  // Spell a quarter of the literals as escapes, to exercise every form.
  switch (below(8)) {
    case 0:
      return code <= 0xff ? `\\x${hex(code, 2)}` : `\\u{${hex(code, 1)}}`;
    case 1:
      return code <= 0xffff
        ? `\\u${hex(code, 4)}`
        : [...String.fromCodePoint(code)]
            .join("")
            .split("")
            .map((unit) => `\\u${hex(unit.charCodeAt(0), 4)}`)
            .join("");
    default:
      if (ch === "\t") return "\\t";
      if (ch === "\n") return "\\n";
      if (ch === "\r") return "\\r";
      if (ch === "\u000b") return "\\v";
      if (ch === "\u000c") return "\\f";
      return ch;
  }
}

function classAtom() {
  const ch = pick(ALPHABET);
  if (ch === "]" || ch === "\\" || ch === "^" || ch === "-") return `\\${ch}`;
  return literal(ch).replace(/^\\([.*+?(){}|$/])$/, "$1");
}

function classItem() {
  switch (below(6)) {
    case 0:
      return pick(["\\d", "\\D", "\\w", "\\W", "\\s", "\\S"]);
    case 1:
      return `\\${pick(["p", "P"])}{${pick(PROPERTIES)}}`;
    case 2: {
      const a = pick(ALPHABET).codePointAt(0);
      const b = pick(ALPHABET).codePointAt(0);
      const [low, high] = a <= b ? [a, b] : [b, a];
      const spell = (code) => {
        const ch = String.fromCodePoint(code);
        return ch === "]" || ch === "\\" || ch === "^" || ch === "-" ? `\\${ch}` : `\\u{${hex(code, 1)}}`;
      };
      return `${spell(low)}-${spell(high)}`;
    }
    default:
      return classAtom();
  }
}

function atom(depth) {
  switch (below(depth > 2 ? 9 : 12)) {
    case 0:
      return ".";
    case 1:
      return pick(["\\d", "\\D", "\\w", "\\W", "\\s", "\\S"]);
    case 2:
      return `\\${pick(["p", "P"])}{${pick(PROPERTIES)}}`;
    case 3: {
      const items = Array.from({ length: 1 + below(4) }, classItem).join("");
      return `[${below(3) === 0 ? "^" : ""}${items}]`;
    }
    case 4:
      return pick(["\\cJ", "\\cj", "\\0", "\\u{1F432}", "\\uD83D\\uDC32", "\\x41"]);
    case 9:
      return `(${disjunction(depth + 1)})`;
    case 10:
      return `(?:${disjunction(depth + 1)})`;
    case 11:
      return `(?<g${below(1_000_000)}>${disjunction(depth + 1)})`;
    default:
      return literal(pick(ALPHABET));
  }
}

function quantifier() {
  const lazy = below(4) === 0 ? "?" : "";
  switch (below(10)) {
    case 0:
      return `*${lazy}`;
    case 1:
      return `+${lazy}`;
    case 2:
      return `?${lazy}`;
    case 3: {
      const low = below(3);
      return `{${low}}${lazy}`;
    }
    case 4: {
      const low = below(3);
      return `{${low},}${lazy}`;
    }
    case 5: {
      const low = below(3);
      return `{${low},${low + below(3)}}${lazy}`;
    }
    default:
      return "";
  }
}

function term(depth) {
  switch (below(14)) {
    case 0:
      return "^";
    case 1:
      return "$";
    case 2:
      return pick(["\\b", "\\B"]);
    default:
      return atom(depth) + quantifier();
  }
}

function alternative(depth) {
  return Array.from({ length: below(4) + (depth === 0 ? 1 : 0) }, () => term(depth)).join("");
}

function disjunction(depth) {
  const parts = [alternative(depth)];
  while (below(depth > 1 ? 8 : 4) === 0) parts.push(alternative(depth));
  return parts.join("|");
}

/** A malformed variant: each is a syntax error under the `u` flag. */
function corrupt(pattern) {
  switch (below(9)) {
    case 0:
      return `${pattern}(`;
    case 1:
      return `${pattern})`;
    case 2:
      return `${pattern}{`;
    case 3:
      return `${pattern}]`;
    case 4:
      return `${pattern}\\a`;
    case 5:
      return `[z-a]${pattern}`;
    case 6:
      return `${pattern}\\p{letter}`;
    case 7:
      return `*${pattern}`;
    default:
      return `${pattern}\\u{110000}`;
  }
}

function randomString() {
  return Array.from({ length: below(9) }, () => pick(ALPHABET)).join("");
}

function corpusPairs() {
  const pairs = [];
  const seen = new Set();
  while (pairs.length < CORPUS_PAIRS) {
    let pattern = disjunction(0);
    if (below(10) === 0) pattern = corrupt(pattern);
    for (let index = 0; index < STRINGS_PER_PATTERN && pairs.length < CORPUS_PAIRS; index += 1) {
      const input = randomString();
      const key = JSON.stringify([pattern, input]);
      if (!seen.has(key)) {
        seen.add(key);
        pairs.push([pattern, input]);
      }
    }
  }
  return pairs;
}

// ---------------------------------------------------------- the syntax list

const SYNTAX_ONLY = [
  "(?=a)b", "(?!a)b", "(?<=a)b", "(?<!a)b", "(a)\\1", "(?<n>a)\\k<n>",
  "(?i:a)", "(?-i:a)", "(?i-m:a)", "(?ims:a)", "(?i)a", "(?ii:a)", "(?-:a)",
  "(?=a)*", "(?<=a)+", "\\1", "(a)\\2", "\\k<n>", "(?<n>a)\\k<m>",
  "(?<n>a)(?<n>b)", "(?<n>a)|(?<n>b)", "(?<1n>a)", "(?<$n>a)", "(?<\\u0061>a)",
  "a{2,1}", "a{,5}", "{", "}", "]", "\\a", "\\-", "[\\d-z]", "[a-\\d]", "\\c1",
  "\\u{110000}", "\\x4", "\\p{L", "\\p{Letter}", "\\p{letter}", "\\P{Is_Latin}",
  "\\p{Script=Latin}", "\\p{Script=latin}", "\\p{Block=Basic_Latin}",
  "\\p{General_Category=L}", "\\p{gc=digit}", "\\p{scx=Grek}", "[]", "[^]",
  "a**", "^*", "$+", "\\b?", "()", "(?:)", "a|", "|", "\\/", "\\0", "\\00",
  "[\\b]", "[\\B]", "\\B", "[\\-]", "[\\1]", "\\ud83d\\udc32", "\\ud83d",
  "[\\ud800-\\udfff]", "\\cz", "[\\cz]", "[\\c_]", "\\u{0}",
];

// ------------------------------------------------------------ the answers

function answer(pattern, input) {
  let regex;
  try {
    regex = new RegExp(pattern, "u");
  } catch (error) {
    if (error instanceof SyntaxError) return "syntax-error";
    throw error;
  }
  return regex.test(input) ? "match" : "no-match";
}

function syntaxAnswer(pattern) {
  try {
    new RegExp(pattern, "u");
    return "valid";
  } catch (error) {
    if (error instanceof SyntaxError) return "syntax-error";
    throw error;
  }
}

/** The vector field encoding of `purrdf_testkit::vectors::encode_str`. */
function encode(text) {
  if (text.length === 0) return "\\0";
  let out = "";
  for (const ch of text) {
    const code = ch.codePointAt(0);
    if (ch === "\\") out += "\\\\";
    else if (code < 0x21 || ch === "#" || code === 0x7f) out += `\\x${hex(code, 2)}`;
    else out += ch;
  }
  return out;
}

function render() {
  const records = [];
  const seen = new Set();
  const counts = { suite: 0, corpus: 0, syntax: 0 };
  const add = (fields) => {
    const line = fields.map(encode).join("\t");
    if (!seen.has(line)) {
      seen.add(line);
      records.push(line);
      counts[fields[0]] += 1;
    }
  };
  for (const [pattern, input] of suitePairs()) add(["suite", pattern, input, answer(pattern, input)]);
  for (const [pattern, input] of corpusPairs()) add(["corpus", pattern, input, answer(pattern, input)]);
  for (const pattern of SYNTAX_ONLY) add(["syntax", pattern, "", syntaxAnswer(pattern)]);
  const body = records.map((line) => `${line}\n`).join("");
  const digest = createHash("sha256").update(body, "utf8").digest("hex");
  const header = [
    "# Verdicts of JavaScript's RegExp with the `u` flag, from crates/jsonschema/tests/pattern_oracle.mjs:",
    "# the official suite's pattern cases, a seeded corpus of generated pairs, and a fixed",
    "# syntax list. Fields: source (suite | corpus | syntax), pattern, input, verdict",
    "# (match | no-match | syntax-error; syntax records: valid | syntax-error).",
    `# seed: ${SEED}`,
    `# suite-pairs: ${counts.suite}`,
    `# corpus-pairs: ${counts.corpus}`,
    `# syntax-patterns: ${counts.syntax}`,
    `# vector-count: ${records.length}`,
    `# body-sha256: ${digest}`,
  ];
  return `${header.join("\n")}\n${body}`;
}

const rendered = render();
if (process.argv.includes("--write")) {
  writeFileSync(VECTORS, rendered);
  console.log(`wrote ${VECTORS}`);
} else {
  if (readFileSync(VECTORS, "utf8") !== rendered) {
    console.error("pattern_oracle: Node's answers differ from the committed vectors");
    process.exit(1);
  }
  console.log("pattern_oracle: the committed vectors are Node's answers");
}
