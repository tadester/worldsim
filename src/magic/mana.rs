use bevy::prelude::*;

use crate::agents::decisions::NpcIntent;
use crate::agents::factions::FactionMember;
use crate::agents::memory::Memory;
use crate::agents::needs::Needs;
use crate::agents::npc::Npc;
use crate::agents::predator::Predator;
use crate::magic::storage::{ManaAction, ManaPractice, ManaStorageStyle};
use crate::systems::logging::{LogEvent, LogEventKind};
use crate::systems::simulation::SimulationClock;
use crate::world::map::{MapSettings, RegionTile};
use crate::world::resources::Campfire;
use crate::world::resources::spawn_transient_effect;

#[derive(Component, Debug, Clone, Copy)]
pub struct ManaReservoir {
    pub capacity: f32,
    pub stored: f32,
    pub stability: f32,
}

pub struct ManaPlugin;

impl Plugin for ManaPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                ambient_mana_drift,
                choose_npc_mana_actions,
                resolve_npc_mana_actions.after(choose_npc_mana_actions),
                resolve_npc_spellcasting.after(resolve_npc_mana_actions),
            ),
        );
    }
}

fn ambient_mana_drift(
    clock: Res<SimulationClock>,
    tiles: Query<&RegionTile>,
    mut reservoirs: Query<&mut ManaReservoir>,
) {
    let (sum, count) = tiles.iter().fold((0.0f32, 0usize), |acc, tile| {
        (acc.0 + tile.mana_density, acc.1 + 1)
    });
    let avg_tile_mana = sum / count.max(1) as f32;
    let delta_days = clock.delta_days();

    for mut reservoir in &mut reservoirs {
        let pull = (avg_tile_mana * reservoir.capacity * 0.02) * delta_days;
        reservoir.stored = (reservoir.stored + pull).clamp(0.0, reservoir.capacity);
        reservoir.stability = (reservoir.stability - delta_days * 0.001).max(0.35);
    }
}

fn choose_npc_mana_actions(
    mut npcs: Query<(
        &Npc,
        &Needs,
        &ManaReservoir,
        &ManaStorageStyle,
        &mut ManaPractice,
    )>,
) {
    for (npc, needs, reservoir, style, mut practice) in &mut npcs {
        let fill_ratio = if reservoir.capacity <= 0.0 {
            0.0
        } else {
            reservoir.stored / reservoir.capacity
        };

        let action = if fill_ratio < 0.25 {
            ManaAction::Absorb
        } else if reservoir.stability < 0.5 || needs.safety < 0.35 {
            ManaAction::Distribute
        } else if needs.curiosity * npc.curiosity > 0.45
            && style.concentration >= style.distribution
            && fill_ratio > 0.35
        {
            ManaAction::Concentrate
        } else if style.circulation > style.distribution && fill_ratio > 0.3 {
            ManaAction::Circulate
        } else if fill_ratio > 0.7 && practice.experimentation_drive > 0.45 {
            ManaAction::Release
        } else {
            ManaAction::Hold
        };

        practice.current_action = action;
    }
}

fn resolve_npc_mana_actions(
    clock: Res<SimulationClock>,
    settings: Res<MapSettings>,
    tiles: Query<&RegionTile>,
    mut writer: MessageWriter<LogEvent>,
    mut npcs: Query<(
        &Npc,
        &Transform,
        &mut ManaReservoir,
        &ManaStorageStyle,
        &mut ManaPractice,
        &mut Needs,
        &mut Memory,
    )>,
) {
    let delta_days = clock.delta_days();

    for (npc, transform, mut reservoir, style, mut practice, mut needs, mut memory) in &mut npcs {
        practice.spell_cooldown = (practice.spell_cooldown - delta_days).max(0.0);
        let coord = settings.tile_coord_for_position(transform.translation.truncate());
        let ambient = tiles
            .iter()
            .find(|tile| tile.coord == coord)
            .map(|tile| tile.mana_density)
            .unwrap_or(0.25);

        if practice.current_action != practice.last_action {
            memory.notable_events.push(format!(
                "{} shifted mana practice to {:?}",
                npc.name, practice.current_action
            ));
            practice.last_action = practice.current_action;
        }

        match practice.current_action {
            ManaAction::Absorb => {
                let gain = ambient
                    * (0.22 + style.distribution * 0.18 + style.circulation * 0.1)
                    * delta_days;
                reservoir.stored = (reservoir.stored + gain).clamp(0.0, reservoir.capacity);
                reservoir.stability = (reservoir.stability - gain * 0.02).clamp(0.2, 1.0);
                practice.control = (practice.control + delta_days * 0.01).min(1.0);
            }
            ManaAction::Hold => {
                reservoir.stability = (reservoir.stability + delta_days * 0.015).min(1.0);
                practice.control = (practice.control + delta_days * 0.004).min(1.0);
            }
            ManaAction::Circulate => {
                let reinforcement = reservoir.stored.min(0.18 * delta_days);
                reservoir.stored = (reservoir.stored - reinforcement * 0.35).max(0.0);
                reservoir.stability = (reservoir.stability
                    + reinforcement * (0.10 + style.circulation * 0.12))
                    .min(1.0);
                needs.fatigue = (needs.fatigue - reinforcement * 0.6).max(0.0);
                needs.safety = (needs.safety + reinforcement * 0.3).min(1.0);
            }
            ManaAction::Concentrate => {
                let gain = ambient * (0.12 + style.concentration * 0.2) * delta_days;
                reservoir.stored = (reservoir.stored + gain).clamp(0.0, reservoir.capacity);
                reservoir.stability = (reservoir.stability
                    - delta_days * (0.03 + style.concentration * 0.04))
                    .clamp(0.1, 1.0);
                practice.control = (practice.control + delta_days * 0.015).min(1.0);
                practice.experimentation_drive =
                    (practice.experimentation_drive + delta_days * 0.01).min(1.0);
            }
            ManaAction::Distribute => {
                let spend = reservoir.stored.min(0.20 * delta_days);
                reservoir.stored = (reservoir.stored - spend).max(0.0);
                reservoir.stability =
                    (reservoir.stability + spend * (0.12 + style.distribution * 0.08)).min(1.0);
                needs.safety = (needs.safety + spend * 0.8).min(1.0);
                needs.fatigue = (needs.fatigue - spend * 0.3).max(0.0);
            }
            ManaAction::Release => {
                let spend = reservoir.stored.min(0.24 * delta_days);
                reservoir.stored = (reservoir.stored - spend).max(0.0);
                reservoir.stability = (reservoir.stability
                    - spend * (0.10 + style.concentration * 0.10))
                    .clamp(0.05, 1.0);
                needs.curiosity = (needs.curiosity + spend * 0.5).min(1.0);
                practice.experimentation_drive =
                    (practice.experimentation_drive + spend * 0.2).min(1.0);
            }
        }

        let overcharge = if reservoir.capacity <= 0.0 {
            0.0
        } else {
            (reservoir.stored / reservoir.capacity - 0.85).max(0.0)
        };

        if reservoir.stability < 0.32 || overcharge > 0.0 {
            let backlash = (0.03 + overcharge * 0.25) * delta_days.max(0.01);
            practice.backlash += backlash;
            needs.safety = (needs.safety - backlash * 3.0).max(0.0);
            reservoir.stability = (reservoir.stability + backlash * 0.2).clamp(0.05, 1.0);

            if practice.backlash > 0.02 {
                let message = format!(
                    "{} suffered mana backlash while {:?}",
                    npc.name, practice.current_action
                );
                writer.write(LogEvent::new(LogEventKind::Discovery, message.clone()));
                memory.notable_events.push(message);
                practice.backlash = 0.0;
            }
        } else if practice.control > 0.55 && reservoir.stability > 0.7 {
            memory.last_decision = format!("Practicing {:?}", practice.current_action);
        }
    }
}

fn resolve_npc_spellcasting(
    mut commands: Commands,
    clock: Res<SimulationClock>,
    mut writer: MessageWriter<LogEvent>,
    mut npcs: ParamSet<(
        Query<(
            Entity,
            &Npc,
            &Transform,
            &NpcIntent,
            &mut ManaReservoir,
            &mut ManaPractice,
            &mut Needs,
            &mut Memory,
            Option<&FactionMember>,
        )>,
        Query<(
            Entity,
            &Transform,
            &mut Npc,
            &mut Needs,
            Option<&FactionMember>,
        )>,
    )>,
    mut predators: Query<(Entity, &Transform, &mut Predator)>,
    mut campfires: Query<(&Transform, &mut Campfire)>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }
    let npc_snapshots = npcs
        .p1()
        .iter_mut()
        .map(|(entity, transform, npc, needs, member)| {
            (
                entity,
                transform.translation.truncate(),
                npc.health,
                needs.safety,
                member.map(|m| m.faction),
            )
        })
        .collect::<Vec<_>>();
    let mut raid_bolt_plans = Vec::<(Entity, Vec2, f32)>::new();
    let mut heal_pulse_plans = Vec::<(Entity, Vec2, f32)>::new();

    for (
        entity,
        npc,
        transform,
        intent,
        mut reservoir,
        mut practice,
        mut needs,
        mut memory,
        member,
    ) in &mut npcs.p0()
    {
        if reservoir.stored <= 0.08 || practice.spell_cooldown > 0.0 {
            continue;
        }
        let pos = transform.translation.truncate();
        let faction = member.map(|m| m.faction);
        let combat_mode = matches!(intent.label.as_str(), "Hunt Predator" | "Raid");
        let support_mode = matches!(intent.label.as_str(), "Build Fire" | "Tend Fire" | "Rest");

        if practice.fireball >= 0.35 && combat_mode && reservoir.stored > 0.18 {
            if let Some((_, target_pos, mut predator)) = predators
                .iter_mut()
                .filter(|(_, other_transform, _)| {
                    pos.distance(other_transform.translation.truncate()) < 110.0
                })
                .min_by(|(_, a, _), (_, b, _)| {
                    pos.distance(a.translation.truncate())
                        .total_cmp(&pos.distance(b.translation.truncate()))
                })
            {
                let power = 5.5 + practice.fireball * 8.0;
                predator.health = (predator.health - power * delta_days * 12.0).max(0.0);
                predator.attack_cooldown += 0.2 + practice.fireball * 0.5;
                reservoir.stored -= 0.14;
                practice.spell_cooldown = 0.18;
                spawn_transient_effect(
                    &mut commands,
                    pos.lerp(target_pos.translation.truncate(), 0.65),
                    Color::srgba(0.98, 0.48, 0.16, 0.82),
                    Vec2::new(11.0, 11.0),
                    Vec2::new(0.0, 8.0),
                    0.18,
                    6.2,
                );
                memory.last_mana_insight = format!("{} cast Fireball", npc.name);
                writer.write(LogEvent::new(
                    LogEventKind::Threat,
                    format!("{} cast Fireball", npc.name),
                ));
                continue;
            }
        }

        if practice.gravity_well >= 0.35 && combat_mode && reservoir.stored > 0.16 {
            if let Some((_, target_pos, mut predator)) = predators
                .iter_mut()
                .filter(|(_, other_transform, _)| {
                    pos.distance(other_transform.translation.truncate()) < 90.0
                })
                .min_by(|(_, a, _), (_, b, _)| {
                    pos.distance(a.translation.truncate())
                        .total_cmp(&pos.distance(b.translation.truncate()))
                })
            {
                predator.attack_cooldown += 0.5 + practice.gravity_well * 1.0;
                predator.hunger = (predator.hunger + 0.02 + practice.gravity_well * 0.03).min(1.0);
                reservoir.stored -= 0.12;
                practice.spell_cooldown = 0.25;
                spawn_transient_effect(
                    &mut commands,
                    target_pos.translation.truncate(),
                    Color::srgba(0.56, 0.66, 0.98, 0.72),
                    Vec2::new(18.0, 18.0),
                    Vec2::new(0.0, 2.0),
                    0.18,
                    6.2,
                );
                writer.write(LogEvent::new(
                    LogEventKind::Threat,
                    format!("{} cast Gravity Well", npc.name),
                ));
                continue;
            }
        }

        if practice.root_snare >= 0.35 && combat_mode && reservoir.stored > 0.14 {
            if let Some((_, target_pos, mut predator)) = predators
                .iter_mut()
                .filter(|(_, other_transform, _)| {
                    pos.distance(other_transform.translation.truncate()) < 88.0
                })
                .min_by(|(_, a, _), (_, b, _)| {
                    pos.distance(a.translation.truncate())
                        .total_cmp(&pos.distance(b.translation.truncate()))
                })
            {
                predator.attack_cooldown += 0.35 + practice.root_snare * 0.8;
                predator.health = (predator.health
                    - (3.0 + practice.root_snare * 4.0) * delta_days * 10.0)
                    .max(0.0);
                reservoir.stored -= 0.10;
                practice.spell_cooldown = 0.20;
                spawn_transient_effect(
                    &mut commands,
                    target_pos.translation.truncate(),
                    Color::srgba(0.34, 0.86, 0.42, 0.76),
                    Vec2::new(16.0, 8.0),
                    Vec2::new(0.0, 4.0),
                    0.18,
                    6.2,
                );
                writer.write(LogEvent::new(
                    LogEventKind::Threat,
                    format!("{} cast Root Snare", npc.name),
                ));
                continue;
            }
        }

        if practice.mana_bolt >= 0.35 && combat_mode && reservoir.stored > 0.10 {
            if intent.label == "Raid" {
                if let Some((target_entity, target_pos, _, _, _)) = npc_snapshots
                    .iter()
                    .filter(|(other, other_pos, _, _, other_faction)| {
                        *other != entity
                            && pos.distance(*other_pos) < 95.0
                            && *other_faction != faction
                    })
                    .min_by(|(_, a, _, _, _), (_, b, _, _, _)| {
                        pos.distance(*a).total_cmp(&pos.distance(*b))
                    })
                {
                    raid_bolt_plans.push((
                        *target_entity,
                        *target_pos,
                        (4.0 + practice.mana_bolt * 6.0) * delta_days * 11.0,
                    ));
                    reservoir.stored -= 0.09;
                    practice.spell_cooldown = 0.14;
                    spawn_transient_effect(
                        &mut commands,
                        pos.lerp(*target_pos, 0.55),
                        Color::srgba(0.80, 0.36, 0.94, 0.78),
                        Vec2::new(8.0, 8.0),
                        Vec2::new(0.0, 6.0),
                        0.18,
                        6.2,
                    );
                    writer.write(LogEvent::new(
                        LogEventKind::Threat,
                        format!("{} cast Mana Bolt", npc.name),
                    ));
                    continue;
                }
            } else if let Some((_, target_pos, mut predator)) = predators
                .iter_mut()
                .filter(|(_, other_transform, _)| {
                    pos.distance(other_transform.translation.truncate()) < 105.0
                })
                .min_by(|(_, a, _), (_, b, _)| {
                    pos.distance(a.translation.truncate())
                        .total_cmp(&pos.distance(b.translation.truncate()))
                })
            {
                predator.health = (predator.health
                    - (4.2 + practice.mana_bolt * 5.8) * delta_days * 11.0)
                    .max(0.0);
                reservoir.stored -= 0.09;
                practice.spell_cooldown = 0.14;
                spawn_transient_effect(
                    &mut commands,
                    pos.lerp(target_pos.translation.truncate(), 0.55),
                    Color::srgba(0.80, 0.36, 0.94, 0.78),
                    Vec2::new(8.0, 8.0),
                    Vec2::new(0.0, 6.0),
                    0.18,
                    6.2,
                );
                writer.write(LogEvent::new(
                    LogEventKind::Threat,
                    format!("{} cast Mana Bolt", npc.name),
                ));
                continue;
            }
        }

        if practice.healing_pulse >= 0.35 && support_mode && reservoir.stored > 0.10 {
            let healing_targets = npc_snapshots
                .iter()
                .filter(|(other, other_pos, health, safety, other_faction)| {
                    *other != entity
                        && pos.distance(*other_pos) <= 46.0
                        && *other_faction == faction
                        && (*health < 54.0 || *safety < 0.55)
                })
                .map(|(other, other_pos, _, _, _)| (*other, *other_pos))
                .collect::<Vec<_>>();
            if !healing_targets.is_empty() {
                for (target, target_pos) in healing_targets {
                    heal_pulse_plans.push((
                        target,
                        target_pos,
                        (2.2 + practice.healing_pulse * 3.5) * delta_days * 9.0,
                    ));
                }
                reservoir.stored -= 0.08;
                practice.spell_cooldown = 0.20;
                spawn_transient_effect(
                    &mut commands,
                    pos,
                    Color::srgba(0.34, 0.92, 0.78, 0.72),
                    Vec2::new(20.0, 20.0),
                    Vec2::new(0.0, 3.0),
                    0.18,
                    6.2,
                );
                writer.write(LogEvent::new(
                    LogEventKind::Discovery,
                    format!("{} released a Healing Pulse", npc.name),
                ));
                continue;
            }
        }

        if practice.fireball >= 0.35 && support_mode && reservoir.stored > 0.06 {
            let mut kindled = false;
            for (fire_transform, mut campfire) in &mut campfires {
                if pos.distance(fire_transform.translation.truncate()) > 28.0 {
                    continue;
                }
                campfire.ember = (campfire.ember + 0.22 + practice.fireball * 0.22).min(1.0);
                campfire.fuel =
                    (campfire.fuel + 0.10 + practice.hearthspark * 0.08).min(campfire.max_fuel);
                kindled = true;
                break;
            }
            if kindled {
                reservoir.stored -= 0.05;
                practice.spell_cooldown = 0.08;
                spawn_transient_effect(
                    &mut commands,
                    pos + Vec2::new(6.0, 4.0),
                    Color::srgba(0.98, 0.56, 0.18, 0.80),
                    Vec2::new(7.0, 10.0),
                    Vec2::new(0.0, 10.0),
                    0.18,
                    6.2,
                );
                continue;
            }
        }

        if practice.stone_skin >= 0.35 && needs.safety < 0.50 && reservoir.stored > 0.08 {
            needs.safety = (needs.safety + 0.10 + practice.stone_skin * 0.08).min(1.0);
            reservoir.stored -= 0.07;
            practice.spell_cooldown = 0.22;
            spawn_transient_effect(
                &mut commands,
                pos,
                Color::srgba(0.64, 0.58, 0.52, 0.70),
                Vec2::new(16.0, 16.0),
                Vec2::new(0.0, 1.0),
                0.18,
                6.2,
            );
            continue;
        }

        if practice.windstep >= 0.35
            && matches!(
                intent.label.as_str(),
                "Flee" | "Retreat" | "Explore" | "Raid"
            )
            && reservoir.stored > 0.07
        {
            needs.fatigue = (needs.fatigue - 0.06 - practice.windstep * 0.05).max(0.0);
            reservoir.stored -= 0.06;
            practice.spell_cooldown = 0.10;
            spawn_transient_effect(
                &mut commands,
                pos,
                Color::srgba(0.66, 0.82, 0.98, 0.58),
                Vec2::new(14.0, 6.0),
                Vec2::new(0.0, 7.0),
                0.18,
                6.2,
            );
            continue;
        }
    }

    for (target, _target_pos, damage) in raid_bolt_plans {
        if let Ok((_, _, mut npc, mut needs, _)) = npcs.p1().get_mut(target) {
            npc.health = (npc.health - damage).max(0.0);
            needs.safety = (needs.safety - 0.06).max(0.0);
        }
    }
    for (target, target_pos, healing) in heal_pulse_plans {
        if let Ok((_, _, mut npc, mut needs, _)) = npcs.p1().get_mut(target) {
            npc.health = (npc.health + healing).min(100.0);
            needs.safety = (needs.safety + 0.10).min(1.0);
            spawn_transient_effect(
                &mut commands,
                target_pos,
                Color::srgba(0.34, 0.92, 0.78, 0.54),
                Vec2::new(10.0, 10.0),
                Vec2::new(0.0, 4.0),
                0.18,
                6.2,
            );
        }
    }
}
