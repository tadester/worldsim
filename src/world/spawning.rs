use bevy::prelude::*;

use crate::agents::animal::{Animal, AnimalBundle};
use crate::agents::npc::{NpcBundle, NpcGender, NpcSex};
use crate::magic::mana::ManaReservoir;
use crate::magic::storage::ManaStorageStyle;
use crate::systems::simulation::SimulationStep;
use crate::world::map::{MapSettings, RegionTile};
use crate::world::resources::{Tree, TreeStage};

#[derive(Resource, Debug, Clone, Copy)]
pub struct AnimalSpawnPolicy {
    pub replenish_below: usize,
    pub replenish_to: usize,
}

impl Default for AnimalSpawnPolicy {
    fn default() -> Self {
        Self {
            replenish_below: 4,
            replenish_to: 6,
        }
    }
}

pub struct WorldSpawningPlugin;

impl Plugin for WorldSpawningPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AnimalSpawnPolicy>()
            .add_systems(PostStartup, seed_world_entities)
            .add_systems(Update, maintain_animal_population);
    }
}

fn seed_world_entities(
    mut commands: Commands,
    settings: Res<MapSettings>,
    tiles: Query<(&RegionTile, &Transform)>,
) {
    let mut tree_index = 0usize;
    let mut npc_index = 0usize;

    for (tile, transform) in &tiles {
        if (tile.coord.x + tile.coord.y) % 3 == 0 {
            let offset = seeded_offset(tree_index as i32, settings.tile_size * 0.2);
            let growth = match (tile.coord.x + tile.coord.y) % 7 {
                0 => 0.95,
                1 | 2 => 0.62,
                _ => 0.24 + tile.soil_fertility * 0.22,
            };
            let stage = if growth >= 0.9 {
                TreeStage::Mature
            } else if growth >= 0.45 {
                TreeStage::Young
            } else {
                TreeStage::Sapling
            };
            commands.spawn((
                Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.0), Vec2::splat(1.0)),
                Transform::from_xyz(
                    transform.translation.x + offset.x,
                    transform.translation.y + offset.y,
                    2.0,
                ),
                Tree {
                    root_coord: tile.coord,
                    stage,
                    growth,
                    chop_progress: 0.0,
                    spread_progress: 0.0,
                },
                ManaReservoir {
                    capacity: 10.0 + tile.mana_density * 10.0,
                    stored: tile.mana_density * 2.0,
                    stability: 0.85,
                },
            ));
            tree_index += 1;
        }

        if tile.coord.y == settings.height / 2 && tile.coord.x % 4 == 0 {
            let offset = seeded_offset(npc_index as i32 + 41, settings.tile_size * 0.12);
            let age_days = (18.0 + (npc_index % 6) as f32 * 3.5) * 365.0;
            let sex = if npc_index % 2 == 0 {
                NpcSex::Female
            } else {
                NpcSex::Male
            };
            let gender = match npc_index % 5 {
                0 => NpcGender::Nonbinary,
                _ if sex == NpcSex::Female => NpcGender::Woman,
                _ => NpcGender::Man,
            };
            let tool_base = if npc_index % 3 == 0 { 0.65 } else { 0.25 };
            let reproduction_drive = 1.05 + (npc_index % 4) as f32 * 0.12;
            let discovery_drive = 0.85 + ((npc_index + 1) % 5) as f32 * 0.10;
            let aggression_drive = match npc_index % 6 {
                0 => 1.2,
                1 => 0.75,
                _ => 0.25 + (npc_index % 3) as f32 * 0.10,
            };
            let risk_tolerance = 0.55 + (npc_index % 5) as f32 * 0.12;
            commands.spawn(
                NpcBundle::new(
                    transform.translation.truncate() + offset,
                    format!("Settler {}", npc_index + 1),
                    65.0,
                    ManaReservoir {
                        capacity: 24.0 + tile.mana_density * 18.0,
                        stored: 4.0 + tile.mana_density * 6.0,
                        stability: 0.9,
                    },
                    ManaStorageStyle {
                        concentration: 0.25 + tile.mana_density * 0.2,
                        circulation: 0.45,
                        distribution: 0.3,
                    },
                )
                .with_identity(sex, gender)
                .with_tooling(0.55 + tool_base * 0.25, tool_base)
                .with_drives(
                    reproduction_drive,
                    discovery_drive,
                    aggression_drive,
                    risk_tolerance,
                )
                .with_age_days(age_days),
            );
            npc_index += 1;
        }
    }
}

fn maintain_animal_population(
    mut commands: Commands,
    policy: Res<AnimalSpawnPolicy>,
    settings: Res<MapSettings>,
    step: Res<SimulationStep>,
    animals: Query<Entity, With<Animal>>,
    tiles: Query<(&RegionTile, &Transform)>,
) {
    let animal_count = animals.iter().count();
    if animal_count > policy.replenish_below {
        return;
    }

    let spawn_count = policy.replenish_to.saturating_sub(animal_count);
    if spawn_count == 0 {
        return;
    }

    let mut candidates = tiles
        .iter()
        .filter(|(tile, _)| tile.animal_capacity > 4.0 && tile.soil_fertility > 0.35)
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return;
    }

    candidates.sort_by(|(a, _), (b, _)| {
        (b.animal_capacity + b.soil_fertility).total_cmp(&(a.animal_capacity + a.soil_fertility))
    });

    let stride = (candidates.len() / spawn_count.max(1)).max(1);
    for idx in 0..spawn_count {
        let candidate_index = ((step.tick as usize / 7) + idx * stride) % candidates.len();
        let (tile, transform) = candidates[candidate_index];
        let offset = seeded_offset(
            (step.tick as i32) + idx as i32 + 17,
            settings.tile_size * 0.28,
        );
        commands.spawn(
            AnimalBundle::new(
                transform.translation.truncate() + offset,
                28.0 + tile.soil_fertility * 10.0,
                0.8 + tile.mana_density * 0.3,
            )
            .with_age_days(180.0 + idx as f32 * 45.0),
        );
    }
}

fn seeded_offset(seed: i32, radius: f32) -> Vec2 {
    let angle = seed as f32 * 1.618_034;
    Vec2::new(angle.cos(), angle.sin()) * radius
}
