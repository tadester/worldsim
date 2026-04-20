use bevy::prelude::*;

use crate::systems::logging::{EventLog, LogEventKind, NpcDeathLog};
use crate::ui::{DiagnosticsLogPane, DiagnosticsNpcDeathPane, DiagnosticsUiCamera};

#[derive(Component)]
struct EventLogText;

#[derive(Component)]
struct NpcDeathLogText;

pub struct LogPanelPlugin;

impl Plugin for LogPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, spawn_log_panel)
            .add_systems(Update, (update_log_panel, update_npc_death_log_panel));
    }
}

fn spawn_log_panel(
    mut commands: Commands,
    diagnostics_camera: Res<DiagnosticsUiCamera>,
    log_pane: Res<DiagnosticsLogPane>,
    npc_death_pane: Res<DiagnosticsNpcDeathPane>,
) {
    commands.entity(log_pane.0).with_children(|parent| {
        parent
            .spawn((
                Node {
                    width: percent(100.0),
                    padding: UiRect::axes(px(14.0), px(12.0)),
                    border: UiRect::all(px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.10, 0.08, 0.08, 0.94)),
                BorderColor::all(Color::srgba(0.42, 0.24, 0.20, 0.86)),
                UiTargetCamera(diagnostics_camera.0),
            ))
            .with_child((
                Text::new("Event log"),
                TextFont::from_font_size(14.0),
                TextColor(Color::srgb(0.95, 0.95, 0.95)),
                EventLogText,
            ));
    });

    commands.entity(npc_death_pane.0).with_children(|parent| {
        parent
            .spawn((
                Node {
                    width: percent(100.0),
                    padding: UiRect::axes(px(14.0), px(12.0)),
                    border: UiRect::all(px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.08, 0.05, 0.05, 0.94)),
                BorderColor::all(Color::srgba(0.48, 0.18, 0.18, 0.88)),
                UiTargetCamera(diagnostics_camera.0),
            ))
            .with_child((
                Text::new("NPC death log"),
                TextFont::from_font_size(14.0),
                TextColor(Color::srgb(0.98, 0.92, 0.92)),
                NpcDeathLogText,
            ));
    });
}

fn update_log_panel(log: Res<EventLog>, mut query: Query<&mut Text, With<EventLogText>>) {
    let lines = {
        let start = log.entries.len().saturating_sub(12);
        let mut out = String::new();
        for (index, entry) in log.entries[start..].iter().enumerate() {
            if index > 0 {
                out.push('\n');
            }
            use std::fmt::Write;
            let _ = write!(
                out,
                "[{:.1}] {} {}",
                entry.day,
                short_label(entry.kind),
                entry.message
            );
        }
        out
    };
    let display = if lines.is_empty() {
        "No events yet".to_string()
    } else {
        lines
    };

    for mut text in &mut query {
        *text = Text::new(format!("Event Log\n{}\n", display));
    }
}

fn update_npc_death_log_panel(
    log: Res<NpcDeathLog>,
    mut query: Query<&mut Text, With<NpcDeathLogText>>,
) {
    let lines = {
        let start = log.entries.len().saturating_sub(14);
        let mut out = String::new();
        for (index, entry) in log.entries[start..].iter().enumerate() {
            if index > 0 {
                out.push('\n');
            }
            use std::fmt::Write;
            let _ = write!(
                out,
                "[{:.1}] {} - {}",
                entry.day, entry.npc_name, entry.reason
            );
        }
        out
    };
    let display = if lines.is_empty() {
        "No NPC deaths yet".to_string()
    } else {
        lines
    };

    for mut text in &mut query {
        *text = Text::new(format!("NPC Death Log\n{}\n", display));
    }
}

fn short_label(kind: LogEventKind) -> &'static str {
    match kind {
        LogEventKind::Birth => "+",
        LogEventKind::Death => "-",
        LogEventKind::Discovery => "*",
        LogEventKind::Construction => "#",
        LogEventKind::Territory => "@",
        LogEventKind::Threat => "!",
        LogEventKind::Climate => "~",
    }
}
