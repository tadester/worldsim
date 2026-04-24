pub mod animal;
pub mod decisions;
pub mod factions;
pub mod inventory;
pub mod memory;
pub mod mind;
pub mod needs;
pub mod npc;
pub mod personality;
pub mod predator;
pub mod programs;
pub mod relationships;
pub mod society;

use animal::AnimalPlugin;
use bevy::prelude::*;
use decisions::DecisionPlugin;
use factions::FactionPlugin;
use memory::MemoryPlugin;
use mind::NpcMindPlugin;
use needs::NeedsPlugin;
use npc::NpcPlugin;
use personality::PersonalityPlugin;
use predator::PredatorPlugin;
use programs::ProgramPlugin;
use relationships::RelationshipsPlugin;
use society::SocietyPlugin;

pub struct AgentsPlugin;

impl Plugin for AgentsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            AnimalPlugin,
            NpcPlugin,
            PersonalityPlugin,
            PredatorPlugin,
            FactionPlugin,
            NeedsPlugin,
            MemoryPlugin,
            RelationshipsPlugin,
            SocietyPlugin,
            DecisionPlugin,
            NpcMindPlugin,
            ProgramPlugin,
        ));
    }
}
