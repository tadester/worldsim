use bevy::prelude::*;

use crate::agents::animal::{Animal, AnimalLifeStage, Pregnancy};
use crate::agents::npc::Npc;
use crate::systems::simulation::SimulationClock;
use crate::world::map::{RegionState, RegionTile};
use crate::world::resources::{Tree, TreeStage};

#[derive(Component, Debug, Clone, Copy)]
pub struct Lifecycle {
    pub age_days: f32,
    pub maturity_age: f32,
    pub max_age: f32,
    pub fertility: f32,
    pub reproduction_cooldown: f32,
}

pub struct GrowthPlugin;

impl Plugin for GrowthPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (advance_lifecycle, grow_trees));
    }
}

fn advance_lifecycle(
    clock: Res<SimulationClock>,
    mut animals: Query<(&mut Animal, &mut Lifecycle, Option<&Pregnancy>)>,
    mut npcs: Query<(&mut Npc, &mut Lifecycle), Without<Animal>>,
) {
    let delta_days = clock.delta_days();

    for (mut animal, mut lifecycle, pregnancy) in &mut animals {
        lifecycle.age_days += delta_days;
        lifecycle.reproduction_cooldown = (lifecycle.reproduction_cooldown - delta_days).max(0.0);
        animal.health = (animal.health - delta_days * 0.0015).max(0.0);
        animal.energy = (animal.energy - delta_days * 0.06).max(0.0);
        animal.life_stage = if lifecycle.age_days < lifecycle.maturity_age {
            AnimalLifeStage::Juvenile
        } else if lifecycle.age_days > lifecycle.max_age * 0.75 {
            AnimalLifeStage::Elder
        } else {
            AnimalLifeStage::Adult
        };

        if pregnancy.is_some() {
            animal.energy = (animal.energy - delta_days * 0.10).max(0.0);
        }
    }

    for (mut npc, mut lifecycle) in &mut npcs {
        lifecycle.age_days += delta_days;
        lifecycle.reproduction_cooldown = (lifecycle.reproduction_cooldown - delta_days).max(0.0);
        npc.health = (npc.health - delta_days * 0.0005).max(0.0);
    }
}

fn grow_trees(
    clock: Res<SimulationClock>,
    mut trees: Query<&mut Tree>,
    mut regions: Query<(&RegionTile, &mut RegionState)>,
) {
    let delta_days = clock.delta_days();

    for mut tree in &mut trees {
        let mut local_growth_factor = 0.1;

        for (tile, mut state) in &mut regions {
            if tile.coord != tree.root_coord {
                continue;
            }

            let consumed = (delta_days * 0.0012).min(state.tree_biomass);
            state.tree_biomass -= consumed;
            local_growth_factor =
                (consumed / (delta_days * 0.0012).max(f32::EPSILON)).clamp(0.1, 1.5);
            break;
        }

        tree.growth += delta_days * 0.0009 * local_growth_factor;

        if tree.growth >= 0.9 {
            tree.stage = TreeStage::Mature;
        } else if tree.growth >= 0.45 {
            tree.stage = TreeStage::Young;
        } else {
            tree.stage = TreeStage::Sapling;
        }
    }
}
