use crate::error::JsonError;
use crate::util;
use crate::JsonValue;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledObjectSchema {
    fields: Vec<CompiledField>,
    capacity_hint: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledRowSchema {
    object: CompiledObjectSchema,
    row_capacity_hint: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonTape {
    pub tokens: Vec<TapeToken>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TapeToken {
    pub kind: TapeTokenKind,
    pub start: usize,
    pub end: usize,
    pub parent: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TapeTokenKind {
    Null,
    Bool,
    Number,
    String,
    Key,
    Array,
    Object,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TapeValue<'a> {
    tape: &'a JsonTape,
    input: &'a str,
    index: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TapeObjectIndex {
    buckets: Vec<Vec<(u64, usize, usize)>>,
}

#[derive(Clone, Copy, Debug)]
pub struct IndexedTapeObject<'a> {
    object: TapeValue<'a>,
    index: &'a TapeObjectIndex,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledTapeKey {
    key: String,
    hash: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledTapeKeys {
    keys: Vec<CompiledTapeKey>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CompiledField {
    key: String,
    rendered_prefix: Vec<u8>,
}

impl JsonTape {
    pub fn root<'a>(&'a self, input: &'a str) -> Option<TapeValue<'a>> {
        (!self.tokens.is_empty()).then_some(TapeValue {
            tape: self,
            input,
            index: 0,
        })
    }
}

impl<'a> TapeValue<'a> {
    pub fn kind(&self) -> TapeTokenKind {
        self.tape.tokens[self.index].kind
    }

    pub fn as_str(&self) -> Option<&'a str> {
        let token = &self.tape.tokens[self.index];
        match token.kind {
            TapeTokenKind::String | TapeTokenKind::Key => {
                if self.input.as_bytes()[token.start] == b'"'
                    && self.input.as_bytes()[token.end - 1] == b'"'
                {
                    Some(&self.input[token.start + 1..token.end - 1])
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<TapeValue<'a>> {
        if self.kind() != TapeTokenKind::Object {
            return None;
        }
        self.get_linear(key)
    }

    pub fn build_object_index(&self) -> Option<TapeObjectIndex> {
        if self.kind() != TapeTokenKind::Object {
            return None;
        }
        let parent = self.index;
        let tokens = &self.tape.tokens;
        let mut entries = Vec::new();
        let mut i = self.index + 1;
        while i + 1 < tokens.len() {
            if tokens[i].parent != Some(parent) {
                i += 1;
                continue;
            }
            if tokens[i].kind == TapeTokenKind::Key && tokens[i + 1].parent == Some(parent) {
                let candidate = TapeValue {
                    tape: self.tape,
                    input: self.input,
                    index: i,
                };
                let key = candidate.as_str().unwrap_or("");
                let hash = util::hash_key(key.as_bytes());
                entries.push((hash, i, i + 1));
                i += 2;
            } else {
                i += 1;
            }
        }
        let bucket_count = (entries.len().next_power_of_two().max(1)) * 2;
        let mut buckets = vec![Vec::new(); bucket_count];
        for entry in entries {
            let bucket = (entry.0 as usize) & (bucket_count - 1);
            buckets[bucket].push(entry);
        }
        Some(TapeObjectIndex { buckets })
    }

    pub fn with_index<'b>(&'b self, index: &'b TapeObjectIndex) -> IndexedTapeObject<'b> {
        IndexedTapeObject {
            object: TapeValue {
                tape: self.tape,
                input: self.input,
                index: self.index,
            },
            index,
        }
    }

    fn get_linear(&self, key: &str) -> Option<TapeValue<'a>> {
        let parent = self.index;
        let tokens = &self.tape.tokens;
        let mut i = self.index + 1;
        while i < tokens.len() {
            if tokens[i].parent != Some(parent) {
                i += 1;
                continue;
            }
            if tokens[i].kind != TapeTokenKind::Key {
                i += 1;
                continue;
            }
            let candidate = TapeValue {
                tape: self.tape,
                input: self.input,
                index: i,
            };
            if candidate.as_str() == Some(key) {
                let value_index = i + 1;
                if value_index < tokens.len() && tokens[value_index].parent == Some(parent) {
                    return Some(TapeValue {
                        tape: self.tape,
                        input: self.input,
                        index: value_index,
                    });
                }
                return None;
            }
            i += 1;
        }
        None
    }
}

impl TapeObjectIndex {
    pub fn get<'a>(&self, object: TapeValue<'a>, key: &str) -> Option<TapeValue<'a>> {
        self.get_hashed(object, util::hash_key(key.as_bytes()), key)
    }

    pub fn get_compiled<'a>(
        &self,
        object: TapeValue<'a>,
        key: &CompiledTapeKey,
    ) -> Option<TapeValue<'a>> {
        self.get_hashed(object, key.hash, &key.key)
    }

    fn get_hashed<'a>(&self, object: TapeValue<'a>, hash: u64, key: &str) -> Option<TapeValue<'a>> {
        let bucket = (hash as usize) & (self.buckets.len() - 1);
        for (entry_hash, key_index, value_index) in &self.buckets[bucket] {
            if *entry_hash != hash {
                continue;
            }
            let candidate = TapeValue {
                tape: object.tape,
                input: object.input,
                index: *key_index,
            };
            if candidate.as_str() == Some(key) {
                return Some(TapeValue {
                    tape: object.tape,
                    input: object.input,
                    index: *value_index,
                });
            }
        }
        None
    }
}

impl CompiledTapeKey {
    pub fn new(key: impl Into<String>) -> Self {
        let key = key.into();
        let hash = util::hash_key(key.as_bytes());
        Self { key, hash }
    }

    pub fn as_str(&self) -> &str {
        &self.key
    }
}

impl CompiledTapeKeys {
    pub fn new(keys: &[&str]) -> Self {
        Self {
            keys: keys.iter().map(|key| CompiledTapeKey::new(*key)).collect(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &CompiledTapeKey> {
        self.keys.iter()
    }
}

impl<'a> IndexedTapeObject<'a> {
    pub fn get(&self, key: &str) -> Option<TapeValue<'a>> {
        self.index.get(self.object, key)
    }

    pub fn get_compiled(&self, key: &CompiledTapeKey) -> Option<TapeValue<'a>> {
        self.index.get_compiled(self.object, key)
    }

    pub fn get_many<'b>(
        &'b self,
        keys: &'b [&'b str],
    ) -> impl Iterator<Item = Option<TapeValue<'a>>> + 'b {
        keys.iter().map(|key| self.get(key))
    }

    pub fn get_compiled_many<'b>(
        &'b self,
        keys: &'b CompiledTapeKeys,
    ) -> impl Iterator<Item = Option<TapeValue<'a>>> + 'b {
        keys.iter().map(|key| self.get_compiled(key))
    }
}

impl CompiledObjectSchema {
    pub fn new(keys: &[&str]) -> Self {
        let mut fields = Vec::with_capacity(keys.len());
        let mut capacity_hint = 2;
        for (index, key) in keys.iter().enumerate() {
            let mut rendered_prefix = Vec::with_capacity(key.len() + 4);
            if index > 0 {
                rendered_prefix.push(b',');
            }
            util::write_json_key(&mut rendered_prefix, key);
            capacity_hint += rendered_prefix.len() + 8;
            fields.push(CompiledField {
                key: (*key).to_owned(),
                rendered_prefix,
            });
        }
        Self {
            fields,
            capacity_hint,
        }
    }

    pub fn keys(&self) -> impl ExactSizeIterator<Item = &str> {
        self.fields.iter().map(|field| field.key.as_str())
    }

    pub fn to_json_string<'a, I>(&self, values: I) -> Result<String, JsonError>
    where
        I: IntoIterator<Item = &'a JsonValue>,
    {
        let mut out = Vec::with_capacity(self.capacity_hint);
        self.write_json_bytes(&mut out, values)?;
        Ok(unsafe { String::from_utf8_unchecked(out) })
    }

    pub fn write_json_bytes<'a, I>(&self, out: &mut Vec<u8>, values: I) -> Result<(), JsonError>
    where
        I: IntoIterator<Item = &'a JsonValue>,
    {
        out.push(b'{');
        let mut iter = values.into_iter();
        for field in &self.fields {
            let Some(value) = iter.next() else {
                panic!(
                    "compiled object schema expected {} values",
                    self.fields.len()
                );
            };
            out.extend_from_slice(&field.rendered_prefix);
            util::write_json_value(out, value)?;
        }
        if iter.next().is_some() {
            panic!(
                "compiled object schema received more than {} values",
                self.fields.len()
            );
        }
        out.push(b'}');
        Ok(())
    }
}

impl CompiledRowSchema {
    pub fn new(keys: &[&str]) -> Self {
        let object = CompiledObjectSchema::new(keys);
        let row_capacity_hint = object.capacity_hint;
        Self {
            object,
            row_capacity_hint,
        }
    }

    pub fn object_schema(&self) -> &CompiledObjectSchema {
        &self.object
    }

    pub fn to_json_string<'a, R, I>(&self, rows: R) -> Result<String, JsonError>
    where
        R: IntoIterator<Item = I>,
        I: IntoIterator<Item = &'a JsonValue>,
    {
        let iter = rows.into_iter();
        let (lower, _) = iter.size_hint();
        let mut out = Vec::with_capacity(2 + lower.saturating_mul(self.row_capacity_hint + 1));
        self.write_json_bytes_from_iter(&mut out, iter)?;
        Ok(unsafe { String::from_utf8_unchecked(out) })
    }

    pub fn write_json_bytes<'a, R, I>(&self, out: &mut Vec<u8>, rows: R) -> Result<(), JsonError>
    where
        R: IntoIterator<Item = I>,
        I: IntoIterator<Item = &'a JsonValue>,
    {
        self.write_json_bytes_from_iter(out, rows.into_iter())
    }

    pub fn write_row_json_bytes<'a, I>(&self, out: &mut Vec<u8>, values: I) -> Result<(), JsonError>
    where
        I: IntoIterator<Item = &'a JsonValue>,
    {
        self.object.write_json_bytes(out, values)
    }

    fn write_json_bytes_from_iter<'a, R, I>(
        &self,
        out: &mut Vec<u8>,
        mut rows: R,
    ) -> Result<(), JsonError>
    where
        R: Iterator<Item = I>,
        I: IntoIterator<Item = &'a JsonValue>,
    {
        out.push(b'[');
        if let Some(first_row) = rows.next() {
            self.object.write_json_bytes(out, first_row)?;
            for row in rows {
                out.push(b',');
                self.object.write_json_bytes(out, row)?;
            }
        }
        out.push(b']');
        Ok(())
    }
}
