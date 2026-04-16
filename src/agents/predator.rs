use bevy::prelude::*;

use crate::agents::animal::Animal;
use crate::agents::decisions::NpcIntent;
use crate::agents::npc::Npc;
use crate::systems::logging::{LogEvent, LogEventKind};
use crate::systems::simulation::{SimulationClock, SimulationStep};
use crate::world::map::{MapSettings, RegionTile};

#[derive(Component, Debug, Clone, Copy)]
pub struct Predator {
    pub health: f32,
    pub speed: f32,
    pub hunger: f32,
    pub attack_cooldown: f32,
    pub wander_angle: f32,
    pub last_log_day: f32,
}

#[derive(Component)]
struct PredatorBody;

#[derive(Component)]
struct PredatorHead;

#[derive(Component)]
struct PredatorClaw;

#[derive(Bundle)]
pub struct PredatorBundle {
    pub sprite: Sprite,
    pub transform: Transform,
    pub predator: Predator,
}

impl PredatorBundle {
    pub fn new(position: Vec2, seed: f32) -> Self {
        Self {
            sprite: Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.0), Vec2::splat(1.0)),
            transform: Transform::from_xyz(position.x, position.y, 4.5),
            predator: Predator {
                health: 52.0 + seed * 10.0,
                speed: 36.0 + seed * 8.0,
                hunger: 0.2,
                attack_cooldown: 0.0,
                wander_angle: seed * 6.28,
                last_log_day: -999.0,
            },
        }
    }
}

pub struct PredatorPlugin;

impl Plugin for PredatorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, seed_predators).add_systems(
            Update,
            (
                attach_predator_visuals,
                sync_predator_visuals,
                predator_hunt,
                predator_attack.after(predator_hunt),
                cleanup_dead_predators.after(predator_attack),
            )
                .chain(),
        );
    }
}

fn seed_predators(
    mut commands: Commands,
    settings: Res<MapSettings>,
    tiles: Query<(&RegionTile, &Transform)>,
) {
    let desired = ((settings.width * settings.height) as usize / 80).clamp(3, 7);
    let mut candidates = tiles
        .iter()
        .filter(|(tile, _)| tile.mana_density > 0.65 && tile.soil_fertility < 0.7)
        .map(|(tile, transform)| (tile.mana_density, transform.translation.truncate()))
        .collect::<Vec<_>>();
    candidates.sort_by(|a, b| a.0.total_cmp(&b.0));

    let bounds = settings.world_bounds() - Vec2::splat(30.0);
    let fallback = [
        Vec2::new(-bounds.x, -bounds.y),
        Vec2::new(bounds.x, -bounds.y),
        Vec2::new(-bounds.x, bounds.y),
        Vec2::new(bounds.x, bounds.y),
        Vec2::new(bounds.x * 0.8, 0.0),
        Vec2::new(-bounds.x * 0.8, 0.0),
        Vec2::new(0.0, bounds.y * 0.8),
    ];

    for idx in 0..desired {
        let seed = (idx as f32 * 0.23 + 0.17).fract();
        let position = if !candidates.is_empty() {
            let stride = (candidates.len() / desired).max(1);
            candidates
                .get((idx * stride).min(candidates.len() - 1))
                .map(|(_, pos)| *pos)
                .unwrap_or_else(|| fallback[idx % fallback.len()])
        } else {
            fallback[idx % fallback.len()]
        };
        commands.spawn(PredatorBundle::new(position, seed));
    }
}

fn attach_predator_visuals(mut commands: Commands, predators: Query<Entity, Added<Predator>>) {
    for entity in &predators {
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                Sprite::from_color(Color::srgb(0.46, 0.12, 0.12), Vec2::new(18.0, 10.0)),
                Transform::from_xyz(0.0, 0.0, 0.1),
                PredatorBody,
            ));
            parent.spawn((
                Sprite::from_color(Color::srgb(0.56, 0.18, 0.18), Vec2::new(8.0, 7.0)),
                Transform::from_xyz(9.0, 2.0, 0.2),
                PredatorHead,
            ));
            parent.spawn((
                Sprite::from_color(Color::srgb(0.25, 0.05, 0.05), Vec2::new(2.0, 6.0)),
                Transform::from_xyz(-4.0, -6.0, 0.0),
                PredatorClaw,
            ));
            parent.spawn((
                Sprite::from_color(Color::srgb(0.25, 0.05, 0.05), Vec2::new(2.0, 6.0)),
                Transform::from_xyz(3.0, -6.0, 0.0),
                PredatorClaw,
            ));
        });
    }
}

fn sync_predator_visuals(
    predators: Query<(&Predator, &Children), Changed<Predator>>,
    mut bodies: Query<&mut Sprite, With<PredatorBody>>,
    mut heads: Query<&mut Sprite, (With<PredatorHead>, Without<PredatorBody>)>,
) {
    for (predator, children) in &predators {
        let hunger = predator.hunger.clamp(0.0, 1.0);
        let health_ratio = (predator.health / 70.0).clamp(0.0, 1.0);
        let body_color = Color::srgb(0.34 + hunger * 0.12, 0.10 + health_ratio * 0.10, 0.10);
        let head_color = Color::srgb(0.44 + hunger * 0.12, 0.14 + health_ratio * 0.10, 0.14);

        for child in children.iter() {
            if let Ok(mut sprite) = bodies.get_mut(child) {
                sprite.color = body_color;
            }
            if let Ok(mut sprite) = heads.get_mut(child) {
                sprite.color = head_color;
            }
        }
    }
}

fn predator_hunt(
    clock: Res<SimulationClock>,
    settings: Res<MapSettings>,
    npcs: Query<(Entity, &Transform), With<Npc>>,
    animals: Query<(Entity, &Transform), With<Animal>>,
    mut predators: Query<(&mut Transform, &mut Predator), (Without<Npc>, Without<Animal>)>,
) {
    let delta_seconds = clock.delta_seconds();
    if delta_seconds <= 0.0 {
        return;
    }

    let bounds = settings.world_bounds() - Vec2::splat(12.0);
    let npc_positions = npcs
        .iter()
        .map(|(entity, transform)| (entity, transform.translation.truncate()))
        .collect::<Vec<_>>();
    let animal_positions = animals
        .iter()
        .map(|(entity, transform)| (entity, transform.translation.truncate()))
        .collect::<Vec<_>>();

    for (mut transform, mut predator) in &mut predators {
        predator.attack_cooldown = (predator.attack_cooldown - delta_seconds).max(0.0);
        predator.hunger = (predator.hunger + delta_seconds * 0.012).min(1.0);
        predator.health = (predator.health - predator.hunger * delta_seconds * 0.08).max(0.0);
        predator.wander_angle += delta_seconds * (0.25 + predator.speed * 0.01);

        let pos = transform.translation.truncate();
        let mut target = None;
        let mut best_dist = f32::MAX;

        for (entity, other_pos) in npc_positions.iter().copied() {
            let dist = pos.distance(other_pos);
            if dist < best_dist && dist < 240.0 {
                best_dist = dist;
                target = Some((entity, other_pos));
            }
        }

        if target.is_none() {
            for (entity, other_pos) in animal_positions.iter().copied() {
                let dist = pos.distance(other_pos);
                if dist < best_dist && dist < 190.0 {
                    best_dist = dist;
                    target = Some((entity, other_pos));
                }
            }
        }

        let direction = if let Some((_, target_pos)) = target {
            (target_pos - pos).normalize_or_zero()
        } else {
            Vec2::new(predator.wander_angle.cos(), predator.wander_angle.sin())
        };

        let pace = predator.speed * (0.65 + predator.hunger * 0.55) * delta_seconds;
        transform.translation.x += direction.x * pace;
        transform.translation.y += direction.y * pace;
        transform.translation.x = transform.translation.x.clamp(-bounds.x, bounds.x);
        transform.translation.y = transform.translation.y.clamp(-bounds.y, bounds.y);
    }
}

fn predator_attack(
    clock: Res<SimulationClock>,
    step: Res<SimulationStep>,
    mut writer: MessageWriter<LogEvent>,
    mut predators: Query<(&Transform, &mut Predator)>,
    mut npcs: Query<(Entity, &Transform, &mut Npc, &NpcIntent)>,
    mut animals: Query<(Entity, &Transform, &mut Animal)>,
) {
    let delta_seconds = clock.delta_seconds();
    if delta_seconds <= 0.0 {
        return;
    }

    let npc_positions = npcs
        .iter()
        .map(|(entity, transform, _, _)| (entity, transform.translation.truncate()))
        .collect::<Vec<_>>();
    let animal_positions = animals
        .iter()
        .map(|(entity, transform, _)| (entity, transform.translation.truncate()))
        .collect::<Vec<_>>();

    for (transform, mut predator) in &mut predators {
        if predator.attack_cooldown > 0.0 || predator.health <= 0.0 {
            continue;
        }

        let pos = transform.translation.truncate();
        let mut best_target = None;
        let mut best_dist = 18.0f32;

        for (entity, other_pos) in npc_positions.iter().copied() {
            let dist = pos.distance(other_pos);
            if dist < best_dist {
                best_dist = dist;
                best_target = Some((entity, true));
            }
        }

        if best_target.is_none() {
            for (entity, other_pos) in animal_positions.iter().copied() {
                let dist = pos.distance(other_pos);
                if dist < best_dist {
                    best_dist = dist;
                    best_target = Some((entity, false));
                }
            }
        }

        let Some((target, is_npc)) = best_target else {
            continue;
        };

        let damage = 7.5 + predator.hunger * 6.5;
        predator.attack_cooldown = 0.9 + predator.hunger * 0.5;
        predator.hunger = (predator.hunger - 0.22).max(0.0);

        if is_npc {
            if let Ok((_, _, mut npc, intent)) = npcs.get_mut(target) {
                npc.health = (npc.health - damage).max(0.0);
                if intent.label == "Defend" {
                    predator.health = (predator.health - 2.4).max(0.0);
                }

                if step.elapsed_days - predator.last_log_day > 0.35 {
                    writer.write(LogEvent::new(
                        LogEventKind::Threat,
                        format!("Predator mauled {}", npc.name),
                    ));
                    predator.last_log_day = step.elapsed_days;
                }
            }
        } else if let Ok((_, _, mut animal)) = animals.get_mut(target) {
            animal.health = (animal.health - damage * 0.85).max(0.0);
        }
    }
}

fn cleanup_dead_predators(
    mut commands: Commands,
    mut writer: MessageWriter<LogEvent>,
    predators: Query<(Entity, &Predator)>,
) {
    for (entity, predator) in &predators {
        if predator.health > 0.0 {
            continue;
        }
        commands.entity(entity).despawn();
        writer.write(LogEvent::new(
            LogEventKind::Death,
            "A predator died".to_string(),
        ));
    }
}
