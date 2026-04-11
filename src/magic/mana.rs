use bevy::prelude::*;

use crate::agents::memory::Memory;
use crate::agents::needs::Needs;
use crate::agents::npc::Npc;
use crate::magic::storage::{ManaAction, ManaPractice, ManaStorageStyle};
use crate::systems::logging::{LogEvent, LogEventKind};
use crate::systems::simulation::SimulationClock;
use crate::world::map::{MapSettings, RegionTile};

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
