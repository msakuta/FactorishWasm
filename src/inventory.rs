use super::ItemType;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::from_value;
use std::{collections::HashMap, convert::TryFrom};
use wasm_bindgen::prelude::*;

pub(crate) const STACK_SIZE: usize = 50;

pub(crate) type Inventory = HashMap<ItemType, usize>;

pub(crate) trait InventoryTrait {
    fn remove_item(&mut self, item: &ItemType) -> bool {
        self.remove_items(item, 1) != 0
    }
    fn remove_items(&mut self, item: &ItemType, count: usize) -> usize;
    fn add_item(&mut self, item: &ItemType) {
        self.add_items(item, 1);
    }
    fn add_items(&mut self, item: &ItemType, count: usize);
    fn count_item(&self, item: &ItemType) -> usize;
    fn merge(&mut self, other: Inventory);
    fn describe(&self) -> String;

    /// Calculate occupied slots
    fn count_slots(&self) -> usize;
}

impl InventoryTrait for Inventory {
    fn remove_items(&mut self, item: &ItemType, count: usize) -> usize {
        use std::collections::hash_map::Entry;
        if let Entry::Occupied(mut entry) = self.entry(*item) {
            if *entry.get() <= count {
                entry.remove()
            } else {
                *entry.get_mut() -= count;
                count
            }
        } else {
            0
        }
    }

    fn add_items(&mut self, item: &ItemType, count: usize) {
        if count == 0 {
            return;
        }
        if let Some(entry) = self.get_mut(item) {
            *entry += count;
        } else {
            self.insert(*item, count);
        }
    }

    fn count_item(&self, item: &ItemType) -> usize {
        *self.get(item).unwrap_or(&0)
    }

    fn merge(&mut self, other: Inventory) {
        for (k, v) in other {
            if let Some(vv) = self.get_mut(&k) {
                *vv += v;
            } else {
                self.insert(k, v);
            }
        }
    }

    fn describe(&self) -> String {
        self.iter()
            .map(|item| format!("{:?}: {}<br>", item.0, item.1))
            .fold(String::from(""), |accum, item| accum + &item)
    }

    fn count_slots(&self) -> usize {
        let mut ret = 0;
        for pair in self {
            let mut amount = *pair.1;
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
        from_value(value).map_err(|e| js_str!("{}", e.to_string()))
    }
}
