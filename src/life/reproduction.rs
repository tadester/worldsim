use bevy::prelude::*;

use crate::agents::animal::{Animal, AnimalBundle, AnimalLifeStage, Pregnancy};
use crate::agents::inventory::Inventory;
use crate::agents::needs::Needs;
use crate::agents::npc::{Npc, NpcBundle, NpcHome};
use crate::life::growth::Lifecycle;
use crate::magic::mana::ManaReservoir;
use crate::magic::storage::ManaStorageStyle;
use crate::systems::logging::{LogEvent, LogEventKind};
use crate::systems::simulation::{SimulationClock, SimulationStep};
use crate::world::map::{MapSettings, RegionState, RegionTile};
use crate::world::resources::{ShelterStockpile, Tree, TreeStage};

pub struct ReproductionPlugin;

#[derive(Component, Debug, Clone, Copy)]
pub struct NpcPregnancy {
    pub gestation_days: f32,
}

impl Plugin for ReproductionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                tree_seed_spread,
                animal_reproduction,
                resolve_animal_births.after(animal_reproduction),
                npc_reproduction.after(resolve_animal_births),
                resolve_npc_births.after(npc_reproduction),
            ),
        );
    }
}

fn tree_seed_spread(
    mut commands: Commands,
    clock: Res<SimulationClock>,
    settings: Res<MapSettings>,
    mut trees: Query<(&Transform, &mut Tree)>,
    mut regions: Query<(&RegionTile, &mut RegionState)>,
) {
    let delta_days = clock.delta_days();

    for (transform, mut tree) in &mut trees {
        if tree.stage != TreeStage::Mature {
            continue;
        }

        let mut biomass_ratio = 0.0;
        for (tile, state) in &mut regions {
            if tile.coord != tree.root_coord {
                continue;
            }

            biomass_ratio = if state.tree_biomass_capacity <= 0.0 {
                0.0
            } else {
                state.tree_biomass / state.tree_biomass_capacity
            };
            break;
        }

        tree.spread_progress += delta_days * 0.08 * biomass_ratio.max(0.2);

        if tree.spread_progress < 1.0 || biomass_ratio <= 0.25 {
            continue;
        }

        tree.spread_progress = 0.0;
        let spawn_offset =
            Vec2::new(transform.translation.x.sin(), transform.translation.y.cos()) * 18.0;
        let spawn_position = transform.translation.truncate() + spawn_offset;
        let spawn_coord = settings.tile_coord_for_position(spawn_position);

        for (tile, mut state) in &mut regions {
            if tile.coord != spawn_coord {
                continue;
            }

            if state.tree_biomass < 0.4 {
                break;
            }

            state.tree_biomass = (state.tree_biomass - 0.35).max(0.0);
            commands.spawn((
                Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.0), Vec2::splat(1.0)),
                Transform::from_xyz(spawn_position.x, spawn_position.y, 2.0),
                Tree {
                    root_coord: spawn_coord,
                    stage: TreeStage::Sapling,
                    growth: 0.1,
                    spread_progress: 0.0,
                },
                ManaReservoir {
                    capacity: 10.0 + tile.mana_density * 10.0,
                    stored: tile.mana_density * 2.0,
                    stability: 0.85,
                },
            ));
            break;
        }
    }
}

fn animal_reproduction(
    mut commands: Commands,
    clock: Res<SimulationClock>,
    mut writer: MessageWriter<LogEvent>,
    mut animals: Query<(
        Entity,
        &Transform,
        &mut Animal,
        &mut Lifecycle,
        Option<&Pregnancy>,
    )>,
) {
    let delta_days = clock.delta_days();

    for (entity, transform, mut animal, mut lifecycle, pregnancy) in &mut animals {
        let mature = lifecycle.age_days >= lifecycle.maturity_age;
        let fertile = lifecycle.reproduction_cooldown <= 0.0;
        let healthy = animal.health >= 25.0 && animal.energy >= 22.0;
        let adult = animal.life_stage == AnimalLifeStage::Adult;

        if !(mature && fertile && healthy && adult && pregnancy.is_none()) {
            continue;
        }

        animal.reproduction_drive += delta_days * 0.55 * lifecycle.fertility.max(0.25);

        if animal.reproduction_drive < 0.55 {
            continue;
        }

        animal.reproduction_drive = 0.0;
        animal.energy = (animal.energy - 8.0).max(0.0);
        lifecycle.reproduction_cooldown = 9.0;
        commands.entity(entity).insert(Pregnancy {
            gestation_days: 5.0,
            offspring_health: 18.0,
            offspring_speed: animal.speed * 0.95,
        });

        writer.write(LogEvent::new(
            LogEventKind::Discovery,
            format!(
                "Animal pregnancy began near {:.0},{:.0}",
                transform.translation.x, transform.translation.y
            ),
        ));
    }
}

fn npc_reproduction(
    mut commands: Commands,
    clock: Res<SimulationClock>,
    step: Res<SimulationStep>,
    mut writer: MessageWriter<LogEvent>,
    shelters: Query<Option<&ShelterStockpile>>,
    mut npcs: Query<(
        Entity,
        &Npc,
        &Transform,
        &Needs,
        &Inventory,
        &NpcHome,
        &mut Lifecycle,
        Option<&NpcPregnancy>,
    )>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    for (entity, npc, transform, needs, inventory, home, mut lifecycle, pregnancy) in &mut npcs {
        if pregnancy.is_some()
            || lifecycle.age_days < lifecycle.maturity_age
            || lifecycle.reproduction_cooldown > 0.0
            || npc.health < 42.0
            || needs.hunger > 0.45
            || needs.fatigue > 0.7
            || needs.safety < 0.42
        {
            continue;
        }

        let Some(home_entity) = home.shelter else {
            continue;
        };
        let stockpile = shelters
            .get(home_entity)
            .ok()
            .flatten()
            .copied()
            .unwrap_or_default();

        let resource_security = stockpile.food + inventory.food + stockpile.wood * 0.35;
        if resource_security < 1.25 {
            continue;
        }

        let entity_seed = entity.to_bits() as f32;
        let cycle_days = 10.0 + (entity.to_bits() % 5) as f32 * 2.0;
        let phase = (step.elapsed_days + entity_seed * 0.37) % cycle_days;
        if phase > delta_days * 1.5 {
            continue;
        }

        lifecycle.reproduction_cooldown = 36.0;
        commands.entity(entity).insert(NpcPregnancy {
            gestation_days: 12.0,
        });
        writer.write(LogEvent::new(
            LogEventKind::Birth,
            format!(
                "{} is expecting a child near {:.0},{:.0}",
                npc.name, transform.translation.x, transform.translation.y
            ),
        ));
    }
}

fn resolve_npc_births(
    mut commands: Commands,
    clock: Res<SimulationClock>,
    step: Res<SimulationStep>,
    mut writer: MessageWriter<LogEvent>,
    mut npcs: Query<(
        Entity,
        &Transform,
        &Npc,
        &ManaReservoir,
        &ManaStorageStyle,
        &mut NpcPregnancy,
    )>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    for (entity, transform, npc, reservoir, mana_style, mut pregnancy) in &mut npcs {
        pregnancy.gestation_days -= delta_days;
        if pregnancy.gestation_days > 0.0 {
            continue;
        }

        let offset = Vec2::new(transform.translation.y.sin(), transform.translation.x.cos()) * 16.0;
        let child_name = format!("{} Kin {}", npc.name, step.tick % 10_000);
        commands.spawn(
            NpcBundle::new(
                transform.translation.truncate() + offset,
                child_name,
                (npc.health * 0.72).clamp(34.0, 60.0),
                ManaReservoir {
                    capacity: reservoir.capacity,
                    stored: (reservoir.stored * 0.35).min(reservoir.capacity),
                    stability: reservoir.stability,
                },
                *mana_style,
            )
            .with_age_days(0.0),
        );
        commands.entity(entity).remove::<NpcPregnancy>();
        writer.write(LogEvent::new(
            LogEventKind::Birth,
            format!("A child was born to {}", npc.name),
        ));
    }
}

fn resolve_animal_births(
    mut commands: Commands,
    clock: Res<SimulationClock>,
    mut writer: MessageWriter<LogEvent>,
    mut animals: Query<(Entity, &Transform, &mut Pregnancy)>,
) {
    let delta_days = clock.delta_days();

    for (entity, transform, mut pregnancy) in &mut animals {
        pregnancy.gestation_days -= delta_days;

        if pregnancy.gestation_days > 0.0 {
            continue;
        }

        let offset = Vec2::new(transform.translation.y.cos(), transform.translation.x.sin()) * 14.0;
        commands.spawn(AnimalBundle::new(
            transform.translation.truncate() + offset,
            pregnancy.offspring_health,
            pregnancy.offspring_speed,
        ));
        commands.entity(entity).remove::<Pregnancy>();

        writer.write(LogEvent::new(
            LogEventKind::Birth,
            format!(
                "Animal offspring born near {:.0},{:.0}",
                transform.translation.x, transform.translation.y
            ),
        ));
    }
}
