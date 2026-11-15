// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `--wasm-imports FILE`: refuse a WebAssembly module that imports anything.
//!
//! A module built from the library crates for `wasm32-unknown-unknown` must run
//! where there is no JavaScript host, so every import is one its embedder cannot
//! provide. This reads the module's import section (WebAssembly Core Specification,
//! Binary Format, section 5.5.5) and fails when it holds an entry, naming each one.

/// The import section's identifier.
const IMPORT_SECTION: u8 = 2;

/// A cursor over a module's bytes; every read is checked against its end.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        let end = self
            .at
            .checked_add(count)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| format!("the module is truncated at byte {}", self.at))?;
        let taken = &self.bytes[self.at..end];
        self.at = end;
        Ok(taken)
    }

    fn byte(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }

    /// An unsigned variable-length integer of at most `bits` bits, seven per byte,
    /// least significant group first.
    fn unsigned(&mut self, bits: u32) -> Result<u64, String> {
        let mut value = 0u64;
        let mut shift = 0u32;
        loop {
            let byte = self.byte()?;
            if shift >= bits {
                return Err(format!("an integer before byte {} is too long", self.at));
            }
            value |= u64::from(byte & 0x7f) << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
            shift += 7;
        }
    }

    fn length(&mut self) -> Result<usize, String> {
        usize::try_from(self.unsigned(32)?).map_err(|_| "a length exceeds this host".to_owned())
    }

    fn name(&mut self) -> Result<&'a str, String> {
        let length = self.length()?;
        let start = self.at;
        std::str::from_utf8(self.take(length)?)
            .map_err(|_| format!("the name at byte {start} is not UTF-8"))
    }

    /// Limits: a flag byte, a minimum, and a maximum when the flag's low bit is set.
    fn limits(&mut self) -> Result<(), String> {
        let flags = self.byte()?;
        self.unsigned(64)?;
        if flags & 1 == 1 {
            self.unsigned(64)?;
        }
        Ok(())
    }

    /// Skip one import description, returning the kind it declares.
    fn description(&mut self) -> Result<&'static str, String> {
        match self.byte()? {
            0 => {
                self.unsigned(32)?;
                Ok("function")
            }
            1 => {
                self.byte()?;
                self.limits()?;
                Ok("table")
            }
            2 => {
                self.limits()?;
                Ok("memory")
            }
            3 => {
                self.take(2)?;
                Ok("global")
            }
            4 => {
                self.byte()?;
                self.unsigned(32)?;
                Ok("tag")
            }
            kind => Err(format!(
                "import kind {kind} before byte {} is not one this reader knows",
                self.at
            )),
        }
    }
}

/// Every import `module` declares, as `module.field (kind)`, in section order.
pub(crate) fn imports(module: &[u8]) -> Result<Vec<String>, String> {
    let mut reader = Reader {
        bytes: module,
        at: 0,
    };
    if reader.take(8)? != b"\0asm\x01\0\0\0" {
        return Err("not a WebAssembly version 1 module".to_owned());
    }
    let mut found = Vec::new();
    while reader.at < module.len() {
        let id = reader.byte()?;
        let size = reader.length()?;
        let payload = reader.take(size)?;
        if id != IMPORT_SECTION {
            continue;
        }
        let mut section = Reader {
            bytes: payload,
            at: 0,
        };
        for _ in 0..section.unsigned(32)? {
            let module_name = section.name()?;
            let field = section.name()?;
            let kind = section.description()?;
            found.push(format!("{module_name}.{field} ({kind})"));
        }
    }
    Ok(found)
}

/// `helper-census --wasm-imports FILE`.
pub(crate) fn run(args: &[String]) -> Result<(), String> {
    let [flag, path] = args else {
        return Err("usage: helper-census --wasm-imports FILE".to_owned());
    };
    if flag != "--wasm-imports" {
        return Err("usage: helper-census --wasm-imports FILE".to_owned());
    }
    let module = std::fs::read(path).map_err(|error| format!("{path}: {error}"))?;
    let found = imports(&module).map_err(|error| format!("{path}: {error}"))?;
    if found.is_empty() {
        println!("OK: {path} imports nothing");
        return Ok(());
    }
    Err(format!(
        "{path} imports {} item(s) its embedder would have to provide:\n  {}",
        found.len(),
        found.join("\n  ")
    ))
}

#[cfg(test)]
mod tests {
    use super::imports;

    const HEADER: &[u8] = b"\0asm\x01\0\0\0";
    /// One type, `() -> f64`.
    const TYPES: &[u8] = &[1, 5, 1, 0x60, 0, 1, 0x7c];
    /// One function of type 0 whose body is `f64.const 0`.
    const FUNCTION_AND_CODE: &[u8] = &[
        3, 2, 1, 0, 10, 13, 1, 11, 0, 0x44, 0, 0, 0, 0, 0, 0, 0, 0, 0x0b,
    ];

    fn module(sections: &[&[u8]]) -> Vec<u8> {
        let mut bytes = HEADER.to_vec();
        for section in sections {
            bytes.extend_from_slice(section);
        }
        bytes
    }

    /// The gate's refusal and its valid neighbour: the same module with and
    /// without an import section.
    #[test]
    fn a_module_with_an_import_is_named_and_the_same_module_without_one_passes() {
        // Import section: `Date.now` as a function of type 0, then a memory.
        let import: &[u8] = &[
            2, 23, 2, 4, b'D', b'a', b't', b'e', 3, b'n', b'o', b'w', 0, 0, 3, b'e', b'n', b'v', 3,
            b'm', b'e', b'm', 2, 0, 1,
        ];
        assert_eq!(
            imports(&module(&[TYPES, import, FUNCTION_AND_CODE])).unwrap(),
            ["Date.now (function)", "env.mem (memory)"]
        );
        assert_eq!(
            imports(&module(&[TYPES, FUNCTION_AND_CODE])).unwrap(),
            Vec::<String>::new()
        );
        // An import section that declares no entry imports nothing.
        assert_eq!(
            imports(&module(&[TYPES, &[2, 1, 0], FUNCTION_AND_CODE])).unwrap(),
            Vec::<String>::new()
        );
    }

    /// A module the reader cannot follow is an error, never "imports nothing".
    #[test]
    fn an_unreadable_module_is_refused_rather_than_passed() {
        assert!(imports(b"\0asm\x02\0\0\0").is_err());
        assert!(imports(&module(&[&[2, 9, 1, 4, b'D']])).is_err());
        assert!(imports(&module(&[&[2, 5, 1, 1, b'a', 1, b'b']])).is_err());
        assert!(imports(&module(&[&[2, 6, 1, 1, b'a', 1, b'b', 9]])).is_err());
    }
}
