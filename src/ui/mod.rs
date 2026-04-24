pub mod controls;
pub mod dashboard;
pub mod inspector;
pub mod log_panel;

use bevy::{
    camera::RenderTarget,
    prelude::*,
    window::{WindowRef, WindowResolution},
};
use controls::ControlsUiPlugin;
use dashboard::DashboardPlugin;
use inspector::InspectorPlugin;
use log_panel::LogPanelPlugin;

#[derive(Resource, Clone, Copy)]
pub struct DiagnosticsUiCamera(pub Entity);

#[derive(Resource, Clone, Copy)]
pub struct GameMenuRoot(pub Entity);

#[derive(Resource, Clone, Copy)]
pub struct DiagnosticsSettingsPane(pub Entity);

#[derive(Resource, Clone, Copy)]
pub struct DiagnosticsLogPane(pub Entity);

#[derive(Resource, Clone, Copy)]
pub struct DiagnosticsNpcDeathPane(pub Entity);

#[derive(Resource, Clone, Copy)]
pub struct DiagnosticsWorldLogPane(pub Entity);

#[derive(Resource, Clone, Copy)]
pub struct DiagnosticsWorldSuggestionsPane(pub Entity);

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_diagnostics_window)
            .add_plugins((
                DashboardPlugin,
                ControlsUiPlugin,
                InspectorPlugin,
                LogPanelPlugin,
            ));
    }
}

fn spawn_diagnostics_window(mut commands: Commands) {
    let diagnostics_window = commands
        .spawn(Window {
            title: "WorldSim Diagnostics".to_string(),
            resolution: WindowResolution::new(460, 860),
            position: WindowPosition::At(IVec2::new(1320, 40)),
            resizable: true,
            focused: false,
            ..default()
        })
        .id();

    let diagnostics_camera = commands
        .spawn((
            Camera2d,
            RenderTarget::Window(WindowRef::Entity(diagnostics_window)),
            Transform::from_xyz(100_000.0, 100_000.0, 999.0),
        ))
        .id();

    let diagnostics_root = commands
        .spawn((
            Node {
                width: percent(100.0),
                height: percent(100.0),
                padding: UiRect::all(px(12.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(12.0),
                ..default()
            },
            UiTargetCamera(diagnostics_camera),
        ))
        .id();

    let game_menu_root = commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(12.0),
                right: px(12.0),
                bottom: px(96.0),
                width: px(350.0),
                padding: UiRect::all(px(12.0)),
                flex_direction: FlexDirection::Column,
                row_gap: px(12.0),
                border: UiRect::all(px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.90)),
            BorderColor::all(Color::srgba(0.22, 0.28, 0.34, 0.85)),
            ZIndex(15),
        ))
        .id();

    let settings_pane = commands
        .spawn(Node {
            width: percent(100.0),
            flex_direction: FlexDirection::Column,
            row_gap: px(12.0),
            ..default()
        })
        .id();

    let log_pane = commands
        .spawn((
            Node {
                width: percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: px(12.0),
                ..default()
            },
            UiTargetCamera(diagnostics_camera),
        ))
        .id();

    let npc_death_pane = commands
        .spawn((
            Node {
                width: percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: px(12.0),
                ..default()
            },
            Visibility::Hidden,
            UiTargetCamera(diagnostics_camera),
        ))
        .id();

    let world_log_pane = commands
        .spawn((
            Node {
                width: percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: px(12.0),
                ..default()
            },
            Visibility::Hidden,
            UiTargetCamera(diagnostics_camera),
        ))
        .id();

    let world_suggestions_pane = commands
        .spawn((
            Node {
                width: percent(100.0),
                flex_direction: FlexDirection::Column,
                row_gap: px(12.0),
                ..default()
            },
            Visibility::Hidden,
            UiTargetCamera(diagnostics_camera),
        ))
        .id();

    commands.entity(diagnostics_root).add_children(&[
        log_pane,
        npc_death_pane,
        world_log_pane,
        world_suggestions_pane,
    ]);
    commands
        .entity(game_menu_root)
        .add_children(&[settings_pane]);

    commands.insert_resource(DiagnosticsUiCamera(diagnostics_camera));
    commands.insert_resource(GameMenuRoot(game_menu_root));
    commands.insert_resource(DiagnosticsSettingsPane(settings_pane));
    commands.insert_resource(DiagnosticsLogPane(log_pane));
    commands.insert_resource(DiagnosticsNpcDeathPane(npc_death_pane));
    commands.insert_resource(DiagnosticsWorldLogPane(world_log_pane));
    commands.insert_resource(DiagnosticsWorldSuggestionsPane(world_suggestions_pane));
}
