use bevy::prelude::*;

use crate::systems::logging::{EventLog, LogEventKind};

#[derive(Component)]
struct EventLogText;

pub struct LogPanelPlugin;

impl Plugin for LogPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_log_panel)
            .add_systems(Update, update_log_panel);
    }
}

fn spawn_log_panel(mut commands: Commands) {
    commands.spawn((
        Text::new("Event log"),
        TextFont::from_font_size(14.0),
        TextColor(Color::srgb(0.95, 0.95, 0.95)),
        Node {
            position_type: PositionType::Absolute,
            top: px(12.0),
            right: px(12.0),
            width: px(350.0),
            ..default()
        },
        EventLogText,
    ));
}

fn update_log_panel(log: Res<EventLog>, mut query: Query<&mut Text, With<EventLogText>>) {
    let lines = log
        .entries
        .iter()
        .rev()
        .take(12)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|entry| {
            format!(
                "[{:.1}] {} {}",
                entry.day,
                short_label(entry.kind),
                entry.message
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
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
    }
}
