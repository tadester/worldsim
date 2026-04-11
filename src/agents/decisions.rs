use bevy::prelude::*;

use crate::agents::memory::Memory;
use crate::agents::needs::Needs;
use crate::agents::npc::{Npc, NpcHome};
use crate::agents::relationships::Relationships;
use crate::systems::logging::{LogEvent, LogEventKind};
use crate::systems::simulation::SimulationClock;
use crate::world::map::{MapSettings, RegionState, RegionTile};
use crate::world::resources::Shelter;

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
        app.add_systems(
            Update,
            (
                evaluate_npc_intents,
                apply_npc_intents,
                apply_shelter_comfort.after(apply_npc_intents),
                repair_npc_shelters.after(apply_shelter_comfort),
                build_npc_shelters.after(repair_npc_shelters),
            )
                .chain(),
        );
    }
}

fn evaluate_npc_intents(
    settings: Res<MapSettings>,
    regions: Query<(&RegionTile, &RegionState)>,
    npc_positions: Query<(Entity, &Transform), With<Npc>>,
    shelters: Query<(&Shelter, &Transform), With<Shelter>>,
    mut npcs: Query<(
        Entity,
        &Transform,
        &Npc,
        &Needs,
        &Relationships,
        &mut Memory,
        &mut NpcIntent,
        &NpcHome,
    )>,
) {
    let positions: Vec<(Entity, Vec2)> = npc_positions
        .iter()
        .map(|(entity, transform)| (entity, transform.translation.truncate()))
        .collect();

    for (entity, transform, npc, needs, relationships, mut memory, mut intent, home) in &mut npcs {
        let pos = transform.translation.truncate();
        let current_coord = settings.tile_coord_for_position(pos);
        let home_shelter = home
            .shelter
            .and_then(|shelter_entity| shelters.get(shelter_entity).ok())
            .map(|(shelter, transform)| (*shelter, transform.translation.truncate()));
        let home_position = home_shelter.map(|(_, pos)| pos);
        let home_integrity = home_shelter
            .map(|(shelter, _)| shelter.integrity.clamp(0.0, 1.0))
            .unwrap_or(0.0);
        let home_coord = home_position
            .map(|p| settings.tile_coord_for_position(p))
            .unwrap_or(current_coord);

        let mut current_forage = 0.0;
        let mut best_forage = -1.0;
        let mut best_forage_coord = current_coord;
        let mut local_biomass = 0.0;
        let mut home_biomass = 0.0;
        let mut safest_score = -1.0;
        let mut safest_coord = current_coord;
        let mut most_interesting = -1.0;
        let mut interesting_coord = current_coord;

        for (tile, state) in &regions {
            if tile.coord == current_coord {
                current_forage = state.forage;
                local_biomass = state.tree_biomass;
            }
            if tile.coord == home_coord {
                home_biomass = state.tree_biomass;
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
        let shelter_nearby = shelters.iter().any(|(_, shelter_transform)| {
            shelter_transform.translation.truncate().distance(pos) < 42.0
        });
        let build_utility = if needs.safety < 0.45
            && local_biomass > 0.9
            && !shelter_nearby
            && home.shelter.is_none()
        {
            (0.55 - needs.safety) + local_biomass * 0.1
        } else {
            0.0
        };
        let rest_utility = if home_position.is_some() {
            needs.fatigue * 1.25 + (1.0 - needs.safety) * 0.25 + home_integrity * 0.15
        } else {
            0.0
        };
        let repair_utility = if home_position.is_some() && home_biomass > 0.5 {
            (1.0 - home_integrity) * 1.1 + (1.0 - needs.safety) * 0.25
        } else {
            0.0
        };

        let (label, target) = if rest_utility >= hunger_utility
            && rest_utility >= safety_utility
            && rest_utility >= social_utility
            && rest_utility >= curiosity_utility
            && rest_utility >= build_utility
            && rest_utility >= repair_utility
            && needs.fatigue > 0.55
        {
            (
                "Rest".to_string(),
                home_position.unwrap_or_else(|| tile_center(&settings, current_coord)),
            )
        } else if repair_utility >= hunger_utility
            && repair_utility >= safety_utility
            && repair_utility >= social_utility
            && repair_utility >= curiosity_utility
            && repair_utility >= build_utility
            && repair_utility >= rest_utility
            && home_position.is_some()
            && home_biomass > 0.75
            && home_integrity < 0.85
        {
            (
                "Repair Shelter".to_string(),
                home_position.unwrap_or_else(|| tile_center(&settings, current_coord)),
            )
        } else if hunger_utility >= social_utility
            && hunger_utility >= safety_utility
            && hunger_utility >= build_utility
            && hunger_utility >= curiosity_utility
            && hunger_utility >= rest_utility
            && hunger_utility >= repair_utility
        {
            let remembered = memory.last_forage_coord.unwrap_or(best_forage_coord);
            ("Forage".to_string(), tile_center(&settings, remembered))
        } else if build_utility >= safety_utility
            && build_utility >= social_utility
            && build_utility >= curiosity_utility
            && build_utility >= rest_utility
            && build_utility >= repair_utility
        {
            ("Build Shelter".to_string(), pos)
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
            "Build Shelter" => {
                needs.fatigue = (needs.fatigue + delta_seconds * 0.02).min(1.0);
            }
            "Rest" => {
                needs.fatigue = (needs.fatigue - delta_seconds * 0.08).max(0.0);
                needs.safety = (needs.safety + delta_seconds * 0.025).min(1.0);
                needs.hunger = (needs.hunger + delta_seconds * 0.01).min(1.0);
                needs.thirst = (needs.thirst + delta_seconds * 0.012).min(1.0);
            }
            "Repair Shelter" => {
                needs.fatigue = (needs.fatigue + delta_seconds * 0.03).min(1.0);
                needs.safety = (needs.safety + delta_seconds * 0.01).min(1.0);
            }
            "Explore" => {
                needs.curiosity = (needs.curiosity - delta_seconds * 0.03).max(0.1);
                needs.fatigue = (needs.fatigue + delta_seconds * 0.015).min(1.0);
            }
            _ => {}
        }
    }
}

fn apply_shelter_comfort(
    clock: Res<SimulationClock>,
    mut npcs: Query<(&Transform, &NpcHome, &mut Needs)>,
    shelters: Query<(Entity, &Shelter, &Transform)>,
) {
    let delta_seconds = clock.delta_seconds();
    if delta_seconds <= 0.0 {
        return;
    }

    for (npc_transform, home, mut needs) in &mut npcs {
        let pos = npc_transform.translation.truncate();

        let home_shelter = home
            .shelter
            .and_then(|entity| shelters.get(entity).ok())
            .map(|(_, shelter, transform)| (*shelter, transform.translation.truncate()));

        let (shelter, shelter_pos, is_home) = if let Some((shelter, shelter_pos)) = home_shelter {
            (shelter, shelter_pos, true)
        } else if let Some((_, shelter, transform)) =
            shelters.iter().min_by(|(_, _, a), (_, _, b)| {
                pos.distance(a.translation.truncate())
                    .total_cmp(&pos.distance(b.translation.truncate()))
            })
        {
            (*shelter, transform.translation.truncate(), false)
        } else {
            continue;
        };

        let distance = pos.distance(shelter_pos);
        if distance > 48.0 {
            continue;
        }

        let integrity = shelter.integrity.clamp(0.0, 1.0);
        let comfort = shelter.safety_bonus * integrity * if is_home { 1.4 } else { 0.7 };
        let falloff = (1.0 - distance / 48.0).clamp(0.0, 1.0);
        let gain = comfort * falloff * delta_seconds;

        needs.safety = (needs.safety + gain).min(1.0);
        needs.fatigue = (needs.fatigue - gain * 0.4).max(0.0);
    }
}

fn repair_npc_shelters(
    clock: Res<SimulationClock>,
    settings: Res<MapSettings>,
    mut writer: MessageWriter<LogEvent>,
    mut npcs: Query<(&Npc, &Transform, &NpcIntent, &NpcHome, &mut Needs)>,
    mut shelters: Query<(&mut Shelter, &Transform)>,
    mut regions: Query<(&RegionTile, &mut RegionState)>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    for (npc, npc_transform, intent, home, mut needs) in &mut npcs {
        if intent.label != "Repair Shelter" {
            continue;
        }

        let Some(shelter_entity) = home.shelter else {
            continue;
        };
        let Ok((mut shelter, shelter_transform)) = shelters.get_mut(shelter_entity) else {
            continue;
        };

        let npc_pos = npc_transform.translation.truncate();
        let shelter_pos = shelter_transform.translation.truncate();
        if npc_pos.distance(shelter_pos) > 26.0 {
            continue;
        }

        if shelter.integrity >= 0.98 {
            needs.safety = (needs.safety + 0.01).min(1.0);
            continue;
        }

        let coord = settings.tile_coord_for_position(shelter_pos);
        let mut spent = 0.0f32;
        for (tile, mut state) in &mut regions {
            if tile.coord != coord {
                continue;
            }

            let effort = (0.12 + (1.0 - needs.fatigue) * 0.08) * delta_days;
            let available = (state.tree_biomass - 0.25).max(0.0);
            spent = available.min(effort).max(0.0);
            state.tree_biomass -= spent;
            break;
        }

        if spent > 0.0 {
            shelter.integrity = (shelter.integrity + spent * 1.4).min(1.0);
            needs.safety = (needs.safety + spent * 0.18).min(1.0);
            if shelter.integrity >= 0.98 {
                writer.write(LogEvent::new(
                    LogEventKind::Construction,
                    format!("{} restored their shelter", npc.name),
                ));
            }
        }
    }
}

fn build_npc_shelters(
    mut commands: Commands,
    settings: Res<MapSettings>,
    mut writer: MessageWriter<LogEvent>,
    mut npcs: Query<
        (
            Entity,
            &Npc,
            &Transform,
            &mut Needs,
            &NpcIntent,
            &mut NpcHome,
        ),
        With<Npc>,
    >,
    shelters: Query<&Transform, With<Shelter>>,
    mut regions: Query<(&RegionTile, &mut RegionState)>,
) {
    for (entity, npc, transform, mut needs, intent, mut home) in &mut npcs {
        if intent.label != "Build Shelter" {
            continue;
        }

        let pos = transform.translation.truncate();
        if shelters
            .iter()
            .any(|shelter_transform| shelter_transform.translation.truncate().distance(pos) < 42.0)
        {
            needs.safety = (needs.safety + 0.02).min(1.0);
            continue;
        }

        let coord = settings.tile_coord_for_position(pos);
        for (tile, mut state) in &mut regions {
            if tile.coord != coord || state.tree_biomass < 1.0 {
                continue;
            }

            state.tree_biomass -= 1.0;
            let shelter_entity = commands
                .spawn((
                    Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.0), Vec2::splat(1.0)),
                    Transform::from_xyz(pos.x + 10.0, pos.y - 4.0, 1.8),
                    Shelter {
                        integrity: 1.0,
                        safety_bonus: 0.25,
                    },
                ))
                .id();

            if home.shelter.is_none() {
                home.shelter = Some(shelter_entity);
            }

            needs.safety = (needs.safety + 0.18).min(1.0);
            writer.write(LogEvent::new(
                LogEventKind::Construction,
                format!("{} built a shelter", npc.name),
            ));
            commands.entity(entity).insert(NpcIntent {
                label: "Rest".to_string(),
                heading: Vec2::ZERO,
            });
            break;
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
