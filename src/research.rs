use crate::{
    assembler::Assembler,
    furnace::RECIPES,
    items::{item_to_str, ItemType},
    FactorishState, ItemSet,
};
use once_cell::unsync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Hash, Clone, Copy)]
pub(crate) enum TechnologyTag {
    Transportation,
    Electricity,
    SteelWorks,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Technology {
    pub tag: TechnologyTag,
    pub input: ItemSet,
    pub steps: usize,
    pub research_time: f64,
}

#[derive(Serialize)]
pub(crate) struct TechnologySerial {
    pub tag: TechnologyTag,
    pub image: &'static str,
    pub input: HashMap<String, usize>,
    pub steps: usize,
    pub research_time: f64,
    pub unlocked: bool,
    pub unlocks: Vec<String>,
}

impl TechnologySerial {
    pub(crate) fn from(tech: &Technology, state: &FactorishState) -> Self {
        Self {
            tag: tech.tag,
            image: match tech.tag {
                TechnologyTag::Transportation => "Transport Belt",
                TechnologyTag::Electricity => "Electric Pole",
                TechnologyTag::SteelWorks => "Steel Plate",
            },
            input: tech
                .input
                .iter()
                .map(|(k, v)| (item_to_str(k), *v))
                .collect(),
            steps: tech.steps,
            research_time: tech.research_time,
            unlocked: state.unlocked_technologies.contains(&tech.tag),
            unlocks: Assembler::get_recipes()
                .iter()
                .chain(RECIPES.iter())
                .filter(|recipe| recipe.requires_technology.contains(&tech.tag))
                .filter_map(|recipe| Some(item_to_str(recipe.output.keys().next()?)))
                .collect(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Research {
    pub technology: TechnologyTag,
    pub progress: usize,
}

#[derive(Serialize)]
pub(crate) struct ResearchSerial {
    pub technology: TechnologyTag,
    pub progress: f64,
}

pub(crate) const TECHNOLOGIES: Lazy<Vec<Technology>> = Lazy::new(|| {
    vec![
        Technology {
            tag: TechnologyTag::Transportation,
            input: hash_map!(ItemType::SciencePack1 => 1),
            steps: 20,
            research_time: 30.,
        },
        Technology {
            tag: TechnologyTag::Electricity,
            input: hash_map!(ItemType::SciencePack1 => 1),
            steps: 30,
            research_time: 30.,
        },
        Technology {
            tag: TechnologyTag::SteelWorks,
            input: hash_map!(ItemType::SciencePack1 => 1, ItemType::SciencePack2 => 1),
            steps: 50,
            research_time: 30.,
        },
    ]
});
