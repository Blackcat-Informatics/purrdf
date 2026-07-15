// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Minimal Thrift Compact Protocol writer/reader for Parquet metadata.
//!
//! There is deliberately no RPC message layer: Parquet serializes bare Thrift
//! structs. The implementation covers every base/container type needed to skip
//! unknown fields safely, while construction helpers expose only the field shapes
//! the fixed Parquet profile emits.

use purrdf_core::ir::pack::bits::{read_varint, write_varint, zigzag_decode, zigzag_encode};

use crate::ColumnarError;

pub(crate) const TYPE_STOP: u8 = 0;
pub(crate) const TYPE_BOOL_TRUE: u8 = 1;
pub(crate) const TYPE_BOOL_FALSE: u8 = 2;
pub(crate) const TYPE_I8: u8 = 3;
pub(crate) const TYPE_I16: u8 = 4;
pub(crate) const TYPE_I32: u8 = 5;
pub(crate) const TYPE_I64: u8 = 6;
pub(crate) const TYPE_DOUBLE: u8 = 7;
pub(crate) const TYPE_BINARY: u8 = 8;
pub(crate) const TYPE_LIST: u8 = 9;
pub(crate) const TYPE_SET: u8 = 10;
pub(crate) const TYPE_MAP: u8 = 11;
pub(crate) const TYPE_STRUCT: u8 = 12;

const MAX_METADATA_COLLECTION_ITEMS: usize = 1_024;
const MAX_SKIP_DEPTH: usize = 64;

#[derive(Debug, Default)]
pub(crate) struct CompactWriter {
    bytes: Vec<u8>,
    last_fields: Vec<i16>,
}

impl CompactWriter {
    pub(crate) fn root(build: impl FnOnce(&mut Self)) -> Vec<u8> {
        let mut writer = Self::default();
        writer.write_struct(build);
        writer.bytes
    }

    fn write_struct(&mut self, build: impl FnOnce(&mut Self)) {
        self.last_fields.push(0);
        build(self);
        self.bytes.push(TYPE_STOP);
        self.last_fields
            .pop()
            .expect("struct field stack is balanced");
    }

    fn field_header(&mut self, id: i16, field_type: u8) {
        let previous = *self
            .last_fields
            .last()
            .expect("field headers are written only inside a struct");
        let delta = id - previous;
        if (1..=15).contains(&delta) {
            self.bytes.push(((delta as u8) << 4) | field_type);
        } else {
            self.bytes.push(field_type);
            write_varint(&mut self.bytes, zigzag_encode(i64::from(id)));
        }
        *self
            .last_fields
            .last_mut()
            .expect("field headers are written only inside a struct") = id;
    }

    pub(crate) fn i16_field(&mut self, id: i16, value: i16) {
        self.field_header(id, TYPE_I16);
        write_varint(&mut self.bytes, zigzag_encode(i64::from(value)));
    }

    pub(crate) fn i32_field(&mut self, id: i16, value: i32) {
        self.field_header(id, TYPE_I32);
        write_varint(&mut self.bytes, zigzag_encode(i64::from(value)));
    }

    pub(crate) fn i64_field(&mut self, id: i16, value: i64) {
        self.field_header(id, TYPE_I64);
        write_varint(&mut self.bytes, zigzag_encode(value));
    }

    pub(crate) fn bool_field(&mut self, id: i16, value: bool) {
        self.field_header(
            id,
            if value {
                TYPE_BOOL_TRUE
            } else {
                TYPE_BOOL_FALSE
            },
        );
    }

    pub(crate) fn binary_field(&mut self, id: i16, value: &[u8]) {
        self.field_header(id, TYPE_BINARY);
        self.binary_value(value);
    }

    pub(crate) fn string_field(&mut self, id: i16, value: &str) {
        self.binary_field(id, value.as_bytes());
    }

    pub(crate) fn struct_field(&mut self, id: i16, build: impl FnOnce(&mut Self)) {
        self.field_header(id, TYPE_STRUCT);
        self.write_struct(build);
    }

    pub(crate) fn list_i32_field(&mut self, id: i16, values: &[i32]) {
        self.field_header(id, TYPE_LIST);
        self.list_header(values.len(), TYPE_I32);
        for &value in values {
            write_varint(&mut self.bytes, zigzag_encode(i64::from(value)));
        }
    }

    pub(crate) fn list_string_field(&mut self, id: i16, values: &[&str]) {
        self.field_header(id, TYPE_LIST);
        self.list_header(values.len(), TYPE_BINARY);
        for value in values {
            self.binary_value(value.as_bytes());
        }
    }

    pub(crate) fn list_struct_field<T>(
        &mut self,
        id: i16,
        values: &[T],
        mut write: impl FnMut(&mut Self, &T),
    ) {
        self.field_header(id, TYPE_LIST);
        self.list_header(values.len(), TYPE_STRUCT);
        for value in values {
            self.write_struct(|writer| write(writer, value));
        }
    }

    fn list_header(&mut self, len: usize, element_type: u8) {
        if len < 15 {
            self.bytes.push(((len as u8) << 4) | element_type);
        } else {
            self.bytes.push(0xf0 | element_type);
            write_varint(&mut self.bytes, len as u64);
        }
    }

    fn binary_value(&mut self, value: &[u8]) {
        write_varint(&mut self.bytes, value.len() as u64);
        self.bytes.extend_from_slice(value);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CompactField {
    pub(crate) id: i16,
    pub(crate) field_type: u8,
}

#[derive(Debug, Default)]
pub(crate) struct StructState {
    last_field: i16,
    stopped: bool,
}

#[derive(Debug)]
pub(crate) struct CompactReader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> CompactReader<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    pub(crate) fn position(&self) -> usize {
        self.pos
    }

    pub(crate) fn is_finished(&self) -> bool {
        self.pos == self.bytes.len()
    }

    pub(crate) fn next_field(
        &mut self,
        state: &mut StructState,
    ) -> Result<Option<CompactField>, ColumnarError> {
        if state.stopped {
            return Ok(None);
        }
        let header = self.read_u8("compact field header")?;
        let field_type = header & 0x0f;
        if field_type == TYPE_STOP {
            if header != TYPE_STOP {
                return Err(ColumnarError::malformed(
                    "compact field header",
                    "STOP field carries a non-zero delta",
                ));
            }
            state.stopped = true;
            return Ok(None);
        }
        Self::validate_type(field_type)?;
        let delta = i16::from(header >> 4);
        let id = if delta == 0 {
            let raw = self.read_signed("compact field id")?;
            i16::try_from(raw)
                .map_err(|_| ColumnarError::malformed("compact field id", "value exceeds i16"))?
        } else {
            state.last_field.checked_add(delta).ok_or_else(|| {
                ColumnarError::malformed("compact field id", "delta overflows i16")
            })?
        };
        if id <= 0 {
            return Err(ColumnarError::malformed(
                "compact field id",
                "field ids must be positive",
            ));
        }
        state.last_field = id;
        Ok(Some(CompactField { id, field_type }))
    }

    pub(crate) fn read_i16(&mut self, field: CompactField) -> Result<i16, ColumnarError> {
        Self::expect_type(field, TYPE_I16, "compact i16")?;
        let value = self.read_signed("compact i16")?;
        i16::try_from(value)
            .map_err(|_| ColumnarError::malformed("compact i16", "value exceeds i16"))
    }

    pub(crate) fn read_i32(&mut self, field: CompactField) -> Result<i32, ColumnarError> {
        Self::expect_type(field, TYPE_I32, "compact i32")?;
        let value = self.read_signed("compact i32")?;
        i32::try_from(value)
            .map_err(|_| ColumnarError::malformed("compact i32", "value exceeds i32"))
    }

    pub(crate) fn read_i64(&mut self, field: CompactField) -> Result<i64, ColumnarError> {
        Self::expect_type(field, TYPE_I64, "compact i64")?;
        self.read_signed("compact i64")
    }

    pub(crate) fn read_bool(&self, field: CompactField) -> Result<bool, ColumnarError> {
        match field.field_type {
            TYPE_BOOL_TRUE => Ok(true),
            TYPE_BOOL_FALSE => Ok(false),
            _ => Err(ColumnarError::malformed(
                "compact bool",
                format!("field {} has type {}", field.id, field.field_type),
            )),
        }
    }

    pub(crate) fn read_binary(&mut self, field: CompactField) -> Result<&'a [u8], ColumnarError> {
        Self::expect_type(field, TYPE_BINARY, "compact binary")?;
        self.binary_value()
    }

    pub(crate) fn read_string(&mut self, field: CompactField) -> Result<&'a str, ColumnarError> {
        let bytes = self.read_binary(field)?;
        std::str::from_utf8(bytes)
            .map_err(|_| ColumnarError::malformed("compact string", "invalid UTF-8"))
    }

    pub(crate) fn expect_struct(field: CompactField) -> Result<StructState, ColumnarError> {
        Self::expect_type(field, TYPE_STRUCT, "compact struct")?;
        Ok(StructState::default())
    }

    pub(crate) fn read_list_header(
        &mut self,
        field: CompactField,
    ) -> Result<(usize, u8), ColumnarError> {
        Self::expect_type(field, TYPE_LIST, "compact list")?;
        self.list_header()
    }

    pub(crate) fn read_list_i32(&mut self, field: CompactField) -> Result<Vec<i32>, ColumnarError> {
        let (len, element_type) = self.read_list_header(field)?;
        if element_type != TYPE_I32 {
            return Err(ColumnarError::malformed(
                "compact i32 list",
                format!("element type is {element_type}"),
            ));
        }
        let mut values = Vec::with_capacity(len);
        for _ in 0..len {
            let value = self.read_signed("compact list i32")?;
            values.push(
                i32::try_from(value).map_err(|_| {
                    ColumnarError::malformed("compact list i32", "value exceeds i32")
                })?,
            );
        }
        Ok(values)
    }

    pub(crate) fn read_list_strings(
        &mut self,
        field: CompactField,
    ) -> Result<Vec<String>, ColumnarError> {
        let (len, element_type) = self.read_list_header(field)?;
        if element_type != TYPE_BINARY {
            return Err(ColumnarError::malformed(
                "compact string list",
                format!("element type is {element_type}"),
            ));
        }
        let mut values = Vec::with_capacity(len);
        for _ in 0..len {
            let value = self.binary_value()?;
            values.push(
                std::str::from_utf8(value)
                    .map_err(|_| ColumnarError::malformed("compact string list", "invalid UTF-8"))?
                    .to_owned(),
            );
        }
        Ok(values)
    }

    pub(crate) fn skip_field(&mut self, field: CompactField) -> Result<(), ColumnarError> {
        if matches!(field.field_type, TYPE_BOOL_TRUE | TYPE_BOOL_FALSE) {
            return Ok(());
        }
        self.skip_value(field.field_type, 0)
    }

    fn skip_value(&mut self, value_type: u8, depth: usize) -> Result<(), ColumnarError> {
        if depth >= MAX_SKIP_DEPTH {
            return Err(ColumnarError::LimitExceeded {
                context: "compact nesting depth",
                value: depth as u64,
                maximum: MAX_SKIP_DEPTH as u64,
            });
        }
        match value_type {
            TYPE_BOOL_TRUE | TYPE_BOOL_FALSE | TYPE_I8 => {
                self.read_u8("compact value")?;
            }
            TYPE_I16 | TYPE_I32 | TYPE_I64 => {
                self.read_unsigned("compact integer")?;
            }
            TYPE_DOUBLE => {
                self.take(8, "compact double")?;
            }
            TYPE_BINARY => {
                self.binary_value()?;
            }
            TYPE_LIST | TYPE_SET => {
                let (len, element_type) = self.list_header()?;
                for _ in 0..len {
                    self.skip_value(element_type, depth + 1)?;
                }
            }
            TYPE_MAP => {
                let len = self.read_collection_len("compact map")?;
                if len > 0 {
                    let types = self.read_u8("compact map types")?;
                    let key_type = types >> 4;
                    let value_type = types & 0x0f;
                    Self::validate_type(key_type)?;
                    Self::validate_type(value_type)?;
                    for _ in 0..len {
                        self.skip_value(key_type, depth + 1)?;
                        self.skip_value(value_type, depth + 1)?;
                    }
                }
            }
            TYPE_STRUCT => {
                let mut state = StructState::default();
                while let Some(field) = self.next_field(&mut state)? {
                    if matches!(field.field_type, TYPE_BOOL_TRUE | TYPE_BOOL_FALSE) {
                        continue;
                    }
                    self.skip_value(field.field_type, depth + 1)?;
                }
            }
            TYPE_STOP => {
                return Err(ColumnarError::malformed(
                    "compact value",
                    "STOP is not a value type",
                ));
            }
            _ => unreachable!("validated compact type"),
        }
        Ok(())
    }

    fn list_header(&mut self) -> Result<(usize, u8), ColumnarError> {
        let header = self.read_u8("compact list header")?;
        let element_type = header & 0x0f;
        Self::validate_type(element_type)?;
        if element_type == TYPE_STOP {
            return Err(ColumnarError::malformed(
                "compact list header",
                "STOP is not a valid element type",
            ));
        }
        let short_len = usize::from(header >> 4);
        let len = if short_len == 15 {
            self.read_collection_len("compact list")?
        } else {
            short_len
        };
        Ok((len, element_type))
    }

    fn read_collection_len(&mut self, context: &'static str) -> Result<usize, ColumnarError> {
        let value = self.read_unsigned(context)?;
        let len = usize::try_from(value)
            .map_err(|_| ColumnarError::malformed(context, "length exceeds usize"))?;
        if len > MAX_METADATA_COLLECTION_ITEMS {
            return Err(ColumnarError::limit(
                context,
                len,
                MAX_METADATA_COLLECTION_ITEMS,
            ));
        }
        Ok(len)
    }

    fn binary_value(&mut self) -> Result<&'a [u8], ColumnarError> {
        let value = self.read_unsigned("compact binary length")?;
        let len = usize::try_from(value).map_err(|_| {
            ColumnarError::malformed("compact binary length", "value exceeds usize")
        })?;
        self.take(len, "compact binary")
    }

    fn read_signed(&mut self, context: &'static str) -> Result<i64, ColumnarError> {
        self.read_unsigned(context).map(zigzag_decode)
    }

    fn read_unsigned(&mut self, context: &'static str) -> Result<u64, ColumnarError> {
        read_varint(self.bytes, &mut self.pos)
            .map_err(|error| ColumnarError::malformed(context, error.to_string()))
    }

    fn read_u8(&mut self, context: &'static str) -> Result<u8, ColumnarError> {
        let byte = *self
            .bytes
            .get(self.pos)
            .ok_or_else(|| ColumnarError::truncated(context, self.pos + 1, self.bytes.len()))?;
        self.pos += 1;
        Ok(byte)
    }

    fn take(&mut self, len: usize, context: &'static str) -> Result<&'a [u8], ColumnarError> {
        let end = self
            .pos
            .checked_add(len)
            .ok_or_else(|| ColumnarError::malformed(context, "byte range overflows usize"))?;
        let value = self
            .bytes
            .get(self.pos..end)
            .ok_or_else(|| ColumnarError::truncated(context, end, self.bytes.len()))?;
        self.pos = end;
        Ok(value)
    }

    fn expect_type(
        field: CompactField,
        expected: u8,
        context: &'static str,
    ) -> Result<(), ColumnarError> {
        if field.field_type == expected {
            Ok(())
        } else {
            Err(ColumnarError::malformed(
                context,
                format!(
                    "field {} has type {}, expected {expected}",
                    field.id, field.field_type
                ),
            ))
        }
    }

    fn validate_type(value_type: u8) -> Result<(), ColumnarError> {
        if value_type <= TYPE_STRUCT {
            Ok(())
        } else {
            Err(ColumnarError::Unsupported {
                context: "compact type",
                value: i64::from(value_type),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_round_trips_scalars_lists_and_nested_structs() {
        let bytes = CompactWriter::root(|writer| {
            writer.i32_field(1, -17);
            writer.i64_field(2, i64::MIN);
            writer.bool_field(3, true);
            writer.string_field(4, "meow");
            writer.list_i32_field(5, &[0, 3, -9]);
            writer.struct_field(21, |writer| {
                writer.i16_field(1, 7);
                writer.bool_field(2, false);
            });
        });

        let mut reader = CompactReader::new(&bytes);
        let mut root = StructState::default();

        let field = reader.next_field(&mut root).unwrap().unwrap();
        assert_eq!(reader.read_i32(field).unwrap(), -17);
        let field = reader.next_field(&mut root).unwrap().unwrap();
        assert_eq!(reader.read_i64(field).unwrap(), i64::MIN);
        let field = reader.next_field(&mut root).unwrap().unwrap();
        assert!(reader.read_bool(field).unwrap());
        let field = reader.next_field(&mut root).unwrap().unwrap();
        assert_eq!(reader.read_string(field).unwrap(), "meow");
        let field = reader.next_field(&mut root).unwrap().unwrap();
        assert_eq!(reader.read_list_i32(field).unwrap(), vec![0, 3, -9]);
        let field = reader.next_field(&mut root).unwrap().unwrap();
        assert_eq!(field.id, 21, "long-form field id survives");
        let mut nested = CompactReader::expect_struct(field).unwrap();
        let field = reader.next_field(&mut nested).unwrap().unwrap();
        assert_eq!(reader.read_i16(field).unwrap(), 7);
        let field = reader.next_field(&mut nested).unwrap().unwrap();
        assert!(!reader.read_bool(field).unwrap());
        assert!(reader.next_field(&mut nested).unwrap().is_none());
        assert!(reader.next_field(&mut root).unwrap().is_none());
        assert!(reader.is_finished());
    }

    #[test]
    fn unknown_nested_field_is_skipped_without_desynchronizing() {
        let bytes = CompactWriter::root(|writer| {
            writer.struct_field(1, |writer| {
                writer.list_string_field(1, &["a", "b"]);
            });
            writer.i32_field(2, 42);
        });
        let mut reader = CompactReader::new(&bytes);
        let mut root = StructState::default();
        let unknown = reader.next_field(&mut root).unwrap().unwrap();
        reader.skip_field(unknown).unwrap();
        let answer = reader.next_field(&mut root).unwrap().unwrap();
        assert_eq!(reader.read_i32(answer).unwrap(), 42);
        assert!(reader.next_field(&mut root).unwrap().is_none());
        assert!(reader.is_finished());
    }

    #[test]
    fn compact_rejects_truncation_and_oversized_collection() {
        let bytes = CompactWriter::root(|writer| writer.string_field(1, "abc"));
        let mut reader = CompactReader::new(&bytes[..bytes.len() - 2]);
        let mut root = StructState::default();
        let field = reader.next_field(&mut root).unwrap().unwrap();
        assert!(matches!(
            reader.read_string(field),
            Err(ColumnarError::Truncated { .. })
        ));

        let mut oversized = vec![0x19, 0xf5];
        write_varint(&mut oversized, (MAX_METADATA_COLLECTION_ITEMS + 1) as u64);
        oversized.push(TYPE_STOP);
        let mut reader = CompactReader::new(&oversized);
        let mut root = StructState::default();
        let field = reader.next_field(&mut root).unwrap().unwrap();
        assert!(matches!(
            reader.read_list_i32(field),
            Err(ColumnarError::LimitExceeded { .. })
        ));
    }
}
