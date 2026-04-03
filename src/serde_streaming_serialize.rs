//! Streaming serde serializer — writes directly to `Vec<u8>` without
//! building an intermediate `JsonValue` tree.
//!
//! This is the single most impactful optimisation for the `to_string<T>` /
//! `to_vec<T>` paths: the old implementation went through
//! `to_value → JsonValue → to_json_string`, allocating a full AST then
//! walking it a second time.

#[cfg(not(feature = "std"))]
use alloc::string::String;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use crate::serde_error::Error;
use crate::util;
use serde_crate::ser::{
    SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant,
};
use serde_crate::{Serialize, Serializer as SerdeSerializer};

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Serialise any `Serialize` value directly into a `Vec<u8>`.
pub fn to_vec<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, Error> {
    let estimate = value.serialize(SizeEstimator).unwrap_or(64);
    let mut out = Vec::with_capacity(estimate);
    value.serialize(StreamingSerializer { out: &mut out })?;
    Ok(out)
}

/// Serialise any `Serialize` value directly into a `String`.
pub fn to_string<T: Serialize + ?Sized>(value: &T) -> Result<String, Error> {
    let bytes = to_vec(value)?;
    Ok(unsafe { String::from_utf8_unchecked(bytes) })
}

// ---------------------------------------------------------------------------
// Streaming serializer
// ---------------------------------------------------------------------------

pub struct StreamingSerializer<'a> {
    out: &'a mut Vec<u8>,
}

impl SerdeSerializer for StreamingSerializer<'_> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = StreamingSeqSerializer<'static>;
    type SerializeTuple = StreamingSeqSerializer<'static>;
    type SerializeTupleStruct = StreamingSeqSerializer<'static>;
    type SerializeTupleVariant = StreamingTupleVariantSerializer<'static>;
    type SerializeMap = StreamingMapSerializer<'static>;
    type SerializeStruct = StreamingStructSerializer<'static>;
    type SerializeStructVariant = StreamingStructVariantSerializer<'static>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Error> {
        if v {
            self.out.extend_from_slice(b"true");
        } else {
            self.out.extend_from_slice(b"false");
        }
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Error> {
        util::append_i64(self.out, i64::from(v));
        Ok(())
    }
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Error> {
        util::append_i64(self.out, i64::from(v));
        Ok(())
    }
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Error> {
        util::append_i64(self.out, i64::from(v));
        Ok(())
    }
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Error> {
        util::append_i64(self.out, v);
        Ok(())
    }
    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Error> {
        util::append_i128(self.out, v);
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Error> {
        util::append_u64(self.out, u64::from(v));
        Ok(())
    }
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Error> {
        util::append_u64(self.out, u64::from(v));
        Ok(())
    }
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Error> {
        util::append_u64(self.out, u64::from(v));
        Ok(())
    }
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Error> {
        util::append_u64(self.out, v);
        Ok(())
    }
    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Error> {
        util::append_u128(self.out, v);
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Error> {
        self.serialize_f64(f64::from(v))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Error> {
        if !v.is_finite() {
            return Err(Error::custom("cannot serialize non-finite float"));
        }
        util::append_f64(self.out, v);
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Error> {
        let mut buf = [0u8; 4];
        let s = v.encode_utf8(&mut buf);
        util::write_escaped_json_string(self.out, s);
        Ok(())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Error> {
        util::write_escaped_json_string(self.out, v);
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Error> {
        self.out.push(b'[');
        for (i, &b) in v.iter().enumerate() {
            if i > 0 {
                self.out.push(b',');
            }
            util::append_u64(self.out, u64::from(b));
        }
        self.out.push(b']');
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok, Error> {
        self.out.extend_from_slice(b"null");
        Ok(())
    }

    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<Self::Ok, Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Error> {
        self.out.extend_from_slice(b"null");
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Error> {
        self.out.extend_from_slice(b"null");
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Error> {
        util::write_escaped_json_string(self.out, variant);
        Ok(())
    }

    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Error> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Error> {
        self.out.push(b'{');
        util::write_escaped_json_string(self.out, variant);
        self.out.push(b':');
        value.serialize(StreamingSerializer { out: self.out })?;
        self.out.push(b'}');
        Ok(())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Error> {
        self.out.push(b'[');
        Ok(StreamingSeqSerializer {
            out: unsafe { extend_lifetime_mut(self.out) },
            first: true,
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Error> {
        self.out.push(b'{');
        util::write_escaped_json_string(self.out, variant);
        self.out.extend_from_slice(b":[");
        Ok(StreamingTupleVariantSerializer {
            out: unsafe { extend_lifetime_mut(self.out) },
            first: true,
            _len: len,
        })
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Error> {
        self.out.push(b'{');
        Ok(StreamingMapSerializer {
            out: unsafe { extend_lifetime_mut(self.out) },
            first: true,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Error> {
        self.out.push(b'{');
        Ok(StreamingStructSerializer {
            out: unsafe { extend_lifetime_mut(self.out) },
            first: true,
            remaining: len,
        })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Error> {
        self.out.push(b'{');
        util::write_escaped_json_string(self.out, variant);
        self.out.push(b':');
        self.out.push(b'{');
        Ok(StreamingStructVariantSerializer {
            out: unsafe { extend_lifetime_mut(self.out) },
            first: true,
            remaining: len,
        })
    }
}

// SAFETY: The serializers borrow `out` from the parent serializer which
// outlives the entire serialization call. We use this to satisfy the
// `SerializeSeq` / `SerializeMap` etc. lifetime requirements.
unsafe fn extend_lifetime_mut<T>(r: &mut T) -> &'static mut T {
    &mut *(r as *mut T)
}

// ---------------------------------------------------------------------------
// Sequence serializer
// ---------------------------------------------------------------------------

pub struct StreamingSeqSerializer<'a> {
    out: &'a mut Vec<u8>,
    first: bool,
}

impl SerializeSeq for StreamingSeqSerializer<'_> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        if !self.first {
            self.out.push(b',');
        }
        self.first = false;
        value.serialize(StreamingSerializer { out: self.out })
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.out.push(b']');
        Ok(())
    }
}

impl SerializeTuple for StreamingSeqSerializer<'_> {
    type Ok = ();
    type Error = Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeSeq::end(self)
    }
}

impl SerializeTupleStruct for StreamingSeqSerializer<'_> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<Self::Ok, Self::Error> {
        SerializeSeq::end(self)
    }
}

// ---------------------------------------------------------------------------
// Tuple variant serializer
// ---------------------------------------------------------------------------

pub struct StreamingTupleVariantSerializer<'a> {
    out: &'a mut Vec<u8>,
    first: bool,
    _len: usize,
}

impl SerializeTupleVariant for StreamingTupleVariantSerializer<'_> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        if !self.first {
            self.out.push(b',');
        }
        self.first = false;
        value.serialize(StreamingSerializer { out: self.out })
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.out.extend_from_slice(b"]}");
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Map serializer
// ---------------------------------------------------------------------------

pub struct StreamingMapSerializer<'a> {
    out: &'a mut Vec<u8>,
    first: bool,
}

impl SerializeMap for StreamingMapSerializer<'_> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Error> {
        if !self.first {
            self.out.push(b',');
        }
        self.first = false;
        key.serialize(StreamingKeySerializer { out: self.out })
    }

    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.out.push(b':');
        value.serialize(StreamingSerializer { out: self.out })
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.out.push(b'}');
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Struct serializer (known keys at compile time — hot path)
// ---------------------------------------------------------------------------

pub struct StreamingStructSerializer<'a> {
    out: &'a mut Vec<u8>,
    first: bool,
    remaining: usize,
}

impl SerializeStruct for StreamingStructSerializer<'_> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        if !self.first {
            self.out.push(b',');
        }
        self.first = false;
        self.remaining -= 1;
        util::write_json_key(self.out, key);
        value.serialize(StreamingSerializer { out: self.out })
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.out.push(b'}');
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Struct variant serializer
// ---------------------------------------------------------------------------

pub struct StreamingStructVariantSerializer<'a> {
    out: &'a mut Vec<u8>,
    first: bool,
    remaining: usize,
}

impl SerializeStructVariant for StreamingStructVariantSerializer<'_> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        if !self.first {
            self.out.push(b',');
        }
        self.first = false;
        self.remaining -= 1;
        util::write_json_key(self.out, key);
        value.serialize(StreamingSerializer { out: self.out })
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.out.extend_from_slice(b"}}");
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Key serializer (for map keys — must produce a JSON string key)
// ---------------------------------------------------------------------------

struct StreamingKeySerializer<'a> {
    out: &'a mut Vec<u8>,
}

impl SerdeSerializer for StreamingKeySerializer<'_> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = serde_crate::ser::Impossible<(), Error>;
    type SerializeTuple = serde_crate::ser::Impossible<(), Error>;
    type SerializeTupleStruct = serde_crate::ser::Impossible<(), Error>;
    type SerializeTupleVariant = serde_crate::ser::Impossible<(), Error>;
    type SerializeMap = serde_crate::ser::Impossible<(), Error>;
    type SerializeStruct = serde_crate::ser::Impossible<(), Error>;
    type SerializeStructVariant = serde_crate::ser::Impossible<(), Error>;

    fn serialize_str(self, value: &str) -> Result<Self::Ok, Error> {
        util::write_escaped_json_string(self.out, value);
        Ok(())
    }

    fn serialize_bool(self, value: bool) -> Result<Self::Ok, Error> {
        if value {
            self.out.extend_from_slice(b"\"true\"");
        } else {
            self.out.extend_from_slice(b"\"false\"");
        }
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Error> {
        self.out.push(b'"');
        util::append_i64(self.out, i64::from(v));
        self.out.push(b'"');
        Ok(())
    }
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Error> {
        self.out.push(b'"');
        util::append_i64(self.out, i64::from(v));
        self.out.push(b'"');
        Ok(())
    }
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Error> {
        self.out.push(b'"');
        util::append_i64(self.out, i64::from(v));
        self.out.push(b'"');
        Ok(())
    }
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Error> {
        self.out.push(b'"');
        util::append_i64(self.out, v);
        self.out.push(b'"');
        Ok(())
    }
    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Error> {
        self.out.push(b'"');
        util::append_i128(self.out, v);
        self.out.push(b'"');
        Ok(())
    }
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Error> {
        self.out.push(b'"');
        util::append_u64(self.out, u64::from(v));
        self.out.push(b'"');
        Ok(())
    }
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Error> {
        self.out.push(b'"');
        util::append_u64(self.out, u64::from(v));
        self.out.push(b'"');
        Ok(())
    }
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Error> {
        self.out.push(b'"');
        util::append_u64(self.out, u64::from(v));
        self.out.push(b'"');
        Ok(())
    }
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Error> {
        self.out.push(b'"');
        util::append_u64(self.out, v);
        self.out.push(b'"');
        Ok(())
    }
    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Error> {
        self.out.push(b'"');
        util::append_u128(self.out, v);
        self.out.push(b'"');
        Ok(())
    }
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Error> {
        self.out.push(b'"');
        util::append_f64(self.out, f64::from(v));
        self.out.push(b'"');
        Ok(())
    }
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Error> {
        self.out.push(b'"');
        util::append_f64(self.out, v);
        self.out.push(b'"');
        Ok(())
    }
    fn serialize_char(self, value: char) -> Result<Self::Ok, Error> {
        let mut buf = [0u8; 4];
        let s = value.encode_utf8(&mut buf);
        util::write_json_key(self.out, s);
        Ok(())
    }

    fn serialize_bytes(self, _value: &[u8]) -> Result<Self::Ok, Error> {
        Err(Error::custom("JSON object keys cannot be bytes"))
    }

    fn serialize_none(self) -> Result<Self::Ok, Error> {
        self.out.extend_from_slice(b"\"null\"");
        Ok(())
    }

    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<Self::Ok, Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Error> {
        self.out.extend_from_slice(b"\"null\"");
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Error> {
        self.out.extend_from_slice(b"\"null\"");
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Error> {
        util::write_json_key(self.out, variant);
        Ok(())
    }

    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Error> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Error> {
        util::write_json_key(self.out, variant);
        Ok(())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Error> {
        Err(Error::custom("JSON object keys must be strings"))
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Error> {
        Err(Error::custom("JSON object keys must be strings"))
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Error> {
        Err(Error::custom("JSON object keys must be strings"))
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Error> {
        Err(Error::custom("JSON object keys must be strings"))
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Error> {
        Err(Error::custom("JSON object keys must be strings"))
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Error> {
        Err(Error::custom("JSON object keys must be strings"))
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Error> {
        Err(Error::custom("JSON object keys must be strings"))
    }
}

// ---------------------------------------------------------------------------
// Size estimator (cheap, used only for Vec capacity pre-allocation)
// ---------------------------------------------------------------------------

struct SizeEstimator;

impl SerdeSerializer for SizeEstimator {
    type Ok = usize;
    type Error = Error;
    type SerializeSeq = SeqSizeEst;
    type SerializeTuple = SeqSizeEst;
    type SerializeTupleStruct = SeqSizeEst;
    type SerializeTupleVariant = TupleVarSizeEst;
    type SerializeMap = MapSizeEst;
    type SerializeStruct = StructSizeEst;
    type SerializeStructVariant = StructVarSizeEst;

    fn serialize_bool(self, v: bool) -> Result<usize, Error> {
        Ok(if v { 4 } else { 5 })
    }
    fn serialize_i8(self, _v: i8) -> Result<usize, Error> {
        Ok(4)
    }
    fn serialize_i16(self, _v: i16) -> Result<usize, Error> {
        Ok(6)
    }
    fn serialize_i32(self, _v: i32) -> Result<usize, Error> {
        Ok(11)
    }
    fn serialize_i64(self, _v: i64) -> Result<usize, Error> {
        Ok(20)
    }
    fn serialize_i128(self, _v: i128) -> Result<usize, Error> {
        Ok(40)
    }
    fn serialize_u8(self, _v: u8) -> Result<usize, Error> {
        Ok(3)
    }
    fn serialize_u16(self, _v: u16) -> Result<usize, Error> {
        Ok(5)
    }
    fn serialize_u32(self, _v: u32) -> Result<usize, Error> {
        Ok(10)
    }
    fn serialize_u64(self, _v: u64) -> Result<usize, Error> {
        Ok(20)
    }
    fn serialize_u128(self, _v: u128) -> Result<usize, Error> {
        Ok(39)
    }
    fn serialize_f32(self, _v: f32) -> Result<usize, Error> {
        Ok(16)
    }
    fn serialize_f64(self, _v: f64) -> Result<usize, Error> {
        Ok(24)
    }
    fn serialize_char(self, v: char) -> Result<usize, Error> {
        Ok(2 + v.len_utf8())
    }
    fn serialize_str(self, v: &str) -> Result<usize, Error> {
        Ok(2 + v.len())
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<usize, Error> {
        Ok(2 + v.len().saturating_mul(4))
    }
    fn serialize_none(self) -> Result<usize, Error> {
        Ok(4)
    }
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<usize, Error> {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<usize, Error> {
        Ok(4)
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<usize, Error> {
        Ok(4)
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<usize, Error> {
        Ok(2 + variant.len())
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<usize, Error> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<usize, Error> {
        let inner = value.serialize(self)?;
        Ok(2 + variant.len() + 1 + inner)
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Error> {
        Ok(SeqSizeEst {
            total: 2 + len.unwrap_or(0) * 16,
        })
    }
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Error> {
        Ok(SeqSizeEst {
            total: 2 + len * 16,
        })
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Error> {
        Ok(SeqSizeEst {
            total: 2 + len * 16,
        })
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Error> {
        Ok(TupleVarSizeEst {
            total: 4 + variant.len() + 1,
            elem_est: len,
        })
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Error> {
        Ok(MapSizeEst { total: 2 })
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Error> {
        Ok(StructSizeEst { total: 2 })
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Error> {
        Ok(StructVarSizeEst {
            total: 4 + variant.len() + 2,
        })
    }
}

struct SeqSizeEst {
    total: usize,
}
impl SerializeSeq for SeqSizeEst {
    type Ok = usize;
    type Error = Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), Error> {
        if self.total > 2 {
            self.total += 1;
        }
        self.total += v.serialize(SizeEstimator).unwrap_or(16);
        Ok(())
    }
    fn end(self) -> Result<usize, Error> {
        Ok(self.total)
    }
}
impl SerializeTuple for SeqSizeEst {
    type Ok = usize;
    type Error = Error;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), Error> {
        SerializeSeq::serialize_element(self, v)
    }
    fn end(self) -> Result<usize, Error> {
        Ok(self.total)
    }
}
impl SerializeTupleStruct for SeqSizeEst {
    type Ok = usize;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), Error> {
        SerializeSeq::serialize_element(self, v)
    }
    fn end(self) -> Result<usize, Error> {
        Ok(self.total)
    }
}

struct TupleVarSizeEst {
    total: usize,
    elem_est: usize,
}
impl SerializeTupleVariant for TupleVarSizeEst {
    type Ok = usize;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), Error> {
        if self.total > 5 + self.elem_est {
            self.total += 1;
        }
        self.total += v.serialize(SizeEstimator).unwrap_or(16);
        Ok(())
    }
    fn end(self) -> Result<usize, Error> {
        Ok(self.total + 2)
    }
}

struct MapSizeEst {
    total: usize,
}
impl SerializeMap for MapSizeEst {
    type Ok = usize;
    type Error = Error;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, k: &T) -> Result<(), Error> {
        if self.total > 2 {
            self.total += 1;
        }
        self.total += k.serialize(SizeEstimator).unwrap_or(16) + 1;
        Ok(())
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, v: &T) -> Result<(), Error> {
        self.total += v.serialize(SizeEstimator).unwrap_or(16);
        Ok(())
    }
    fn end(self) -> Result<usize, Error> {
        Ok(self.total)
    }
}

struct StructSizeEst {
    total: usize,
}
impl SerializeStruct for StructSizeEst {
    type Ok = usize;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        k: &'static str,
        v: &T,
    ) -> Result<(), Error> {
        if self.total > 2 {
            self.total += 1;
        }
        self.total += 2 + k.len() + 1 + v.serialize(SizeEstimator).unwrap_or(16);
        Ok(())
    }
    fn end(self) -> Result<usize, Error> {
        Ok(self.total)
    }
}

struct StructVarSizeEst {
    total: usize,
}
impl SerializeStructVariant for StructVarSizeEst {
    type Ok = usize;
    type Error = Error;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        k: &'static str,
        v: &T,
    ) -> Result<(), Error> {
        if self.total > 5 {
            self.total += 1;
        }
        self.total += 2 + k.len() + 1 + v.serialize(SizeEstimator).unwrap_or(16);
        Ok(())
    }
    fn end(self) -> Result<usize, Error> {
        Ok(self.total + 2)
    }
}
