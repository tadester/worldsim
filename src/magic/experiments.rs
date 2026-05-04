use bevy::prelude::*;

use crate::agents::memory::Memory;
use crate::agents::npc::Npc;
use crate::magic::mana::ManaReservoir;
use crate::magic::storage::{ManaAction, ManaDiscipline, ManaPractice, ManaStorageStyle};
use crate::systems::logging::{LogEvent, LogEventKind};
use crate::world::map::{MapSettings, RegionTile};

pub struct ExperimentsPlugin;

impl Plugin for ExperimentsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                record_mana_bias,
                evolve_mana_preferences,
                discover_mana_abilities.after(evolve_mana_preferences),
            ),
        );
    }
}

fn record_mana_bias(
    mut query: Query<(&ManaReservoir, &ManaStorageStyle, &mut Memory), Added<ManaReservoir>>,
    mut writer: MessageWriter<LogEvent>,
) {
    for (reservoir, style, mut memory) in &mut query {
        let pattern = dominant_pattern(*style);

        let note = if reservoir.stored > reservoir.capacity * 0.4 {
            format!("Spawned with notable mana affinity and a {pattern} storage bias")
        } else {
            format!("Spawned with low internal mana and a {pattern} storage bias")
        };

        memory.last_mana_insight = format!("Initial bias: {pattern}");
        writer.write(LogEvent::new(LogEventKind::Discovery, note.clone()));
        memory.notable_events.push(note);
    }
}

fn evolve_mana_preferences(
    settings: Res<MapSettings>,
    tiles: Query<&RegionTile>,
    mut writer: MessageWriter<LogEvent>,
    mut npcs: Query<(
        &Transform,
        &ManaReservoir,
        &mut ManaStorageStyle,
        &ManaPractice,
        &mut Memory,
    )>,
) {
    for (transform, reservoir, mut style, practice, mut memory) in &mut npcs {
        let coord = settings.tile_coord_for_position(transform.translation.truncate());
        let ambient = tiles
            .iter()
            .find(|tile| tile.coord == coord)
            .map(|tile| tile.mana_density)
            .unwrap_or(0.25);

        let prior_pattern = dominant_pattern(*style);
        let stable_success = reservoir.stability > 0.7 && practice.control > 0.45;
        let unstable = reservoir.stability < 0.4 || practice.backlash > 0.01;

        match practice.current_action {
            ManaAction::Concentrate if stable_success => {
                style.concentration = (style.concentration + 0.002 + ambient * 0.001).min(0.8);
                style.distribution = (style.distribution - 0.0015).max(0.1);
                practice_discipline_hint(&mut memory, ManaDiscipline::Hunt);
            }
            ManaAction::Circulate if stable_success => {
                style.circulation = (style.circulation + 0.0025).min(0.8);
                style.concentration = (style.concentration - 0.001).max(0.1);
                practice_discipline_hint(&mut memory, ManaDiscipline::Kinesis);
            }
            ManaAction::Distribute if stable_success => {
                style.distribution = (style.distribution + 0.0025).min(0.8);
                style.concentration = (style.concentration - 0.001).max(0.1);
                practice_discipline_hint(&mut memory, ManaDiscipline::Warding);
            }
            ManaAction::Release if unstable => {
                style.concentration = (style.concentration - 0.002).max(0.1);
                style.distribution = (style.distribution + 0.001).min(0.8);
            }
            ManaAction::Absorb if unstable => {
                style.distribution = (style.distribution + 0.0015).min(0.8);
                style.circulation = (style.circulation + 0.001).min(0.8);
            }
            _ => {}
        }

        normalize_style(&mut style);
        let new_pattern = dominant_pattern(*style);

        if new_pattern != prior_pattern {
            let insight = format!("Mana preference shifted from {prior_pattern} to {new_pattern}");
            if memory.last_mana_insight != insight {
                memory.last_mana_insight = insight.clone();
                memory.notable_events.push(insight.clone());
                writer.write(LogEvent::new(LogEventKind::Discovery, insight));
            }
        }
    }
}

fn dominant_pattern(style: ManaStorageStyle) -> &'static str {
    if style.concentration >= style.circulation && style.concentration >= style.distribution {
        "concentrated"
    } else if style.circulation >= style.distribution {
        "circulating"
    } else {
        "distributed"
    }
}

fn normalize_style(style: &mut ManaStorageStyle) {
    let total = style.concentration + style.circulation + style.distribution;
    if total <= f32::EPSILON {
        *style = ManaStorageStyle::default();
        return;
    }

    style.concentration /= total;
    style.circulation /= total;
    style.distribution /= total;
}

fn discover_mana_abilities(
    settings: Res<MapSettings>,
    tiles: Query<&RegionTile>,
    mut writer: MessageWriter<LogEvent>,
    mut npcs: Query<(
        &Npc,
        &Transform,
        &ManaReservoir,
        &ManaStorageStyle,
        &mut ManaPractice,
        &mut Memory,
    )>,
) {
    for (npc, transform, reservoir, style, mut practice, mut memory) in &mut npcs {
        let coord = settings.tile_coord_for_position(transform.translation.truncate());
        let ambient = tiles
            .iter()
            .find(|tile| tile.coord == coord)
            .map(|tile| tile.mana_density)
            .unwrap_or(0.25);
        let stable = reservoir.stability > 0.62 && practice.control > 0.42;
        let catalyst =
            practice.experimentation_drive * 0.45 + npc.discovery_drive * 0.30 + ambient * 0.25;
        if !stable || catalyst < 0.44 {
            continue;
        }

        let (discipline, label) = if style.circulation >= style.concentration
            && style.circulation >= style.distribution
        {
            (ManaDiscipline::Kinesis, "Telekinesis")
        } else if style.distribution >= style.concentration && ambient > 0.62 {
            (ManaDiscipline::Verdant, "Verdant Touch")
        } else if style.distribution >= style.circulation
            && practice.current_action == ManaAction::Distribute
        {
            (ManaDiscipline::Warding, "Warding")
        } else if ambient < 0.40 && practice.current_action == ManaAction::Absorb {
            (ManaDiscipline::Hearth, "Hearthspark")
        } else {
            (ManaDiscipline::Hunt, "Hunter Focus")
        };

        practice.discipline = discipline;
        let gain = (0.0018 + catalyst * 0.0024 + practice.control * 0.0015)
            * if ambient > 0.72 { 1.35 } else { 1.0 };
        let power = match discipline {
            ManaDiscipline::Kinesis => &mut practice.telekinesis,
            ManaDiscipline::Hearth => &mut practice.hearthspark,
            ManaDiscipline::Warding => &mut practice.warding,
            ManaDiscipline::Hunt => &mut practice.hunter_focus,
            ManaDiscipline::Verdant => &mut practice.verdant_touch,
        };
        let previous_tier = (*power * 3.0).floor() as i32;
        *power = (*power + gain).clamp(0.0, 1.0);
        let new_tier = (*power * 3.0).floor() as i32;
        if new_tier > previous_tier && *power >= 0.35 {
            let message = format!("{} discovered {} through mana research", npc.name, label);
            if memory.last_mana_insight != message {
                memory.last_mana_insight = message.clone();
                memory.notable_events.push(message.clone());
                writer.write(LogEvent::new(LogEventKind::Discovery, message));
            }
        }

        let spell_signal = catalyst
            + practice.control * 0.25
            + reservoir.stored / reservoir.capacity.max(1.0) * 0.15;
        if spell_signal > 0.64 {
            let (spell_label, spell_power) =
                if practice.telekinesis >= 0.35 && style.circulation > 0.42 && ambient > 0.56 {
                    if practice.windstep < 0.35 {
                        ("Windstep", &mut practice.windstep)
                    } else {
                        ("Gravity Well", &mut practice.gravity_well)
                    }
                } else if practice.hearthspark >= 0.35
                    && (ambient < 0.48 || practice.current_action == ManaAction::Release)
                {
                    ("Fireball", &mut practice.fireball)
                } else if practice.warding >= 0.35
                    && practice.current_action == ManaAction::Distribute
                {
                    if practice.healing_pulse < 0.35 {
                        ("Healing Pulse", &mut practice.healing_pulse)
                    } else {
                        ("Stone Skin", &mut practice.stone_skin)
                    }
                } else if practice.verdant_touch >= 0.35 && ambient > 0.60 {
                    ("Root Snare", &mut practice.root_snare)
                } else if practice.hunter_focus >= 0.35 {
                    ("Mana Bolt", &mut practice.mana_bolt)
                } else {
                    continue;
                };

            let previous_tier = (*spell_power * 3.0).floor() as i32;
            *spell_power = (*spell_power + (0.0012 + spell_signal * 0.0018 + ambient * 0.0010))
                .clamp(0.0, 1.0);
            let new_tier = (*spell_power * 3.0).floor() as i32;
            if new_tier > previous_tier && *spell_power >= 0.35 {
                let message = format!("{} invented the spell {}", npc.name, spell_label);
                if memory.last_mana_insight != message {
                    memory.last_mana_insight = message.clone();
                    memory.notable_events.push(message.clone());
                    writer.write(LogEvent::new(LogEventKind::Discovery, message));
                }
            }
        }
    }
}

fn practice_discipline_hint(memory: &mut Memory, discipline: ManaDiscipline) {
    let hint = format!("Their mana keeps favoring {}", discipline.label());
    if memory.last_mana_insight != hint {
        memory.last_mana_insight = hint;
    }
}
