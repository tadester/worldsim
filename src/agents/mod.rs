pub mod animal;
pub mod decisions;
pub mod memory;
pub mod needs;
pub mod npc;
pub mod relationships;

use animal::AnimalPlugin;
use bevy::prelude::*;
use decisions::DecisionPlugin;
use memory::MemoryPlugin;
use needs::NeedsPlugin;
use npc::NpcPlugin;
use relationships::RelationshipsPlugin;

pub struct AgentsPlugin;

impl Plugin for AgentsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            AnimalPlugin,
            NpcPlugin,
            NeedsPlugin,
            MemoryPlugin,
            RelationshipsPlugin,
            DecisionPlugin,
        ));
    }
}
