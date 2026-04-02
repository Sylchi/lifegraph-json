use crate::serde_error::{self, Error};
use crate::{
    initial_json_capacity, write_json_value, write_json_value_pretty, JsonNumber, JsonValue, Map,
};
use serde_crate::ser::{
    SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant,
};
use serde_crate::{Serialize, Serializer as SerdeSerializer};
use std::io::Write;

pub struct JsonValueSerializer;

impl SerdeSerializer for JsonValueSerializer {
    type Ok = JsonValue;
    type Error = serde_error::Error;
    type SerializeSeq = JsonArraySerializer;
    type SerializeTuple = JsonArraySerializer;
    type SerializeTupleStruct = JsonArraySerializer;
    type SerializeTupleVariant = JsonTupleVariantSerializer;
    type SerializeMap = JsonObjectSerializer;
    type SerializeStruct = JsonObjectSerializer;
    type SerializeStructVariant = JsonStructVariantSerializer;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, serde_error::Error> {
        Ok(JsonValue::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, serde_error::Error> {
        Ok(JsonValue::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, serde_error::Error> {
        Ok(JsonValue::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, serde_error::Error> {
        Ok(JsonValue::from(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, serde_error::Error> {
        Ok(JsonValue::from(v))
    }

    fn serialize_i128(self, v: i128) -> Result<Self::Ok, serde_error::Error> {
        Ok(JsonValue::from(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, serde_error::Error> {
        Ok(JsonValue::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, serde_error::Error> {
        Ok(JsonValue::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, serde_error::Error> {
        Ok(JsonValue::from(v))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, serde_error::Error> {
        Ok(JsonValue::from(v))
    }

    fn serialize_u128(self, v: u128) -> Result<Self::Ok, serde_error::Error> {
        Ok(JsonValue::from(v))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, serde_error::Error> {
        JsonNumber::from_f64(v as f64)
            .map(JsonValue::Number)
            .ok_or_else(|| {
                serde_error::Error::custom("cannot serialize non-finite floating-point value")
            })
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, serde_error::Error> {
        JsonNumber::from_f64(v)
            .map(JsonValue::Number)
            .ok_or_else(|| {
                serde_error::Error::custom("cannot serialize non-finite floating-point value")
            })
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, serde_error::Error> {
        Ok(JsonValue::String(v.to_string()))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, serde_error::Error> {
        Ok(JsonValue::String(v.to_string()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, serde_error::Error> {
        Ok(JsonValue::Array(
            v.iter().copied().map(JsonValue::from).collect(),
        ))
    }

    fn serialize_none(self) -> Result<Self::Ok, serde_error::Error> {
        Ok(JsonValue::Null)
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, serde_error::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, serde_error::Error> {
        Ok(JsonValue::Null)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, serde_error::Error> {
        Ok(JsonValue::Null)
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, serde_error::Error> {
        Ok(JsonValue::String(variant.to_string()))
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, serde_error::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, serde_error::Error>
    where
        T: ?Sized + Serialize,
    {
        let mut map = Map::new();
        map.insert(variant.to_string(), value.serialize(JsonValueSerializer)?);
        Ok(JsonValue::Object(map))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(JsonArraySerializer {
            items: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(JsonTupleVariantSerializer {
            variant: variant.to_string(),
            items: Vec::with_capacity(len),
        })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(JsonObjectSerializer {
            map: Map::new(),
            next_key: None,
            len,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(JsonStructVariantSerializer {
            variant: variant.to_string(),
            map: Map::new(),
            len,
        })
    }
}

pub struct JsonArraySerializer {
    items: Vec<JsonValue>,
}

impl SerializeSeq for JsonArraySerializer {
    type Ok = JsonValue;
    type Error = serde_error::Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), serde_error::Error>
    where
        T: ?Sized + Serialize,
    {
        self.items.push(value.serialize(JsonValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, serde_error::Error> {
        Ok(JsonValue::Array(self.items))
    }
}

impl SerializeTuple for JsonArraySerializer {
    type Ok = JsonValue;
    type Error = serde_error::Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), serde_error::Error>
    where
        T: ?Sized + Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, serde_error::Error> {
        SerializeSeq::end(self)
    }
}

impl SerializeTupleStruct for JsonArraySerializer {
    type Ok = JsonValue;
    type Error = serde_error::Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), serde_error::Error>
    where
        T: ?Sized + Serialize,
    {
        SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, serde_error::Error> {
        SerializeSeq::end(self)
    }
}

pub struct JsonTupleVariantSerializer {
    variant: String,
    items: Vec<JsonValue>,
}

impl SerializeTupleVariant for JsonTupleVariantSerializer {
    type Ok = JsonValue;
    type Error = serde_error::Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), serde_error::Error>
    where
        T: ?Sized + Serialize,
    {
        self.items.push(value.serialize(JsonValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, serde_error::Error> {
        let mut map = Map::new();
        map.insert(self.variant, JsonValue::Array(self.items));
        Ok(JsonValue::Object(map))
    }
}

pub struct JsonObjectSerializer {
    map: Map,
    next_key: Option<String>,
    len: Option<usize>,
}

impl SerializeMap for JsonObjectSerializer {
    type Ok = JsonValue;
    type Error = serde_error::Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), serde_error::Error>
    where
        T: ?Sized + Serialize,
    {
        let _ = self.len;
        self.next_key = Some(key.serialize(JsonKeySerializer)?);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), serde_error::Error>
    where
        T: ?Sized + Serialize,
    {
        let key = self.next_key.take().ok_or_else(|| {
            serde_error::Error::custom("serialize_value called before serialize_key")
        })?;
        self.map.insert(key, value.serialize(JsonValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, serde_error::Error> {
        Ok(JsonValue::Object(self.map))
    }
}

impl SerializeStruct for JsonObjectSerializer {
    type Ok = JsonValue;
    type Error = serde_error::Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), serde_error::Error>
    where
        T: ?Sized + Serialize,
    {
        self.map
            .insert(key.to_string(), value.serialize(JsonValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, serde_error::Error> {
        Ok(JsonValue::Object(self.map))
    }
}

pub struct JsonStructVariantSerializer {
    variant: String,
    map: Map,
    len: usize,
}

impl SerializeStructVariant for JsonStructVariantSerializer {
    type Ok = JsonValue;
    type Error = serde_error::Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), serde_error::Error>
    where
        T: ?Sized + Serialize,
    {
        let _ = self.len;
        self.map
            .insert(key.to_string(), value.serialize(JsonValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, serde_error::Error> {
        let mut outer = Map::new();
        outer.insert(self.variant, JsonValue::Object(self.map));
        Ok(JsonValue::Object(outer))
    }
}

struct JsonKeySerializer;

impl SerdeSerializer for JsonKeySerializer {
    type Ok = String;
    type Error = serde_error::Error;
    type SerializeSeq = serde_crate::ser::Impossible<String, Error>;
    type SerializeTuple = serde_crate::ser::Impossible<String, Error>;
    type SerializeTupleStruct = serde_crate::ser::Impossible<String, Error>;
    type SerializeTupleVariant = serde_crate::ser::Impossible<String, Error>;
    type SerializeMap = serde_crate::ser::Impossible<String, Error>;
    type SerializeStruct = serde_crate::ser::Impossible<String, Error>;
    type SerializeStructVariant = serde_crate::ser::Impossible<String, Error>;

    fn serialize_str(self, value: &str) -> Result<Self::Ok, serde_error::Error> {
        Ok(value.to_string())
    }

    fn serialize_bool(self, value: bool) -> Result<Self::Ok, serde_error::Error> {
        Ok(value.to_string())
    }

    fn serialize_i8(self, value: i8) -> Result<Self::Ok, serde_error::Error> {
        Ok(value.to_string())
    }
    fn serialize_i16(self, value: i16) -> Result<Self::Ok, serde_error::Error> {
        Ok(value.to_string())
    }
    fn serialize_i32(self, value: i32) -> Result<Self::Ok, serde_error::Error> {
        Ok(value.to_string())
    }
    fn serialize_i64(self, value: i64) -> Result<Self::Ok, serde_error::Error> {
        Ok(value.to_string())
    }
    fn serialize_i128(self, value: i128) -> Result<Self::Ok, serde_error::Error> {
        Ok(value.to_string())
    }
    fn serialize_u8(self, value: u8) -> Result<Self::Ok, serde_error::Error> {
        Ok(value.to_string())
    }
    fn serialize_u16(self, value: u16) -> Result<Self::Ok, serde_error::Error> {
        Ok(value.to_string())
    }
    fn serialize_u32(self, value: u32) -> Result<Self::Ok, serde_error::Error> {
        Ok(value.to_string())
    }
    fn serialize_u64(self, value: u64) -> Result<Self::Ok, serde_error::Error> {
        Ok(value.to_string())
    }
    fn serialize_u128(self, value: u128) -> Result<Self::Ok, serde_error::Error> {
        Ok(value.to_string())
    }
    fn serialize_f32(self, value: f32) -> Result<Self::Ok, serde_error::Error> {
        Ok(value.to_string())
    }
    fn serialize_f64(self, value: f64) -> Result<Self::Ok, serde_error::Error> {
        Ok(value.to_string())
    }
    fn serialize_char(self, value: char) -> Result<Self::Ok, serde_error::Error> {
        Ok(value.to_string())
    }

    fn serialize_bytes(self, _value: &[u8]) -> Result<Self::Ok, serde_error::Error> {
        Err(serde_error::Error::custom(
            "JSON object keys must be strings",
        ))
    }

    fn serialize_none(self) -> Result<Self::Ok, serde_error::Error> {
        Ok("null".to_string())
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, serde_error::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, serde_error::Error> {
        Ok("null".to_string())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, serde_error::Error> {
        Ok("null".to_string())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, serde_error::Error> {
        Ok(variant.to_string())
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, serde_error::Error>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, serde_error::Error>
    where
        T: ?Sized + Serialize,
    {
        Ok(variant.to_string())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(serde_error::Error::custom(
            "JSON object keys must be strings",
        ))
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(serde_error::Error::custom(
            "JSON object keys must be strings",
        ))
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(serde_error::Error::custom(
            "JSON object keys must be strings",
        ))
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(serde_error::Error::custom(
            "JSON object keys must be strings",
        ))
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(serde_error::Error::custom(
            "JSON object keys must be strings",
        ))
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(serde_error::Error::custom(
            "JSON object keys must be strings",
        ))
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(serde_error::Error::custom(
            "JSON object keys must be strings",
        ))
    }
}

impl serde_crate::Serialize for JsonNumber {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: SerdeSerializer,
    {
        match self {
            JsonNumber::I64(v) => serializer.serialize_i64(*v),
            JsonNumber::U64(v) => serializer.serialize_u64(*v),
            JsonNumber::F64(v) => serializer.serialize_f64(*v),
        }
    }
}

impl serde_crate::Serialize for Map {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: SerdeSerializer,
    {
        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (key, value) in self.iter() {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

impl serde_crate::Serialize for JsonValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: SerdeSerializer,
    {
        match self {
            JsonValue::Null => serializer.serialize_unit(),
            JsonValue::Bool(v) => serializer.serialize_bool(*v),
            JsonValue::Number(v) => v.serialize(serializer),
            JsonValue::String(v) => serializer.serialize_str(v),
            JsonValue::Array(v) => {
                let mut seq = serializer.serialize_seq(Some(v.len()))?;
                for item in v {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
            JsonValue::Object(v) => v.serialize(serializer),
        }
    }
}
