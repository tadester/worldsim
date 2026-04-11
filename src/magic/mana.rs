use bevy::prelude::*;

use crate::systems::simulation::SimulationClock;
use crate::world::map::RegionTile;

#[derive(Component, Debug, Clone, Copy)]
pub struct ManaReservoir {
    pub capacity: f32,
    pub stored: f32,
    pub stability: f32,
}

pub struct ManaPlugin;

impl Plugin for ManaPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, ambient_mana_drift);
    }
}

fn ambient_mana_drift(
    clock: Res<SimulationClock>,
    tiles: Query<&RegionTile>,
    mut reservoirs: Query<&mut ManaReservoir>,
) {
    let avg_tile_mana =
        tiles.iter().map(|tile| tile.mana_density).sum::<f32>() / tiles.iter().len().max(1) as f32;
    let delta_days = clock.delta_days();

    for mut reservoir in &mut reservoirs {
        let pull = (avg_tile_mana * reservoir.capacity * 0.02) * delta_days;
        reservoir.stored = (reservoir.stored + pull).clamp(0.0, reservoir.capacity);
        reservoir.stability = (reservoir.stability - delta_days * 0.001).max(0.4);
    }
}
