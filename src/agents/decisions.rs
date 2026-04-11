use bevy::prelude::*;

use crate::agents::needs::Needs;
use crate::agents::npc::Npc;
use crate::agents::relationships::Relationships;

pub struct DecisionPlugin;

impl Plugin for DecisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, npc_decisions);
    }
}

fn npc_decisions(mut npcs: Query<(&mut Transform, &Npc, &Needs, &Relationships)>) {
    for (mut transform, npc, needs, relationships) in &mut npcs {
        let hunger_bias = (needs.hunger - 0.5).max(0.0);
        let curiosity_bias = npc.curiosity * needs.curiosity;
        let social_bias = relationships.social_drive * needs.social;
        let caution_bias = (1.0 - needs.safety) * (1.0 - relationships.trust_baseline);

        transform.translation.x += (curiosity_bias - hunger_bias - caution_bias) * 0.15;
        transform.translation.y += social_bias * 0.05;
    }
}
