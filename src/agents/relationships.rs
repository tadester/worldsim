use bevy::prelude::*;

use crate::agents::memory::Memory;
use crate::agents::needs::Needs;
use crate::systems::simulation::SimulationClock;

#[derive(Component, Debug, Clone)]
pub struct Relationships {
    pub social_drive: f32,
    pub trust_baseline: f32,
    pub affinity: f32,
    pub fear: f32,
}

impl Default for Relationships {
    fn default() -> Self {
        Self {
            social_drive: 0.5,
            trust_baseline: 0.5,
            affinity: 0.5,
            fear: 0.15,
        }
    }
}

pub struct RelationshipsPlugin;

impl Plugin for RelationshipsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_relationships);
    }
}

fn update_relationships(
    clock: Res<SimulationClock>,
    npc_positions: Query<(Entity, &Transform)>,
    mut npcs: Query<(Entity, &Transform, &Needs, &mut Relationships, &mut Memory)>,
) {
    let delta_days = clock.delta_days();
    let positions: Vec<(Entity, Vec2)> = npc_positions
        .iter()
        .map(|(entity, transform)| (entity, transform.translation.truncate()))
        .collect();

    for (entity, transform, needs, mut relationships, mut memory) in &mut npcs {
        let my_pos = transform.translation.truncate();
        let mut nearest_distance = f32::MAX;

        for (other_entity, other_pos) in &positions {
            if *other_entity == entity {
                continue;
            }

            nearest_distance = nearest_distance.min(my_pos.distance(*other_pos));
        }

        if nearest_distance.is_finite() && nearest_distance < 70.0 {
            relationships.affinity = (relationships.affinity + delta_days * 0.12).min(1.0);
            relationships.trust_baseline =
                (relationships.trust_baseline + delta_days * 0.08).min(1.0);
            relationships.fear = (relationships.fear - delta_days * 0.05).max(0.0);
            memory.last_social_contact_days = 0.0;
        } else {
            relationships.affinity = (relationships.affinity - delta_days * 0.03).max(0.0);
            relationships.fear =
                (relationships.fear + (1.0 - needs.safety) * delta_days * 0.05).min(1.0);
            memory.last_social_contact_days += delta_days;
        }

        relationships.social_drive =
            (0.35 + needs.social * 0.4 + relationships.affinity * 0.25).clamp(0.1, 1.0);

        if needs.safety > 0.7 {
            memory.last_safe_position = Some(my_pos);
        }
    }
}
