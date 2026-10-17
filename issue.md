# Issue #562: purrdf-hash: BLAKE3 keyed_hash, derive_key and extendable output

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## What is needed

`purrdf_hash::blake3` implements unkeyed BLAKE3-256 only. Katamari wants to drop its direct
dependency on the external `blake3` crate (13 crates link it today, and its C/asm backend shows up
in profiles beside purrdf's), but three of the specification's modes have no purrdf equivalent:

1. **`keyed_hash(key: &[u8; 32], data)`** (spec §2.3, `KEYED_HASH` flag, key as the IV).
   Katamari's locators: `keyed_hash(locator_key, identifier)`.
2. **`derive_key(context: &str, key_material)`** (spec §2.3, `DERIVE_KEY_CONTEXT` then
   `DERIVE_KEY_MATERIAL`). Katamari's frozen key constructions (vault locator key, dictionary key,
   spool frame key, key-server site wrap, and others) all go through it -- about 22 call sites.
3. **Extendable output** (spec §2.6: more than 32 bytes from the root node, by output block
   counter), one-shot is enough: `hash_xof(data, out: &mut [u8])`, and the keyed/derive variants
   if cheap. Katamari's representation salt XORs an XOF stream over a high-durability copy's full
   length, so the output can be megabytes; a reader with a seek (`set_position`) as in the
   reference crate would be ideal, streaming fill is sufficient.

Streaming forms of (1) and (2) (`Streaming` with a key or a derived key as the IV and the
mode flag) would cover the remaining sites, which build preimages with several `update` calls.

## Why

Measured 2026-10-16 on x86-64 with AVX-512, `--release`, same inputs, one-shot `hash`:

| bytes | `blake3` 1.8.7 | `purrdf_hash::blake3` |
|---:|---:|---:|
| 32 | 164 ns | 91 ns |
| 100 | 188 ns | 131 ns |
| 1 KiB | 1,409 ns | 976 ns |
| 4 KiB | 1,540 ns | 1,508 ns |
| 64 KiB | 7,889 ns | 8,056 ns |
| 1 MiB | 128 us | 129 us |

Streaming (two `update`s, 16 KiB buffer) is level below 1 KiB and 1.6-2x faster at 4-64 KiB.
Most of Katamari's hashing is identifiers and records under 1 KiB, so the unkeyed sites move now;
the keyed, derive-key and XOF sites wait on this.

## Acceptance

- The three modes against the official BLAKE3 test vectors (`test_vectors.json`: `hash`,
  `keyed_hash`, `derive_key`, each at the extended output length), on every backend the unkeyed
  path dispatches to.
- No new dependency; same dispatch as the unkeyed path.


## Comments (0)

