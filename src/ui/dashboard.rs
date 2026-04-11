use bevy::prelude::*;

use crate::systems::logging::EventLog;
use crate::systems::simulation::{SimulationClock, SimulationStep};
use crate::world::resources::WorldStats;

#[derive(Component)]
struct DashboardText;

pub struct DashboardPlugin;

impl Plugin for DashboardPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_dashboard)
            .add_systems(Update, update_dashboard_text);
    }
}

fn spawn_dashboard(mut commands: Commands) {
    commands.spawn((
        Text::new("WorldSim dashboard"),
        TextFont::from_font_size(16.0),
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: px(12.0),
            left: px(12.0),
            ..default()
        },
        DashboardText,
    ));
}

fn update_dashboard_text(
    stats: Res<WorldStats>,
    clock: Res<SimulationClock>,
    step: Res<SimulationStep>,
    log: Res<EventLog>,
    mut text_query: Query<&mut Text, With<DashboardText>>,
) {
    let latest = log
        .entries
        .last()
        .cloned()
        .unwrap_or_else(|| "No events yet".to_string());

    for mut text in &mut text_query {
        *text = Text::new(format!(
            "Ticks: {}\nDays: {:.2}\nSpeed: {}{}\nTrees: {}\nAnimals: {}\nNPCs: {}\nAvg mana: {:.2}\nAvg animal cap: {:.2}\nAvg tree cap: {:.2}\nAvg temp: {:.2}\nForage: {:.1}\nTree biomass: {:.1}\nLatest: {}",
            step.tick,
            step.elapsed_days,
            clock.speed_label(),
            if clock.paused { " (paused)" } else { "" },
            stats.trees,
            stats.animals,
            stats.npcs,
            stats.avg_mana_density,
            stats.avg_animal_capacity,
            stats.avg_tree_capacity,
            stats.avg_temperature,
            stats.total_forage,
            stats.total_tree_biomass,
            latest
        ));
    }
}
