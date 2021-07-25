use super::{items::ItemType, Position, TILE_SIZE_I};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
#[allow(dead_code)]
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

pub(crate) type DropItemIndex = HashMap<(i32, i32), Vec<GenId>>;

pub(crate) const INDEX_CHUNK_SIZE: usize = 16;
const INDEX_GRID_SIZE: usize = INDEX_CHUNK_SIZE * TILE_SIZE_I as usize;
const INDEX_GRID_SIZE_I: i32 = INDEX_GRID_SIZE as i32;

pub(crate) fn build_index(items: &[DropItemEntry]) -> DropItemIndex {
    let mut ret = DropItemIndex::new();
    for (id, item) in items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| Some((GenId::new(i as u32, item.gen), item.item.as_ref()?)))
    {
        ret.entry((
            item.x.div_euclid(INDEX_GRID_SIZE_I),
            item.y.div_euclid(INDEX_GRID_SIZE_I),
        ))
        .or_default()
        .push(id);
    }
    ret
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

pub(crate) fn add_index(index: &mut DropItemIndex, id: GenId, x: i32, y: i32) {
    let new_chunk = (
        x.div_euclid(INDEX_GRID_SIZE_I),
        y.div_euclid(INDEX_GRID_SIZE_I),
    );
    index.entry(new_chunk).or_default().push(id);
}

pub(crate) fn update_index(
    index: &mut DropItemIndex,
    id: GenId,
    old_x: i32,
    old_y: i32,
    x: i32,
    y: i32,
) {
    let old_chunk = (
        old_x.div_euclid(INDEX_GRID_SIZE_I),
        old_y.div_euclid(INDEX_GRID_SIZE_I),
    );
    let new_chunk = (
        x.div_euclid(INDEX_GRID_SIZE_I),
        y.div_euclid(INDEX_GRID_SIZE_I),
    );
    if old_chunk == new_chunk {
        return;
    }
    remove_index(index, id, old_x, old_y);
    index.entry(new_chunk).or_default().push(id);
}

pub(crate) fn remove_index(index: &mut DropItemIndex, id: GenId, old_x: i32, old_y: i32) {
    let old_chunk = (
        old_x.div_euclid(INDEX_GRID_SIZE_I),
        old_y.div_euclid(INDEX_GRID_SIZE_I),
    );
    if let Some(chunk) = index.get_mut(&old_chunk) {
        if let Some((remove_idx, _)) = chunk.iter().enumerate().find(|(_, item)| **item == id) {
            chunk.swap_remove(remove_idx);
        }
    }
}

fn intersecting_chunks(x: i32, y: i32) -> [i32; 4] {
    let left = (x - DROP_ITEM_SIZE_I).div_euclid(INDEX_GRID_SIZE_I);
    let top = (y - DROP_ITEM_SIZE_I).div_euclid(INDEX_GRID_SIZE_I);
    let right = (x + DROP_ITEM_SIZE_I).div_euclid(INDEX_GRID_SIZE_I);
    let bottom = (y + DROP_ITEM_SIZE_I).div_euclid(INDEX_GRID_SIZE_I);
    [left, top, right, bottom]
}

/// Check whether given coordinates hits some object
pub(crate) fn hit_check_with_index(
    items: &[DropItemEntry],
    index: &DropItemIndex,
    x: i32,
    y: i32,
    ignore: Option<DropItemId>,
) -> bool {
    let [left, top, right, bottom] = intersecting_chunks(x, y);
    for cy in top..=bottom {
        for cx in left..=right {
            if let Some(start) = index.get(&(cx, cy)) {
                for id in start {
                    if Some(*id) == ignore {
                        continue;
                    }
                    if let Some(item) = items.get(id.id as usize).and_then(|entry| {
                        if entry.gen != id.gen {
                            None
                        } else {
                            entry.item.as_ref()
                        }
                    }) {
                        if (x - item.x).abs() < DROP_ITEM_SIZE_I
                            && (y - item.y).abs() < DROP_ITEM_SIZE_I
                        {
                            return true;
                        }
                    }
                }
            }
        }
    }
    false
}

#[test]
fn test_hit_check() {
    fn tr(x: i32) -> i32 {
        x * TILE_SIZE_I * 4
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
    ]
    .into_iter()
    .map(|(x, y)| DropItemEntry {
        gen: 0,
        item: Some(DropItem {
            type_: ItemType::CoalOre,
            x: tr(x),
            y: tr(y),
        }),
    })
    .collect::<Vec<_>>();

    let index = build_index(&items);

    assert_eq!(index.len(), 4);
    assert_eq!(
        index
            .values()
            .map(|v| v.len())
            .reduce(|acc, v| acc + v)
            .unwrap(),
        items.len()
    );

    assert_eq!(
        hit_check_with_index(&items, &index, tr(3), tr(1), None),
        true
    );
    assert_eq!(
        hit_check_with_index(&items, &index, tr(3), tr(10), None),
        true
    );
    assert_eq!(
        hit_check_with_index(&items, &index, tr(3), tr(5), None),
        false
    );
}

#[test]
fn test_rounding() {
    assert_eq!(
        intersecting_chunks(INDEX_GRID_SIZE_I / 2, INDEX_GRID_SIZE_I / 2),
        [0; 4]
    );
    assert_eq!(intersecting_chunks(0, INDEX_GRID_SIZE_I / 2), [-1, 0, 0, 0]);
    assert_eq!(intersecting_chunks(0, 0), [-1, -1, 0, 0]);
    assert_eq!(intersecting_chunks(-INDEX_GRID_SIZE_I, 0), [-2, -1, -1, 0]);
    assert_eq!(
        intersecting_chunks(INDEX_GRID_SIZE_I, DROP_ITEM_SIZE_I),
        [0, 0, 1, 0]
    );
}
