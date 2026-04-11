use bevy::prelude::*;

use crate::systems::logging::{EventLog, LogEventKind};
use crate::systems::simulation::{SimulationClock, SimulationStep};
use crate::world::resources::WorldStats;

#[derive(Component)]
struct DashboardText;

#[derive(Resource, Default)]
pub struct TrendHistory {
    pub samples: Vec<TrendSample>,
    pub sample_timer_days: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct TrendSample {
    pub day: f32,
    pub trees: usize,
    pub animals: usize,
    pub npcs: usize,
    pub births: usize,
    pub deaths: usize,
}

pub struct DashboardPlugin;

impl Plugin for DashboardPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TrendHistory>()
            .add_systems(Startup, spawn_dashboard)
            .add_systems(Update, (update_trend_history, update_dashboard_text));
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
    trends: Res<TrendHistory>,
    mut text_query: Query<&mut Text, With<DashboardText>>,
) {
    let latest = log
        .entries
        .last()
        .map(|entry| {
            format!(
                "[day {:.2}] {}: {}",
                entry.day,
                event_label(entry.kind),
                entry.message
            )
        })
        .unwrap_or_else(|| "No events yet".to_string());
    let trend_line = trends
        .samples
        .iter()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|sample| {
            format!(
                "{:.0}:{}/{}/{}",
                sample.day, sample.trees, sample.animals, sample.npcs
            )
        })
        .collect::<Vec<_>>()
        .join("  ");
    let recent_births = trends
        .samples
        .last()
        .map(|sample| sample.births)
        .unwrap_or(0);
    let recent_deaths = trends
        .samples
        .last()
        .map(|sample| sample.deaths)
        .unwrap_or(0);

    for mut text in &mut text_query {
        *text = Text::new(format!(
            "Ticks: {}\nDays: {:.2}\nSpeed: {}{}\nTrees: {}\nAnimals: {}\nNPCs: {}\nAvg mana: {:.2}\nAvg animal cap: {:.2}\nAvg tree cap: {:.2}\nAvg temp: {:.2}\nForage: {:.1}\nTree biomass: {:.1}\nRecent births: {}\nRecent deaths: {}\nTrend T/A/N: {}\nLatest: {}",
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
            recent_births,
            recent_deaths,
            if trend_line.is_empty() {
                "No samples yet"
            } else {
                &trend_line
            },
            latest
        ));
    }
}

fn update_trend_history(
    clock: Res<SimulationClock>,
    step: Res<SimulationStep>,
    stats: Res<WorldStats>,
    log: Res<EventLog>,
    mut trends: ResMut<TrendHistory>,
) {
    if clock.paused {
        return;
    }

    trends.sample_timer_days += clock.delta_days();
    if trends.sample_timer_days < 0.75 {
        return;
    }
    trends.sample_timer_days = 0.0;

    let mut births = 0usize;
    let mut deaths = 0usize;
    for entry in log.entries.iter().rev() {
        if step.elapsed_days - entry.day > 1.0 {
            break;
        }
        match entry.kind {
            LogEventKind::Birth => births += 1,
            LogEventKind::Death => deaths += 1,
            LogEventKind::Discovery => {}
        }
    }

    trends.samples.push(TrendSample {
        day: step.elapsed_days,
        trees: stats.trees,
        animals: stats.animals,
        npcs: stats.npcs,
        births,
        deaths,
    });

    if trends.samples.len() > 24 {
        let overflow = trends.samples.len() - 24;
        trends.samples.drain(0..overflow);
    }
}

fn event_label(kind: LogEventKind) -> &'static str {
    match kind {
        LogEventKind::Birth => "Birth",
        LogEventKind::Death => "Death",
        LogEventKind::Discovery => "Discovery",
    }
}
