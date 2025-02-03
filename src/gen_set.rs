use std::iter::FromIterator;

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use wasm_bindgen::JsValue;

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct GenId<T> {
    pub id: u32,
    pub gen: u32,
    #[serde(skip)]
    _ph: std::marker::PhantomData<fn(T)>,
}

impl<T> std::clone::Clone for GenId<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            gen: self.gen,
            _ph: self._ph,
        }
    }
}

impl<T> std::marker::Copy for GenId<T> {}

impl<T> std::cmp::PartialEq for GenId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.gen == other.gen
    }
}

impl<T> std::cmp::Eq for GenId<T> {}

impl<T> std::hash::Hash for GenId<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.gen.hash(state);
    }
}

impl<T> GenId<T> {
    pub(crate) fn new(id: u32, gen: u32) -> Self {
        Self {
            id,
            gen,
            _ph: std::marker::PhantomData,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum GenPayload<T> {
    Occupied(T),
    Free(Option<usize>),
}

impl<T> GenPayload<T> {
    pub fn new(v: T) -> Self {
        Self::Occupied(v)
    }

    #[allow(dead_code)]
    pub fn is_free(&self) -> bool {
        matches!(self, Self::Free(_))
    }

    #[allow(dead_code)]
    pub fn is_occupied(&self) -> bool {
        matches!(self, Self::Occupied(_))
    }

    pub fn as_ref(&self) -> Option<&T> {
        match self {
            Self::Occupied(v) => Some(v),
            Self::Free(_) => None,
        }
    }

    pub fn as_mut(&mut self) -> Option<&mut T> {
        match self {
            Self::Occupied(v) => Some(v),
            Self::Free(_) => None,
        }
    }

    pub fn take(&mut self) -> Option<T> {
        match std::mem::replace(self, Self::Free(None)) {
            Self::Occupied(v) => Some(v),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct GenEntry<T> {
    pub gen: u32,
    pub item: GenPayload<T>,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct GenSet<T> {
    v: Vec<GenEntry<T>>,
    pub free_head: Option<usize>,
}

impl<T> GenSet<T> {
    pub fn new() -> Self {
        Self {
            v: vec![],
            free_head: None,
        }
    }

    pub fn add(&mut self, item: T) -> GenId<T> {
        match self.free_head {
            Some(i) => {
                if let GenPayload::Free(prev_i) = self.v[i].item {
                    self.free_head = prev_i;
                    self.v[i].gen += 1;
                    self.v[i].item = GenPayload::new(item);
                    GenId::new(i as u32, self.v[i].gen)
                } else {
                    panic!("GenSet: free_head is not free");
                }
            }
            _ => {
                let ret = self.v.len();
                self.v.push(GenEntry {
                    gen: 0,
                    item: GenPayload::new(item),
                });
                GenId::new(ret as u32, 0)
            }
        }
    }

    pub fn remove(&mut self, id: GenId<T>) -> Option<T> {
        if let Some(i) = self.v.get_mut(id.id as usize) {
            if i.gen == id.gen {
                let ret = i.item.take();

                self.free_head = Some(id.id as usize);
                return ret;
            }
        }
        None
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.v.iter().filter_map(|entry| entry.item.as_ref())
    }

    #[allow(dead_code)]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.v.iter_mut().filter_map(|entry| entry.item.as_mut())
    }

    pub fn items(&self) -> impl Iterator<Item = (GenId<T>, &T)> {
        self.v.iter().enumerate().filter_map(|(i, entry)| {
            if let Some(item) = entry.item.as_ref() {
                Some((GenId::new(i as u32, entry.gen), item))
            } else {
                None
            }
        })
    }

    pub fn len(&self) -> usize {
        self.v.len()
    }

    pub fn extend(&mut self, items: impl IntoIterator<Item = T>) {
        for item in items.into_iter() {
            self.add(item);
        }
    }

    pub fn clear(&mut self) {
        self.v.clear();
        self.free_head = None;
    }

    pub fn get(&self, id: GenId<T>) -> Option<&T> {
        self.v.get(id.id as usize).and_then(|entry| {
            if entry.gen == id.gen {
                entry.item.as_ref()
            } else {
                None
            }
        })
    }

    pub fn get_mut(&mut self, id: GenId<T>) -> Option<&mut T> {
        self.v.get_mut(id.id as usize).and_then(|entry| {
            if entry.gen == id.gen {
                entry.item.as_mut()
            } else {
                None
            }
        })
    }

    pub fn retain(&mut self, mut f: impl FnMut(&mut T) -> bool) {
        for (i, entry) in self.v.iter_mut().enumerate() {
            if entry.item.as_mut().is_some_and(|item| !f(item)) {
                entry.item = GenPayload::Free(self.free_head);
                self.free_head = Some(i);
            }
        }
    }

    #[allow(dead_code)]
    pub fn retain_with_id(&mut self, mut f: impl FnMut(GenId<T>, &mut T) -> bool) {
        for (i, entry) in self.v.iter_mut().enumerate() {
            let gen = entry.gen;
            if entry
                .item
                .as_mut()
                .is_some_and(|item| !f(GenId::new(i as u32, gen), item))
            {
                entry.item = GenPayload::Free(self.free_head);
                self.free_head = Some(i);
            }
        }
    }

    #[allow(dead_code)]
    pub fn get_by_index(&self, index: usize) -> Option<&GenEntry<T>> {
        self.v.get(index)
    }

    pub fn get_by_index_mut(&mut self, index: usize) -> Option<&mut GenEntry<T>> {
        self.v.get_mut(index)
    }
}

// TODO: This is not the most scalable implementation of serialization.
// In particular, it exposes the internal structure and the implementation of
// the free list, which should be reset when deserialized, because the free list
// is an empty payload for data whose sole purpose is performance, but serialized
// save data shouldn't care and should be cleaned up on deserialization.
// However, it's not trivial to implement, because the entries in the entity set may have gaps,
// which should be filled with the free list on deserialization.
// Only the elements after the last valid entry should be removed, but the elements in the
// free list in the gaps should also be reconstructed, otherwise the list may be broken if we remove
// those at the end. We could also ignore the gaps and assign None to the free list, but it would
// waste memory by leaking the entries, and the amount of wasted memory will increase every time
// the save data is loaded.
//
// Note also that we need to serialize `gen` field of `GenEntry`, even though only one generation is valid at
// any given time, because there may be some outdated references that points to an old entry with previous
// generation, and unless we actively invalidate them, we don't know if the reference points to a valid entry.
// Theoretically, if the serializer has the knowledge of which reference is valid, we can actively remove them
// in serialized data, but serde is not designed to have access to such a context.
// We could abuse globals or thread locals to implement such architecture, but it just feels overengineered.
//
// No, actually we can't remove free entries at the end, because the generation of free entries
// need to be kept even after serialize/deserialize. I need to think more about it.
// In the meantime, I serialize everything with one linear.
impl<T> GenSet<T>
where
    T: Serialize,
{
    pub fn serialize_json(&self) -> Result<serde_json::Value, JsValue> {
        serde_json::to_value(&self).map_err(|e| js_str!("Serialize error: {}", e))
    }
}

impl<T> GenSet<T>
where
    T: DeserializeOwned,
{
    pub fn deserialize_json(json: serde_json::Value) -> Result<Self, JsValue> {
        serde_json::from_value(json).map_err(move |e| js_str!("Deserialize error: {}", e))
    }
}

impl<T> FromIterator<T> for GenSet<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut set = GenSet::new();
        for item in iter {
            set.add(item);
        }
        set
    }
}
