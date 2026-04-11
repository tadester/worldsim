pub mod experiments;
pub mod mana;
pub mod storage;

use bevy::prelude::*;
use experiments::ExperimentsPlugin;
use mana::ManaPlugin;
use storage::StoragePlugin;

pub struct MagicPlugin;

impl Plugin for MagicPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((ManaPlugin, StoragePlugin, ExperimentsPlugin));
    }
}
