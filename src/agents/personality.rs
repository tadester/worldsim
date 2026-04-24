use bevy::prelude::*;

use crate::agents::inventory::Inventory;
use crate::agents::needs::Needs;
use crate::agents::npc::Npc;
use crate::systems::simulation::SimulationClock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersonalityType {
    Caregiver,
    Builder,
    Raider,
    Mystic,
    Hedonist,
    Sovereign,
    Scholar,
}

impl PersonalityType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Caregiver => "Caregiver",
            Self::Builder => "Builder",
            Self::Raider => "Raider",
            Self::Mystic => "Mystic",
            Self::Hedonist => "Hedonist",
            Self::Sovereign => "Sovereign",
            Self::Scholar => "Scholar",
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct NpcPsyche {
    pub personality: PersonalityType,
    pub happiness: f32,
    pub pride: f32,
    pub greed: f32,
    pub lust: f32,
    pub envy: f32,
    pub gluttony: f32,
    pub wrath: f32,
    pub sloth: f32,
    pub children_born: u32,
    pub successful_raids: u32,
    pub loot_taken: f32,
    pub structures_built: u32,
}

impl Default for NpcPsyche {
    fn default() -> Self {
        Self {
            personality: PersonalityType::Builder,
            happiness: 0.55,
            pride: 0.35,
            greed: 0.30,
            lust: 0.35,
            envy: 0.22,
            gluttony: 0.24,
            wrath: 0.20,
            sloth: 0.20,
            children_born: 0,
            successful_raids: 0,
            loot_taken: 0.0,
            structures_built: 0,
        }
    }
}

impl NpcPsyche {
    pub fn reward_reproduction(&mut self) {
        self.children_born += 1;
        self.happiness = (self.happiness + 0.08 + self.lust * 0.04).clamp(0.0, 1.0);
    }

    pub fn reward_loot(&mut self, amount: f32) {
        self.successful_raids += 1;
        self.loot_taken += amount;
        self.happiness = (self.happiness + amount * (0.05 + self.greed * 0.05)).clamp(0.0, 1.0);
    }

    pub fn reward_building(&mut self) {
        self.structures_built += 1;
        self.happiness = (self.happiness + 0.06 + self.pride * 0.03).clamp(0.0, 1.0);
    }
}

pub struct PersonalityPlugin;

impl Plugin for PersonalityPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_psyche_happiness);
    }
}

fn update_psyche_happiness(
    clock: Res<SimulationClock>,
    mut npcs: Query<(&Npc, &Needs, &Inventory, &mut NpcPsyche)>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    for (npc, needs, inventory, mut psyche) in &mut npcs {
        let comfort = (1.0 - needs.hunger) * 0.22
            + needs.safety * 0.18
            + (1.0 - needs.fatigue) * 0.12
            + needs.social * 0.10;
        let strain = needs.hunger * (0.12 + psyche.gluttony * 0.08)
            + (1.0 - needs.safety) * (0.10 + psyche.wrath * 0.06)
            + needs.fatigue * (0.05 + psyche.sloth * 0.08)
            + npc.exposure * 0.08;

        let personality_bonus = match psyche.personality {
            PersonalityType::Caregiver => psyche.children_born as f32 * 0.002 + needs.social * 0.03,
            PersonalityType::Builder => {
                psyche.structures_built as f32 * 0.002 + inventory.wood * 0.01
            }
            PersonalityType::Raider => psyche.successful_raids as f32 * 0.003 + psyche.wrath * 0.03,
            PersonalityType::Mystic => npc.discovery_drive * 0.02 + npc.curiosity * 0.02,
            PersonalityType::Hedonist => inventory.food * 0.015 + psyche.gluttony * 0.03,
            PersonalityType::Sovereign => psyche.pride * 0.03 + psyche.envy * 0.01,
            PersonalityType::Scholar => npc.tool_knowledge * 0.03 + npc.discovery_drive * 0.025,
        };

        psyche.happiness = (psyche.happiness
            + (comfort + personality_bonus - strain) * delta_days * 0.12)
            .clamp(0.0, 1.0);
    }
}
