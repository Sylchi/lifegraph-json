use crate::JsonValue;

pub struct Iter<'a>(pub std::slice::Iter<'a, (String, JsonValue)>);
pub struct IterMut<'a>(pub std::slice::IterMut<'a, (String, JsonValue)>);
pub struct IntoIter(pub std::vec::IntoIter<(String, JsonValue)>);
pub struct Keys<'a>(pub std::slice::Iter<'a, (String, JsonValue)>);
pub struct Values<'a>(pub std::slice::Iter<'a, (String, JsonValue)>);
pub struct ValuesMut<'a>(pub std::slice::IterMut<'a, (String, JsonValue)>);
pub struct IntoValues(pub std::vec::IntoIter<(String, JsonValue)>);

pub enum MapEntry<'a> {
    Occupied(OccupiedEntry<'a>),
    Vacant(VacantEntry<'a>),
}

pub struct OccupiedEntry<'a> {
    pub(crate) map: &'a mut crate::Map,
    pub(crate) index: usize,
}

pub struct VacantEntry<'a> {
    pub(crate) map: &'a mut crate::Map,
    pub(crate) key: String,
}

impl<'a> MapEntry<'a> {
    pub fn key(&self) -> &String {
        match self {
            Self::Occupied(entry) => entry.key(),
            Self::Vacant(entry) => entry.key(),
        }
    }

    pub fn or_insert(self, default: JsonValue) -> &'a mut JsonValue {
        match self {
            Self::Occupied(entry) => entry.into_mut(),
            Self::Vacant(entry) => entry.insert(default),
        }
    }

    pub fn or_insert_with<F>(self, default: F) -> &'a mut JsonValue
    where
        F: FnOnce() -> JsonValue,
    {
        match self {
            Self::Occupied(entry) => entry.into_mut(),
            Self::Vacant(entry) => entry.insert(default()),
        }
    }

    pub fn or_default(self) -> &'a mut JsonValue {
        self.or_insert(JsonValue::default())
    }

    pub fn and_modify<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut JsonValue),
    {
        match self {
            Self::Occupied(mut entry) => {
                f(entry.get_mut());
                Self::Occupied(entry)
            }
            Self::Vacant(entry) => Self::Vacant(entry),
        }
    }
}

impl<'a> OccupiedEntry<'a> {
    pub fn key(&self) -> &String {
        &self.map.0[self.index].0
    }

    pub fn get(&self) -> &JsonValue {
        &self.map.0[self.index].1
    }

    pub fn get_mut(&mut self) -> &mut JsonValue {
        &mut self.map.0[self.index].1
    }

    pub fn into_mut(self) -> &'a mut JsonValue {
        &mut self.map.0[self.index].1
    }

    pub fn insert(&mut self, value: JsonValue) -> JsonValue {
        std::mem::replace(&mut self.map.0[self.index].1, value)
    }

    pub fn remove(self) -> JsonValue {
        self.map.0.remove(self.index).1
    }

    pub fn remove_entry(self) -> (String, JsonValue) {
        self.map.0.remove(self.index)
    }

    pub fn swap_remove(self) -> JsonValue {
        self.map.0.swap_remove(self.index).1
    }

    pub fn shift_remove(self) -> JsonValue {
        self.remove()
    }

    pub fn swap_remove_entry(self) -> (String, JsonValue) {
        self.map.0.swap_remove(self.index)
    }

    pub fn shift_remove_entry(self) -> (String, JsonValue) {
        self.remove_entry()
    }
}

impl<'a> VacantEntry<'a> {
    pub fn key(&self) -> &String {
        &self.key
    }

    pub fn insert(self, value: JsonValue) -> &'a mut JsonValue {
        self.map.0.push((self.key, value));
        let index = self.map.0.len() - 1;
        &mut self.map.0[index].1
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a String, &'a JsonValue);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(key, value)| (key, value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl ExactSizeIterator for Iter<'_> {}

impl<'a> Iterator for IterMut<'a> {
    type Item = (&'a String, &'a mut JsonValue);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(key, value)| (&*key, value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl ExactSizeIterator for IterMut<'_> {}

impl Iterator for IntoIter {
    type Item = (String, JsonValue);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl ExactSizeIterator for IntoIter {}

impl<'a> Iterator for Keys<'a> {
    type Item = &'a String;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(key, _)| key)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl ExactSizeIterator for Keys<'_> {}

impl<'a> Iterator for Values<'a> {
    type Item = &'a JsonValue;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(_, value)| value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl ExactSizeIterator for Values<'_> {}

impl<'a> Iterator for ValuesMut<'a> {
    type Item = &'a mut JsonValue;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(_, value)| value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl ExactSizeIterator for ValuesMut<'_> {}

impl Iterator for IntoValues {
    type Item = JsonValue;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(_, value)| value)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl ExactSizeIterator for IntoValues {}
