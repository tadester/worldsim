use bevy::prelude::*;

use crate::agents::animal::Animal;
use crate::agents::inventory::Inventory;
use crate::agents::predator::Predator;
use crate::magic::mana::ManaReservoir;
use crate::systems::simulation::SimulationClock;
use crate::world::climate::RegionClimate;
use crate::world::director::WorldMind;
use crate::world::map::{MapSettings, RegionState, RegionTile};

#[derive(Component, Debug, Clone, Copy)]
pub struct Tree {
    pub root_coord: IVec2,
    pub stage: TreeStage,
    pub growth: f32,
    pub chop_progress: f32,
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
    pub insulation: f32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Campfire {
    pub fuel: f32,
    pub max_fuel: f32,
    pub heat: f32,
    pub ember: f32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct ShelterStockpile {
    pub food: f32,
    pub wood: f32,
    pub seeds: f32,
    pub fiber: f32,
    pub hides: f32,
    pub ore: f32,
    pub metal: f32,
    pub clothing: f32,
    pub weapons: f32,
    pub max_food: f32,
    pub max_wood: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CivicStructureKind {
    Fence,
    Workshop,
    Nursery,
    WatchPost,
    Granary,
    Forge,
    TownHall,
}

impl CivicStructureKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Fence => "Fence",
            Self::Workshop => "Workshop",
            Self::Nursery => "Nursery",
            Self::WatchPost => "Watch Post",
            Self::Granary => "Granary",
            Self::Forge => "Forge",
            Self::TownHall => "Town Hall",
        }
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct CivicStructure {
    pub kind: CivicStructureKind,
    pub progress: f32,
}

impl Default for ShelterStockpile {
    fn default() -> Self {
        Self {
            food: 0.0,
            wood: 0.0,
            seeds: 0.0,
            fiber: 0.0,
            hides: 0.0,
            ore: 0.0,
            metal: 0.0,
            clothing: 0.0,
            weapons: 0.0,
            max_food: 12.0,
            max_wood: 12.0,
        }
    }
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

#[derive(Component)]
struct CampfireCore;

#[derive(Component)]
struct CampfireGlow;

#[derive(Component)]
struct CivicStructureBody;

#[derive(Component)]
struct CivicStructureAccent;

#[derive(Resource, Default)]
pub struct WorldStats {
    pub trees: usize,
    pub animals: usize,
    pub predators: usize,
    pub npcs: usize,
    pub shelters: usize,
    pub campfires: usize,
    pub civic_structures: usize,
    pub avg_mana_density: f32,
    pub avg_animal_capacity: f32,
    pub avg_tree_capacity: f32,
    pub avg_temperature: f32,
    pub avg_climate_pressure: f32,
    pub animal_load_ratio: f32,
    pub total_forage: f32,
    pub total_tree_biomass: f32,
    pub total_food_carried: f32,
    pub total_wood_carried: f32,
    pub total_food_stockpiled: f32,
    pub total_wood_stockpiled: f32,
    pub total_ore: f32,
    pub total_metal: f32,
    pub total_clothing: f32,
    pub total_weapons: f32,
    pub avg_npc_exposure: f32,
    pub cold_stressed_npcs: usize,
}

pub struct WorldResourcesPlugin;

impl Plugin for WorldResourcesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldStats>().add_systems(
            Update,
            (
                attach_shelter_stockpiles,
                attach_tree_visuals,
                sync_tree_visuals,
                attach_shelter_visuals,
                sync_shelter_visuals,
                attach_campfire_visuals,
                sync_campfire_visuals,
                attach_civic_structure_visuals,
                sync_civic_structure_visuals,
                burn_campfires,
                decay_shelter_integrity,
                regrow_region_resources,
                update_world_stats,
            ),
        );
    }
}

fn attach_shelter_stockpiles(
    mut commands: Commands,
    shelters: Query<Entity, (Added<Shelter>, Without<ShelterStockpile>)>,
) {
    for entity in &shelters {
        commands.entity(entity).insert(ShelterStockpile::default());
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

fn attach_campfire_visuals(mut commands: Commands, campfires: Query<Entity, Added<Campfire>>) {
    for entity in &campfires {
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                Sprite::from_color(Color::srgb(0.38, 0.20, 0.08), Vec2::new(10.0, 4.0)),
                Transform::from_xyz(0.0, -4.0, 0.1),
                CampfireCore,
            ));
            parent.spawn((
                Sprite::from_color(Color::srgba(0.96, 0.58, 0.14, 0.65), Vec2::new(12.0, 14.0)),
                Transform::from_xyz(0.0, 3.0, 0.2),
                CampfireGlow,
            ));
        });
    }
}

fn attach_civic_structure_visuals(
    mut commands: Commands,
    structures: Query<(Entity, &CivicStructure), Added<CivicStructure>>,
) {
    for (entity, structure) in &structures {
        let (body_size, accent_size) = civic_structure_sizes(structure.kind);
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                Sprite::from_color(civic_structure_color(structure.kind), body_size),
                Transform::from_xyz(0.0, 0.0, 0.1),
                CivicStructureBody,
            ));
            parent.spawn((
                Sprite::from_color(civic_structure_accent_color(structure.kind), accent_size),
                Transform::from_xyz(0.0, body_size.y * 0.35, 0.2),
                CivicStructureAccent,
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

fn sync_campfire_visuals(
    campfires: Query<(&Campfire, &Children), Or<(Changed<Campfire>, Added<Campfire>)>>,
    mut cores: Query<&mut Sprite, With<CampfireCore>>,
    mut glows: Query<&mut Sprite, (With<CampfireGlow>, Without<CampfireCore>)>,
) {
    for (campfire, children) in &campfires {
        let fuel_ratio = (campfire.fuel / campfire.max_fuel.max(0.1)).clamp(0.0, 1.0);
        let ember = campfire.ember.clamp(0.0, 1.0);
        for child in children.iter() {
            if let Ok(mut sprite) = cores.get_mut(child) {
                sprite.color = Color::srgb(0.28 + ember * 0.20, 0.14 + ember * 0.10, 0.06);
            }
            if let Ok(mut sprite) = glows.get_mut(child) {
                sprite.color = Color::srgba(
                    0.85 + ember * 0.12,
                    0.30 + ember * 0.35,
                    0.10 + fuel_ratio * 0.08,
                    0.10 + ember * 0.60,
                );
                sprite.custom_size = Some(Vec2::new(8.0 + ember * 10.0, 8.0 + ember * 14.0));
            }
        }
    }
}

fn sync_civic_structure_visuals(
    structures: Query<
        (&CivicStructure, &Children),
        Or<(Changed<CivicStructure>, Added<CivicStructure>)>,
    >,
    mut bodies: Query<&mut Sprite, With<CivicStructureBody>>,
    mut accents: Query<&mut Sprite, (With<CivicStructureAccent>, Without<CivicStructureBody>)>,
) {
    for (structure, children) in &structures {
        let completion = structure.progress.clamp(0.0, 1.0);
        for child in children.iter() {
            if let Ok(mut sprite) = bodies.get_mut(child) {
                sprite.color =
                    civic_structure_color(structure.kind).with_alpha(0.35 + completion * 0.65);
            }
            if let Ok(mut sprite) = accents.get_mut(child) {
                sprite.color = civic_structure_accent_color(structure.kind)
                    .with_alpha(0.25 + completion * 0.75);
            }
        }
    }
}

fn burn_campfires(clock: Res<SimulationClock>, mut campfires: Query<&mut Campfire>) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    for mut campfire in &mut campfires {
        campfire.fuel = (campfire.fuel - delta_days * 0.09).max(0.0);
        let fuel_ratio = (campfire.fuel / campfire.max_fuel.max(0.1)).clamp(0.0, 1.0);
        campfire.ember = if campfire.fuel > 0.0 {
            (campfire.ember + delta_days * 0.35).clamp(0.45, 1.0) * fuel_ratio.max(0.45)
        } else {
            (campfire.ember - delta_days * 0.25).max(0.0)
        };
        campfire.heat = 0.22 + campfire.ember * 0.72;
    }
}

fn decay_shelter_integrity(
    clock: Res<SimulationClock>,
    settings: Res<MapSettings>,
    regions: Query<(&RegionTile, &RegionClimate)>,
    mut shelters: Query<(&Transform, &mut Shelter)>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    let pressure_by_coord: std::collections::HashMap<IVec2, f32> = regions
        .iter()
        .map(|(tile, climate)| (tile.coord, climate.pressure))
        .collect();

    for (transform, mut shelter) in &mut shelters {
        let coord = settings.tile_coord_for_position(transform.translation.truncate());
        let pressure = pressure_by_coord.get(&coord).copied().unwrap_or(0.0);
        let rate = 0.0015 * (1.0 + pressure.clamp(0.0, 1.0) * 1.8);
        shelter.integrity = (shelter.integrity - delta_days * rate).max(0.0);
    }
}

fn regrow_region_resources(
    clock: Res<SimulationClock>,
    world_mind: Option<Res<WorldMind>>,
    mut regions: Query<(&RegionTile, &RegionClimate, &mut RegionState)>,
) {
    let delta_days = clock.delta_days();
    let resource_bias = world_mind
        .as_ref()
        .map(|mind| mind.resource_bias)
        .unwrap_or(1.0);

    for (tile, climate, mut state) in &mut regions {
        let suitability = (1.0 - climate.pressure * 0.75).clamp(0.15, 1.0);
        let forage_growth = (0.16 + tile.soil_fertility * 0.20 + tile.temperature * 0.04)
            * suitability
            * resource_bias
            * delta_days;
        let biomass_growth = (0.08 + tile.soil_fertility * 0.12 + tile.mana_density * 0.04)
            * (0.55 + suitability * 0.45)
            * (0.88 + resource_bias * 0.12)
            * delta_days;

        state.forage = (state.forage + forage_growth).clamp(0.0, state.forage_capacity);
        state.tree_biomass =
            (state.tree_biomass + biomass_growth).clamp(0.0, state.tree_biomass_capacity);
    }
}

fn update_world_stats(
    mut stats: ResMut<WorldStats>,
    trees: Query<&Tree>,
    animals: Query<&Animal>,
    predators: Query<&Predator>,
    npcs: Query<&crate::agents::npc::Npc>,
    shelters: Query<&Shelter>,
    campfires: Query<&Campfire>,
    civic_structures: Query<&CivicStructure>,
    shelter_stockpiles: Query<&ShelterStockpile>,
    inventories: Query<&Inventory>,
    regions: Query<(&RegionTile, &RegionState, &RegionClimate)>,
) {
    let (
        mana_total,
        animal_capacity_total,
        tree_capacity_total,
        temperature_total,
        pressure_total,
        tile_count,
    ) = regions.iter().fold(
        (0.0, 0.0, 0.0, 0.0, 0.0, 0usize),
        |acc, (tile, _, climate)| {
            (
                acc.0 + tile.mana_density,
                acc.1 + tile.animal_capacity,
                acc.2 + tile.tree_capacity,
                acc.3 + tile.temperature,
                acc.4 + climate.pressure,
                acc.5 + 1,
            )
        },
    );
    let (total_forage, total_tree_biomass) =
        regions.iter().fold((0.0, 0.0), |acc, (_, state, _)| {
            (acc.0 + state.forage, acc.1 + state.tree_biomass)
        });

    stats.trees = trees.iter().count();
    stats.animals = animals.iter().count();
    stats.predators = predators.iter().count();
    stats.npcs = npcs.iter().count();
    stats.shelters = shelters.iter().count();
    stats.campfires = campfires.iter().count();
    stats.civic_structures = civic_structures.iter().count();
    let divisor = tile_count.max(1) as f32;
    stats.avg_mana_density = mana_total / divisor;
    stats.avg_animal_capacity = animal_capacity_total / divisor;
    stats.avg_tree_capacity = tree_capacity_total / divisor;
    stats.avg_temperature = temperature_total / divisor;
    stats.avg_climate_pressure = pressure_total / divisor;
    stats.animal_load_ratio = stats.animals as f32 / animal_capacity_total.max(1.0);
    stats.total_forage = total_forage;
    stats.total_tree_biomass = total_tree_biomass;
    stats.total_food_carried = inventories.iter().map(|inv| inv.food).sum();
    stats.total_wood_carried = inventories.iter().map(|inv| inv.wood).sum();
    stats.total_food_stockpiled = shelter_stockpiles.iter().map(|pile| pile.food).sum();
    stats.total_wood_stockpiled = shelter_stockpiles.iter().map(|pile| pile.wood).sum();
    stats.total_ore = inventories.iter().map(|inv| inv.ore).sum::<f32>()
        + shelter_stockpiles.iter().map(|pile| pile.ore).sum::<f32>();
    stats.total_metal = inventories.iter().map(|inv| inv.metal).sum::<f32>()
        + shelter_stockpiles
            .iter()
            .map(|pile| pile.metal)
            .sum::<f32>();
    stats.total_clothing = inventories.iter().map(|inv| inv.clothing).sum::<f32>()
        + shelter_stockpiles
            .iter()
            .map(|pile| pile.clothing)
            .sum::<f32>();
    stats.total_weapons = inventories.iter().map(|inv| inv.weapons).sum::<f32>()
        + shelter_stockpiles
            .iter()
            .map(|pile| pile.weapons)
            .sum::<f32>();
    let npc_count = stats.npcs.max(1) as f32;
    stats.avg_npc_exposure = npcs.iter().map(|npc| npc.exposure).sum::<f32>() / npc_count;
    stats.cold_stressed_npcs = npcs.iter().filter(|npc| npc.exposure > 0.45).count();
}

fn civic_structure_sizes(kind: CivicStructureKind) -> (Vec2, Vec2) {
    match kind {
        CivicStructureKind::Fence => (Vec2::new(38.0, 4.0), Vec2::new(4.0, 12.0)),
        CivicStructureKind::Workshop => (Vec2::new(24.0, 16.0), Vec2::new(18.0, 5.0)),
        CivicStructureKind::Nursery => (Vec2::new(22.0, 14.0), Vec2::new(10.0, 7.0)),
        CivicStructureKind::WatchPost => (Vec2::new(10.0, 30.0), Vec2::new(18.0, 5.0)),
        CivicStructureKind::Granary => (Vec2::new(20.0, 24.0), Vec2::new(24.0, 6.0)),
        CivicStructureKind::Forge => (Vec2::new(24.0, 14.0), Vec2::new(10.0, 10.0)),
        CivicStructureKind::TownHall => (Vec2::new(34.0, 22.0), Vec2::new(26.0, 7.0)),
    }
}

fn civic_structure_color(kind: CivicStructureKind) -> Color {
    match kind {
        CivicStructureKind::Fence => Color::srgb(0.45, 0.30, 0.16),
        CivicStructureKind::Workshop => Color::srgb(0.38, 0.34, 0.28),
        CivicStructureKind::Nursery => Color::srgb(0.50, 0.42, 0.30),
        CivicStructureKind::WatchPost => Color::srgb(0.33, 0.25, 0.18),
        CivicStructureKind::Granary => Color::srgb(0.55, 0.44, 0.22),
        CivicStructureKind::Forge => Color::srgb(0.26, 0.25, 0.24),
        CivicStructureKind::TownHall => Color::srgb(0.44, 0.37, 0.29),
    }
}

fn civic_structure_accent_color(kind: CivicStructureKind) -> Color {
    match kind {
        CivicStructureKind::Fence => Color::srgb(0.65, 0.45, 0.25),
        CivicStructureKind::Workshop => Color::srgb(0.56, 0.48, 0.36),
        CivicStructureKind::Nursery => Color::srgb(0.72, 0.60, 0.42),
        CivicStructureKind::WatchPost => Color::srgb(0.70, 0.58, 0.36),
        CivicStructureKind::Granary => Color::srgb(0.78, 0.62, 0.25),
        CivicStructureKind::Forge => Color::srgb(0.94, 0.34, 0.12),
        CivicStructureKind::TownHall => Color::srgb(0.68, 0.54, 0.34),
    }
}
