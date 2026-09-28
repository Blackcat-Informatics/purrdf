// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Byte-preserving core WebAssembly reader. Instruction boundaries follow the
//! WebAssembly binary grammar, <https://webassembly.github.io/spec/core/binary/>.
//! Unknown opcodes fail closed; semantic validation is delegated to Binaryen.
use crate::{LinkError, template::Op};
use wasm_encoder::{AbstractHeapType, HeapType, RefType, ValType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExternalKind {
    Func,
    Table,
    Memory,
    Global,
    Tag,
}
impl ExternalKind {
    pub(crate) fn read(reader: &mut Reader<'_>) -> Result<Self, LinkError> {
        match reader.byte()? {
            0 => Ok(Self::Func),
            1 => Ok(Self::Table),
            2 => Ok(Self::Memory),
            3 => Ok(Self::Global),
            4 => Ok(Self::Tag),
            n => Err(error(format!("unknown external kind {n}"))),
        }
    }
    pub(crate) fn encoded(self) -> wasm_encoder::ExportKind {
        match self {
            Self::Func => wasm_encoder::ExportKind::Func,
            Self::Table => wasm_encoder::ExportKind::Table,
            Self::Memory => wasm_encoder::ExportKind::Memory,
            Self::Global => wasm_encoder::ExportKind::Global,
            Self::Tag => wasm_encoder::ExportKind::Tag,
        }
    }
}
type DecodedOp = (Op, Option<(usize, usize, u32)>);

pub(crate) fn error(message: impl Into<String>) -> LinkError {
    LinkError::Parse(message.into())
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct Section<'a> {
    pub(crate) id: u8,
    pub(crate) data: &'a [u8],
}
pub(crate) fn sections(bytes: &[u8]) -> Result<Vec<Section<'_>>, LinkError> {
    let mut r = Reader::new(bytes);
    if r.take(8)? != b"\0asm\x01\0\0\0" {
        return Err(error("not a version 1 core module"));
    }
    let mut result = Vec::new();
    while !r.done() {
        let id = r.byte()?;
        let len = r.u32()? as usize;
        result.push(Section {
            id,
            data: r.take(len)?,
        });
    }
    Ok(result)
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct FunctionBody<'a> {
    pub(crate) data: &'a [u8],
}
#[derive(Debug)]
pub(crate) struct Reader<'a> {
    pub(crate) data: &'a [u8],
    pub(crate) pos: usize,
}
impl<'a> Reader<'a> {
    pub(crate) const fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }
    pub(crate) const fn done(&self) -> bool {
        self.pos == self.data.len()
    }
    pub(crate) fn finish(&self) -> Result<(), LinkError> {
        if self.done() {
            Ok(())
        } else {
            Err(error("trailing section bytes"))
        }
    }
    pub(crate) fn take(&mut self, n: usize) -> Result<&'a [u8], LinkError> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or_else(|| error("length overflow"))?;
        let bytes = self
            .data
            .get(self.pos..end)
            .ok_or_else(|| error("truncated module"))?;
        self.pos = end;
        Ok(bytes)
    }
    pub(crate) fn byte(&mut self) -> Result<u8, LinkError> {
        Ok(self.take(1)?[0])
    }
    pub(crate) fn leb(&mut self, bits: u32, signed: bool) -> Result<u64, LinkError> {
        let mut value = 0u64;
        for shift in (0..bits).step_by(7) {
            let b = self.byte()?;
            let payload = b & 127;
            let remaining = bits - shift;
            if remaining < 7 {
                let mask = 127u8 << remaining;
                let extra = payload & mask;
                let sign = signed && payload & (1 << (remaining - 1)) != 0;
                if extra != if sign { mask & 127 } else { 0 } {
                    return Err(error("LEB integer overflow"));
                }
            }
            value |= u64::from(payload) << shift;
            if b & 128 == 0 {
                if signed && b & 64 != 0 && shift + 7 < 64 {
                    value |= u64::MAX << (shift + 7);
                }
                return Ok(value);
            }
        }
        Err(error("LEB integer exceeds its width"))
    }
    pub(crate) fn u32(&mut self) -> Result<u32, LinkError> {
        Ok(self.leb(32, false)? as u32)
    }
    pub(crate) fn name(&mut self) -> Result<&'a str, LinkError> {
        let n = self.u32()? as usize;
        std::str::from_utf8(self.take(n)?).map_err(|_| error("invalid UTF-8 name"))
    }
    pub(crate) fn val(&mut self) -> Result<ValType, LinkError> {
        match self.byte()? {
            0x7f => Ok(ValType::I32),
            0x7e => Ok(ValType::I64),
            0x7d => Ok(ValType::F32),
            0x7c => Ok(ValType::F64),
            0x7b => Ok(ValType::V128),
            b @ (0x63 | 0x64) => Ok(ValType::Ref(RefType {
                nullable: b == 0x63,
                heap_type: self.heap_type()?,
            })),
            b @ 0x69..=0x74 => Ok(ValType::Ref(RefType {
                nullable: true,
                heap_type: abstract_heap(b)?,
            })),
            other => Err(error(format!("unsupported value type {other:#x}"))),
        }
    }
    fn heap_type(&mut self) -> Result<HeapType, LinkError> {
        let h = self.leb(33, true)? as i64;
        if h >= 0 {
            Ok(HeapType::Concrete(h as u32))
        } else {
            abstract_heap((h & 0x7f) as u8)
        }
    }
    pub(crate) fn limits(&mut self) -> Result<(), LinkError> {
        let flags = self.u32()?;
        let bits = if flags & 4 != 0 { 64 } else { 32 };
        self.leb(bits, false)?;
        if flags & 1 != 0 {
            self.leb(bits, false)?;
        }
        if flags & 8 != 0 {
            self.u32()?;
        }
        Ok(())
    }
    /// A block result is empty, a value type, or a signed type index. Explicit
    /// reference value types carry a heap type after their nullability prefix.
    fn block_type(&mut self) -> Result<(), LinkError> {
        if matches!(self.data.get(self.pos), Some(0x63 | 0x64)) {
            self.val()?;
        } else {
            self.leb(33, true)?;
        }
        Ok(())
    }
    fn memarg(&mut self) -> Result<(), LinkError> {
        let align = self.u32()?;
        if align & 64 != 0 {
            self.u32()?;
        }
        self.leb(64, false)?;
        Ok(())
    }
    /// Decode a whole instruction, returning its template form and a function-index
    /// immediate span when present. Immediates are never searched as instruction bytes.
    pub(crate) fn op(&mut self) -> Result<DecodedOp, LinkError> {
        let opcode = self.byte()?;
        let mut reference = None;
        let op = match opcode {
            0x00 => Op::Unreachable,
            0x0b => Op::End,
            0x0f => Op::Return,
            0x1a => Op::Drop,
            0x02..=0x04 | 0x06 => {
                let start = self.pos;
                self.block_type()?;
                if opcode == 4 && self.data[start] == 0x40 {
                    Op::If
                } else {
                    Op::Other
                }
            }
            0x07 | 0x08 | 0x09 | 0x0c | 0x0d | 0x18 | 0x20..=0x26 | 0xd5 | 0xd6 => {
                let n = self.u32()?;
                match opcode {
                    0x20 => Op::LocalGet(n),
                    0x21 => Op::LocalSet(n),
                    0x23 => Op::GlobalGet(n),
                    0x24 => Op::GlobalSet(n),
                    _ => Op::Other,
                }
            }
            0x0e => {
                let count = self.u32()?;
                for _ in 0..=count {
                    self.u32()?;
                }
                Op::Other
            }
            0x10 | 0x12 | 0xd2 => {
                let start = self.pos;
                let n = self.u32()?;
                reference = Some((start, self.pos, n));
                match opcode {
                    0x10 => Op::Call(n),
                    0x12 => Op::ReturnCall(n),
                    _ => Op::RefFunc(n),
                }
            }
            0x11 | 0x13 => {
                self.u32()?;
                self.u32()?;
                Op::Other
            }
            0x14 | 0x15 => {
                self.u32()?;
                Op::Other
            }
            0x1c => {
                let n = self.u32()?;
                for _ in 0..n {
                    self.val()?;
                }
                Op::Other
            }
            0x1f => {
                self.block_type()?;
                let n = self.u32()?;
                for _ in 0..n {
                    match self.byte()? {
                        0 | 1 => {
                            self.u32()?;
                            self.u32()?;
                        }
                        2 | 3 => {
                            self.u32()?;
                        }
                        _ => return Err(error("unknown catch kind")),
                    }
                }
                Op::Other
            }
            0x28..=0x3e => {
                self.memarg()?;
                Op::Other
            }
            0x3f | 0x40 => {
                self.u32()?;
                Op::Other
            }
            0x41 => Op::I32Const(self.leb(32, true)? as i32),
            0x42 => {
                self.leb(64, true)?;
                Op::Other
            }
            0x43 => {
                self.take(4)?;
                Op::Other
            }
            0x44 => {
                self.take(8)?;
                Op::Other
            }
            0x47 => Op::I32Ne,
            0x6a => Op::I32Add,
            0x6b => Op::I32Sub,
            0xd0 => {
                self.leb(33, true)?;
                Op::Other
            }
            0xfc => {
                match self.u32()? {
                    0..=7 | 19..=22 => {}
                    8 | 10 | 12 | 14 => {
                        self.u32()?;
                        self.u32()?;
                    }
                    9 | 11 | 13 | 15..=18 => {
                        self.u32()?;
                    }
                    n => return Err(error(format!("unknown misc opcode {n}"))),
                }
                Op::Other
            }
            0xfd => {
                let n = self.u32()?;
                match n {
                    0..=11 | 92 | 93 => self.memarg()?,
                    12 | 13 => {
                        self.take(16)?;
                    }
                    21..=34 => {
                        self.byte()?;
                    }
                    84..=91 => {
                        self.memarg()?;
                        self.byte()?;
                    }
                    14..=20 | 35..=83 | 94..=255 | 256..=275 => {}
                    _ => return Err(error(format!("unknown SIMD opcode {n}"))),
                }
                Op::Other
            }
            0xfe => {
                let n = self.u32()?;
                match n {
                    0..=2 | 0x10..=0x4e => self.memarg()?,
                    3 => {
                        self.byte()?;
                    }
                    _ => return Err(error(format!("unknown atomic opcode {n}"))),
                }
                Op::Other
            }
            0xfb => {
                let n = self.u32()?;
                match n {
                    0 | 1 | 6 | 7 | 11..=14 | 16 => {
                        self.u32()?;
                    }
                    2..=5 | 8..=10 | 17..=19 => {
                        self.u32()?;
                        self.u32()?;
                    }
                    15 | 26..=30 => {}
                    20..=23 => {
                        self.leb(33, true)?;
                    }
                    24 | 25 => {
                        self.byte()?;
                        self.u32()?;
                        self.leb(33, true)?;
                        self.leb(33, true)?;
                    }
                    _ => return Err(error(format!("unknown GC opcode {n}"))),
                }
                Op::Other
            }
            0x01 | 0x05 | 0x0a | 0x19 | 0x1b | 0x45..=0xc4 | 0xd1 | 0xd3 | 0xd4 => Op::Other,
            other => return Err(error(format!("unknown opcode {other:#x}"))),
        };
        Ok((op, reference))
    }
}
pub(crate) fn put_u32(mut n: u32, out: &mut Vec<u8>) {
    loop {
        let b = (n & 127) as u8;
        n >>= 7;
        out.push(b | if n == 0 { 0 } else { 128 });
        if n == 0 {
            break;
        }
    }
}

/// Abstract reference type codes from Core 3.0 binary/types.html.
fn abstract_heap(code: u8) -> Result<HeapType, LinkError> {
    let ty = match code {
        0x69 => AbstractHeapType::Exn,
        0x6a => AbstractHeapType::Array,
        0x6b => AbstractHeapType::Struct,
        0x6c => AbstractHeapType::I31,
        0x6d => AbstractHeapType::Eq,
        0x6e => AbstractHeapType::Any,
        0x6f => AbstractHeapType::Extern,
        0x70 => AbstractHeapType::Func,
        0x71 => AbstractHeapType::None,
        0x72 => AbstractHeapType::NoExtern,
        0x73 => AbstractHeapType::NoFunc,
        0x74 => AbstractHeapType::NoExn,
        _ => return Err(error(format!("unknown abstract heap type {code:#x}"))),
    };
    Ok(HeapType::Abstract { shared: false, ty })
}
