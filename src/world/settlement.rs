use bevy::prelude::*;

use crate::agents::factions::{Faction, FactionMember};
use crate::agents::npc::{Npc, NpcHome};
use crate::agents::personality::NpcPsyche;
use crate::life::population::PopulationStats;
use crate::systems::simulation::SimulationStep;
use crate::world::resources::{CivicStructure, Shelter, ShelterStockpile};

#[derive(Component, Debug, Clone)]
pub struct Settlement {
    pub name: String,
    pub faction: Entity,
    pub center: Vec2,
    pub population: usize,
    pub shelters: usize,
    pub civic_structures: usize,
    pub food_security: f32,
    pub housing_security: f32,
    pub happiness: f32,
    pub births: usize,
    pub deaths: usize,
    pub civic_level: u8,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct SettlementMember {
    pub settlement: Entity,
}

pub struct SettlementPlugin;

impl Plugin for SettlementPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (spawn_missing_settlements, update_settlements).chain(),
        );
    }
}

fn spawn_missing_settlements(
    mut commands: Commands,
    factions: Query<(Entity, &Faction), Without<Settlement>>,
    settlements: Query<&Settlement>,
) {
    for (faction_entity, faction) in &factions {
        if settlements
            .iter()
            .any(|settlement| settlement.faction == faction_entity)
        {
            continue;
        }
        commands.spawn((
            Sprite::from_color(Color::srgba(0.95, 0.86, 0.45, 0.0), Vec2::splat(1.0)),
            Transform::from_xyz(0.0, 0.0, 1.25),
            Settlement {
                name: format!("{} Hearth", faction.name),
                faction: faction_entity,
                center: Vec2::ZERO,
                population: 0,
                shelters: 0,
                civic_structures: 0,
                food_security: 0.0,
                housing_security: 0.0,
                happiness: 0.5,
                births: 0,
                deaths: 0,
                civic_level: 0,
            },
        ));
    }
}

fn update_settlements(
    mut commands: Commands,
    step: Res<SimulationStep>,
    population: Res<PopulationStats>,
    mut settlements: Query<(Entity, &mut Settlement, &mut Transform, &mut Sprite)>,
    npcs: Query<
        (
            Entity,
            &Transform,
            &FactionMember,
            &NpcHome,
            Option<&NpcPsyche>,
            Option<&SettlementMember>,
        ),
        (With<Npc>, Without<Settlement>),
    >,
    shelters: Query<
        (&Transform, &FactionMember, Option<&ShelterStockpile>),
        (With<Shelter>, Without<Settlement>),
    >,
    civic_structures: Query<&Transform, (With<CivicStructure>, Without<Settlement>)>,
) {
    for (settlement_entity, mut settlement, mut transform, mut sprite) in &mut settlements {
        let mut center_sum = Vec2::ZERO;
        let mut anchors = 0.0f32;
        let mut food = 0.0f32;
        let mut wood = 0.0f32;
        let mut shelter_count = 0usize;
        for (shelter_transform, member, stockpile) in &shelters {
            if member.faction != settlement.faction {
                continue;
            }
            center_sum += shelter_transform.translation.truncate();
            anchors += 1.0;
            shelter_count += 1;
            if let Some(stockpile) = stockpile {
                food += stockpile.food;
                wood += stockpile.wood;
            }
        }

        let mut npc_count = 0usize;
        let mut happiness = 0.0f32;
        for (npc_entity, npc_transform, member, home, psyche, existing_membership) in &npcs {
            if member.faction != settlement.faction {
                continue;
            }
            npc_count += 1;
            happiness += psyche.map(|p| p.happiness).unwrap_or(0.5);
            if home.shelter.is_none() {
                center_sum += npc_transform.translation.truncate();
                anchors += 0.25;
            }
            if existing_membership
                .map(|membership| membership.settlement != settlement_entity)
                .unwrap_or(true)
            {
                commands.entity(npc_entity).insert(SettlementMember {
                    settlement: settlement_entity,
                });
            }
        }

        if anchors > 0.0 {
            settlement.center = center_sum / anchors;
        }

        settlement.population = npc_count;
        settlement.shelters = shelter_count;
        settlement.civic_structures = civic_structures
            .iter()
            .filter(|transform| {
                transform.translation.truncate().distance(settlement.center) < 180.0
            })
            .count();
        settlement.food_security = (food / npc_count.max(1) as f32).clamp(0.0, 3.0);
        settlement.housing_security =
            (shelter_count as f32 * 2.0 / npc_count.max(1) as f32).clamp(0.0, 1.4);
        settlement.happiness = if npc_count > 0 {
            happiness / npc_count as f32
        } else {
            (settlement.happiness - 0.002).max(0.0)
        };
        settlement.births = population.npc_births;
        settlement.deaths = population.npc_deaths;
        settlement.civic_level = civic_level(
            settlement.population,
            settlement.shelters,
            settlement.civic_structures,
            settlement.happiness,
        );

        let name_warmth = (settlement.name.len() as f32 / 32.0).clamp(0.0, 0.08);
        transform.translation.x = settlement.center.x;
        transform.translation.y = settlement.center.y;
        transform.translation.z = 1.25;
        let pulse = ((step.elapsed_days * 4.0).sin() * 0.5 + 0.5) * 0.12;
        let size = 26.0 + settlement.civic_level as f32 * 6.0 + wood.min(8.0);
        sprite.custom_size = Some(Vec2::splat(size));
        sprite.color = Color::srgba(
            0.95,
            0.80 + settlement.happiness * 0.12 + name_warmth,
            0.34,
            (0.08 + settlement.happiness * 0.10 + pulse).clamp(0.0, 0.32),
        );
    }
}

fn civic_level(population: usize, shelters: usize, civic_structures: usize, happiness: f32) -> u8 {
    if population >= 18 && shelters >= 9 && civic_structures >= 7 && happiness > 0.58 {
        4
    } else if population >= 12 && shelters >= 6 && civic_structures >= 4 && happiness > 0.50 {
        3
    } else if population >= 7 && shelters >= 3 && civic_structures >= 2 {
        2
    } else if population >= 4 && shelters >= 2 {
        1
    } else {
        0
    }
}
