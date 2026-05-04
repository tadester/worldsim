use bevy::prelude::*;

use crate::agents::animal::Animal;
use crate::agents::decisions::NpcIntent;
use crate::agents::evolution::EvolutionPressure;
use crate::agents::npc::{Npc, NpcCondition};
use crate::systems::logging::{LogEvent, LogEventKind};
use crate::systems::simulation::{SimulationClock, SimulationStep};
use crate::world::map::{MapSettings, RegionTile};
use crate::world::resources::WorldStats;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PredatorAbility {
    None,
    Blink,
    Thornhide,
    DreadHowl,
    ManaFangs,
}

impl PredatorAbility {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Blink => "Blink",
            Self::Thornhide => "Thornhide",
            Self::DreadHowl => "Dread Howl",
            Self::ManaFangs => "Mana Fangs",
        }
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Predator {
    pub health: f32,
    pub speed: f32,
    pub hunger: f32,
    pub attack_cooldown: f32,
    pub satiated_days: f32,
    pub wander_angle: f32,
    pub last_log_day: f32,
    pub age_days: f32,
    pub mana_mutation: f32,
    pub ability: PredatorAbility,
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
                attack_cooldown: 1.8,
                satiated_days: 0.9 + seed * 0.6,
                wander_angle: seed * 6.28,
                last_log_day: -999.0,
                age_days: 120.0 + seed * 180.0,
                mana_mutation: seed * 0.22,
                ability: PredatorAbility::None,
            },
        }
    }
}

pub struct PredatorPlugin;

#[derive(Resource, Debug, Clone, Copy)]
struct PredatorPressure {
    last_spawn_day: f32,
}

impl Default for PredatorPressure {
    fn default() -> Self {
        Self {
            last_spawn_day: -999.0,
        }
    }
}

impl Plugin for PredatorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PredatorPressure>()
            .add_systems(PostStartup, seed_predators)
            .add_systems(
                Update,
                (
                    attach_predator_visuals,
                    sync_predator_visuals,
                    escalate_predator_pressure,
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
    let settler_band = settings.height / 2;
    let mut candidates = tiles
        .iter()
        .filter(|(tile, _)| {
            tile.mana_density > 0.65
                && tile.soil_fertility < 0.7
                && (tile.coord.y - settler_band).abs() >= 3
        })
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
    step: Res<SimulationStep>,
    predators: Query<(&Predator, &Children)>,
    mut bodies: Query<
        (&mut Sprite, &mut Transform),
        (
            With<PredatorBody>,
            Without<PredatorHead>,
            Without<PredatorClaw>,
        ),
    >,
    mut heads: Query<
        (&mut Sprite, &mut Transform),
        (
            With<PredatorHead>,
            Without<PredatorBody>,
            Without<PredatorClaw>,
        ),
    >,
    mut claws: Query<
        &mut Transform,
        (
            With<PredatorClaw>,
            Without<PredatorBody>,
            Without<PredatorHead>,
        ),
    >,
) {
    let phase = step.elapsed_days * 24.0;
    for (predator, children) in &predators {
        let hunger = predator.hunger.clamp(0.0, 1.0);
        let health_ratio = (predator.health / 70.0).clamp(0.0, 1.0);
        let mana_glow = predator.mana_mutation.clamp(0.0, 1.0) * 0.24;
        let hunt_cycle = if predator.attack_cooldown < 0.35 {
            (phase * 2.8).sin()
        } else {
            phase.sin() * 0.25
        };
        let body_color = match predator.ability {
            PredatorAbility::Blink => Color::srgb(0.28 + hunger * 0.08, 0.14, 0.26 + mana_glow),
            PredatorAbility::Thornhide => {
                Color::srgb(0.24 + mana_glow * 0.2, 0.26 + mana_glow * 0.4, 0.14)
            }
            PredatorAbility::DreadHowl => {
                Color::srgb(0.22 + mana_glow * 0.2, 0.10, 0.10 + mana_glow * 0.5)
            }
            PredatorAbility::ManaFangs => Color::srgb(
                0.18 + mana_glow * 0.3,
                0.12 + health_ratio * 0.08,
                0.30 + mana_glow,
            ),
            PredatorAbility::None => {
                Color::srgb(0.34 + hunger * 0.12, 0.10 + health_ratio * 0.10, 0.10)
            }
        };
        let head_color = match predator.ability {
            PredatorAbility::Blink => Color::srgb(0.48, 0.22, 0.58 + mana_glow),
            PredatorAbility::Thornhide => Color::srgb(0.36, 0.42 + mana_glow, 0.20),
            PredatorAbility::DreadHowl => Color::srgb(0.60 + mana_glow, 0.12, 0.18),
            PredatorAbility::ManaFangs => Color::srgb(0.42, 0.18, 0.72 + mana_glow),
            PredatorAbility::None => {
                Color::srgb(0.44 + hunger * 0.12, 0.14 + health_ratio * 0.10, 0.14)
            }
        };

        for child in children.iter() {
            if let Ok((mut sprite, mut transform)) = bodies.get_mut(child) {
                sprite.color = body_color;
                let leap_scale = if predator.ability == PredatorAbility::Blink {
                    1.0 + hunt_cycle.abs() * 0.16
                } else {
                    1.0 + hunt_cycle.abs() * 0.06
                };
                transform.scale = Vec3::splat(leap_scale);
                transform.translation.y = if predator.ability == PredatorAbility::Thornhide {
                    hunt_cycle.abs() * 0.8
                } else {
                    0.0
                };
            }
            if let Ok((mut sprite, mut transform)) = heads.get_mut(child) {
                sprite.color = head_color;
                transform.translation.x = 9.0
                    + if predator.ability == PredatorAbility::Blink {
                        hunt_cycle * 2.4
                    } else {
                        hunt_cycle * 0.6
                    };
                transform.translation.y = 2.0
                    + if predator.ability == PredatorAbility::DreadHowl {
                        hunt_cycle.abs() * 3.6
                    } else {
                        hunt_cycle.abs() * 1.2
                    };
            }
            if let Ok(mut transform) = claws.get_mut(child) {
                transform.translation.y = -6.0
                    - if predator.ability == PredatorAbility::ManaFangs {
                        hunt_cycle.abs() * 2.0
                    } else {
                        hunt_cycle.abs() * 0.6
                    };
                transform.scale = Vec3::new(
                    1.0,
                    if predator.ability == PredatorAbility::ManaFangs {
                        1.0 + hunt_cycle.abs() * 0.45
                    } else {
                        1.0
                    },
                    1.0,
                );
            }
        }
    }
}

fn predator_hunt(
    clock: Res<SimulationClock>,
    settings: Res<MapSettings>,
    tiles: Query<&RegionTile>,
    npcs: Query<(Entity, &Transform), With<Npc>>,
    animals: Query<(Entity, &Transform), With<Animal>>,
    mut predators: Query<(&mut Transform, &mut Predator), (Without<Npc>, Without<Animal>)>,
) {
    let delta_seconds = clock.delta_seconds();
    let delta_days = clock.delta_days();
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
        predator.age_days += delta_days;
        predator.attack_cooldown = (predator.attack_cooldown - delta_seconds).max(0.0);
        predator.satiated_days = (predator.satiated_days - delta_days).max(0.0);
        let hunger_gain = if predator.satiated_days > 0.0 {
            delta_seconds * 0.0035
        } else {
            delta_seconds * 0.014
        };
        predator.hunger = (predator.hunger + hunger_gain).min(1.0);
        predator.health = (predator.health - predator.hunger * delta_seconds * 0.08).max(0.0);
        predator.wander_angle += delta_seconds * (0.25 + predator.speed * 0.01);
        let coord = settings.tile_coord_for_position(transform.translation.truncate());
        let ambient = tiles
            .iter()
            .find(|tile| tile.coord == coord)
            .map(|tile| tile.mana_density)
            .unwrap_or(0.25);
        predator.mana_mutation = (predator.mana_mutation + ambient * delta_days * 0.0018).min(1.0);
        if predator.ability == PredatorAbility::None
            && predator.age_days > 180.0
            && predator.mana_mutation > 0.52
        {
            predator.ability = if ambient > 0.82 {
                PredatorAbility::Blink
            } else if ambient > 0.72 {
                PredatorAbility::ManaFangs
            } else if predator.health > 58.0 {
                PredatorAbility::Thornhide
            } else {
                PredatorAbility::DreadHowl
            };
        }

        let pos = transform.translation.truncate();
        let mut best_dist = f32::MAX;
        let actively_hunting = predator.satiated_days <= 0.0 && predator.hunger >= 0.38;

        let mut nearest_animal = None;
        if actively_hunting {
            for (entity, other_pos) in animal_positions.iter().copied() {
                let dist = pos.distance(other_pos);
                if dist < best_dist && dist < 220.0 {
                    best_dist = dist;
                    nearest_animal = Some((entity, other_pos));
                }
            }
        }

        let mut nearest_npc = None;
        best_dist = f32::MAX;
        if actively_hunting {
            for (entity, other_pos) in npc_positions.iter().copied() {
                let dist = pos.distance(other_pos);
                if dist < best_dist && dist < 150.0 {
                    best_dist = dist;
                    nearest_npc = Some((entity, other_pos));
                }
            }
        }

        let mut target = if predator.hunger < 0.68 {
            nearest_animal.or(nearest_npc)
        } else {
            nearest_npc.or(nearest_animal)
        };

        if actively_hunting && target.is_none() {
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
        } else if predator.satiated_days > 0.0 {
            let drift = Vec2::new(-predator.wander_angle.sin(), predator.wander_angle.cos());
            drift.normalize_or_zero()
        } else {
            Vec2::new(predator.wander_angle.cos(), predator.wander_angle.sin())
        };

        let blink_boost = if predator.ability == PredatorAbility::Blink {
            1.24
        } else {
            1.0
        };
        let pace = if predator.satiated_days > 0.0 {
            predator.speed * 0.42 * blink_boost * delta_seconds
        } else {
            predator.speed * (0.62 + predator.hunger * 0.55) * blink_boost * delta_seconds
        };
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
    mut npcs: Query<(Entity, &Transform, &mut Npc, &NpcIntent, &mut NpcCondition)>,
    mut animals: Query<(Entity, &Transform, &mut Animal)>,
) {
    let delta_seconds = clock.delta_seconds();
    if delta_seconds <= 0.0 {
        return;
    }

    let npc_positions = npcs
        .iter()
        .map(|(entity, transform, _, _, _)| (entity, transform.translation.truncate()))
        .collect::<Vec<_>>();
    let animal_positions = animals
        .iter()
        .map(|(entity, transform, _)| (entity, transform.translation.truncate()))
        .collect::<Vec<_>>();

    for (transform, mut predator) in &mut predators {
        if predator.attack_cooldown > 0.0
            || predator.health <= 0.0
            || predator.satiated_days > 0.0
            || predator.hunger < 0.34
        {
            continue;
        }

        let pos = transform.translation.truncate();
        let mut best_target = None;
        let mut best_dist = if predator.hunger >= 0.78 { 14.0 } else { 11.0 };
        let mut nearest_animal = None;
        let mut animal_dist = 18.0f32;

        for (entity, other_pos) in animal_positions.iter().copied() {
            let dist = pos.distance(other_pos);
            if dist < animal_dist {
                animal_dist = dist;
                nearest_animal = Some((entity, false));
            }
        }

        if predator.hunger >= 0.78 || nearest_animal.is_none() {
            for (entity, other_pos) in npc_positions.iter().copied() {
                let dist = pos.distance(other_pos);
                if dist < best_dist {
                    best_dist = dist;
                    best_target = Some((entity, true));
                }
            }
        }

        if best_target.is_none() {
            best_target = nearest_animal;
        }

        let Some((target, is_npc)) = best_target else {
            continue;
        };

        let mut base_damage = if is_npc {
            4.5 + predator.hunger * 3.5
        } else {
            6.5 + predator.hunger * 5.0
        };
        if predator.ability == PredatorAbility::ManaFangs {
            base_damage += 2.6;
        }
        predator.attack_cooldown = 1.8 + predator.hunger;
        predator.hunger = if is_npc {
            (predator.hunger - 0.28).max(0.0)
        } else {
            (predator.hunger - 0.55).max(0.0)
        };
        predator.satiated_days = if is_npc { 0.55 } else { 1.35 };

        if is_npc {
            if let Ok((_, _, mut npc, intent, mut condition)) = npcs.get_mut(target) {
                let damage = if intent.label == "Flee" || intent.label == "Retreat" {
                    base_damage * 0.65
                } else {
                    base_damage
                };
                npc.health = (npc.health - damage).max(0.0);
                condition.last_damage_reason = format!(
                    "was mauled by a predator while {}",
                    intent.label.to_lowercase()
                );
                condition.last_damage_day = step.elapsed_days;
                if intent.label == "Defend" {
                    let retaliation = if predator.ability == PredatorAbility::Thornhide {
                        1.2
                    } else {
                        2.4
                    };
                    predator.health = (predator.health - retaliation).max(0.0);
                }
                if predator.ability == PredatorAbility::DreadHowl {
                    npc.exposure = (npc.exposure + 0.08).min(1.0);
                }

                if step.elapsed_days - predator.last_log_day > 0.35 {
                    writer.write(LogEvent::new(
                        LogEventKind::Threat,
                        format!(
                            "Predator mauled {}{}",
                            npc.name,
                            if predator.ability != PredatorAbility::None {
                                format!(" with {}", predator.ability.label())
                            } else {
                                String::new()
                            }
                        ),
                    ));
                    predator.last_log_day = step.elapsed_days;
                }
            }
        } else if let Ok((_, _, mut animal)) = animals.get_mut(target) {
            animal.health = (animal.health - base_damage * 0.9).max(0.0);
        }
    }
}

fn escalate_predator_pressure(
    mut commands: Commands,
    step: Res<SimulationStep>,
    settings: Res<MapSettings>,
    stats: Res<WorldStats>,
    evolution: Option<Res<EvolutionPressure>>,
    mut pressure: ResMut<PredatorPressure>,
    predators: Query<Entity, With<Predator>>,
    tiles: Query<(&RegionTile, &Transform)>,
    mut writer: MessageWriter<LogEvent>,
) {
    let flourishing = evolution
        .as_ref()
        .map(|pressure| {
            (pressure.survival_fitness + pressure.community_fitness + pressure.happiness_fitness)
                / 3.0
        })
        .unwrap_or(0.5);
    let minimum_pressure = if stats.npcs <= 3 || flourishing < 0.34 {
        1
    } else {
        3
    };
    let desired =
        (minimum_pressure + stats.shelters / 3 + stats.civic_structures / 4 + stats.npcs / 10)
            .clamp(minimum_pressure, 10);
    let current = predators.iter().count();
    if current >= desired || step.elapsed_days - pressure.last_spawn_day < 18.0 {
        return;
    }

    let mut candidates = tiles
        .iter()
        .filter(|(tile, _)| tile.mana_density > 0.58 && tile.soil_fertility < 0.75)
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return;
    }
    candidates.sort_by(|(a, _), (b, _)| {
        (b.mana_density + (1.0 - b.soil_fertility) * 0.25)
            .total_cmp(&(a.mana_density + (1.0 - a.soil_fertility) * 0.25))
    });
    let spawn_count = (desired - current).clamp(1, 2);
    for idx in 0..spawn_count {
        let (tile, transform) = candidates[idx % candidates.len()];
        let offset = Vec2::new((idx as f32 * 1.7).cos(), (idx as f32 * 1.7).sin())
            * settings.tile_size
            * 0.22;
        commands.spawn(PredatorBundle::new(
            transform.translation.truncate() + offset,
            (tile.mana_density * 0.73 + idx as f32 * 0.11).fract(),
        ));
    }
    pressure.last_spawn_day = step.elapsed_days;
    writer.write(LogEvent::new(
        LogEventKind::Threat,
        format!("New predators entered the region ({spawn_count})"),
    ));
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
