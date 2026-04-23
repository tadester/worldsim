pub mod climate;
pub mod director;
pub mod map;
pub mod proposals;
pub mod resources;
pub mod spawning;
pub mod territory;

use bevy::prelude::*;
use climate::ClimatePlugin;
use director::WorldDirectorPlugin;
use map::{MapPlugin, MapSettings};
use proposals::WorldProposalPlugin;
use resources::WorldResourcesPlugin;
use spawning::WorldSpawningPlugin;
use territory::TerritoryPlugin;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MapSettings>().add_plugins((
            MapPlugin,
            ClimatePlugin,
            WorldResourcesPlugin,
            TerritoryPlugin,
            WorldSpawningPlugin,
            WorldDirectorPlugin,
            WorldProposalPlugin,
        ));
    }
}
