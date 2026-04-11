use bevy::prelude::*;

use crate::life::growth::Lifecycle;
use crate::systems::simulation::SimulationClock;
use crate::world::map::{MapSettings, RegionState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimalLifeStage {
    Juvenile,
    Adult,
    Elder,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Animal {
    pub health: f32,
    pub speed: f32,
    pub hunger: f32,
    pub energy: f32,
    pub reproduction_drive: f32,
    pub wander_angle: f32,
    pub life_stage: AnimalLifeStage,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Pregnancy {
    pub gestation_days: f32,
    pub offspring_health: f32,
    pub offspring_speed: f32,
}

#[derive(Bundle)]
pub struct AnimalBundle {
    pub sprite: Sprite,
    pub transform: Transform,
    pub animal: Animal,
    pub lifecycle: Lifecycle,
}

impl AnimalBundle {
    pub fn new(position: Vec2, health: f32, speed: f32) -> Self {
        Self {
            sprite: Sprite::from_color(Color::srgb(0.92, 0.80, 0.26), Vec2::splat(12.0)),
            transform: Transform::from_xyz(position.x, position.y, 3.0),
            animal: Animal {
                health,
                speed,
                hunger: 0.1,
                energy: 32.0,
                reproduction_drive: 0.0,
                wander_angle: 0.0,
                life_stage: AnimalLifeStage::Juvenile,
            },
            lifecycle: Lifecycle {
                age_days: 0.0,
                maturity_age: 18.0,
                max_age: 220.0,
                fertility: 0.75,
                reproduction_cooldown: 4.0,
            },
        }
    }
}

pub struct AnimalPlugin;

impl Plugin for AnimalPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (animal_wander, animal_forage).chain());
    }
}

fn animal_wander(
    clock: Res<SimulationClock>,
    settings: Res<MapSettings>,
    mut animals: Query<(&mut Transform, &mut Animal)>,
) {
    let delta_seconds = clock.delta_seconds();
    let bounds = settings.world_bounds() - Vec2::splat(8.0);

    for (mut transform, mut animal) in &mut animals {
        let speed_factor = match animal.life_stage {
            AnimalLifeStage::Juvenile => 0.8,
            AnimalLifeStage::Adult => 1.0,
            AnimalLifeStage::Elder => 0.7,
        };
        animal.wander_angle += delta_seconds * (0.4 + animal.speed * 0.1);
        animal.hunger = (animal.hunger + delta_seconds * 0.015).min(1.0);
        animal.energy = (animal.energy - delta_seconds * 0.3).max(0.0);
        animal.health = (animal.health - animal.hunger * delta_seconds * 0.12).max(0.0);

        let direction = Vec2::new(animal.wander_angle.cos(), animal.wander_angle.sin());
        transform.translation.x += direction.x * animal.speed * speed_factor * delta_seconds * 5.0;
        transform.translation.y += direction.y * animal.speed * speed_factor * delta_seconds * 5.0;

        if transform.translation.x.abs() >= bounds.x {
            animal.wander_angle = std::f32::consts::PI - animal.wander_angle;
        }

        if transform.translation.y.abs() >= bounds.y {
            animal.wander_angle = -animal.wander_angle;
        }

        transform.translation.x = transform.translation.x.clamp(-bounds.x, bounds.x);
        transform.translation.y = transform.translation.y.clamp(-bounds.y, bounds.y);
    }
}

fn animal_forage(
    clock: Res<SimulationClock>,
    settings: Res<MapSettings>,
    mut animals: Query<(&Transform, &mut Animal)>,
    mut regions: Query<(&crate::world::map::RegionTile, &mut RegionState)>,
) {
    let delta_days = clock.delta_days();

    for (transform, mut animal) in &mut animals {
        let coord = settings.tile_coord_for_position(transform.translation.truncate());

        for (tile, mut state) in &mut regions {
            if tile.coord != coord {
                continue;
            }

            let appetite = (0.35 + animal.hunger * 0.8) * delta_days;
            let taken = appetite.min(state.forage);
            state.forage -= taken;

            if taken > 0.0 {
                animal.hunger = (animal.hunger - taken * 0.9).max(0.0);
                animal.energy = (animal.energy + taken * 20.0).min(60.0);
                animal.health = (animal.health + taken * 4.0).min(40.0);
                state.tree_biomass =
                    (state.tree_biomass + taken * 0.08).min(state.tree_biomass_capacity);
            } else {
                animal.hunger = (animal.hunger + delta_days * 0.08).min(1.0);
            }

            break;
        }
    }
}
