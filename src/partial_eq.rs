#[cfg(not(feature = "std"))]
use alloc::string::String;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use crate::JsonValue;

fn eq_i64(value: &JsonValue, other: i64) -> bool {
    value.as_i64() == Some(other)
}

fn eq_u64(value: &JsonValue, other: u64) -> bool {
    value.as_u64() == Some(other)
}

fn eq_f32(value: &JsonValue, other: f32) -> bool {
    value.as_f32() == Some(other)
}

fn eq_f64(value: &JsonValue, other: f64) -> bool {
    value.as_f64() == Some(other)
}

fn eq_bool(value: &JsonValue, other: bool) -> bool {
    value.as_bool() == Some(other)
}

fn eq_str(value: &JsonValue, other: &str) -> bool {
    value.as_str() == Some(other)
}

impl PartialEq<str> for JsonValue {
    fn eq(&self, other: &str) -> bool {
        eq_str(self, other)
    }
}

impl PartialEq<&str> for JsonValue {
    fn eq(&self, other: &&str) -> bool {
        eq_str(self, other)
    }
}

impl PartialEq<JsonValue> for str {
    fn eq(&self, other: &JsonValue) -> bool {
        eq_str(other, self)
    }
}

impl PartialEq<JsonValue> for &str {
    fn eq(&self, other: &JsonValue) -> bool {
        eq_str(other, self)
    }
}

impl PartialEq<String> for JsonValue {
    fn eq(&self, other: &String) -> bool {
        eq_str(self, other.as_str())
    }
}

impl PartialEq<JsonValue> for String {
    fn eq(&self, other: &JsonValue) -> bool {
        eq_str(other, self.as_str())
    }
}

macro_rules! partialeq_numeric {
    ($($eq:ident [$($ty:ty)*])*) => {
        $($(
            impl PartialEq<$ty> for JsonValue {
                fn eq(&self, other: &$ty) -> bool {
                    $eq(self, *other as _)
                }
            }

            impl PartialEq<JsonValue> for $ty {
                fn eq(&self, other: &JsonValue) -> bool {
                    $eq(other, *self as _)
                }
            }

            impl<'a> PartialEq<$ty> for &'a JsonValue {
                fn eq(&self, other: &$ty) -> bool {
                    $eq(*self, *other as _)
                }
            }

            impl<'a> PartialEq<$ty> for &'a mut JsonValue {
                fn eq(&self, other: &$ty) -> bool {
                    $eq(*self, *other as _)
                }
            }
        )*)*
    }
}

partialeq_numeric! {
    eq_i64[i8 i16 i32 i64 isize]
    eq_u64[u8 u16 u32 u64 usize]
    eq_f32[f32]
    eq_f64[f64]
    eq_bool[bool]
}
