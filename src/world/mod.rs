pub mod map;
pub mod resources;
pub mod spawning;

use bevy::prelude::*;
use map::{MapPlugin, MapSettings};
use resources::WorldResourcesPlugin;
use spawning::WorldSpawningPlugin;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MapSettings>().add_plugins((
            MapPlugin,
            WorldResourcesPlugin,
            WorldSpawningPlugin,
        ));
    }
}
