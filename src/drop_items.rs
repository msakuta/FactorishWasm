use super::{items::ItemType, TILE_SIZE, TILE_SIZE_I};
use crate::gen_set::{GenId, GenSet};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub(crate) const DROP_ITEM_SIZE: f64 = 8.;
pub(crate) const DROP_ITEM_SIZE_I: i32 = DROP_ITEM_SIZE as i32;

pub(crate) type DropItemId = GenId<DropItem>;

#[derive(Serialize, Deserialize)]
pub(crate) struct DropItem {
    pub type_: ItemType,
    pub x: f64,
    pub y: f64,
}

impl DropItem {
    pub(crate) fn new(type_: ItemType, c: i32, r: i32) -> Self {
        DropItem {
            type_,
            x: c as f64 * TILE_SIZE + TILE_SIZE / 2.,
            y: r as f64 * TILE_SIZE + TILE_SIZE / 2.,
        }
    }
}

pub(crate) type DropItemIndex = HashMap<(i32, i32), Vec<GenId<DropItem>>>;

pub(crate) const INDEX_CHUNK_SIZE: usize = 16;
const INDEX_GRID_SIZE: usize = INDEX_CHUNK_SIZE * TILE_SIZE_I as usize;
const INDEX_GRID_SIZE_D: f64 = INDEX_GRID_SIZE as f64;

pub(crate) fn build_index(items: &GenSet<DropItem>) -> DropItemIndex {
    let mut ret = DropItemIndex::new();
    for (id, item) in items.items() {
        ret.entry((
            item.x.div_euclid(INDEX_GRID_SIZE_D) as i32,
            item.y.div_euclid(INDEX_GRID_SIZE_D) as i32,
        ))
        .or_default()
        .push(id);
    }
    ret
}

/// Check whether given coordinates hits some DropItem
pub(crate) fn hit_check(
    items: &GenSet<DropItem>,
    x: f64,
    y: f64,
    ignore: Option<DropItemId>,
) -> bool {
    for (id, item) in items.items() {
        if ignore == Some(id) {
            continue;
        }
        if (x - item.x).abs() < DROP_ITEM_SIZE && (y - item.y).abs() < DROP_ITEM_SIZE {
            return true;
        }
    }
    false
}

pub(crate) fn add_index(index: &mut DropItemIndex, id: GenId<DropItem>, x: f64, y: f64) {
    let new_chunk = (
        x.div_euclid(INDEX_GRID_SIZE_D) as i32,
        y.div_euclid(INDEX_GRID_SIZE_D) as i32,
    );
    index.entry(new_chunk).or_default().push(id);
}

pub(crate) fn update_index(
    index: &mut DropItemIndex,
    id: GenId<DropItem>,
    old_x: f64,
    old_y: f64,
    x: f64,
    y: f64,
) {
    let old_chunk = (
        old_x.div_euclid(INDEX_GRID_SIZE_D) as i32,
        old_y.div_euclid(INDEX_GRID_SIZE_D) as i32,
    );
    let new_chunk = (
        x.div_euclid(INDEX_GRID_SIZE_D) as i32,
        y.div_euclid(INDEX_GRID_SIZE_D) as i32,
    );
    if old_chunk == new_chunk {
        return;
    }
    remove_index(index, id, old_x, old_y);
    index.entry(new_chunk).or_default().push(id);
}

pub(crate) fn remove_index(index: &mut DropItemIndex, id: GenId<DropItem>, old_x: f64, old_y: f64) {
    let old_chunk = (
        old_x.div_euclid(INDEX_GRID_SIZE_D) as i32,
        old_y.div_euclid(INDEX_GRID_SIZE_D) as i32,
    );

    if let Some(chunk) = index.get_mut(&old_chunk) {
        if let Some((remove_idx, _)) = chunk.iter().enumerate().find(|(_, item)| **item == id) {
            chunk.swap_remove(remove_idx);
        }
    }
}

fn intersecting_chunks(x: f64, y: f64) -> [i32; 4] {
    let left = (x - DROP_ITEM_SIZE).div_euclid(INDEX_GRID_SIZE_D) as i32;
    let top = (y - DROP_ITEM_SIZE).div_euclid(INDEX_GRID_SIZE_D) as i32;
    let right = (x + DROP_ITEM_SIZE).div_euclid(INDEX_GRID_SIZE_D) as i32;
    let bottom = (y + DROP_ITEM_SIZE).div_euclid(INDEX_GRID_SIZE_D) as i32;
    [left, top, right, bottom]
}

/// Check whether given coordinates hits some object
pub(crate) fn hit_check_with_index(
    items: &GenSet<DropItem>,
    index: &DropItemIndex,
    x: f64,
    y: f64,
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
                    if let Some(item) = items.get(*id) {
                        if (x as f64 - item.x).abs() < DROP_ITEM_SIZE
                            && (y as f64 - item.y).abs() < DROP_ITEM_SIZE
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
    fn tr(x: f64) -> f64 {
        x * TILE_SIZE * 4.
    }

    let items = vec![
        (4., 1.),
        (6., 1.),
        (3., 1.),
        (1., 1.),
        (7., 1.),
        (5., 1.),
        (2., 1.),
        (8., 1.),
        (3., 10.),
    ]
    .into_iter()
    .map(|(x, y)| DropItem {
        type_: ItemType::CoalOre,
        x: tr(x),
        y: tr(y),
    })
    .collect::<GenSet<_>>();

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
        hit_check_with_index(&items, &index, tr(3.), tr(1.), None),
        true
    );
    assert_eq!(
        hit_check_with_index(&items, &index, tr(3.), tr(10.), None),
        true
    );
    assert_eq!(
        hit_check_with_index(&items, &index, tr(3.), tr(5.), None),
        false
    );
}

#[test]
fn test_rounding() {
    assert_eq!(
        intersecting_chunks(INDEX_GRID_SIZE_D / 2., INDEX_GRID_SIZE_D / 2.),
        [0; 4]
    );
    assert_eq!(
        intersecting_chunks(0., INDEX_GRID_SIZE_D / 2.),
        [-1, 0, 0, 0]
    );
    assert_eq!(intersecting_chunks(0., 0.), [-1, -1, 0, 0]);
    assert_eq!(intersecting_chunks(-INDEX_GRID_SIZE_D, 0.), [-2, -1, -1, 0]);
    assert_eq!(
        intersecting_chunks(INDEX_GRID_SIZE_D, DROP_ITEM_SIZE),
        [0, 0, 1, 0]
    );
}
