pub mod logging;
pub mod simulation;
pub mod timeskip;

use bevy::prelude::*;
use logging::LoggingPlugin;
use simulation::SimulationCorePlugin;
use timeskip::TimeSkipPlugin;

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((SimulationCorePlugin, TimeSkipPlugin, LoggingPlugin));
    }
}
