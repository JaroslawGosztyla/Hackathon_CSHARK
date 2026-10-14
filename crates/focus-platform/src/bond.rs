//! A strict subset of Microsoft Bond CompactBinary v1, enough for the Windows CloudStore values of Night light.
//!
//! Format (<https://microsoft.github.io/bond/reference/cpp/compact__binary_8h_source.html>):
//! * marshalled header `43 42 01 00` ("CB", version 1, little endian),
//! * a struct is a sequence of fields followed by `BT_STOP` (`00`); a struct with a base struct writes the base's
//!   fields, then `BT_STOP_BASE` (`01`), then its own fields,
//! * field header: type in the low 5 bits; id 0..=5 in the high 3 bits, otherwise `0xC0 | type` + one id byte,
//!   or `0xE0 | type` + two id bytes (little endian),
//! * `bool`, `int8`: one byte; `uint64`: varint (7 bits per byte, little endian); `int16`, `int32`: zigzag varint,
//! * `list` (v1): one element-type byte, a varint count, the elements.
//!
//! Only the types the Night light schemas use are accepted (bool, int8, int16, int32, uint64, struct and
//! `list<int8>` blobs). Anything else is an error, so a value is either understood completely or not touched.
//! Decoding then re-encoding reproduces the input byte for byte (checked by [`decode_marshalled`]).

/// Bond type ids used here.
const BT_STOP: u8 = 0;
const BT_STOP_BASE: u8 = 1;
const BT_BOOL: u8 = 2;
const BT_UINT64: u8 = 6;
const BT_STRUCT: u8 = 10;
const BT_LIST: u8 = 11;
const BT_INT8: u8 = 14;
const BT_INT16: u8 = 15;
const BT_INT32: u8 = 16;

/// Marshalled CompactBinary v1 header.
pub(crate) const HEADER: [u8; 4] = [0x43, 0x42, 0x01, 0x00];
/// Deepest struct nesting accepted (the Night light values use 3).
const MAX_DEPTH: usize = 8;
/// Longest varint of a 64-bit value.
const MAX_VARINT_BYTES: usize = 10;

/// Why a byte sequence was rejected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum BondError {
    #[error("not a Bond CompactBinary v1 value")]
    Header,
    #[error("value ends early")]
    Truncated,
    #[error("unsupported Bond type {0}")]
    UnsupportedType(u8),
    #[error("invalid varint")]
    Varint,
    #[error("value out of range for field {0}")]
    Range(u16),
    #[error("structs nested too deep")]
    TooDeep,
    #[error("field ids out of order")]
    Order,
    #[error("{0} trailing bytes")]
    Trailing(usize),
    #[error("non-canonical encoding")]
    NonCanonical,
}

/// A decoded field value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Value {
    Bool(bool),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    UInt64(u64),
    Struct(BondStruct),
    /// `list<int8>`: a nested blob.
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Field {
    pub(crate) id: u16,
    pub(crate) value: Value,
}

/// A struct: its fields in ascending id order, and `derived` when a `BT_STOP_BASE` marker separates the base
/// struct's fields (`fields`) from the derived struct's own fields.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct BondStruct {
    pub(crate) fields: Vec<Field>,
    pub(crate) derived: Option<Vec<Field>>,
}

impl BondStruct {
    pub(crate) fn get(&self, id: u16) -> Option<&Value> {
        self.fields
            .iter()
            .find(|field| field.id == id)
            .map(|field| &field.value)
    }

    /// Sets (or adds, keeping ids ascending) a base field.
    pub(crate) fn set(&mut self, id: u16, value: Value) {
        match self.fields.binary_search_by_key(&id, |field| field.id) {
            Ok(index) => self.fields[index].value = value,
            Err(index) => self.fields.insert(index, Field { id, value }),
        }
    }

    /// Removes a base field (no-op when absent).
    pub(crate) fn remove(&mut self, id: u16) {
        self.fields.retain(|field| field.id != id);
    }
}

/// Decodes a marshalled value (`HEADER` + top-level struct) and checks that re-encoding gives the same bytes.
pub(crate) fn decode_marshalled(bytes: &[u8]) -> Result<BondStruct, BondError> {
    let body = bytes.strip_prefix(&HEADER).ok_or(BondError::Header)?;
    let mut reader = Reader {
        bytes: body,
        pos: 0,
    };
    let value = reader.read_struct(0)?;
    if reader.pos != body.len() {
        return Err(BondError::Trailing(body.len() - reader.pos));
    }
    if encode_marshalled(&value) != bytes {
        return Err(BondError::NonCanonical);
    }
    Ok(value)
}

/// `HEADER` + the struct.
pub(crate) fn encode_marshalled(value: &BondStruct) -> Vec<u8> {
    let mut out = HEADER.to_vec();
    write_struct(&mut out, value);
    out
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn byte(&mut self) -> Result<u8, BondError> {
        let byte = *self.bytes.get(self.pos).ok_or(BondError::Truncated)?;
        self.pos += 1;
        Ok(byte)
    }

    fn take(&mut self, len: usize) -> Result<&[u8], BondError> {
        let end = self.pos.checked_add(len).ok_or(BondError::Truncated)?;
        let slice = self.bytes.get(self.pos..end).ok_or(BondError::Truncated)?;
        self.pos = end;
        Ok(slice)
    }

    fn varint(&mut self) -> Result<u64, BondError> {
        let mut value = 0u64;
        for index in 0..MAX_VARINT_BYTES {
            let byte = self.byte()?;
            let bits = u64::from(byte & 0x7F);
            let shift = 7 * index as u32;
            if shift == 63 && bits > 1 {
                return Err(BondError::Varint);
            }
            value |= bits << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(BondError::Varint)
    }

    /// `(type, id)`; `None` for `BT_STOP`, `Some((BT_STOP_BASE, 0))` for the base marker.
    fn field_header(&mut self) -> Result<Option<(u8, u16)>, BondError> {
        let raw = self.byte()?;
        let kind = raw & 0x1F;
        let id = match raw >> 5 {
            6 => u16::from(self.byte()?),
            7 => {
                let low = self.byte()?;
                let high = self.byte()?;
                u16::from_le_bytes([low, high])
            }
            short => u16::from(short),
        };
        match (kind, id) {
            (BT_STOP, 0) => Ok(None),
            (BT_STOP_BASE, 0) => Ok(Some((BT_STOP_BASE, 0))),
            (BT_STOP | BT_STOP_BASE, _) => Err(BondError::UnsupportedType(kind)),
            _ => Ok(Some((kind, id))),
        }
    }

    fn read_struct(&mut self, depth: usize) -> Result<BondStruct, BondError> {
        if depth >= MAX_DEPTH {
            return Err(BondError::TooDeep);
        }
        let mut result = BondStruct::default();
        let mut current: Vec<Field> = Vec::new();
        loop {
            match self.field_header()? {
                None => break,
                Some((BT_STOP_BASE, _)) => {
                    // Only one inheritance level is supported (the per-device Night light values).
                    if result.derived.is_some() {
                        return Err(BondError::UnsupportedType(BT_STOP_BASE));
                    }
                    result.fields = std::mem::take(&mut current);
                    result.derived = Some(Vec::new());
                }
                Some((kind, id)) => {
                    if current.last().is_some_and(|last| last.id >= id) {
                        return Err(BondError::Order);
                    }
                    let value = self.read_value(kind, id, depth)?;
                    current.push(Field { id, value });
                }
            }
        }
        match result.derived {
            Some(ref mut derived) => *derived = current,
            None => result.fields = current,
        }
        Ok(result)
    }

    fn read_value(&mut self, kind: u8, id: u16, depth: usize) -> Result<Value, BondError> {
        Ok(match kind {
            BT_BOOL => match self.byte()? {
                0 => Value::Bool(false),
                1 => Value::Bool(true),
                _ => return Err(BondError::Range(id)),
            },
            BT_INT8 => Value::Int8(i8::from_le_bytes([self.byte()?])),
            BT_INT16 => {
                let raw = unzigzag(self.varint()?);
                Value::Int16(i16::try_from(raw).map_err(|_| BondError::Range(id))?)
            }
            BT_INT32 => {
                let raw = unzigzag(self.varint()?);
                Value::Int32(i32::try_from(raw).map_err(|_| BondError::Range(id))?)
            }
            BT_UINT64 => Value::UInt64(self.varint()?),
            BT_STRUCT => Value::Struct(self.read_struct(depth + 1)?),
            BT_LIST => {
                let element = self.byte()?;
                if element != BT_INT8 {
                    return Err(BondError::UnsupportedType(element));
                }
                let count = usize::try_from(self.varint()?).map_err(|_| BondError::Truncated)?;
                Value::Bytes(self.take(count)?.to_vec())
            }
            other => return Err(BondError::UnsupportedType(other)),
        })
    }
}

fn write_varint(out: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        out.push((value & 0x7F) as u8 | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

fn zigzag(value: i64) -> u64 {
    ((value << 1) ^ (value >> 63)) as u64
}

fn unzigzag(value: u64) -> i64 {
    ((value >> 1) as i64) ^ -((value & 1) as i64)
}

fn write_field_header(out: &mut Vec<u8>, kind: u8, id: u16) {
    if id <= 5 {
        out.push(kind | ((id as u8) << 5));
    } else if let Ok(byte) = u8::try_from(id) {
        out.push(kind | 0xC0);
        out.push(byte);
    } else {
        out.push(kind | 0xE0);
        out.extend_from_slice(&id.to_le_bytes());
    }
}

fn write_fields(out: &mut Vec<u8>, fields: &[Field]) {
    for field in fields {
        let kind = match field.value {
            Value::Bool(_) => BT_BOOL,
            Value::Int8(_) => BT_INT8,
            Value::Int16(_) => BT_INT16,
            Value::Int32(_) => BT_INT32,
            Value::UInt64(_) => BT_UINT64,
            Value::Struct(_) => BT_STRUCT,
            Value::Bytes(_) => BT_LIST,
        };
        write_field_header(out, kind, field.id);
        match &field.value {
            Value::Bool(value) => out.push(u8::from(*value)),
            Value::Int8(value) => out.extend_from_slice(&value.to_le_bytes()),
            Value::Int16(value) => write_varint(out, zigzag(i64::from(*value))),
            Value::Int32(value) => write_varint(out, zigzag(i64::from(*value))),
            Value::UInt64(value) => write_varint(out, *value),
            Value::Struct(value) => write_struct(out, value),
            Value::Bytes(bytes) => {
                out.push(BT_INT8);
                write_varint(out, bytes.len() as u64);
                out.extend_from_slice(bytes);
            }
        }
    }
}

fn write_struct(out: &mut Vec<u8>, value: &BondStruct) {
    write_fields(out, &value.fields);
    if let Some(derived) = &value.derived {
        out.push(BT_STOP_BASE);
        write_fields(out, derived);
    }
    out.push(BT_STOP);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(text: &str) -> Vec<u8> {
        let clean: String = text.chars().filter(|c| !c.is_whitespace()).collect();
        (0..clean.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&clean[i..i + 2], 16).expect("hex"))
            .collect()
    }

    #[test]
    fn varints_round_trip() {
        for value in [0u64, 1, 127, 128, 300, 1_731_063_302, u64::MAX] {
            let mut out = Vec::new();
            write_varint(&mut out, value);
            let mut reader = Reader {
                bytes: &out,
                pos: 0,
            };
            assert_eq!(reader.varint(), Ok(value));
            assert_eq!(reader.pos, out.len());
        }
        // 11 continuation bytes, and a 10th byte with more than one bit, are invalid.
        let mut reader = Reader {
            bytes: &[0xFF; 11],
            pos: 0,
        };
        assert_eq!(reader.varint(), Err(BondError::Varint));
        let mut too_big = vec![0xFF; 9];
        too_big.push(0x02);
        let mut reader = Reader {
            bytes: &too_big,
            pos: 0,
        };
        assert_eq!(reader.varint(), Err(BondError::Varint));
    }

    #[test]
    fn zigzag_matches_bond() {
        assert_eq!(zigzag(0), 0);
        assert_eq!(zigzag(-1), 1);
        assert_eq!(zigzag(1), 2);
        assert_eq!(zigzag(4000), 8000);
        for value in [-70_000i64, -1, 0, 1, 2000, 6500, i64::from(i32::MAX)] {
            assert_eq!(unzigzag(zigzag(value)), value);
        }
    }

    #[test]
    fn field_headers_use_the_short_forms() {
        let mut out = Vec::new();
        write_field_header(&mut out, BT_STRUCT, 1);
        write_field_header(&mut out, BT_BOOL, 10);
        write_field_header(&mut out, BT_INT16, 40);
        write_field_header(&mut out, BT_INT32, 300);
        assert_eq!(out, vec![0x2A, 0xC2, 0x0A, 0xCF, 0x28, 0xF0, 0x2C, 0x01]);
    }

    #[test]
    fn decodes_a_night_light_state_value() {
        // Night light state as stored by Windows 11 (this machine, 2024).
        let bytes = hex(
            "434201000A0201002A0686DCB7B9062A2B0E1343420100D00A02C614F580A78BE8B2A7EC0100000000",
        );
        let outer = decode_marshalled(&bytes).expect("decode");
        let Some(Value::Struct(meta)) = outer.get(0) else {
            panic!("field 0");
        };
        assert_eq!(meta.get(0), Some(&Value::Bool(true)));
        let Some(Value::Struct(container)) = outer.get(1) else {
            panic!("field 1");
        };
        assert_eq!(container.get(0), Some(&Value::UInt64(1_731_063_302)));
        let Some(Value::Struct(wrapper)) = container.get(1) else {
            panic!("field 1.1");
        };
        let Some(Value::Bytes(payload)) = wrapper.get(1) else {
            panic!("payload");
        };
        let inner = decode_marshalled(payload).expect("payload");
        assert_eq!(inner.get(10), Some(&Value::Int32(1)));
        assert_eq!(inner.get(20), Some(&Value::UInt64(133_029_458_749_276_277)));
        assert_eq!(inner.derived, None);
        assert_eq!(encode_marshalled(&outer), bytes);
    }

    #[test]
    fn keeps_the_base_marker_of_derived_structs() {
        // Per-device state (Windows 11 24H2+): the base fields, BT_STOP_BASE, no own fields.
        let payload = hex("43420100D00A02C614F580A78BE8B2A7EC010100");
        let value = decode_marshalled(&payload).expect("decode");
        assert_eq!(value.fields.len(), 2);
        assert_eq!(value.derived, Some(Vec::new()));
        assert_eq!(encode_marshalled(&value), payload);
    }

    #[test]
    fn set_and_remove_keep_ids_ascending() {
        let mut value = BondStruct::default();
        value.set(50, Value::Bool(true));
        value.set(10, Value::Bool(false));
        value.set(40, Value::Int16(3400));
        value.set(40, Value::Int16(2000));
        let ids: Vec<u16> = value.fields.iter().map(|field| field.id).collect();
        assert_eq!(ids, vec![10, 40, 50]);
        assert_eq!(value.get(40), Some(&Value::Int16(2000)));
        value.remove(40);
        value.remove(70);
        assert_eq!(value.get(40), None);
        assert_eq!(value.fields.len(), 2);
    }

    #[test]
    fn rejects_what_it_does_not_understand() {
        assert_eq!(
            decode_marshalled(&hex("43420200 00")),
            Err(BondError::Header)
        );
        assert_eq!(
            decode_marshalled(&hex("43420100")),
            Err(BondError::Truncated)
        );
        // A double (type 8) field.
        assert_eq!(
            decode_marshalled(&hex("43420100 08 0000000000000000 00")),
            Err(BondError::UnsupportedType(8))
        );
        // A string (type 9) field.
        assert_eq!(
            decode_marshalled(&hex("43420100 09 01 41 00")),
            Err(BondError::UnsupportedType(9))
        );
        // A list of something other than int8.
        assert_eq!(
            decode_marshalled(&hex("43420100 2B 10 01 02 00")),
            Err(BondError::UnsupportedType(16))
        );
        // A bool that is neither 0 nor 1.
        assert_eq!(
            decode_marshalled(&hex("43420100 02 05 00")),
            Err(BondError::Range(0))
        );
        // Field ids going down.
        assert_eq!(
            decode_marshalled(&hex("43420100 C2 0A 00 02 01 00")),
            Err(BondError::Order)
        );
        // Bytes after the final stop.
        assert_eq!(
            decode_marshalled(&hex("43420100 00 00")),
            Err(BondError::Trailing(1))
        );
        // A long-form header for an id that has a short form.
        assert_eq!(
            decode_marshalled(&hex("43420100 C2 01 01 00")),
            Err(BondError::NonCanonical)
        );
        // A non-minimal varint.
        assert_eq!(
            decode_marshalled(&hex("43420100 06 80 00 00")),
            Err(BondError::NonCanonical)
        );
        // Deep nesting.
        let mut deep = HEADER.to_vec();
        deep.extend(std::iter::repeat_n(0x0A, MAX_DEPTH + 1));
        deep.extend(std::iter::repeat_n(0x00, MAX_DEPTH + 2));
        assert_eq!(decode_marshalled(&deep), Err(BondError::TooDeep));
        // Truncated list.
        assert_eq!(
            decode_marshalled(&hex("43420100 2B 0E 05 01 02")),
            Err(BondError::Truncated)
        );
    }

    #[test]
    fn int16_out_of_range_is_rejected() {
        // zigzag(40000) = 80000 = 80 f1 04
        assert_eq!(
            decode_marshalled(&hex("43420100 CF 28 80 F1 04 00")),
            Err(BondError::Range(40))
        );
    }
}
