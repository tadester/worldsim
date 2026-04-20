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

#[derive(Component)]
struct AnimalBody;

#[derive(Component)]
struct AnimalHead;

#[derive(Component)]
struct AnimalLeg;

#[derive(Component)]
struct AnimalEar;

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
            sprite: Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.0), Vec2::splat(1.0)),
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
                maturity_age: 420.0,
                max_age: 4_380.0,
                fertility: 0.32,
                reproduction_cooldown: 180.0,
            },
        }
    }

    pub fn with_age_days(mut self, age_days: f32) -> Self {
        self.lifecycle.age_days = age_days.max(0.0);
        self.animal.life_stage = if age_days < self.lifecycle.maturity_age {
            AnimalLifeStage::Juvenile
        } else if age_days > self.lifecycle.max_age * 0.75 {
            AnimalLifeStage::Elder
        } else {
            AnimalLifeStage::Adult
        };
        self
    }
}

pub struct AnimalPlugin;

impl Plugin for AnimalPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                attach_animal_visuals,
                sync_animal_visuals,
                (animal_wander, animal_forage).chain(),
            ),
        );
    }
}

fn attach_animal_visuals(mut commands: Commands, animals: Query<Entity, Added<Animal>>) {
    for entity in &animals {
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                Sprite::from_color(Color::srgb(0.69, 0.56, 0.24), Vec2::new(18.0, 10.0)),
                Transform::from_xyz(0.0, 0.0, 0.1),
                AnimalBody,
            ));
            parent.spawn((
                Sprite::from_color(Color::srgb(0.78, 0.66, 0.32), Vec2::new(8.0, 7.0)),
                Transform::from_xyz(9.0, 2.0, 0.2),
                AnimalHead,
            ));
            parent.spawn((
                Sprite::from_color(Color::srgb(0.42, 0.27, 0.11), Vec2::new(2.0, 8.0)),
                Transform::from_xyz(-5.0, -7.0, 0.0),
                AnimalLeg,
            ));
            parent.spawn((
                Sprite::from_color(Color::srgb(0.42, 0.27, 0.11), Vec2::new(2.0, 8.0)),
                Transform::from_xyz(4.0, -7.0, 0.0),
                AnimalLeg,
            ));
            parent.spawn((
                Sprite::from_color(Color::srgb(0.89, 0.80, 0.55), Vec2::new(2.0, 3.0)),
                Transform::from_xyz(11.0, 6.0, 0.3),
                AnimalEar,
            ));
        });
    }
}

fn sync_animal_visuals(
    animals: Query<(&Animal, &Children), Changed<Animal>>,
    mut bodies: Query<(&mut Sprite, &mut Transform), With<AnimalBody>>,
    mut heads: Query<(&mut Sprite, &mut Transform), (With<AnimalHead>, Without<AnimalBody>)>,
    mut legs: Query<&mut Transform, (With<AnimalLeg>, Without<AnimalBody>, Without<AnimalHead>)>,
    mut ears: Query<
        (&mut Sprite, &mut Transform),
        (
            With<AnimalEar>,
            Without<AnimalBody>,
            Without<AnimalHead>,
            Without<AnimalLeg>,
        ),
    >,
) {
    for (animal, children) in &animals {
        let scale = match animal.life_stage {
            AnimalLifeStage::Juvenile => 0.8,
            AnimalLifeStage::Adult => 1.0,
            AnimalLifeStage::Elder => 1.1,
        };
        let body_color = if animal.hunger > 0.75 {
            Color::srgb(0.59, 0.47, 0.21)
        } else {
            Color::srgb(0.69, 0.56, 0.24)
        };
        let head_color = if animal.energy < 14.0 {
            Color::srgb(0.68, 0.57, 0.30)
        } else {
            Color::srgb(0.78, 0.66, 0.32)
        };

        for child in children.iter() {
            if let Ok((mut sprite, mut transform)) = bodies.get_mut(child) {
                sprite.color = body_color;
                sprite.custom_size = Some(Vec2::new(18.0 * scale, 10.0 * scale));
                transform.translation.y = if animal.hunger > 0.8 { -0.5 } else { 0.0 };
            }

            if let Ok((mut sprite, mut transform)) = heads.get_mut(child) {
                sprite.color = head_color;
                sprite.custom_size = Some(Vec2::new(8.0 * scale, 7.0 * scale));
                transform.translation.x = 9.0 * scale;
                transform.translation.y = 2.0 * scale;
            }

            if let Ok(mut transform) = legs.get_mut(child) {
                transform.scale.y = if animal.energy < 10.0 { 0.85 } else { 1.0 };
            }

            if let Ok((mut sprite, mut transform)) = ears.get_mut(child) {
                sprite.color = if animal.life_stage == AnimalLifeStage::Juvenile {
                    Color::srgb(0.94, 0.86, 0.66)
                } else {
                    Color::srgb(0.89, 0.80, 0.55)
                };
                transform.translation.x = 11.0 * scale;
                transform.translation.y = 6.0 * scale;
            }
        }
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
