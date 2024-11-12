use super::ItemType;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::TryFrom, iter::IntoIterator};
use wasm_bindgen::prelude::*;

pub(crate) const STACK_SIZE: usize = 50;

pub(crate) type Inventory = HashMap<ItemType, ItemEntry>;

#[derive(Default, Clone, Copy, Debug, Serialize, Deserialize)]
pub(crate) struct ItemEntry {
    pub(crate) count: usize,
    pub(crate) spoil_time: f64,
}

impl ItemEntry {
    pub const ONE: Self = Self {
        count: 1,
        spoil_time: 0.,
    };

    pub fn new(count: usize, spoil_time: f64) -> Self {
        Self { count, spoil_time }
    }
}

impl std::fmt::Display for ItemEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.spoil_time != 0. {
            write!(f, "{} (spoil: {})", self.count, self.spoil_time)
        } else {
            write!(f, "{}", self.count)
        }
    }
}

// Because of the orphan rule, we cannot implement From<ItemSet> for Inventory.
pub(crate) fn item_set_to_inventory(
    items: impl IntoIterator<Item = (ItemType, usize)>,
) -> Inventory {
    items
        .into_iter()
        .map(|(ty, count)| (ty, ItemEntry::new(count, 0.)))
        .collect()
}

pub(crate) fn inventory_to_vec(items: &Inventory) -> Vec<(ItemType, ItemEntry)> {
    items
        .iter()
        .map(|(item, entry)| (*item, *entry))
        .collect::<Vec<_>>()
}

pub(crate) fn _inventory_to_counts(items: &Inventory) -> Vec<(ItemType, usize)> {
    items
        .iter()
        .map(|(item, entry)| (*item, entry.count))
        .collect::<Vec<_>>()
}

pub(crate) trait InventoryTrait {
    fn remove_item(&mut self, item: &ItemType) -> bool {
        self.remove_items(item, 1).count != 0
    }
    fn remove_items(&mut self, item: &ItemType, count: usize) -> ItemEntry;
    fn add_item(&mut self, item: &ItemType) {
        self.add_items(
            item,
            ItemEntry {
                count: 1,
                spoil_time: 0.,
            },
        );
    }
    fn add_items(&mut self, item: &ItemType, entry: ItemEntry);
    fn count_item(&self, item: &ItemType) -> usize;
    fn merge(&mut self, other: Inventory);
    fn describe(&self) -> String;

    /// Calculate occupied slots
    fn count_slots(&self) -> usize;
}

impl InventoryTrait for Inventory {
    fn remove_items(&mut self, item: &ItemType, count: usize) -> ItemEntry {
        use std::collections::hash_map::Entry;
        if let Entry::Occupied(mut entry) = self.entry(*item) {
            if entry.get().count <= count {
                entry.remove()
            } else {
                let entry = entry.get_mut();
                entry.count -= count;
                ItemEntry {
                    count: count,
                    spoil_time: entry.spoil_time,
                }
            }
        } else {
            ItemEntry::default()
        }
    }

    fn add_items(&mut self, item: &ItemType, input: ItemEntry) {
        if input.count == 0 {
            return;
        }
        if let Some(entry) = self.get_mut(item) {
            // Weighted sum by count
            entry.spoil_time = (entry.spoil_time * input.count as f64
                + input.spoil_time * entry.count as f64)
                / (entry.count + input.count) as f64;
            entry.count += input.count;
        } else {
            self.insert(*item, input);
        }
    }

    fn count_item(&self, item: &ItemType) -> usize {
        self.get(item).map_or(0, |item| item.count)
    }

    fn merge(&mut self, other: Inventory) {
        for (k, v) in other {
            if let Some(vv) = self.get_mut(&k) {
                vv.spoil_time = (vv.spoil_time * v.count as f64 + v.spoil_time * vv.count as f64)
                    / (vv.count + v.count) as f64;
                vv.count += v.count;
            } else {
                self.insert(k, v);
            }
        }
    }

    fn describe(&self) -> String {
        self.iter()
            .map(|item| format!("{:?}: {}<br>", item.0, item.1.count))
            .fold(String::from(""), |accum, item| accum + &item)
    }

    fn count_slots(&self) -> usize {
        let mut ret = 0;
        for (_, entry) in self {
            let mut amount = entry.count;
            while STACK_SIZE < amount {
                ret += 1;
                amount -= STACK_SIZE;
            }
            ret += 1;
        }
        ret
    }
}

/// Filter given inventory with a function and return a copy of inventory with items that `filter` returned true.
/// Filtered out items will be added to `residual`.
pub(crate) fn filter_inventory(
    inventory: Inventory,
    filter: impl Fn(&ItemType) -> bool,
    residual: &mut Inventory,
) -> Inventory {
    let mut ret = Inventory::new();
    for (item, count) in inventory {
        if filter(&item) {
            ret.insert(item, count);
        } else {
            residual.add_items(&item, count);
        }
    }
    ret
}

#[derive(PartialEq, Eq, Debug, Serialize, Deserialize, Clone, Copy)]
pub(crate) enum InventoryType {
    Input,
    Output,
    Storage,
    Burner,
}

impl TryFrom<JsValue> for InventoryType {
    type Error = JsValue;
    fn try_from(value: JsValue) -> Result<Self, JsValue> {
        value.into_serde().map_err(|e| js_str!("{}", e.to_string()))
    }
}
