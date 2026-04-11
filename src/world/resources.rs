use bevy::prelude::*;

use crate::agents::animal::Animal;
use crate::magic::mana::ManaReservoir;
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

#[derive(Component)]
struct TreeTrunk;

#[derive(Component)]
struct TreeCanopy;

#[derive(Component)]
struct TreeCanopyAccent;

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
        app.init_resource::<WorldStats>().add_systems(
            Update,
            (
                attach_tree_visuals,
                sync_tree_visuals,
                regrow_region_resources,
                update_world_stats,
            ),
        );
    }
}

fn attach_tree_visuals(mut commands: Commands, trees: Query<Entity, Added<Tree>>) {
    for entity in &trees {
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                Sprite::from_color(Color::srgb(0.40, 0.24, 0.12), Vec2::new(5.0, 14.0)),
                Transform::from_xyz(0.0, -7.0, 0.1),
                TreeTrunk,
            ));
            parent.spawn((
                Sprite::from_color(Color::srgb(0.17, 0.56, 0.22), Vec2::new(20.0, 16.0)),
                Transform::from_xyz(0.0, 2.0, 0.2),
                TreeCanopy,
            ));
            parent.spawn((
                Sprite::from_color(Color::srgb(0.26, 0.68, 0.30), Vec2::new(12.0, 10.0)),
                Transform::from_xyz(5.0, 7.0, 0.3),
                TreeCanopyAccent,
            ));
        });
    }
}

fn sync_tree_visuals(
    trees: Query<(&Tree, &ManaReservoir, &Children), Changed<Tree>>,
    mut trunks: Query<(&mut Sprite, &mut Transform), With<TreeTrunk>>,
    mut canopies: Query<(&mut Sprite, &mut Transform), (With<TreeCanopy>, Without<TreeTrunk>)>,
    mut accents: Query<
        (&mut Sprite, &mut Transform),
        (
            With<TreeCanopyAccent>,
            Without<TreeTrunk>,
            Without<TreeCanopy>,
        ),
    >,
) {
    for (tree, mana, children) in &trees {
        let (trunk_size, canopy_size, accent_size, canopy_color, accent_color) = match tree.stage {
            TreeStage::Sapling => (
                Vec2::new(3.0, 8.0),
                Vec2::new(10.0, 8.0),
                Vec2::new(6.0, 5.0),
                Color::srgb(0.29, 0.63, 0.24),
                Color::srgb(0.38, 0.76, 0.32),
            ),
            TreeStage::Young => (
                Vec2::new(4.0, 11.0),
                Vec2::new(16.0, 12.0),
                Vec2::new(10.0, 8.0),
                Color::srgb(0.22, 0.60, 0.24),
                Color::srgb(0.32, 0.73, 0.31),
            ),
            TreeStage::Mature => (
                Vec2::new(6.0, 16.0),
                Vec2::new(24.0, 18.0),
                Vec2::new(14.0, 10.0),
                Color::srgb(0.17, 0.52, 0.21),
                Color::srgb(0.28, 0.67, 0.27),
            ),
        };
        let mana_tint = (mana.stored / mana.capacity.max(1.0)).clamp(0.0, 1.0);

        for child in children.iter() {
            if let Ok((mut sprite, mut transform)) = trunks.get_mut(child) {
                sprite.custom_size = Some(trunk_size);
                transform.translation.y = -trunk_size.y * 0.45;
            }

            if let Ok((mut sprite, mut transform)) = canopies.get_mut(child) {
                sprite.custom_size = Some(canopy_size);
                sprite.color = Color::srgb(
                    canopy_color.to_srgba().red + mana_tint * 0.04,
                    canopy_color.to_srgba().green + mana_tint * 0.05,
                    canopy_color.to_srgba().blue + mana_tint * 0.08,
                );
                transform.translation.y = canopy_size.y * 0.08;
            }

            if let Ok((mut sprite, mut transform)) = accents.get_mut(child) {
                sprite.custom_size = Some(accent_size);
                sprite.color = Color::srgb(
                    accent_color.to_srgba().red + mana_tint * 0.05,
                    accent_color.to_srgba().green + mana_tint * 0.06,
                    accent_color.to_srgba().blue + mana_tint * 0.10,
                );
                transform.translation.x = canopy_size.x * 0.22;
                transform.translation.y = canopy_size.y * 0.28;
            }
        }
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
