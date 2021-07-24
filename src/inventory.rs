use super::ItemType;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::TryFrom};
use wasm_bindgen::prelude::*;

pub(crate) type Inventory = HashMap<ItemType, usize>;

pub(crate) trait InventoryTrait {
    fn remove_item(&mut self, item: &ItemType) -> bool {
        self.remove_items(item, 1)
    }
    fn remove_items(&mut self, item: &ItemType, count: usize) -> bool;
    fn add_item(&mut self, item: &ItemType) {
        self.add_items(item, 1);
    }
    fn add_items(&mut self, item: &ItemType, count: usize);
    fn count_item(&self, item: &ItemType) -> usize;
    fn merge(&mut self, other: Inventory);
    fn describe(&self) -> String;
}

impl InventoryTrait for Inventory {
    fn remove_items(&mut self, item: &ItemType, count: usize) -> bool {
        if let Some(entry) = self.get_mut(item) {
            if *entry <= count {
                self.remove(item);
            } else {
                *entry -= count;
            }
            true
        } else {
            false
        }
    }

    fn add_items(&mut self, item: &ItemType, count: usize) {
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
}

#[derive(PartialEq, Debug, Serialize, Deserialize)]
pub(crate) enum InventoryType {
    Input,
    Output,
    Burner,
}

impl TryFrom<JsValue> for InventoryType {
    type Error = JsValue;
    fn try_from(value: JsValue) -> Result<Self, JsValue> {
        value.into_serde().map_err(|e| js_str!("{}", e.to_string()))
    }
}
