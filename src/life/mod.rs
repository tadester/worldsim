pub mod death;
pub mod growth;
pub mod reproduction;

use bevy::prelude::*;
use death::DeathPlugin;
use growth::GrowthPlugin;
use reproduction::ReproductionPlugin;

pub struct LifePlugin;

impl Plugin for LifePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((GrowthPlugin, ReproductionPlugin, DeathPlugin));
    }
}
