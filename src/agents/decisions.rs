use bevy::prelude::*;

use std::collections::HashMap;

use crate::agents::factions::FactionMember;
use crate::agents::inventory::Inventory;
use crate::agents::memory::Memory;
use crate::agents::needs::Needs;
use crate::agents::npc::{Npc, NpcHome};
use crate::agents::predator::Predator;
use crate::agents::relationships::Relationships;
use crate::systems::logging::{LogEvent, LogEventKind};
use crate::systems::simulation::SimulationClock;
use crate::world::climate::{ClimateModel, RegionClimate};
use crate::world::map::{MapSettings, RegionState, RegionTile};
use crate::world::resources::{Campfire, Shelter, ShelterStockpile, Tree, TreeStage};
use crate::world::territory::Territory;

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
                avoid_predators,
                apply_npc_intents,
                harvest_npc_resources,
                develop_and_craft_tools,
                resolve_npc_violence,
                transfer_home_stockpiles,
                withdraw_home_food,
                consume_carried_food,
                apply_shelter_comfort.after(apply_npc_intents),
                apply_climate_stress.after(apply_shelter_comfort),
                tend_npc_campfires.after(apply_climate_stress),
                repair_npc_shelters.after(apply_climate_stress),
                build_npc_campfires.after(tend_npc_campfires),
                build_npc_shelters.after(build_npc_campfires),
            )
                .chain(),
        );
    }
}

fn evaluate_npc_intents(
    settings: Res<MapSettings>,
    regions: Query<(
        &RegionTile,
        &RegionState,
        &RegionClimate,
        Option<&Territory>,
    )>,
    npc_positions: Query<(Entity, &Transform, Option<&FactionMember>), With<Npc>>,
    predators: Query<(Entity, &Transform), With<Predator>>,
    campfires: Query<(Entity, &Campfire, &Transform, Option<&FactionMember>)>,
    shelters: Query<(
        Entity,
        &Shelter,
        Option<&ShelterStockpile>,
        &Transform,
        Option<&FactionMember>,
    )>,
    mut npcs: Query<(
        Entity,
        &Transform,
        &Npc,
        &Needs,
        &Relationships,
        &mut Memory,
        &mut NpcIntent,
        &NpcHome,
        &Inventory,
        Option<&FactionMember>,
    )>,
) {
    let mut region_index: HashMap<IVec2, (RegionTile, RegionState, f32, f32, Option<Entity>)> =
        HashMap::new();
    let mut best_forage = -1.0;
    let mut best_forage_coord = IVec2::ZERO;
    let mut best_forage_by_faction: HashMap<Entity, (f32, IVec2)> = HashMap::new();
    let mut best_biomass = -1.0;
    let mut best_biomass_coord = IVec2::ZERO;
    let mut best_biomass_by_faction: HashMap<Entity, (f32, IVec2)> = HashMap::new();
    let mut safest_score = -1.0;
    let mut safest_coord = IVec2::ZERO;
    let mut safest_by_faction: HashMap<Entity, (f32, IVec2)> = HashMap::new();
    let mut most_interesting = -1.0;
    let mut interesting_coord = IVec2::ZERO;
    let mut interesting_by_faction: HashMap<Entity, (f32, IVec2)> = HashMap::new();
    let mut unclaimed_interesting = -1.0;
    let mut unclaimed_interesting_coord = IVec2::ZERO;

    for (tile, state, climate, territory) in &regions {
        let owner = territory.and_then(|territory| territory.owner);
        region_index.insert(
            tile.coord,
            (*tile, *state, climate.pressure, tile.temperature, owner),
        );

        if state.forage > best_forage {
            best_forage = state.forage;
            best_forage_coord = tile.coord;
        }

        if state.tree_biomass > best_biomass {
            best_biomass = state.tree_biomass;
            best_biomass_coord = tile.coord;
        }

        if let Some(owner) = owner {
            let entry = best_forage_by_faction
                .entry(owner)
                .or_insert((state.forage, tile.coord));
            if state.forage > entry.0 {
                *entry = (state.forage, tile.coord);
            }

            let entry = best_biomass_by_faction
                .entry(owner)
                .or_insert((state.tree_biomass, tile.coord));
            if state.tree_biomass > entry.0 {
                *entry = (state.tree_biomass, tile.coord);
            }
        }

        let safe_score = tile.soil_fertility + state.forage * 0.2
            - tile.mana_density * 0.05
            - climate.pressure * 0.35;
        if safe_score > safest_score {
            safest_score = safe_score;
            safest_coord = tile.coord;
        }
        if let Some(owner) = owner {
            let entry = safest_by_faction
                .entry(owner)
                .or_insert((safe_score, tile.coord));
            if safe_score > entry.0 {
                *entry = (safe_score, tile.coord);
            }
        }

        let interesting_score =
            tile.mana_density + tile.temperature * 0.3 - climate.pressure * 0.15;
        if interesting_score > most_interesting {
            most_interesting = interesting_score;
            interesting_coord = tile.coord;
        }
        match owner {
            Some(owner) => {
                let entry = interesting_by_faction
                    .entry(owner)
                    .or_insert((interesting_score, tile.coord));
                if interesting_score > entry.0 {
                    *entry = (interesting_score, tile.coord);
                }
            }
            None => {
                if interesting_score > unclaimed_interesting {
                    unclaimed_interesting = interesting_score;
                    unclaimed_interesting_coord = tile.coord;
                }
            }
        }
    }

    let positions: Vec<(Entity, Vec2, Option<Entity>)> = npc_positions
        .iter()
        .map(|(entity, transform, member)| {
            (
                entity,
                transform.translation.truncate(),
                member.map(|member| member.faction),
            )
        })
        .collect();
    let predator_positions: Vec<(Entity, Vec2)> = predators
        .iter()
        .map(|(entity, transform)| (entity, transform.translation.truncate()))
        .collect();

    for (
        entity,
        transform,
        npc,
        needs,
        relationships,
        mut memory,
        mut intent,
        home,
        inventory,
        member,
    ) in &mut npcs
    {
        let pos = transform.translation.truncate();
        let current_coord = settings.tile_coord_for_position(pos);
        let faction = member.map(|member| member.faction);
        let home_shelter = home.shelter.and_then(|shelter_entity| {
            shelters
                .get(shelter_entity)
                .ok()
                .map(|(_, shelter, stockpile, transform, _)| {
                    (
                        *shelter,
                        stockpile.copied(),
                        transform.translation.truncate(),
                    )
                })
        });
        let ally_shelter = if home_shelter.is_none() {
            faction.and_then(|faction| {
                shelters
                    .iter()
                    .filter(|(_, _, _, _, member)| {
                        member.map(|member| member.faction) == Some(faction)
                    })
                    .min_by(|(_, _, _, a, _), (_, _, _, b, _)| {
                        pos.distance(a.translation.truncate())
                            .total_cmp(&pos.distance(b.translation.truncate()))
                    })
                    .map(|(_, shelter, stockpile, transform, _)| {
                        (
                            *shelter,
                            stockpile.copied(),
                            transform.translation.truncate(),
                        )
                    })
            })
        } else {
            None
        };
        let home_position = home_shelter.map(|(_, _, pos)| pos);
        let home_integrity = home_shelter
            .map(|(shelter, _, _)| shelter.integrity.clamp(0.0, 1.0))
            .unwrap_or(0.0);
        let home_stockpiled_wood = home_shelter
            .and_then(|(_, stockpile, _)| stockpile.map(|pile| pile.wood))
            .unwrap_or(0.0);
        let ally_position = ally_shelter.map(|(_, _, pos)| pos);
        let rest_position = home_position.or(ally_position);
        let rest_integrity = if home_position.is_some() {
            home_integrity
        } else {
            ally_shelter
                .map(|(shelter, _, _)| (shelter.integrity * 0.85).clamp(0.0, 1.0))
                .unwrap_or(0.0)
        };

        let (current_forage, local_biomass, local_pressure, local_temperature) = region_index
            .get(&current_coord)
            .map(|(_, state, pressure, temperature, _)| {
                (state.forage, state.tree_biomass, *pressure, *temperature)
            })
            .unwrap_or((0.0, 0.0, 0.0, 0.5));

        if current_forage > 0.5 {
            memory.last_forage_coord = Some(current_coord);
        }

        let nearest_same_faction = faction.and_then(|faction| {
            positions
                .iter()
                .filter(|(other, _, other_faction)| {
                    *other != entity && *other_faction == Some(faction)
                })
                .min_by(|(_, a, _), (_, b, _)| pos.distance(*a).total_cmp(&pos.distance(*b)))
                .map(|(_, other_pos, _)| *other_pos)
        });
        let nearest_other = nearest_same_faction.or_else(|| {
            positions
                .iter()
                .filter(|(other, _, _)| *other != entity)
                .min_by(|(_, a, _), (_, b, _)| pos.distance(*a).total_cmp(&pos.distance(*b)))
                .map(|(_, other_pos, _)| *other_pos)
        });

        let food_ratio = inventory.food_ratio();
        let hunger_utility = (needs.hunger * 1.4 + needs.thirst * 0.4)
            * (1.15 - food_ratio * 0.55)
            * (0.85 + npc.reproduction_drive * 0.35);
        let social_utility = (1.0 - needs.social)
            * relationships.social_drive
            * (0.7 + npc.reproduction_drive * 0.45);
        let safety_utility = (1.0 - needs.safety) * (0.8 + relationships.fear)
            + local_pressure * 0.55
            - npc.risk_tolerance * 0.20;
        let curiosity_utility =
            needs.curiosity * npc.curiosity * 0.8 * (0.75 + npc.discovery_drive * 0.5);
        let cold_risk = ((0.42 - local_temperature).max(0.0) / 0.42).clamp(0.0, 1.0)
            * (0.55 + local_pressure * 0.45);
        let nearest_fire = campfires
            .iter()
            .filter(|(_, _, _, member)| {
                member.map(|member| member.faction) == faction || member.is_none() || faction.is_none()
            })
            .min_by(|(_, _, a, _), (_, _, b, _)| {
                pos.distance(a.translation.truncate())
                    .total_cmp(&pos.distance(b.translation.truncate()))
            })
            .map(|(_, fire, transform, _)| (*fire, transform.translation.truncate()));
        let shelter_nearby = shelters.iter().any(|(_, _, _, shelter_transform, _)| {
            shelter_transform.translation.truncate().distance(pos) < 42.0
        });
        let fire_nearby = nearest_fire
            .map(|(_, fire_pos)| pos.distance(fire_pos) < 38.0)
            .unwrap_or(false);
        let carrying_ratio = inventory.carry_ratio();
        let has_usable_tools = npc.woodcutting_tools > 0.18;
        let build_utility = if needs.safety < 0.78
            && local_biomass > 0.5
            && !shelter_nearby
            && home.shelter.is_none()
            && inventory.wood >= 0.9
        {
            ((0.9 - needs.safety)
                + local_biomass * 0.18
                + cold_risk * 1.25
                + npc.exposure * 0.75
                + npc.reproduction_drive * 0.45)
                * (1.12 - local_pressure * 0.35)
        } else {
            0.0
        };
        let build_fire_utility = if inventory.wood >= 0.35 && !fire_nearby {
            cold_risk * 1.6 + npc.exposure * 1.4 + (1.0 - needs.safety) * 0.35
        } else {
            0.0
        };
        let tend_fire_utility = nearest_fire
            .map(|(fire, fire_pos)| {
                let distance = pos.distance(fire_pos);
                if inventory.wood < 0.12 || distance > 52.0 {
                    0.0
                } else {
                    ((1.0 - fire.ember) + (1.0 - fire.fuel / fire.max_fuel.max(0.1))) * 0.8
                        + cold_risk * 1.1
                        + npc.exposure * 1.2
                }
            })
            .unwrap_or(0.0);
        let rest_utility = if rest_position.is_some() {
            let home_factor = if home_position.is_some() { 1.0 } else { 0.7 };
            (needs.fatigue * 1.25
                + (1.0 - needs.safety) * 0.25
                + rest_integrity * 0.15
                + npc.exposure * 0.80)
                * home_factor
        } else {
            0.0
        };
        let repair_utility = if home_position.is_some()
            && home_integrity < 0.85
            && (home_stockpiled_wood + inventory.wood) > 0.15
        {
            (1.0 - home_integrity) * 1.1 + (1.0 - needs.safety) * 0.25
        } else {
            0.0
        };
        let wood_total = home_stockpiled_wood + inventory.wood;
        let wants_wood = (home_position.is_some() && home_integrity < 0.85)
            || (home_position.is_none()
                && (needs.safety < 0.82 || wood_total < 1.2 || npc.reproduction_drive > 1.0));
        let gather_wood_utility =
            if wants_wood && wood_total < 3.2 && (local_biomass > 0.25 || best_biomass > 0.35) {
                (1.0 - needs.safety) * 0.95
                    + (1.0 - home_integrity) * 0.95
                    + local_biomass * 0.14
                    + if home_position.is_none() { 0.42 } else { 0.0 }
                    + cold_risk * 0.55
                    + if has_usable_tools { 0.25 } else { 0.0 }
            } else {
                0.0
            };
        let toolmaking_utility = if wants_wood && (!has_usable_tools || npc.tool_knowledge < 1.0) {
            0.65 + npc.discovery_drive * 0.55 + cold_risk * 0.30 + npc.reproduction_drive * 0.15
        } else {
            0.0
        };
        let stockpile_utility = if home_position.is_some() && carrying_ratio > 0.55 {
            carrying_ratio * 0.85
                + (inventory.food + inventory.wood) * 0.03
                + npc.reproduction_drive * 0.22
        } else {
            0.0
        };
        let violence_utility = predator_positions
            .iter()
            .map(|(_, predator_pos)| pos.distance(*predator_pos))
            .filter(|distance| *distance < 130.0)
            .min_by(|a, b| a.total_cmp(b))
            .map(|distance| {
                let proximity = (1.0 - distance / 130.0).clamp(0.0, 1.0);
                proximity * (npc.aggression_drive * 1.2 + npc.risk_tolerance * 0.5)
            })
            .unwrap_or(0.0);

        let best_forage_for_faction = faction.and_then(|faction| {
            best_forage_by_faction
                .get(&faction)
                .map(|(_, coord)| *coord)
        });
        let best_biomass_for_faction = faction.and_then(|faction| {
            best_biomass_by_faction
                .get(&faction)
                .map(|(_, coord)| *coord)
        });
        let safest_for_faction = faction
            .and_then(|faction| safest_by_faction.get(&faction).map(|(_, coord)| *coord))
            .unwrap_or(safest_coord);
        let explore_for_faction = faction
            .and_then(|faction| {
                interesting_by_faction
                    .get(&faction)
                    .map(|(_, coord)| *coord)
            })
            .or_else(|| {
                if unclaimed_interesting > 0.0 {
                    Some(unclaimed_interesting_coord)
                } else {
                    None
                }
            })
            .unwrap_or(interesting_coord);

        let (label, target) = if rest_utility >= hunger_utility
            && rest_utility >= safety_utility
            && rest_utility >= social_utility
            && rest_utility >= curiosity_utility
            && rest_utility >= build_utility
            && rest_utility >= build_fire_utility
            && rest_utility >= tend_fire_utility
            && rest_utility >= repair_utility
            && rest_utility >= gather_wood_utility
            && rest_utility >= toolmaking_utility
            && rest_utility >= violence_utility
            && rest_utility >= stockpile_utility
            && (needs.fatigue > 0.55 || npc.exposure > 0.35)
        {
            (
                "Rest".to_string(),
                rest_position.unwrap_or_else(|| tile_center(&settings, current_coord)),
            )
        } else if repair_utility >= hunger_utility
            && repair_utility >= safety_utility
            && repair_utility >= social_utility
            && repair_utility >= curiosity_utility
            && repair_utility >= build_utility
            && repair_utility >= build_fire_utility
            && repair_utility >= tend_fire_utility
            && repair_utility >= rest_utility
            && repair_utility >= gather_wood_utility
            && repair_utility >= toolmaking_utility
            && repair_utility >= violence_utility
            && repair_utility >= stockpile_utility
            && home_position.is_some()
            && home_integrity < 0.85
        {
            (
                "Repair Shelter".to_string(),
                home_position.unwrap_or_else(|| tile_center(&settings, current_coord)),
            )
        } else if stockpile_utility >= hunger_utility
            && stockpile_utility >= safety_utility
            && stockpile_utility >= social_utility
            && stockpile_utility >= curiosity_utility
            && stockpile_utility >= build_utility
            && stockpile_utility >= build_fire_utility
            && stockpile_utility >= tend_fire_utility
            && stockpile_utility >= rest_utility
            && stockpile_utility >= repair_utility
            && stockpile_utility >= gather_wood_utility
            && stockpile_utility >= toolmaking_utility
            && stockpile_utility >= violence_utility
            && home_position.is_some()
        {
            ("Stockpile".to_string(), home_position.unwrap())
        } else if gather_wood_utility >= hunger_utility
            && gather_wood_utility >= safety_utility
            && gather_wood_utility >= social_utility
            && gather_wood_utility >= curiosity_utility
            && gather_wood_utility >= build_utility
            && gather_wood_utility >= build_fire_utility
            && gather_wood_utility >= tend_fire_utility
            && gather_wood_utility >= rest_utility
            && gather_wood_utility >= repair_utility
            && gather_wood_utility >= toolmaking_utility
            && gather_wood_utility >= violence_utility
            && gather_wood_utility >= stockpile_utility
        {
            let target_coord = if local_biomass > 0.25 {
                current_coord
            } else {
                best_biomass_for_faction.unwrap_or(best_biomass_coord)
            };
            (
                "Gather Wood".to_string(),
                tile_center(&settings, target_coord),
            )
        } else if tend_fire_utility >= hunger_utility
            && tend_fire_utility >= safety_utility
            && tend_fire_utility >= social_utility
            && tend_fire_utility >= curiosity_utility
            && tend_fire_utility >= build_utility
            && tend_fire_utility >= build_fire_utility
            && tend_fire_utility >= rest_utility
            && tend_fire_utility >= repair_utility
            && tend_fire_utility >= gather_wood_utility
            && tend_fire_utility >= toolmaking_utility
            && tend_fire_utility >= violence_utility
            && tend_fire_utility >= stockpile_utility
        {
            (
                "Tend Fire".to_string(),
                nearest_fire.map(|(_, fire_pos)| fire_pos).unwrap_or(pos),
            )
        } else if build_fire_utility >= hunger_utility
            && build_fire_utility >= safety_utility
            && build_fire_utility >= social_utility
            && build_fire_utility >= curiosity_utility
            && build_fire_utility >= build_utility
            && build_fire_utility >= rest_utility
            && build_fire_utility >= repair_utility
            && build_fire_utility >= gather_wood_utility
            && build_fire_utility >= toolmaking_utility
            && build_fire_utility >= violence_utility
            && build_fire_utility >= stockpile_utility
        {
            ("Build Fire".to_string(), pos)
        } else if toolmaking_utility >= hunger_utility
            && toolmaking_utility >= safety_utility
            && toolmaking_utility >= social_utility
            && toolmaking_utility >= curiosity_utility
            && toolmaking_utility >= build_utility
            && toolmaking_utility >= rest_utility
            && toolmaking_utility >= repair_utility
            && toolmaking_utility >= gather_wood_utility
            && toolmaking_utility >= violence_utility
            && toolmaking_utility >= stockpile_utility
        {
            ("Make Tools".to_string(), pos)
        } else if violence_utility >= hunger_utility
            && violence_utility >= safety_utility
            && violence_utility >= social_utility
            && violence_utility >= curiosity_utility
            && violence_utility >= build_utility
            && violence_utility >= rest_utility
            && violence_utility >= repair_utility
            && violence_utility >= gather_wood_utility
            && violence_utility >= toolmaking_utility
            && violence_utility >= stockpile_utility
        {
            let target = predator_positions
                .iter()
                .min_by(|(_, a), (_, b)| pos.distance(*a).total_cmp(&pos.distance(*b)))
                .map(|(_, predator_pos)| *predator_pos)
                .unwrap_or(pos);
            ("Hunt Predator".to_string(), target)
        } else if hunger_utility >= social_utility
            && hunger_utility >= safety_utility
            && hunger_utility >= build_utility
            && hunger_utility >= curiosity_utility
            && hunger_utility >= rest_utility
            && hunger_utility >= repair_utility
            && hunger_utility >= gather_wood_utility
            && hunger_utility >= toolmaking_utility
            && hunger_utility >= violence_utility
            && hunger_utility >= stockpile_utility
        {
            let remembered = memory
                .last_forage_coord
                .or(best_forage_for_faction)
                .unwrap_or(best_forage_coord);
            ("Forage".to_string(), tile_center(&settings, remembered))
        } else if build_utility >= safety_utility
            && build_utility >= social_utility
            && build_utility >= curiosity_utility
            && build_utility >= rest_utility
            && build_utility >= repair_utility
            && build_utility >= gather_wood_utility
            && build_utility >= toolmaking_utility
            && build_utility >= violence_utility
            && build_utility >= stockpile_utility
        {
            ("Build Shelter".to_string(), pos)
        } else if safety_utility >= social_utility
            && safety_utility >= curiosity_utility
            && safety_utility >= violence_utility
        {
            let remembered_safe = memory
                .last_safe_position
                .unwrap_or_else(|| tile_center(&settings, safest_for_faction));
            ("Retreat".to_string(), remembered_safe)
        } else if social_utility >= curiosity_utility && social_utility >= violence_utility {
            (
                "Socialize".to_string(),
                nearest_other.unwrap_or_else(|| tile_center(&settings, safest_for_faction)),
            )
        } else {
            (
                "Explore".to_string(),
                tile_center(&settings, explore_for_faction),
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

fn avoid_predators(
    clock: Res<SimulationClock>,
    mut npcs: Query<(&Npc, &Transform, &mut Needs, &mut NpcIntent), With<Npc>>,
    predators: Query<&Transform, With<Predator>>,
) {
    let delta_seconds = clock.delta_seconds();
    if delta_seconds <= 0.0 {
        return;
    }

    let predator_positions: Vec<Vec2> =
        predators.iter().map(|t| t.translation.truncate()).collect();
    if predator_positions.is_empty() {
        return;
    }

    let threat_radius = 160.0;
    let flee_radius = 120.0;

    for (npc, transform, mut needs, mut intent) in &mut npcs {
        let pos = transform.translation.truncate();
        let mut nearest: Option<(Vec2, f32)> = None;

        for predator_pos in predator_positions.iter().copied() {
            let d = pos.distance(predator_pos);
            if d > threat_radius {
                continue;
            }
            if nearest.map(|(_, best)| d < best).unwrap_or(true) {
                nearest = Some((predator_pos, d));
            }
        }

        let Some((predator_pos, distance)) = nearest else {
            continue;
        };

        let mut away = pos - predator_pos;
        if away.length_squared() > 0.001 {
            away = away.normalize();
        }

        if distance <= flee_radius
            && !(intent.label == "Hunt Predator" && npc.aggression_drive > 0.7)
        {
            if intent.label != "Flee" {
                intent.label.clear();
                intent.label.push_str("Flee");
            }
            intent.heading = away;
            needs.safety = (needs.safety - delta_seconds * 0.10).max(0.0);
            needs.fatigue = (needs.fatigue + delta_seconds * 0.06).min(1.0);
        } else if intent.label == "Explore" {
            intent.heading = (intent.heading + away * 0.55).normalize_or_zero();
            needs.safety = (needs.safety - delta_seconds * 0.02).max(0.0);
        }
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
        let pace_boost = if intent.label == "Flee" { 1.55 } else { 1.0 };
        let pace = npc.speed
            * (1.0 - needs.fatigue * 0.4)
            * (0.8 + relationships.trust_baseline * 0.2)
            * pace_boost
            * delta_seconds;

        transform.translation.x += intent.heading.x * pace;
        transform.translation.y += intent.heading.y * pace;
        transform.translation.x = transform.translation.x.clamp(-bounds.x, bounds.x);
        transform.translation.y = transform.translation.y.clamp(-bounds.y, bounds.y);

        match intent.label.as_str() {
            "Forage" => {
                needs.safety = (needs.safety + delta_seconds * 0.01).min(1.0);
                needs.fatigue = (needs.fatigue + delta_seconds * 0.012).min(1.0);
            }
            "Gather Wood" => {
                needs.safety = (needs.safety + delta_seconds * 0.008).min(1.0);
                needs.fatigue = (needs.fatigue + delta_seconds * 0.018).min(1.0);
            }
            "Build Fire" | "Tend Fire" => {
                needs.safety = (needs.safety + delta_seconds * 0.02).min(1.0);
                needs.fatigue = (needs.fatigue + delta_seconds * 0.012).min(1.0);
            }
            "Make Tools" => {
                needs.curiosity = (needs.curiosity - delta_seconds * 0.015).max(0.05);
                needs.fatigue = (needs.fatigue + delta_seconds * 0.012).min(1.0);
                needs.safety = (needs.safety + delta_seconds * 0.01).min(1.0);
            }
            "Stockpile" => {
                needs.safety = (needs.safety + delta_seconds * 0.02).min(1.0);
            }
            "Socialize" => {
                needs.social = (needs.social + delta_seconds * 0.08).min(1.0);
                needs.safety = (needs.safety + delta_seconds * 0.02).min(1.0);
            }
            "Retreat" => {
                needs.safety = (needs.safety + delta_seconds * 0.05).min(1.0);
                needs.fatigue = (needs.fatigue + delta_seconds * 0.01).min(1.0);
            }
            "Flee" => {
                needs.safety = (needs.safety + delta_seconds * 0.01).min(1.0);
                needs.fatigue = (needs.fatigue + delta_seconds * 0.04).min(1.0);
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
            "Hunt Predator" => {
                needs.safety = (needs.safety - delta_seconds * 0.02).max(0.0);
                needs.fatigue = (needs.fatigue + delta_seconds * 0.024).min(1.0);
            }
            "Explore" => {
                needs.curiosity = (needs.curiosity - delta_seconds * 0.03).max(0.1);
                needs.fatigue = (needs.fatigue + delta_seconds * 0.015).min(1.0);
            }
            _ => {}
        }
    }
}

fn harvest_npc_resources(
    mut commands: Commands,
    clock: Res<SimulationClock>,
    settings: Res<MapSettings>,
    mut writer: MessageWriter<LogEvent>,
    mut npcs: Query<
        (
            &Transform,
            &NpcIntent,
            &Needs,
            &mut Inventory,
            &mut Memory,
            &mut Npc,
        ),
        With<Npc>,
    >,
    mut regions: Query<(&RegionTile, &mut RegionState)>,
    mut trees: Query<(Entity, &Transform, &mut Tree)>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    for (transform, intent, needs, mut inventory, mut memory, mut npc) in &mut npcs {
        let coord = settings.tile_coord_for_position(transform.translation.truncate());

        match intent.label.as_str() {
            "Forage" => {
                if inventory.food_space() <= 0.0 {
                    continue;
                }

                for (tile, mut state) in &mut regions {
                    if tile.coord != coord {
                        continue;
                    }

                    let harvest = (0.55 + needs.hunger * 0.95) * delta_days;
                    let taken = harvest
                        .min(state.forage)
                        .min(inventory.food_space())
                        .max(0.0);
                    state.forage -= taken;
                    inventory.food += taken;

                    if taken > 0.0 {
                        memory.last_forage_coord = Some(coord);
                    }
                    break;
                }
            }
            "Gather Wood" => {
                if inventory.wood_space() <= 0.0 {
                    continue;
                }

                let npc_pos = transform.translation.truncate();
                let mut felled_tree = None;
                let mut chopped_any_tree = false;
                let mut chopped_amount = 0.0f32;

                for (tree_entity, tree_transform, mut tree) in &mut trees {
                    if tree.root_coord != coord {
                        continue;
                    }
                    if tree_transform.translation.truncate().distance(npc_pos)
                        > settings.tile_size * 0.95
                    {
                        continue;
                    }

                    let tool_bonus = npc.woodcutting_tools * 0.95;
                    let harvest = (0.16
                        + (1.0 - needs.safety) * 0.28
                        + npc.discovery_drive * 0.08
                        + tool_bonus)
                        * delta_days;
                    let tree_yield = match tree.stage {
                        TreeStage::Sapling => 0.06,
                        TreeStage::Young => 0.18,
                        TreeStage::Mature => 0.32,
                    };
                    let taken = harvest.min(inventory.wood_space()).min(tree_yield).max(0.0);

                    if taken <= 0.0 {
                        chopped_any_tree = true;
                        break;
                    }

                    inventory.wood += taken;
                    chopped_any_tree = true;
                    chopped_amount = taken;
                    tree.chop_progress += taken * (0.8 + tool_bonus * 1.8);
                    tree.growth = (tree.growth - taken * (0.55 + tool_bonus * 0.65)).max(0.0);
                    tree.stage = if tree.growth >= 0.9 {
                        TreeStage::Mature
                    } else if tree.growth >= 0.45 {
                        TreeStage::Young
                    } else {
                        TreeStage::Sapling
                    };

                    if tree.chop_progress >= 1.0 || tree.growth <= 0.05 {
                        felled_tree = Some(tree_entity);
                    }
                    if npc.woodcutting_tools > 0.0 {
                        npc.woodcutting_tools =
                            (npc.woodcutting_tools - delta_days * 0.002 - taken * 0.08).max(0.0);
                    }
                    break;
                }

                if !chopped_any_tree {
                    continue;
                }

                for (tile, mut state) in &mut regions {
                    if tile.coord != coord {
                        continue;
                    }
                    state.tree_biomass = (state.tree_biomass - chopped_amount).max(0.0);
                    break;
                }

                if let Some(tree_entity) = felled_tree {
                    commands.entity(tree_entity).despawn();
                    writer.write(LogEvent::new(
                        LogEventKind::Construction,
                        "A settler felled a tree for lumber".to_string(),
                    ));
                }
            }
            _ => {}
        }
    }
}

fn develop_and_craft_tools(
    clock: Res<SimulationClock>,
    mut writer: MessageWriter<LogEvent>,
    mut npcs: Query<(&mut Npc, &NpcIntent, &mut Needs), With<Npc>>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    for (mut npc, intent, needs) in &mut npcs {
        if intent.label != "Make Tools" {
            continue;
        }

        if npc.tool_knowledge < 1.0 {
            let before = npc.tool_knowledge;
            npc.tool_knowledge = (npc.tool_knowledge
                + delta_days * (0.004 + npc.discovery_drive * 0.005 + needs.curiosity * 0.003))
                .clamp(0.0, 1.0);
            if before < 1.0 && npc.tool_knowledge >= 1.0 {
                writer.write(LogEvent::new(
                    LogEventKind::Discovery,
                    format!("{} discovered how to make primitive tools", npc.name),
                ));
            }
        } else {
            let before = npc.woodcutting_tools;
            npc.woodcutting_tools = (npc.woodcutting_tools
                + delta_days
                    * (0.005 + npc.discovery_drive * 0.004)
                    * (1.08 - needs.fatigue * 0.30))
                .clamp(0.0, 1.0);
            if before < 0.35 && npc.woodcutting_tools >= 0.35 {
                writer.write(LogEvent::new(
                    LogEventKind::Construction,
                    format!("{} crafted an improved hand tool", npc.name),
                ));
            }
        }
    }
}

fn resolve_npc_violence(
    clock: Res<SimulationClock>,
    mut writer: MessageWriter<LogEvent>,
    mut npcs: Query<(&mut Npc, &Transform, &NpcIntent, &mut Needs), With<Npc>>,
    mut predators: Query<(Entity, &Transform, &mut Predator)>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    for (mut npc, transform, intent, mut needs) in &mut npcs {
        if intent.label != "Hunt Predator" {
            continue;
        }

        let pos = transform.translation.truncate();
        for (_, predator_transform, mut predator) in &mut predators {
            let distance = pos.distance(predator_transform.translation.truncate());
            if distance > 28.0 {
                continue;
            }

            let attack_power =
                (0.6 + npc.aggression_drive * 1.2 + npc.woodcutting_tools * 0.8) * delta_days;
            let retaliation = (0.35 + predator.hunger * 0.6 + predator.speed * 0.015) * delta_days;
            predator.health = (predator.health - attack_power).max(0.0);
            npc.health = (npc.health - retaliation * (1.0 - npc.risk_tolerance * 0.18)).max(0.0);
            needs.safety = (needs.safety - retaliation * 0.12).max(0.0);
            needs.fatigue = (needs.fatigue + retaliation * 0.08).min(1.0);

            if predator.health <= 0.0 {
                writer.write(LogEvent::new(
                    LogEventKind::Threat,
                    format!("{} killed a predator", npc.name),
                ));
            }
            break;
        }
    }
}

fn transfer_home_stockpiles(
    clock: Res<SimulationClock>,
    mut commands: Commands,
    mut writer: MessageWriter<LogEvent>,
    mut npcs: Query<(&Npc, &Transform, &NpcHome, &mut Inventory), With<Npc>>,
    mut shelters: Query<(Entity, Option<&mut ShelterStockpile>, &Transform), With<Shelter>>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    for (npc, npc_transform, home, mut inventory) in &mut npcs {
        let carried_before = inventory.food + inventory.wood;
        let Some(home_entity) = home.shelter else {
            continue;
        };
        let Ok((shelter_entity, stockpile, shelter_transform)) = shelters.get_mut(home_entity)
        else {
            continue;
        };
        let Some(mut stockpile) = stockpile else {
            commands
                .entity(shelter_entity)
                .insert(ShelterStockpile::default());
            continue;
        };

        let npc_pos = npc_transform.translation.truncate();
        let shelter_pos = shelter_transform.translation.truncate();
        if npc_pos.distance(shelter_pos) > 30.0 {
            continue;
        }

        let mut deposited = false;

        if inventory.food > 0.05 && stockpile.food < stockpile.max_food {
            let moved = (0.9 * delta_days)
                .min(inventory.food)
                .min((stockpile.max_food - stockpile.food).max(0.0));
            if moved > 0.0 {
                inventory.food -= moved;
                stockpile.food += moved;
                deposited = true;
            }
        }

        if inventory.wood > 0.05 && stockpile.wood < stockpile.max_wood {
            let moved = (0.9 * delta_days)
                .min(inventory.wood)
                .min((stockpile.max_wood - stockpile.wood).max(0.0));
            if moved > 0.0 {
                inventory.wood -= moved;
                stockpile.wood += moved;
                deposited = true;
            }
        }

        if deposited && carried_before > 0.2 && (inventory.food + inventory.wood) <= 0.02 {
            writer.write(LogEvent::new(
                LogEventKind::Discovery,
                format!(
                    "{} stocked supplies (F {:.1}, W {:.1})",
                    npc.name, stockpile.food, stockpile.wood
                ),
            ));
        }
    }
}

fn withdraw_home_food(
    clock: Res<SimulationClock>,
    mut commands: Commands,
    mut npcs: Query<
        (
            &Transform,
            &Needs,
            &NpcHome,
            Option<&FactionMember>,
            &mut Inventory,
        ),
        With<Npc>,
    >,
    shelter_positions: Query<(Entity, &Transform, Option<&FactionMember>), With<Shelter>>,
    mut shelters: Query<(Entity, Option<&mut ShelterStockpile>, &Transform), With<Shelter>>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    for (npc_transform, needs, home, member, mut inventory) in &mut npcs {
        if needs.hunger < 0.45 || inventory.food > 0.4 || inventory.food_space() <= 0.0 {
            continue;
        }

        let mut target_shelter = home.shelter;
        if target_shelter.is_none() {
            let Some(faction) = member.map(|member| member.faction) else {
                continue;
            };
            let npc_pos = npc_transform.translation.truncate();
            target_shelter = shelter_positions
                .iter()
                .filter(|(_, _, shelter_member)| {
                    shelter_member.map(|member| member.faction) == Some(faction)
                })
                .min_by(|(_, a, _), (_, b, _)| {
                    npc_pos
                        .distance(a.translation.truncate())
                        .total_cmp(&npc_pos.distance(b.translation.truncate()))
                })
                .map(|(entity, _, _)| entity);
        }

        let Some(target_shelter) = target_shelter else {
            continue;
        };
        let Ok((shelter_entity, stockpile, shelter_transform)) = shelters.get_mut(target_shelter)
        else {
            continue;
        };
        let Some(mut stockpile) = stockpile else {
            commands
                .entity(shelter_entity)
                .insert(ShelterStockpile::default());
            continue;
        };

        let npc_pos = npc_transform.translation.truncate();
        let shelter_pos = shelter_transform.translation.truncate();
        if npc_pos.distance(shelter_pos) > 30.0 {
            continue;
        }

        let moved = (0.9 * delta_days)
            .min(stockpile.food)
            .min(inventory.food_space());
        if moved > 0.0 {
            stockpile.food -= moved;
            inventory.food += moved;
        }
    }
}

fn consume_carried_food(
    clock: Res<SimulationClock>,
    mut npcs: Query<(&mut Needs, &mut Inventory), With<Npc>>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    for (mut needs, mut inventory) in &mut npcs {
        if inventory.food <= 0.0 || needs.hunger <= 0.02 {
            continue;
        }

        let appetite = (0.26 + needs.hunger * 0.7) * delta_days;
        let eaten = appetite.min(inventory.food);
        if eaten > 0.0 {
            inventory.food -= eaten;
            needs.hunger = (needs.hunger - eaten * 1.35).max(0.0);
            needs.fatigue = (needs.fatigue - eaten * 0.12).max(0.0);
        }
    }
}

fn apply_shelter_comfort(
    clock: Res<SimulationClock>,
    mut npcs: Query<(&Transform, &NpcHome, Option<&FactionMember>, &mut Needs)>,
    shelters: Query<(Entity, &Shelter, &Transform, Option<&FactionMember>)>,
    campfires: Query<(&Campfire, &Transform, Option<&FactionMember>)>,
) {
    let delta_seconds = clock.delta_seconds();
    if delta_seconds <= 0.0 {
        return;
    }

    for (npc_transform, home, member, mut needs) in &mut npcs {
        let pos = npc_transform.translation.truncate();
        let faction = member.map(|member| member.faction);

        let home_shelter = home
            .shelter
            .and_then(|entity| shelters.get(entity).ok())
            .map(|(_, shelter, transform, _)| (*shelter, transform.translation.truncate()));

        let (shelter, shelter_pos, is_home, is_friendly) = if let Some((shelter, shelter_pos)) =
            home_shelter
        {
            (shelter, shelter_pos, true, true)
        } else {
            let friendly = faction.and_then(|faction| {
                shelters
                    .iter()
                    .filter(|(_, _, _, member)| {
                        member.map(|member| member.faction) == Some(faction)
                    })
                    .min_by(|(_, _, a, _), (_, _, b, _)| {
                        pos.distance(a.translation.truncate())
                            .total_cmp(&pos.distance(b.translation.truncate()))
                    })
                    .map(|(_, shelter, transform, _)| (*shelter, transform.translation.truncate()))
            });

            if let Some((shelter, shelter_pos)) = friendly {
                (shelter, shelter_pos, false, true)
            } else if let Some((_, shelter, transform, _)) =
                shelters.iter().min_by(|(_, _, a, _), (_, _, b, _)| {
                    pos.distance(a.translation.truncate())
                        .total_cmp(&pos.distance(b.translation.truncate()))
                })
            {
                (*shelter, transform.translation.truncate(), false, false)
            } else {
                continue;
            }
        };

        let distance = pos.distance(shelter_pos);
        if distance > 48.0 {
            continue;
        }

        let integrity = shelter.integrity.clamp(0.0, 1.0);
        let comfort = shelter.safety_bonus
            * integrity
            * if is_home {
                1.4
            } else if is_friendly {
                0.9
            } else {
                0.7
            };
        let falloff = (1.0 - distance / 48.0).clamp(0.0, 1.0);
        let gain = comfort * falloff * delta_seconds;

        needs.safety = (needs.safety + gain).min(1.0);
        needs.fatigue = (needs.fatigue - gain * 0.4).max(0.0);

        for (campfire, fire_transform, fire_member) in &campfires {
            if fire_member.map(|member| member.faction) != faction
                && fire_member.is_some()
                && faction.is_some()
            {
                continue;
            }
            let fire_distance = pos.distance(fire_transform.translation.truncate());
            if fire_distance > 42.0 || campfire.ember <= 0.02 {
                continue;
            }

            let fire_falloff = (1.0 - fire_distance / 42.0).clamp(0.0, 1.0);
            let fire_gain = campfire.heat * campfire.ember * fire_falloff * delta_seconds * 0.7;
            needs.safety = (needs.safety + fire_gain).min(1.0);
            needs.fatigue = (needs.fatigue - fire_gain * 0.6).max(0.0);
        }
    }
}

fn apply_climate_stress(
    clock: Res<SimulationClock>,
    climate: Res<ClimateModel>,
    settings: Res<MapSettings>,
    regions: Query<(&RegionTile, &RegionClimate)>,
    shelters: Query<(&Shelter, &Transform)>,
    campfires: Query<(&Campfire, &Transform)>,
    mut npcs: Query<(&mut Npc, &Transform, &mut Needs)>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    let climate_by_coord: std::collections::HashMap<IVec2, (f32, f32)> = regions
        .iter()
        .map(|(tile, climate)| (tile.coord, (climate.pressure, tile.temperature)))
        .collect();

    let shelter_radius = 48.0;

    for (mut npc, transform, mut needs) in &mut npcs {
        let pos = transform.translation.truncate();
        let coord = settings.tile_coord_for_position(pos);
        let (pressure, temperature) = climate_by_coord
            .get(&coord)
            .copied()
            .unwrap_or((0.0, climate.comfort_temp));
        let pressure = pressure.clamp(0.0, 1.0);
        let cold_stress =
            ((climate.comfort_temp - temperature) / climate.comfort_band.max(0.01)).clamp(0.0, 1.3);

        let mut shelter_protection = 0.0f32;
        let mut fire_warmth = 0.0f32;
        for (shelter, shelter_transform) in &shelters {
            let d = pos.distance(shelter_transform.translation.truncate());
            if d > shelter_radius {
                continue;
            }
            let falloff = (1.0 - d / shelter_radius).clamp(0.0, 1.0);
            let integrity = shelter.integrity.clamp(0.0, 1.0);
            let protection = shelter.insulation * integrity * falloff;
            shelter_protection = shelter_protection.max(protection);
        }
        for (campfire, fire_transform) in &campfires {
            let d = pos.distance(fire_transform.translation.truncate());
            if d > 52.0 || campfire.ember <= 0.02 {
                continue;
            }
            let falloff = (1.0 - d / 52.0).clamp(0.0, 1.0);
            fire_warmth = fire_warmth.max(campfire.heat * campfire.ember * falloff);
        }

        let effective_pressure = (pressure - shelter_protection * 0.65).clamp(0.0, 1.0);
        let effective_cold = (cold_stress
            - shelter_protection * 1.25
            - fire_warmth * 1.6
            - climate.solar_factor() * 0.45
            + climate.lunar_factor() * 0.08)
            .clamp(0.0, 1.25);

        if effective_pressure > 0.01 {
            needs.thirst = (needs.thirst + effective_pressure * delta_days * 0.04).min(1.0);
            needs.hunger = (needs.hunger + effective_pressure * delta_days * 0.03).min(1.0);
            needs.fatigue = (needs.fatigue + effective_pressure * delta_days * 0.02).min(1.0);
            needs.safety = (needs.safety - effective_pressure * delta_days * 0.02).clamp(0.0, 1.0);
        }

        if effective_cold > 0.01 {
            npc.exposure = (npc.exposure + effective_cold * delta_days * 0.022).clamp(0.0, 2.0);
            needs.hunger = (needs.hunger + effective_cold * delta_days * 0.022).min(1.0);
            needs.fatigue = (needs.fatigue + effective_cold * delta_days * 0.018).min(1.0);
            needs.safety = (needs.safety - effective_cold * delta_days * 0.035).clamp(0.0, 1.0);
        } else {
            npc.exposure = (npc.exposure
                - delta_days
                    * (0.12
                        + shelter_protection * 0.10
                        + fire_warmth * 0.22
                        + climate.solar_factor() * 0.10))
            .max(0.0);
        }

        if (effective_pressure > 0.9 && shelter_protection < 0.10 && fire_warmth < 0.08)
            || npc.exposure > 1.30
        {
            let damage = (effective_pressure - 0.9).max(0.0) * delta_days * 4.0
                + (npc.exposure - 1.30).max(0.0) * delta_days * 6.0
                + effective_cold * delta_days * 0.45;
            npc.health = (npc.health - damage).max(0.0);
        }
    }
}

fn repair_npc_shelters(
    clock: Res<SimulationClock>,
    mut commands: Commands,
    mut writer: MessageWriter<LogEvent>,
    mut npcs: Query<(
        &Npc,
        &Transform,
        &NpcIntent,
        &NpcHome,
        &mut Needs,
        &mut Inventory,
    )>,
    mut shelters: Query<(&mut Shelter, Option<&mut ShelterStockpile>, &Transform), With<Shelter>>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    for (npc, npc_transform, intent, home, mut needs, mut inventory) in &mut npcs {
        if intent.label != "Repair Shelter" {
            continue;
        }

        let Some(shelter_entity) = home.shelter else {
            continue;
        };
        let Ok((mut shelter, stockpile, shelter_transform)) = shelters.get_mut(shelter_entity)
        else {
            continue;
        };
        let Some(mut stockpile) = stockpile else {
            commands
                .entity(shelter_entity)
                .insert(ShelterStockpile::default());
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

        let effort = (0.16 + (1.0 - needs.fatigue) * 0.10) * delta_days;
        let mut spent = 0.0f32;

        if stockpile.wood > 0.0 {
            let taken = effort.min(stockpile.wood).max(0.0);
            stockpile.wood -= taken;
            spent += taken;
        }

        if spent < effort && inventory.wood > 0.0 {
            let taken = (effort - spent).min(inventory.wood).max(0.0);
            inventory.wood -= taken;
            spent += taken;
        }

        if spent > 0.0 {
            shelter.integrity = (shelter.integrity + spent * 1.2).min(1.0);
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

fn tend_npc_campfires(
    clock: Res<SimulationClock>,
    mut npcs: Query<(&Transform, &NpcIntent, &mut Inventory), With<Npc>>,
    mut campfires: Query<(&mut Campfire, &Transform), With<Campfire>>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    for (npc_transform, intent, mut inventory) in &mut npcs {
        if intent.label != "Tend Fire" || inventory.wood <= 0.0 {
            continue;
        }

        let pos = npc_transform.translation.truncate();
        for (mut campfire, fire_transform) in &mut campfires {
            let distance = pos.distance(fire_transform.translation.truncate());
            if distance > 26.0 {
                continue;
            }

            let added = (0.35 * delta_days)
                .min(inventory.wood)
                .min((campfire.max_fuel - campfire.fuel).max(0.0));
            if added > 0.0 {
                inventory.wood -= added;
                campfire.fuel += added;
                campfire.ember = (campfire.ember + added * 0.9).clamp(0.0, 1.0);
                campfire.heat = 0.22 + campfire.ember * 0.72;
            }
            break;
        }
    }
}

fn build_npc_campfires(
    mut commands: Commands,
    mut npcs: Query<
        (
            &Transform,
            &NpcIntent,
            &mut Needs,
            &mut Inventory,
            Option<&FactionMember>,
        ),
        With<Npc>,
    >,
    campfires: Query<&Transform, With<Campfire>>,
) {
    for (transform, intent, mut needs, mut inventory, member) in &mut npcs {
        if intent.label != "Build Fire" {
            continue;
        }

        let pos = transform.translation.truncate();
        if campfires
            .iter()
            .any(|fire_transform| fire_transform.translation.truncate().distance(pos) < 34.0)
        {
            needs.safety = (needs.safety + 0.03).min(1.0);
            continue;
        }

        let fire_cost = 0.35;
        if inventory.wood < fire_cost {
            continue;
        }

        inventory.wood -= fire_cost;
        let fire_entity = commands
            .spawn((
                Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.0), Vec2::splat(1.0)),
                Transform::from_xyz(pos.x + 6.0, pos.y - 2.0, 1.7),
                Campfire {
                    fuel: 0.8,
                    max_fuel: 2.5,
                    heat: 0.8,
                    ember: 1.0,
                },
            ))
            .id();
        if let Some(member) = member {
            commands.entity(fire_entity).insert(*member);
        }

        needs.safety = (needs.safety + 0.10).min(1.0);
        needs.fatigue = (needs.fatigue - 0.03).max(0.0);
    }
}

fn build_npc_shelters(
    mut commands: Commands,
    mut writer: MessageWriter<LogEvent>,
    mut npcs: Query<
        (
            Entity,
            &Npc,
            &Transform,
            &mut Needs,
            &NpcIntent,
            &mut NpcHome,
            &mut Inventory,
            Option<&FactionMember>,
        ),
        With<Npc>,
    >,
    shelters: Query<&Transform, With<Shelter>>,
) {
    for (entity, npc, transform, mut needs, intent, mut home, mut inventory, member) in &mut npcs {
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

        let build_cost = 0.9;
        if inventory.wood < build_cost {
            needs.safety = (needs.safety + 0.01).min(1.0);
            continue;
        }

        inventory.wood -= build_cost;
        let faction = member.map(|member| member.faction);
        let shelter_entity = commands
            .spawn((
                Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.0), Vec2::splat(1.0)),
                Transform::from_xyz(pos.x + 10.0, pos.y - 4.0, 1.8),
                Shelter {
                    integrity: 1.0,
                    safety_bonus: 0.25,
                    insulation: 0.42,
                },
                ShelterStockpile::default(),
            ))
            .id();
        if let Some(faction) = faction {
            commands
                .entity(shelter_entity)
                .insert(FactionMember { faction });
        }

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
    }
}

fn tile_center(settings: &MapSettings, coord: IVec2) -> Vec2 {
    let bounds = settings.world_bounds();
    Vec2::new(
        coord.x as f32 * settings.tile_size - bounds.x + settings.tile_size * 0.5,
        coord.y as f32 * settings.tile_size - bounds.y + settings.tile_size * 0.5,
    )
}
