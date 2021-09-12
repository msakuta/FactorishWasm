use crate::{inventory::Inventory, items::item_to_str};
use serde::Serialize;
use std::collections::HashMap;

pub(crate) struct Technology {
    pub name: &'static str,
    pub image: &'static str,
    pub input: Inventory,
    pub steps: usize,
    pub research_time: f64,
    pub unlocked: bool,
}

#[derive(Serialize)]
pub(crate) struct TechnologySerial {
    pub name: &'static str,
    pub image: &'static str,
    pub input: HashMap<String, usize>,
    pub steps: usize,
    pub research_time: f64,
    pub unlocked: bool,
}

impl From<&Technology> for TechnologySerial {
    fn from(tech: &Technology) -> Self {
        Self {
            name: tech.name,
            image: tech.image,
            input: tech
                .input
                .iter()
                .map(|(k, v)| (item_to_str(k), *v))
                .collect(),
            steps: tech.steps,
            research_time: tech.research_time,
            unlocked: tech.unlocked,
        }
    }
}

pub(crate) struct Research {
    pub technology_name: &'static str,
    pub progress: usize,
}

#[derive(Serialize)]
pub(crate) struct ResearchSerial {
    pub technology_name: &'static str,
    pub progress: f64,
}
