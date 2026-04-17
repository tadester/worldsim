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
pub struct DiagnosticsUiRoot(pub Entity);

#[derive(Resource, Clone, Copy)]
pub struct DiagnosticsSettingsPane(pub Entity);

#[derive(Resource, Clone, Copy)]
pub struct DiagnosticsLogPane(pub Entity);

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

    let settings_pane = commands
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

    commands
        .entity(diagnostics_root)
        .add_children(&[settings_pane, log_pane]);

    commands.insert_resource(DiagnosticsUiCamera(diagnostics_camera));
    commands.insert_resource(DiagnosticsUiRoot(diagnostics_root));
    commands.insert_resource(DiagnosticsSettingsPane(settings_pane));
    commands.insert_resource(DiagnosticsLogPane(log_pane));
}
