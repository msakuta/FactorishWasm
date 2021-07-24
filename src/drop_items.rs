use crate::TILE_SIZE;

use super::{dyn_iter::DynIter, items::ItemType, Position, TILE_SIZE_I};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use wasm_bindgen::prelude::*;

pub(crate) const DROP_ITEM_SIZE: f64 = 8.;
pub(crate) const DROP_ITEM_SIZE_I: i32 = DROP_ITEM_SIZE as i32;

pub(crate) type DropItemId = GenId;

#[derive(Serialize, Deserialize)]
pub(crate) struct DropItem {
    pub type_: ItemType,
    pub x: i32,
    pub y: i32,
}

impl DropItem {
    pub(crate) fn new(type_: ItemType, c: i32, r: i32) -> Self {
        let ret = DropItem {
            type_,
            x: c * TILE_SIZE_I + TILE_SIZE_I / 2,
            y: r * TILE_SIZE_I + TILE_SIZE_I / 2,
        };
        ret
    }
}

pub(crate) type DropItemEntry = GenEntry<DropItem>;

impl DropItemEntry {
    pub(crate) fn new(type_: ItemType, position: &Position) -> Self {
        Self {
            gen: 0,
            item: Some(DropItem::new(type_, position.x, position.y)),
        }
    }

    pub(crate) fn from_value(value: DropItem) -> Self {
        Self {
            gen: 0,
            item: Some(value),
        }
    }
}

/// Returns an iterator over valid structures
pub(crate) fn drop_item_id_iter(
    drop_items: &[DropItemEntry],
) -> impl Iterator<Item = (DropItemId, &DropItem)> {
    drop_items.iter().enumerate().filter_map(|(id, item)| {
        Some((
            DropItemId {
                id: id as u32,
                gen: item.gen,
            },
            item.item.as_ref()?,
        ))
    })
}

/// Returns an iterator over valid structures
pub(crate) fn drop_item_id_iter_mut(
    drop_items: &mut [DropItemEntry],
) -> impl Iterator<Item = (DropItemId, &mut DropItem)> {
    drop_items.iter_mut().enumerate().filter_map(|(id, item)| {
        Some((
            DropItemId {
                id: id as u32,
                gen: item.gen,
            },
            item.item.as_mut()?,
        ))
    })
}

/// Returns an iterator over valid structures
pub(crate) fn drop_item_iter(drop_items: &[DropItemEntry]) -> impl Iterator<Item = &DropItem> {
    drop_items
        .iter()
        .filter_map(|item| Some(item.item.as_ref()?))
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Copy, Debug)]
pub(crate) struct GenId {
    pub id: u32,
    pub gen: u32,
}

impl GenId {
    pub(crate) fn new(id: u32, gen: u32) -> Self {
        Self { id, gen }
    }
}

pub(crate) struct GenEntry<T> {
    pub gen: u32,
    pub item: Option<T>,
}

/// A structure that allow random access to structure array excluding single element.
/// It is convenient when you want to have mutable reference to two elements in the array at the same time.
pub(crate) struct SplitSlice<'a, T> {
    left_start: usize,
    left: &'a mut [GenEntry<T>],
    right_start: usize,
    right: &'a mut [GenEntry<T>],
}

impl<'a, T> SplitSlice<'a, T> {
    #[allow(dead_code)]
    pub(crate) fn new_all(source: &'a mut [GenEntry<T>]) -> Self {
        Self {
            left_start: 0,
            right_start: source.len(),
            left: source,
            right: &mut [],
        }
    }

    pub(crate) fn new(
        source: &'a mut [GenEntry<T>],
        split_idx: usize,
    ) -> Result<(&'a mut GenEntry<T>, Self), JsValue> {
        let (left, right) = source.split_at_mut(split_idx);
        let (center, right) = right
            .split_first_mut()
            .ok_or_else(|| JsValue::from_str("Structures split fail"))?;
        Ok((
            center,
            Self {
                left_start: 0,
                left,
                right_start: split_idx + 1,
                right,
            },
        ))
    }

    /// Accessor without generation checking.
    #[allow(dead_code)]
    pub(crate) fn get_at(&self, idx: usize) -> Option<&GenEntry<T>> {
        if self.left_start <= idx && idx < self.left_start + self.left.len() {
            self.left.get(idx - self.left_start)
        } else if self.right_start <= idx && idx < self.right_start + self.right.len() {
            self.right.get(idx - self.right_start)
        } else {
            None
        }
    }

    /// Mutable accessor without generation checking.
    #[allow(dead_code)]
    pub(crate) fn get_at_mut(&mut self, idx: usize) -> Option<&mut GenEntry<T>> {
        if self.left_start <= idx && idx < self.left_start + self.left.len() {
            self.left.get_mut(idx - self.left_start)
        } else if self.right_start <= idx && idx < self.right_start + self.right.len() {
            self.right.get_mut(idx - self.right_start)
        } else {
            None
        }
    }

    /// Accessor with generation checking.
    #[allow(dead_code)]
    pub(crate) fn get(&self, id: GenId) -> Option<&T> {
        let idx = id.id as usize;
        if self.left_start <= idx && idx < self.left_start + self.left.len() {
            self.left
                .get(idx - self.left_start)
                .filter(|s| s.gen == id.gen)
                .map(|s| s.item.as_ref())
                .flatten()
        } else if self.right_start <= idx && idx < self.right_start + self.right.len() {
            self.right
                .get(idx - self.right_start)
                .filter(|s| s.gen == id.gen)
                .map(|s| s.item.as_ref())
                .flatten()
        } else {
            None
        }
    }

    /// Mutable accessor with generation checking.
    #[allow(dead_code)]
    pub(crate) fn get_mut(&mut self, id: GenId) -> Option<&mut T> {
        let idx = id.id as usize;
        if self.left_start <= idx && idx < self.left_start + self.left.len() {
            self.left
                .get_mut(idx - self.left_start)
                .filter(|s| s.gen == id.gen)
                .map(|s| s.item.as_mut())
                // Interestingly, we need .map(|s| s as &mut dyn Structure) to compile.
                // .map(|s| s.item.as_deref_mut())
                .flatten()
        } else if self.right_start <= idx && idx < self.right_start + self.right.len() {
            self.right
                .get_mut(idx - self.right_start)
                .filter(|s| s.gen == id.gen)
                .map(|s| s.item.as_mut())
                // .map(|s| s.item.as_deref_mut())
                .flatten()
        } else {
            None
        }
    }

    pub(crate) fn dyn_iter_id(&self) -> impl Iterator<Item = (GenId, &T)> + '_ {
        self.left
            .iter()
            .enumerate()
            .map(move |(i, val)| (GenId::new((i + self.left_start) as u32, val.gen), val))
            .chain(
                self.right
                    .iter()
                    .enumerate()
                    .map(move |(i, val)| (GenId::new((i + self.right_start) as u32, val.gen), val)),
            )
            .filter_map(|(i, s)| Some((i, s.item.as_ref()?)))
    }
}

impl<'a, T> DynIter for SplitSlice<'a, T> {
    type Item = T;
    fn dyn_iter(&self) -> Box<dyn Iterator<Item = &Self::Item> + '_> {
        Box::new(
            self.left
                .iter()
                .chain(self.right.iter())
                .filter_map(|s| s.item.as_ref()),
        )
    }
    fn as_dyn_iter(&self) -> &dyn DynIter<Item = Self::Item> {
        self
    }
}

pub(crate) fn build_index(items: &[DropItemEntry]) -> Vec<GenId> {
    let mut sorted = items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| Some((GenId::new(i as u32, item.gen), item.item.as_ref()?)))
        .collect::<Vec<_>>();
    sorted.sort_by_key(|(_, item)| item.x);
    sorted.iter().map(|(id, item)| *id).collect()
}

/// Check whether given coordinates hits some object
pub(crate) fn hit_check(
    items: &[DropItemEntry],
    x: i32,
    y: i32,
    ignore: Option<DropItemId>,
) -> bool {
    for (id, entry) in items.iter().enumerate() {
        if let Some(item) = entry.item.as_ref() {
            if let Some(ignore_id) = ignore {
                let id = DropItemId::new(id as u32, entry.gen);
                if ignore_id == id {
                    continue;
                }
            }
            if (x - item.x).abs() < DROP_ITEM_SIZE_I && (y - item.y).abs() < DROP_ITEM_SIZE_I {
                return true;
            }
        }
    }
    false
}

/// Check whether given coordinates hits some object
pub(crate) fn hit_check_with_index(
    items: &[DropItemEntry],
    index: &[DropItemId],
    x: i32,
    y: i32,
    ignore: Option<DropItemId>,
) -> bool {
    let start = index.binary_search_by(|id| {
        if id.gen != items[id.id as usize].gen {
            return Ordering::Less;
        }
        items
            .get(id.id as usize)
            .and_then(|item| item.item.as_ref())
            .map(|item| {
                if item.x < x {
                    Ordering::Less
                } else if x < item.x {
                    Ordering::Greater
                } else {
                    Ordering::Equal
                }
            })
            .unwrap_or(Ordering::Less)
    });

    println!("x: {}, start: {:?}", x, start);
    if let Ok(start) | Err(start) = start {
        for num in 0.. {
            if start < num {
                break;
            }
            let left = index
                .get((start - num) as usize)
                .and_then(|id| Some((id, items.get(id.id as usize)?)))
                .and_then(|(id, entry)| Some((id, entry.item.as_ref()?)));
            println!("  start - num: {:?}, {:?}", start - num, left.is_some());
            let left_lim = if let Some((id, item)) = left {
                println!("  x, y: ({:?} {:?}), ({:?}, {:?})", item.x, item.y, x, y);
                if Some(*id) != ignore && (x - item.x).abs() < DROP_ITEM_SIZE_I && (y - item.y).abs() < DROP_ITEM_SIZE_I {
                    return true;
                } 
                DROP_ITEM_SIZE_I < (x - item.x).abs()
            } else {
                false
            };

            if items.len() < start + num {
                break;
            }
            let right = index
                .get((start + num) as usize)
                .and_then(|id| Some((id, items.get(id.id as usize)?)))
                .and_then(|(id, entry)| Some((id, entry.item.as_ref()?)));
            println!("  start + num: {:?}, {:?}", start + num, right.is_some());
            let right_lim = if let Some((id, item)) = right {
                println!("  x, y: ({:?} {:?}), ({:?}, {:?})", item.x, item.y, x, y);
                if Some(*id) != ignore && (x - item.x).abs() < DROP_ITEM_SIZE_I && (y - item.y).abs() < DROP_ITEM_SIZE_I {
                    return true;
                }
                DROP_ITEM_SIZE_I < (x - item.x).abs()
            } else {
                false
            };

            if left_lim && right_lim {
                break;
            }
        }
    }
    false
}

#[test]
fn test_hit_check() {
    fn tr(x: i32) -> i32 {
        x * 4
    }
 
    let items = vec![
        (4, 1),
        (6, 1),
        (3, 1),
        (1, 1),
        (7, 1),
        (5, 1),
        (2, 1),
        (8, 1),
        (3, 10),
    ].into_iter().map(|(x, y)| {
        DropItemEntry {
            gen: 0,
            item: Some(DropItem {
                type_: ItemType::CoalOre,
                x: tr(x),
                y: tr(y),
            })
        }
    }).collect::<Vec<_>>();

    let index = build_index(&items);

    assert_eq!(
        index
            .iter()
            .map(|i| items.get(i.id as usize).and_then(|entry| entry.item.as_ref()).map(|item| item.x).unwrap())
            .collect::<Vec<_>>(),
        vec![1,2, 3, 3, 4,  5, 6, 7, 8].into_iter().map(tr).collect::<Vec<_>>()
    );

    println!("index: {:?}, {:?}", index,         index
    .iter()
    .map(|i| items.get(i.id as usize).and_then(|entry| entry.item.as_ref()).map(|item| item.x).unwrap())
    .collect::<Vec<_>>());

    assert_eq!(hit_check_with_index(&items, &index, tr(3), tr(1), None), true);
    assert_eq!(hit_check_with_index(&items, &index, tr(3), tr(10), None), true);
    assert_eq!(hit_check_with_index(&items, &index, tr(3), tr(5), None), true);
}
