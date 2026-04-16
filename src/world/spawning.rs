use bevy::prelude::*;

use crate::agents::animal::AnimalBundle;
use crate::agents::npc::NpcBundle;
use crate::magic::mana::ManaReservoir;
use crate::magic::storage::ManaStorageStyle;
use crate::world::map::{MapSettings, RegionTile};
use crate::world::resources::{Tree, TreeStage};

pub struct WorldSpawningPlugin;

impl Plugin for WorldSpawningPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, seed_world_entities);
    }
}

fn seed_world_entities(
    mut commands: Commands,
    settings: Res<MapSettings>,
    tiles: Query<(&RegionTile, &Transform)>,
) {
    let mut tree_index = 0usize;
    let mut animal_index = 0usize;
    let mut npc_index = 0usize;

    for (tile, transform) in &tiles {
        if (tile.coord.x + tile.coord.y) % 3 == 0 {
            let offset = seeded_offset(tree_index as i32, settings.tile_size * 0.2);
            commands.spawn((
                Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.0), Vec2::splat(1.0)),
                Transform::from_xyz(
                    transform.translation.x + offset.x,
                    transform.translation.y + offset.y,
                    2.0,
                ),
                Tree {
                    root_coord: tile.coord,
                    stage: TreeStage::Sapling,
                    growth: 0.15 + tile.soil_fertility * 0.4,
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

        if (tile.coord.x * 2 + tile.coord.y) % 11 == 0 {
            let offset = seeded_offset(animal_index as i32 + 17, settings.tile_size * 0.28);
            commands.spawn(AnimalBundle::new(
                transform.translation.truncate() + offset,
                28.0 + tile.soil_fertility * 10.0,
                0.8 + tile.mana_density * 0.3,
            ));
            animal_index += 1;
        }

        if tile.coord.y == settings.height / 2 && tile.coord.x % 8 == 0 {
            let offset = seeded_offset(npc_index as i32 + 41, settings.tile_size * 0.12);
            commands.spawn(NpcBundle::new(
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
            ));
            npc_index += 1;
        }
    }
}

fn seeded_offset(seed: i32, radius: f32) -> Vec2 {
    let angle = seed as f32 * 1.618_034;
    Vec2::new(angle.cos(), angle.sin()) * radius
}
