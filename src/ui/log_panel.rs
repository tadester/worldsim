use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;

use crate::systems::logging::{EventLog, LogEventKind, NpcDeathLog};
use crate::ui::{
    DiagnosticsLogPane, DiagnosticsNpcDeathPane, DiagnosticsUiCamera, DiagnosticsWorldLogPane,
    DiagnosticsWorldSuggestionsPane,
};
use crate::world::proposals::{WorldActionLog, WorldProposalQueue};

#[derive(Component)]
struct EventLogText;

#[derive(Component)]
struct NpcDeathLogText;

#[derive(Component)]
struct WorldLogText;

#[derive(Component)]
struct WorldSuggestionsText;

#[derive(Component)]
struct ScrollArea;

pub struct LogPanelPlugin;

impl Plugin for LogPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, spawn_log_panel).add_systems(
            Update,
            (
                update_log_panel,
                update_npc_death_log_panel,
                update_world_log_panel,
                update_world_suggestions_panel,
                scroll_log_panels,
            ),
        );
    }
}

fn spawn_log_panel(
    mut commands: Commands,
    diagnostics_camera: Res<DiagnosticsUiCamera>,
    log_pane: Res<DiagnosticsLogPane>,
    npc_death_pane: Res<DiagnosticsNpcDeathPane>,
    world_log_pane: Res<DiagnosticsWorldLogPane>,
    world_suggestions_pane: Res<DiagnosticsWorldSuggestionsPane>,
) {
    spawn_scroll_panel(
        &mut commands,
        log_pane.0,
        diagnostics_camera.0,
        Color::srgba(0.10, 0.08, 0.08, 0.94),
        Color::srgba(0.42, 0.24, 0.20, 0.86),
        "Event log",
        EventLogText,
        Color::srgb(0.95, 0.95, 0.95),
    );
    spawn_scroll_panel(
        &mut commands,
        npc_death_pane.0,
        diagnostics_camera.0,
        Color::srgba(0.08, 0.05, 0.05, 0.94),
        Color::srgba(0.48, 0.18, 0.18, 0.88),
        "NPC death log",
        NpcDeathLogText,
        Color::srgb(0.98, 0.92, 0.92),
    );
    spawn_scroll_panel(
        &mut commands,
        world_log_pane.0,
        diagnostics_camera.0,
        Color::srgba(0.06, 0.09, 0.08, 0.94),
        Color::srgba(0.22, 0.44, 0.30, 0.88),
        "World log",
        WorldLogText,
        Color::srgb(0.90, 0.97, 0.92),
    );
    spawn_scroll_panel(
        &mut commands,
        world_suggestions_pane.0,
        diagnostics_camera.0,
        Color::srgba(0.09, 0.08, 0.05, 0.94),
        Color::srgba(0.52, 0.44, 0.18, 0.88),
        "WSPG",
        WorldSuggestionsText,
        Color::srgb(0.98, 0.95, 0.88),
    );
}

fn spawn_scroll_panel<T: Component>(
    commands: &mut Commands,
    parent_entity: Entity,
    camera: Entity,
    background: Color,
    border: Color,
    initial_text: &str,
    marker: T,
    text_color: Color,
) {
    commands.entity(parent_entity).with_children(|parent| {
        parent
            .spawn((
                Node {
                    width: percent(100.0),
                    max_height: px(220.0),
                    padding: UiRect::axes(px(14.0), px(12.0)),
                    border: UiRect::all(px(1.0)),
                    overflow: Overflow::clip_y(),
                    ..default()
                },
                BackgroundColor(background),
                BorderColor::all(border),
                UiTargetCamera(camera),
                ScrollPosition::default(),
                ScrollArea,
            ))
            .with_child((
                Text::new(initial_text),
                TextFont::from_font_size(14.0),
                TextColor(text_color),
                marker,
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

fn proposal_lines(proposals: &WorldProposalQueue) -> String {
    let start = proposals.proposals.len().saturating_sub(3);
    let mut out = String::new();
    for proposal in &proposals.proposals[start..] {
        if !out.is_empty() {
            out.push('\n');
        }
        use std::fmt::Write;
        let _ = write!(
            out,
            "[{:.1}] {}\nProblem: {}\nSolution: {}\nRequest: {}",
            proposal.day,
            proposal.title,
            proposal.problem,
            proposal.proposed_solution,
            proposal.request
        );
    }
    out
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

fn update_world_log_panel(
    log: Res<WorldActionLog>,
    mut query: Query<&mut Text, With<WorldLogText>>,
) {
    let lines = {
        let start = log.entries.len().saturating_sub(16);
        let mut out = String::new();
        for (index, entry) in log.entries[start..].iter().enumerate() {
            if index > 0 {
                out.push('\n');
            }
            use std::fmt::Write;
            let _ = write!(out, "[{:.1}] {}\n{}", entry.day, entry.title, entry.detail);
        }
        out
    };
    let display = if lines.is_empty() {
        "No world actions yet".to_string()
    } else {
        lines
    };
    for mut text in &mut query {
        *text = Text::new(format!("World Log\n{}\n", display));
    }
}

fn update_world_suggestions_panel(
    proposals: Res<WorldProposalQueue>,
    mut query: Query<&mut Text, With<WorldSuggestionsText>>,
) {
    let lines = proposal_lines(&proposals);
    let display = if lines.is_empty() {
        "No missing-system suggestions yet".to_string()
    } else {
        lines
    };
    for mut text in &mut query {
        *text = Text::new(format!(
            "World Suggestions To Program Into Game (WSPG)\n{}\n",
            display
        ));
    }
}

fn scroll_log_panels(
    mouse_wheel: Option<MessageReader<MouseWheel>>,
    mut areas: Query<&mut ScrollPosition, With<ScrollArea>>,
) {
    let Some(mut mouse_wheel) = mouse_wheel else {
        return;
    };
    let delta: f32 = mouse_wheel.read().map(|event| event.y * 24.0).sum();
    if delta == 0.0 {
        return;
    }

    for mut position in &mut areas {
        position.y = (position.y - delta).max(0.0);
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
        LogEventKind::Proposal => "?",
    }
}
