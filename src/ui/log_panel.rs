use bevy::prelude::*;

use crate::systems::logging::{EventLog, LogEventKind};
use crate::ui::DiagnosticsUiCamera;

#[derive(Component)]
struct EventLogText;

pub struct LogPanelPlugin;

impl Plugin for LogPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, spawn_log_panel)
            .add_systems(Update, update_log_panel);
    }
}

fn spawn_log_panel(mut commands: Commands, diagnostics_camera: Res<DiagnosticsUiCamera>) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(346.0),
            left: px(12.0),
            width: px(420.0),
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
