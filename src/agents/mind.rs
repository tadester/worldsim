use bevy::prelude::*;

use crate::agents::decisions::NpcIntent;
use crate::agents::inventory::Inventory;
use crate::agents::memory::Memory;
use crate::agents::needs::Needs;
use crate::agents::npc::Npc;
use crate::agents::relationships::Relationships;
use crate::life::growth::Lifecycle;
use crate::life::reproduction::NpcPregnancy;
use crate::systems::simulation::{SimulationClock, SimulationStep};
use crate::world::director::WorldMind;

#[derive(Component, Debug, Clone)]
pub struct NpcMind {
    pub mood: String,
    pub goal: String,
    pub plan: String,
    pub belief: String,
    pub pressure: f32,
    pub confidence: f32,
    pub last_reflection_day: f32,
}

impl Default for NpcMind {
    fn default() -> Self {
        Self {
            mood: "waking".to_string(),
            goal: "Observe surroundings".to_string(),
            plan: "Wait for first decision".to_string(),
            belief: "The world has not been interpreted yet".to_string(),
            pressure: 0.0,
            confidence: 0.25,
            last_reflection_day: 0.0,
        }
    }
}

pub struct NpcMindPlugin;

impl Plugin for NpcMindPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (attach_npc_minds, update_npc_minds));
    }
}

fn attach_npc_minds(mut commands: Commands, npcs: Query<Entity, (Added<Npc>, Without<NpcMind>)>) {
    for entity in &npcs {
        commands.entity(entity).insert(NpcMind::default());
    }
}

fn update_npc_minds(
    clock: Res<SimulationClock>,
    step: Res<SimulationStep>,
    world_mind: Option<Res<WorldMind>>,
    mut npcs: Query<(
        &Npc,
        &Needs,
        &Relationships,
        &Inventory,
        &Memory,
        &Lifecycle,
        &NpcIntent,
        Option<&NpcPregnancy>,
        &mut NpcMind,
    )>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    for (npc, needs, relationships, inventory, memory, lifecycle, intent, pregnancy, mut mind) in
        &mut npcs
    {
        mind.pressure = personal_pressure(needs, relationships, inventory, npc, pregnancy);
        mind.confidence = confidence_from_state(needs, relationships, inventory, lifecycle);

        let world_bias = world_mind
            .as_ref()
            .map(|mind| mind.stance.as_str())
            .unwrap_or("Unformed");
        mind.mood = mood_label(mind.pressure, needs, relationships, pregnancy).to_string();
        mind.goal = goal_from_intent(intent, needs, inventory, npc, pregnancy).to_string();
        mind.plan = plan_from_intent(intent).to_string();
        mind.belief = belief_summary(
            world_bias,
            needs,
            relationships,
            memory,
            inventory,
            lifecycle,
            pregnancy,
        );
        mind.last_reflection_day = step.elapsed_days;
    }
}

fn personal_pressure(
    needs: &Needs,
    relationships: &Relationships,
    inventory: &Inventory,
    npc: &Npc,
    pregnancy: Option<&NpcPregnancy>,
) -> f32 {
    let scarcity = needs.hunger.max(needs.thirst) * 0.35
        + (1.0 - needs.safety) * 0.30
        + needs.fatigue * 0.15
        + (1.0 - inventory.carry_ratio()).max(0.0) * 0.05;
    let social_stress = relationships.fear * 0.15 + (1.0 - relationships.trust_baseline) * 0.08;
    let pregnancy_stress = if pregnancy.is_some() { 0.08 } else { 0.0 };
    (scarcity + social_stress + pregnancy_stress + npc.exposure * 0.12).clamp(0.0, 1.0)
}

fn confidence_from_state(
    needs: &Needs,
    relationships: &Relationships,
    inventory: &Inventory,
    lifecycle: &Lifecycle,
) -> f32 {
    let maturity = if lifecycle.age_days >= lifecycle.maturity_age {
        0.18
    } else {
        0.04
    };
    (0.30
        + needs.safety * 0.20
        + relationships.trust_baseline * 0.18
        + inventory.carry_ratio() * 0.08
        + maturity
        - needs.fatigue * 0.16)
        .clamp(0.05, 1.0)
}

fn mood_label(
    pressure: f32,
    needs: &Needs,
    relationships: &Relationships,
    pregnancy: Option<&NpcPregnancy>,
) -> &'static str {
    if pregnancy.is_some() && needs.safety > 0.55 {
        "protective"
    } else if pressure > 0.76 {
        "desperate"
    } else if needs.safety < 0.28 || relationships.fear > 0.70 {
        "afraid"
    } else if needs.curiosity > 0.75 && needs.fatigue < 0.55 {
        "curious"
    } else if needs.social < 0.28 {
        "lonely"
    } else if needs.fatigue > 0.72 {
        "weary"
    } else {
        "steady"
    }
}

fn goal_from_intent(
    intent: &NpcIntent,
    needs: &Needs,
    inventory: &Inventory,
    npc: &Npc,
    pregnancy: Option<&NpcPregnancy>,
) -> &'static str {
    match intent.label.as_str() {
        "Forage" => "Secure food",
        "Gather Wood" => "Secure building material",
        "Build Shelter" => "Create a safe home",
        "Repair Shelter" => "Restore home safety",
        "Build Fire" | "Tend Fire" => "Stay warm",
        "Rest" => "Recover strength",
        "Socialize" => "Find companionship",
        "Retreat" | "Flee" => "Survive danger",
        "Hunt Predator" => "Remove a threat",
        "Make Tools" => "Improve future work",
        "Stockpile" => "Prepare for scarcity",
        "Explore" if pregnancy.is_some() => "Scout for a safer future",
        "Explore" if needs.curiosity > 0.65 => "Understand the world",
        "Explore" if inventory.food < 0.2 && needs.hunger > 0.55 => "Search for resources",
        "Explore" if npc.discovery_drive > npc.reproduction_drive => "Follow curiosity",
        _ => "Adapt to conditions",
    }
}

fn plan_from_intent(intent: &NpcIntent) -> &'static str {
    match intent.label.as_str() {
        "Flee" => "Move away from the nearest predator until safety rises",
        "Retreat" => "Travel toward the safest remembered location",
        "Forage" => "Reach forage and fill carried food",
        "Gather Wood" => "Reach trees and convert biomass into carried wood",
        "Build Shelter" => "Spend carried wood where no shelter is nearby",
        "Repair Shelter" => "Return home and spend wood on integrity",
        "Build Fire" => "Spend a small bundle of wood at the current location",
        "Tend Fire" => "Find the nearest fire and add carried wood",
        "Make Tools" => "Practice until knowledge or tools improve",
        "Stockpile" => "Carry supplies home and deposit them",
        "Rest" => "Stay near shelter and reduce fatigue",
        "Socialize" => "Close distance to another NPC",
        "Hunt Predator" => "Close with a predator and attack if near enough",
        "Explore" => "Move toward the most interesting known tile",
        _ => "Hold position and wait for a clearer signal",
    }
}

fn belief_summary(
    world_bias: &str,
    needs: &Needs,
    relationships: &Relationships,
    memory: &Memory,
    inventory: &Inventory,
    lifecycle: &Lifecycle,
    pregnancy: Option<&NpcPregnancy>,
) -> String {
    let food_belief = if inventory.food > 0.5 {
        "I have food"
    } else if memory.last_forage_coord.is_some() {
        "I remember forage"
    } else {
        "food is uncertain"
    };
    let safety_belief = if needs.safety > 0.65 {
        "this place feels safe"
    } else if memory.last_safe_position.is_some() {
        "I remember a safer place"
    } else {
        "safety is unresolved"
    };
    let kin_belief = if pregnancy.is_some() {
        "a child is coming"
    } else if lifecycle.reproduction_cooldown > 0.0 {
        "family must wait"
    } else if relationships.affinity > 0.65 {
        "others are close"
    } else {
        "kinship is distant"
    };

    format!(
        "{}; {}; {}; world feels {}",
        food_belief, safety_belief, kin_belief, world_bias
    )
}
