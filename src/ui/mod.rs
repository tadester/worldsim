pub mod controls;
pub mod dashboard;
pub mod inspector;
pub mod log_panel;

use bevy::prelude::*;
use controls::ControlsUiPlugin;
use dashboard::DashboardPlugin;
use inspector::InspectorPlugin;
use log_panel::LogPanelPlugin;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            DashboardPlugin,
            ControlsUiPlugin,
            InspectorPlugin,
            LogPanelPlugin,
        ));
    }
}
