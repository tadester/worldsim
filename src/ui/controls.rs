use bevy::prelude::*;

use crate::systems::simulation::{SimulationClock, SimulationStep};
use crate::ui::{
    DiagnosticsLogPane, DiagnosticsNpcDeathPane, DiagnosticsSettingsPane, GameMenuRoot,
};
use crate::world::resources::WorldStats;

#[derive(Component)]
struct FooterText;

#[derive(Component)]
struct ToggleButton {
    target: ToggleTarget,
}

#[derive(Component)]
struct ToggleButtonText {
    target: ToggleTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToggleTarget {
    Settings,
    Logs,
    NpcDeaths,
}

#[derive(Resource, Debug, Clone, Copy)]
struct DiagnosticsPanelState {
    settings_visible: bool,
    logs_visible: bool,
    npc_deaths_visible: bool,
}

impl Default for DiagnosticsPanelState {
    fn default() -> Self {
        Self {
            settings_visible: true,
            logs_visible: true,
            npc_deaths_visible: true,
        }
    }
}

pub struct ControlsUiPlugin;

impl Plugin for ControlsUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DiagnosticsPanelState>()
            .add_systems(PostStartup, (spawn_game_menu_toolbar, spawn_game_footer))
            .add_systems(
                Update,
                (
                    toggle_diagnostics_panels,
                    sync_diagnostics_panel_visibility,
                    update_toggle_button_text,
                    update_footer_text,
                ),
            );
    }
}

fn spawn_game_menu_toolbar(mut commands: Commands, game_menu_root: Res<GameMenuRoot>) {
    commands.entity(game_menu_root.0).with_children(|parent| {
        parent
            .spawn((
                Node {
                    width: percent(100.0),
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: px(10.0),
                    padding: UiRect::axes(px(10.0), px(8.0)),
                    border: UiRect::all(px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.07, 0.07, 0.09, 0.92)),
                BorderColor::all(Color::srgba(0.26, 0.30, 0.36, 0.85)),
            ))
            .with_children(|row| {
                spawn_toggle_button(row, ToggleTarget::Settings, "Settings: On");
                spawn_toggle_button(row, ToggleTarget::Logs, "Logs: On");
                spawn_toggle_button(row, ToggleTarget::NpcDeaths, "NPC Death Log: On");
            });
    });
}

fn spawn_toggle_button(parent: &mut ChildSpawnerCommands<'_>, target: ToggleTarget, label: &str) {
    parent
        .spawn((
            Button,
            Node {
                padding: UiRect::axes(px(12.0), px(8.0)),
                border: UiRect::all(px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.16, 0.19, 0.24, 0.96)),
            BorderColor::all(Color::srgba(0.36, 0.42, 0.52, 0.92)),
            ToggleButton { target },
        ))
        .with_child((
            Text::new(label),
            TextFont::from_font_size(14.0),
            TextColor(Color::srgb(0.9, 0.94, 0.98)),
            ToggleButtonText { target },
        ));
}

fn spawn_game_footer(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: px(0.0),
                right: px(0.0),
                bottom: px(0.0),
                min_height: px(78.0),
                padding: UiRect::axes(px(16.0), px(12.0)),
                border: UiRect::top(px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.02, 0.03, 0.96)),
            BorderColor::all(Color::srgba(0.24, 0.27, 0.32, 0.9)),
            ZIndex(20),
        ))
        .with_child((
            Text::new("World controls"),
            TextFont::from_font_size(15.0),
            TextColor(Color::srgb(0.85, 0.89, 0.94)),
            FooterText,
        ));
}

fn toggle_diagnostics_panels(
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor, &ToggleButton),
        (Changed<Interaction>, With<Button>),
    >,
    mut state: ResMut<DiagnosticsPanelState>,
) {
    for (interaction, mut background, button) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                match button.target {
                    ToggleTarget::Settings => {
                        state.settings_visible = !state.settings_visible;
                    }
                    ToggleTarget::Logs => {
                        state.logs_visible = !state.logs_visible;
                    }
                    ToggleTarget::NpcDeaths => {
                        state.npc_deaths_visible = !state.npc_deaths_visible;
                    }
                }
                background.0 = Color::srgba(0.26, 0.32, 0.40, 0.98);
            }
            Interaction::Hovered => {
                background.0 = Color::srgba(0.22, 0.27, 0.34, 0.98);
            }
            Interaction::None => {
                background.0 = Color::srgba(0.16, 0.19, 0.24, 0.96);
            }
        }
    }
}

fn sync_diagnostics_panel_visibility(
    state: Res<DiagnosticsPanelState>,
    settings_pane: Res<DiagnosticsSettingsPane>,
    log_pane: Res<DiagnosticsLogPane>,
    npc_death_pane: Res<DiagnosticsNpcDeathPane>,
    mut visibilities: Query<&mut Visibility>,
) {
    if !state.is_changed() {
        return;
    }

    if let Ok(mut visibility) = visibilities.get_mut(settings_pane.0) {
        *visibility = if state.settings_visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    if let Ok(mut visibility) = visibilities.get_mut(log_pane.0) {
        *visibility = if state.logs_visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    if let Ok(mut visibility) = visibilities.get_mut(npc_death_pane.0) {
        *visibility = if state.npc_deaths_visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn update_toggle_button_text(
    state: Res<DiagnosticsPanelState>,
    mut texts: Query<(&ToggleButtonText, &mut Text)>,
) {
    if !state.is_changed() {
        return;
    }

    for (button, mut text) in &mut texts {
        let label = match button.target {
            ToggleTarget::Settings => {
                if state.settings_visible {
                    "Settings: On"
                } else {
                    "Settings: Off"
                }
            }
            ToggleTarget::Logs => {
                if state.logs_visible {
                    "Logs: On"
                } else {
                    "Logs: Off"
                }
            }
            ToggleTarget::NpcDeaths => {
                if state.npc_deaths_visible {
                    "NPC Death Log: On"
                } else {
                    "NPC Death Log: Off"
                }
            }
        };
        *text = Text::new(label);
    }
}

fn update_footer_text(
    clock: Res<SimulationClock>,
    step: Res<SimulationStep>,
    stats: Res<WorldStats>,
    mut texts: Query<&mut Text, With<FooterText>>,
) {
    for mut text in &mut texts {
        *text = Text::new(format!(
            "Time day {:.1} | Speed {}{} | Space pause | 1 = 1x | 2 = 5x | 3 = 20x | 4 = 120x | 5 = 300x | 6 = 900x | Tab = cycle entity | Animals {} | Trees {} | NPCs {} | Predators {} | Shelters {}",
            step.elapsed_days,
            clock.speed_label(),
            if clock.paused { " (paused)" } else { "" },
            stats.animals,
            stats.trees,
            stats.npcs,
            stats.predators,
            stats.shelters,
        ));
    }
}
