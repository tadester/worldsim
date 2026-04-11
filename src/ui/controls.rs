use bevy::prelude::*;

#[derive(Component)]
struct ControlsText;

pub struct ControlsUiPlugin;

impl Plugin for ControlsUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_controls_hint);
    }
}

fn spawn_controls_hint(mut commands: Commands) {
    commands.spawn((
        Text::new("Space pause | 1 = 1x | 2 = 5x | 3 = 20x | 4 = hard skip | Tab = cycle entity"),
        TextFont::from_font_size(14.0),
        TextColor(Color::srgb(0.8, 0.86, 0.95)),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12.0),
            left: px(12.0),
            ..default()
        },
        ControlsText,
    ));
}
