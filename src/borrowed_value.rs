#[cfg(not(feature = "std"))]
use alloc::string::String;
#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use crate::JsonValue;
#[cfg(not(feature = "std"))]
use alloc::borrow::Cow;
#[cfg(feature = "std")]
use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq)]
pub enum BorrowedJsonValue<'a> {
    Null,
    Bool(bool),
    Number(crate::JsonNumber),
    String(Cow<'a, str>),
    Array(Vec<BorrowedJsonValue<'a>>),
    Object(Vec<(Cow<'a, str>, BorrowedJsonValue<'a>)>),
}

impl BorrowedJsonValue<'_> {
    pub fn into_owned(self) -> JsonValue {
        match self {
            Self::Null => JsonValue::Null,
            Self::Bool(v) => JsonValue::Bool(v),
            Self::Number(n) => JsonValue::Number(n),
            Self::String(s) => JsonValue::String(s.into_owned()),
            Self::Array(v) => {
                JsonValue::Array(v.into_iter().map(BorrowedJsonValue::into_owned).collect())
            }
            Self::Object(e) => JsonValue::Object(
                e.into_iter()
                    .map(|(k, v)| (k.into_owned(), v.into_owned()))
                    .collect(),
            ),
        }
    }
}
