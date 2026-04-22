pub mod animal;
pub mod decisions;
pub mod factions;
pub mod inventory;
pub mod memory;
pub mod mind;
pub mod needs;
pub mod npc;
pub mod predator;
pub mod programs;
pub mod relationships;

use animal::AnimalPlugin;
use bevy::prelude::*;
use decisions::DecisionPlugin;
use factions::FactionPlugin;
use memory::MemoryPlugin;
use mind::NpcMindPlugin;
use needs::NeedsPlugin;
use npc::NpcPlugin;
use predator::PredatorPlugin;
use programs::ProgramPlugin;
use relationships::RelationshipsPlugin;

pub struct AgentsPlugin;

impl Plugin for AgentsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            AnimalPlugin,
            NpcPlugin,
            PredatorPlugin,
            FactionPlugin,
            NeedsPlugin,
            MemoryPlugin,
            RelationshipsPlugin,
            DecisionPlugin,
            NpcMindPlugin,
            ProgramPlugin,
        ));
    }
}
