use bevy::prelude::*;

use crate::agents::memory::Memory;
use crate::agents::needs::Needs;
use crate::agents::npc::Npc;
use crate::agents::relationships::Relationships;
use crate::systems::simulation::SimulationClock;
use crate::world::map::{MapSettings, RegionState, RegionTile};

#[derive(Component, Debug, Clone)]
pub struct NpcIntent {
    pub label: String,
    pub heading: Vec2,
}

impl Default for NpcIntent {
    fn default() -> Self {
        Self {
            label: "Idle".to_string(),
            heading: Vec2::ZERO,
        }
    }
}

pub struct DecisionPlugin;

impl Plugin for DecisionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (evaluate_npc_intents, apply_npc_intents).chain());
    }
}

fn evaluate_npc_intents(
    settings: Res<MapSettings>,
    regions: Query<(&RegionTile, &RegionState)>,
    npc_positions: Query<(Entity, &Transform), With<Npc>>,
    mut npcs: Query<(
        Entity,
        &Transform,
        &Npc,
        &Needs,
        &Relationships,
        &mut Memory,
        &mut NpcIntent,
    )>,
) {
    let positions: Vec<(Entity, Vec2)> = npc_positions
        .iter()
        .map(|(entity, transform)| (entity, transform.translation.truncate()))
        .collect();

    for (entity, transform, npc, needs, relationships, mut memory, mut intent) in &mut npcs {
        let pos = transform.translation.truncate();
        let current_coord = settings.tile_coord_for_position(pos);

        let mut current_forage = 0.0;
        let mut best_forage = -1.0;
        let mut best_forage_coord = current_coord;
        let mut safest_score = -1.0;
        let mut safest_coord = current_coord;
        let mut most_interesting = -1.0;
        let mut interesting_coord = current_coord;

        for (tile, state) in &regions {
            if tile.coord == current_coord {
                current_forage = state.forage;
            }

            if state.forage > best_forage {
                best_forage = state.forage;
                best_forage_coord = tile.coord;
            }

            let safe_score = tile.soil_fertility + state.forage * 0.2 - tile.mana_density * 0.05;
            if safe_score > safest_score {
                safest_score = safe_score;
                safest_coord = tile.coord;
            }

            let interesting_score = tile.mana_density + tile.temperature * 0.3;
            if interesting_score > most_interesting {
                most_interesting = interesting_score;
                interesting_coord = tile.coord;
            }
        }

        if current_forage > 0.5 {
            memory.last_forage_coord = Some(current_coord);
        }

        let nearest_other = positions
            .iter()
            .filter(|(other, _)| *other != entity)
            .min_by(|(_, a), (_, b)| pos.distance(*a).total_cmp(&pos.distance(*b)))
            .map(|(_, other_pos)| *other_pos);

        let hunger_utility = needs.hunger * 1.4 + needs.thirst * 0.4;
        let social_utility = (1.0 - needs.social) * relationships.social_drive;
        let safety_utility = (1.0 - needs.safety) * (0.8 + relationships.fear);
        let curiosity_utility = needs.curiosity * npc.curiosity * 0.8;

        let (label, target) = if hunger_utility >= social_utility
            && hunger_utility >= safety_utility
            && hunger_utility >= curiosity_utility
        {
            let remembered = memory.last_forage_coord.unwrap_or(best_forage_coord);
            ("Forage".to_string(), tile_center(&settings, remembered))
        } else if safety_utility >= social_utility && safety_utility >= curiosity_utility {
            let remembered_safe = memory
                .last_safe_position
                .unwrap_or_else(|| tile_center(&settings, safest_coord));
            ("Retreat".to_string(), remembered_safe)
        } else if social_utility >= curiosity_utility {
            (
                "Socialize".to_string(),
                nearest_other.unwrap_or_else(|| tile_center(&settings, safest_coord)),
            )
        } else {
            (
                "Explore".to_string(),
                tile_center(&settings, interesting_coord),
            )
        };

        let mut heading = target - pos;
        if heading.length_squared() > 0.001 {
            heading = heading.normalize();
        }

        intent.label = label.clone();
        intent.heading = heading;
        memory.last_decision = label;
    }
}

fn apply_npc_intents(
    clock: Res<SimulationClock>,
    settings: Res<MapSettings>,
    mut npcs: Query<(&mut Transform, &Npc, &mut Needs, &Relationships, &NpcIntent)>,
) {
    let delta_seconds = clock.delta_seconds();
    let bounds = settings.world_bounds() - Vec2::splat(10.0);

    for (mut transform, npc, mut needs, relationships, intent) in &mut npcs {
        let pace = npc.speed
            * (1.0 - needs.fatigue * 0.4)
            * (0.8 + relationships.trust_baseline * 0.2)
            * delta_seconds;

        transform.translation.x += intent.heading.x * pace;
        transform.translation.y += intent.heading.y * pace;
        transform.translation.x = transform.translation.x.clamp(-bounds.x, bounds.x);
        transform.translation.y = transform.translation.y.clamp(-bounds.y, bounds.y);

        match intent.label.as_str() {
            "Forage" => {
                needs.hunger = (needs.hunger - delta_seconds * 0.05).max(0.0);
                needs.safety = (needs.safety + delta_seconds * 0.01).min(1.0);
            }
            "Socialize" => {
                needs.social = (needs.social + delta_seconds * 0.08).min(1.0);
                needs.safety = (needs.safety + delta_seconds * 0.02).min(1.0);
            }
            "Retreat" => {
                needs.safety = (needs.safety + delta_seconds * 0.05).min(1.0);
                needs.fatigue = (needs.fatigue + delta_seconds * 0.01).min(1.0);
            }
            "Explore" => {
                needs.curiosity = (needs.curiosity - delta_seconds * 0.03).max(0.1);
                needs.fatigue = (needs.fatigue + delta_seconds * 0.015).min(1.0);
            }
            _ => {}
        }
    }
}

fn tile_center(settings: &MapSettings, coord: IVec2) -> Vec2 {
    let bounds = settings.world_bounds();
    Vec2::new(
        coord.x as f32 * settings.tile_size - bounds.x + settings.tile_size * 0.5,
        coord.y as f32 * settings.tile_size - bounds.y + settings.tile_size * 0.5,
    )
}
