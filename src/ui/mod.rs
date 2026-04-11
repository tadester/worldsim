pub mod controls;
pub mod dashboard;

use bevy::prelude::*;
use controls::ControlsUiPlugin;
use dashboard::DashboardPlugin;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((DashboardPlugin, ControlsUiPlugin));
    }
}
