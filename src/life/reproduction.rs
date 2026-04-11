use bevy::prelude::*;

use crate::agents::animal::{Animal, AnimalBundle, AnimalLifeStage, Pregnancy};
use crate::life::growth::Lifecycle;
use crate::systems::logging::{LogEvent, LogEventKind};
use crate::systems::simulation::SimulationClock;
use crate::world::map::{MapSettings, RegionState, RegionTile};
use crate::world::resources::{Tree, TreeStage};

pub struct ReproductionPlugin;

impl Plugin for ReproductionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                tree_seed_spread,
                animal_reproduction,
                resolve_animal_births.after(animal_reproduction),
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
                Sprite::from_color(Color::srgb(0.22, 0.78, 0.30), Vec2::splat(10.0)),
                Transform::from_xyz(spawn_position.x, spawn_position.y, 2.0),
                Tree {
                    root_coord: spawn_coord,
                    stage: TreeStage::Sapling,
                    growth: 0.1,
                    spread_progress: 0.0,
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

        animal.reproduction_drive += delta_days * 0.12 * lifecycle.fertility.max(0.1);

        if animal.reproduction_drive < 1.0 {
            continue;
        }

        animal.reproduction_drive = 0.0;
        animal.energy = (animal.energy - 8.0).max(0.0);
        lifecycle.reproduction_cooldown = 22.0;
        commands.entity(entity).insert(Pregnancy {
            gestation_days: 9.0,
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
