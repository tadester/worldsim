use bevy::prelude::*;

use crate::systems::simulation::SimulationClock;

pub struct TimeSkipPlugin;

impl Plugin for TimeSkipPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, keyboard_time_controls);
    }
}

fn keyboard_time_controls(keys: Res<ButtonInput<KeyCode>>, mut clock: ResMut<SimulationClock>) {
    if keys.just_pressed(KeyCode::Space) {
        clock.paused = !clock.paused;
    }

    if keys.just_pressed(KeyCode::Digit1) {
        clock.steps_per_frame = 1;
        clock.paused = false;
    }

    if keys.just_pressed(KeyCode::Digit2) {
        clock.steps_per_frame = 5;
        clock.paused = false;
    }

    if keys.just_pressed(KeyCode::Digit3) {
        clock.steps_per_frame = 20;
        clock.paused = false;
    }

    if keys.just_pressed(KeyCode::Digit4) {
        clock.steps_per_frame = 120;
        clock.paused = false;
    }

    if keys.just_pressed(KeyCode::Digit5) {
        clock.steps_per_frame = 300;
        clock.paused = false;
    }

    if keys.just_pressed(KeyCode::Digit6) {
        clock.steps_per_frame = 900;
        clock.paused = false;
    }
}
