use bevy::prelude::*;

use crate::agents::animal::Animal;
use crate::systems::simulation::SimulationClock;
use crate::world::map::{RegionState, RegionTile};

#[derive(Component, Debug, Clone, Copy)]
pub struct Tree {
    pub root_coord: IVec2,
    pub stage: TreeStage,
    pub growth: f32,
    pub spread_progress: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeStage {
    Sapling,
    Young,
    Mature,
}

#[derive(Resource, Default)]
pub struct WorldStats {
    pub trees: usize,
    pub animals: usize,
    pub npcs: usize,
    pub avg_mana_density: f32,
    pub avg_animal_capacity: f32,
    pub avg_tree_capacity: f32,
    pub avg_temperature: f32,
    pub total_forage: f32,
    pub total_tree_biomass: f32,
}

pub struct WorldResourcesPlugin;

impl Plugin for WorldResourcesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldStats>()
            .add_systems(Update, (regrow_region_resources, update_world_stats));
    }
}

fn regrow_region_resources(
    clock: Res<SimulationClock>,
    mut regions: Query<(&RegionTile, &mut RegionState)>,
) {
    let delta_days = clock.delta_days();

    for (tile, mut state) in &mut regions {
        let forage_growth =
            (0.16 + tile.soil_fertility * 0.20 + tile.temperature * 0.04) * delta_days;
        let biomass_growth =
            (0.08 + tile.soil_fertility * 0.12 + tile.mana_density * 0.04) * delta_days;

        state.forage = (state.forage + forage_growth).clamp(0.0, state.forage_capacity);
        state.tree_biomass =
            (state.tree_biomass + biomass_growth).clamp(0.0, state.tree_biomass_capacity);
    }
}

fn update_world_stats(
    mut stats: ResMut<WorldStats>,
    trees: Query<&Tree>,
    animals: Query<&Animal>,
    npcs: Query<&crate::agents::npc::Npc>,
    regions: Query<(&RegionTile, &RegionState)>,
) {
    let (mana_total, animal_capacity_total, tree_capacity_total, temperature_total, tile_count) =
        regions
            .iter()
            .fold((0.0, 0.0, 0.0, 0.0, 0usize), |acc, (tile, _)| {
                (
                    acc.0 + tile.mana_density,
                    acc.1 + tile.animal_capacity,
                    acc.2 + tile.tree_capacity,
                    acc.3 + tile.temperature,
                    acc.4 + 1,
                )
            });
    let (total_forage, total_tree_biomass) = regions.iter().fold((0.0, 0.0), |acc, (_, state)| {
        (acc.0 + state.forage, acc.1 + state.tree_biomass)
    });

    stats.trees = trees.iter().count();
    stats.animals = animals.iter().count();
    stats.npcs = npcs.iter().count();
    let divisor = tile_count.max(1) as f32;
    stats.avg_mana_density = mana_total / divisor;
    stats.avg_animal_capacity = animal_capacity_total / divisor;
    stats.avg_tree_capacity = tree_capacity_total / divisor;
    stats.avg_temperature = temperature_total / divisor;
    stats.total_forage = total_forage;
    stats.total_tree_biomass = total_tree_biomass;
}
