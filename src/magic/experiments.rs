use bevy::prelude::*;

use crate::agents::memory::Memory;
use crate::magic::mana::ManaReservoir;
use crate::magic::storage::{ManaAction, ManaPractice, ManaStorageStyle};
use crate::systems::logging::{LogEvent, LogEventKind};
use crate::world::map::{MapSettings, RegionTile};

pub struct ExperimentsPlugin;

impl Plugin for ExperimentsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (record_mana_bias, evolve_mana_preferences));
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
            }
            ManaAction::Circulate if stable_success => {
                style.circulation = (style.circulation + 0.0025).min(0.8);
                style.concentration = (style.concentration - 0.001).max(0.1);
            }
            ManaAction::Distribute if stable_success => {
                style.distribution = (style.distribution + 0.0025).min(0.8);
                style.concentration = (style.concentration - 0.001).max(0.1);
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
