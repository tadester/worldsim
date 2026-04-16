use bevy::prelude::*;

use crate::ui::DiagnosticsUiCamera;

#[derive(Component)]
struct ControlsText;

pub struct ControlsUiPlugin;

impl Plugin for ControlsUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, spawn_controls_hint);
    }
}

fn spawn_controls_hint(mut commands: Commands, diagnostics_camera: Res<DiagnosticsUiCamera>) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(286.0),
            left: px(12.0),
            width: px(420.0),
            padding: UiRect::axes(px(14.0), px(10.0)),
            border: UiRect::all(px(1.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.07, 0.07, 0.09, 0.92)),
        BorderColor::all(Color::srgba(0.26, 0.30, 0.36, 0.85)),
        UiTargetCamera(diagnostics_camera.0),
    ))
    .with_child((
        Text::new("Space pause | 1 = 1x | 2 = 5x | 3 = 20x | 4 = hard skip | Tab = cycle entity"),
        TextFont::from_font_size(14.0),
        TextColor(Color::srgb(0.8, 0.86, 0.95)),
        ControlsText,
    ));
}
