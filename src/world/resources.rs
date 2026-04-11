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

#[derive(Component, Debug, Clone, Copy)]
pub struct Shelter {
    pub integrity: f32,
    pub safety_bonus: f32,
}

#[derive(Component)]
struct TreeTrunk;

#[derive(Component)]
struct TreeCanopy;

#[derive(Component)]
struct TreeCanopyAccent;

#[derive(Component)]
struct ShelterBase;

#[derive(Component)]
struct ShelterRoof;

#[derive(Resource, Default)]
pub struct WorldStats {
    pub trees: usize,
    pub animals: usize,
    pub npcs: usize,
    pub shelters: usize,
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
                attach_shelter_visuals,
                sync_shelter_visuals,
                decay_shelter_integrity,
                regrow_region_resources,
                update_world_stats,
            ),
        );
    }
}

fn attach_shelter_visuals(mut commands: Commands, shelters: Query<Entity, Added<Shelter>>) {
    for entity in &shelters {
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                Sprite::from_color(Color::srgb(0.49, 0.36, 0.22), Vec2::new(18.0, 10.0)),
                Transform::from_xyz(0.0, -3.0, 0.1),
                ShelterBase,
            ));
            parent.spawn((
                Sprite::from_color(Color::srgb(0.30, 0.17, 0.11), Vec2::new(22.0, 8.0)),
                Transform::from_xyz(0.0, 4.0, 0.2).with_rotation(Quat::from_rotation_z(0.08)),
                ShelterRoof,
            ));
        });
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
    trees: Query<
        (&Tree, Option<&ManaReservoir>, &Children),
        Or<(
            Changed<Tree>,
            Added<Tree>,
            Changed<ManaReservoir>,
            Added<ManaReservoir>,
        )>,
    >,
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
        let mana_tint = mana
            .map(|mana| (mana.stored / mana.capacity.max(1.0)).clamp(0.0, 1.0))
            .unwrap_or(0.0);

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

fn sync_shelter_visuals(
    shelters: Query<(&Shelter, &Children), Or<(Changed<Shelter>, Added<Shelter>)>>,
    mut bases: Query<&mut Sprite, With<ShelterBase>>,
    mut roofs: Query<&mut Sprite, (With<ShelterRoof>, Without<ShelterBase>)>,
) {
    for (shelter, children) in &shelters {
        let integrity = shelter.integrity.clamp(0.0, 1.0);
        let wear = 1.0 - integrity;

        let base_color = Color::srgb(0.49 - wear * 0.10, 0.36 - wear * 0.08, 0.22 - wear * 0.06);
        let roof_color = Color::srgb(0.30 - wear * 0.06, 0.17 - wear * 0.05, 0.11 - wear * 0.04);

        for child in children.iter() {
            if let Ok(mut sprite) = bases.get_mut(child) {
                sprite.color = base_color;
            }
            if let Ok(mut sprite) = roofs.get_mut(child) {
                sprite.color = roof_color;
            }
        }
    }
}

fn decay_shelter_integrity(clock: Res<SimulationClock>, mut shelters: Query<&mut Shelter>) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    for mut shelter in &mut shelters {
        shelter.integrity = (shelter.integrity - delta_days * 0.0015).max(0.0);
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
    shelters: Query<&Shelter>,
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
    stats.shelters = shelters.iter().count();
    let divisor = tile_count.max(1) as f32;
    stats.avg_mana_density = mana_total / divisor;
    stats.avg_animal_capacity = animal_capacity_total / divisor;
    stats.avg_tree_capacity = tree_capacity_total / divisor;
    stats.avg_temperature = temperature_total / divisor;
    stats.total_forage = total_forage;
    stats.total_tree_biomass = total_tree_biomass;
}
