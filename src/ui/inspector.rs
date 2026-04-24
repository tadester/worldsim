use bevy::prelude::*;

use crate::agents::animal::{Animal, AnimalLifeStage, Pregnancy};
use crate::agents::decisions::NpcIntent;
use crate::agents::factions::{Faction, FactionMember};
use crate::agents::inventory::Inventory;
use crate::agents::memory::Memory;
use crate::agents::mind::NpcMind;
use crate::agents::needs::Needs;
use crate::agents::npc::{Npc, NpcHome};
use crate::agents::personality::NpcPsyche;
use crate::agents::predator::Predator;
use crate::agents::programs::KnownPrograms;
use crate::agents::society::{DiplomacyState, FactionSociety};
use crate::life::growth::Lifecycle;
use crate::life::reproduction::NpcPregnancy;
use crate::magic::mana::ManaReservoir;
use crate::ui::DiagnosticsSettingsPane;
use crate::world::climate::RegionClimate;
use crate::world::map::{MapSettings, RegionTile};
use crate::world::resources::{CivicStructure, Shelter, ShelterStockpile, Tree, TreeStage};
use crate::world::territory::Territory;

#[derive(Resource, Default)]
struct SelectedEntity {
    entity: Option<Entity>,
    index: usize,
}

#[derive(Component)]
struct InspectorText;

pub struct InspectorPlugin;

impl Plugin for InspectorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedEntity>()
            .add_systems(PostStartup, spawn_inspector)
            .add_systems(Update, (cycle_selected_entity, update_inspector));
    }
}

fn spawn_inspector(mut commands: Commands, settings_pane: Res<DiagnosticsSettingsPane>) {
    commands.entity(settings_pane.0).with_children(|parent| {
        parent
            .spawn((
                Node {
                    width: percent(100.0),
                    padding: UiRect::axes(px(14.0), px(12.0)),
                    border: UiRect::all(px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.10, 0.12, 0.08, 0.94)),
                BorderColor::all(Color::srgba(0.36, 0.42, 0.24, 0.86)),
            ))
            .with_child((
                Text::new("Inspector"),
                TextFont::from_font_size(14.0),
                TextColor(Color::srgb(0.96, 0.92, 0.84)),
                InspectorText,
            ));
    });
}

fn cycle_selected_entity(
    keys: Res<ButtonInput<KeyCode>>,
    trees: Query<(Entity, &Transform), With<Tree>>,
    shelters: Query<(Entity, &Transform), With<Shelter>>,
    civic_structures: Query<(Entity, &Transform), With<CivicStructure>>,
    animals: Query<(Entity, &Transform), With<Animal>>,
    predators: Query<(Entity, &Transform), With<Predator>>,
    npcs: Query<(Entity, &Transform), With<Npc>>,
    mut selected: ResMut<SelectedEntity>,
) {
    if !keys.just_pressed(KeyCode::Tab) {
        return;
    }

    let mut entities: Vec<(Entity, f32, f32)> =
        trees
            .iter()
            .map(|(entity, transform)| (entity, transform.translation.x, transform.translation.y))
            .chain(shelters.iter().map(|(entity, transform)| {
                (entity, transform.translation.x, transform.translation.y)
            }))
            .chain(civic_structures.iter().map(|(entity, transform)| {
                (entity, transform.translation.x, transform.translation.y)
            }))
            .chain(animals.iter().map(|(entity, transform)| {
                (entity, transform.translation.x, transform.translation.y)
            }))
            .chain(predators.iter().map(|(entity, transform)| {
                (entity, transform.translation.x, transform.translation.y)
            }))
            .chain(npcs.iter().map(|(entity, transform)| {
                (entity, transform.translation.x, transform.translation.y)
            }))
            .collect();

    entities.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.2.total_cmp(&b.2)));

    if entities.is_empty() {
        selected.entity = None;
        selected.index = 0;
        return;
    }

    selected.index = (selected.index + 1) % entities.len();
    selected.entity = Some(entities[selected.index].0);
}

fn update_inspector(
    selected: Res<SelectedEntity>,
    settings: Res<MapSettings>,
    trees: Query<(&Tree, &Transform, Option<&ManaReservoir>)>,
    shelters: Query<(
        &Shelter,
        Option<&ShelterStockpile>,
        &Transform,
        Option<&FactionMember>,
    )>,
    civic_structures: Query<(&CivicStructure, &Transform)>,
    animals: Query<(&Animal, &Lifecycle, &Transform, Option<&Pregnancy>)>,
    predators: Query<(&Predator, &Transform)>,
    factions: Query<&Faction>,
    faction_societies: Query<&FactionSociety>,
    regions: Query<(&RegionTile, &Territory)>,
    climates: Query<(&RegionTile, &RegionClimate)>,
    diplomacy: Res<DiplomacyState>,
    npcs: Query<(
        &Npc,
        &Lifecycle,
        &Needs,
        &Memory,
        &NpcIntent,
        &NpcHome,
        &Inventory,
        &ManaReservoir,
        &Transform,
        Option<&FactionMember>,
        Option<&NpcPregnancy>,
        Option<&NpcMind>,
        Option<&NpcPsyche>,
        Option<&KnownPrograms>,
    )>,
    mut query: Query<&mut Text, With<InspectorText>>,
) {
    let body = if let Some(entity) = selected.entity {
        if let Ok((tree, transform, mana)) = trees.get(entity) {
            format!(
                "Type: Tree\nStage: {}\nGrowth: {:.2}\nChop: {:.2}\nPos: {:.0}, {:.0}\nMana: {:.1}",
                tree_stage_label(tree.stage),
                tree.growth,
                tree.chop_progress,
                transform.translation.x,
                transform.translation.y,
                mana.map(|m| m.stored).unwrap_or(0.0),
            )
        } else if let Ok((shelter, stockpile, transform, member)) = shelters.get(entity) {
            let stockpile_line = stockpile
                .map(|pile| {
                    format!(
                        "Stockpile F/W: {:.1}/{:.1} (max {:.0}/{:.0})",
                        pile.food, pile.wood, pile.max_food, pile.max_wood
                    )
                })
                .unwrap_or_else(|| "Stockpile: none".to_string());
            let faction_line = member
                .and_then(|member| factions.get(member.faction).ok())
                .map(|faction| format!("Faction: {}", faction.name))
                .unwrap_or_else(|| "Faction: none".to_string());
            format!(
                "Type: Shelter\n{}\nIntegrity: {:.2}\nSafety bonus: {:.2}\nInsulation: {:.2}\n{}\nPos: {:.0}, {:.0}",
                faction_line,
                shelter.integrity,
                shelter.safety_bonus,
                shelter.insulation,
                stockpile_line,
                transform.translation.x,
                transform.translation.y,
            )
        } else if let Ok((structure, transform)) = civic_structures.get(entity) {
            format!(
                "Type: Civic Structure\nKind: {}\nProgress: {:.0}%\nPos: {:.0}, {:.0}",
                structure.kind.label(),
                structure.progress.clamp(0.0, 1.0) * 100.0,
                transform.translation.x,
                transform.translation.y,
            )
        } else if let Ok((animal, lifecycle, transform, pregnancy)) = animals.get(entity) {
            let animal_action = animal_action_label(animal, pregnancy);
            let animal_blocker = animal_blocked_reason(animal, lifecycle, pregnancy);
            let pregnancy_line = pregnancy
                .map(|pregnancy| {
                    format!(
                        "Pregnancy: yes ({:.1}d left)\nOffspring H/S: {:.1}/{:.1}",
                        pregnancy.gestation_days,
                        pregnancy.offspring_health,
                        pregnancy.offspring_speed
                    )
                })
                .unwrap_or_else(|| "Pregnancy: no".to_string());
            format!(
                "Type: Animal\nAction: {}\nTarget: wander heading {:.0}deg\nTop needs: {}\nBlocked: {}\nStage: {}\nAge: {:.1}d / mature {:.1}d\nHealth: {:.1}\nEnergy: {:.1}\nHunger: {:.2}\nCooldowns: reproduction {:.1}d\nFertility: {:.2}\nReproduction drive: {:.2}\n{}\nPos: {:.0}, {:.0}",
                animal_action,
                animal.wander_angle.to_degrees().rem_euclid(360.0),
                top_animal_needs(animal),
                animal_blocker,
                animal_life_stage_label(animal.life_stage),
                lifecycle.age_days,
                lifecycle.maturity_age,
                animal.health,
                animal.energy,
                animal.hunger,
                lifecycle.reproduction_cooldown,
                lifecycle.fertility,
                animal.reproduction_drive,
                pregnancy_line,
                transform.translation.x,
                transform.translation.y,
            )
        } else if let Ok((predator, transform)) = predators.get(entity) {
            format!(
                "Type: Predator\nHealth: {:.1}\nHunger: {:.2}\nSpeed: {:.1}\nPos: {:.0}, {:.0}",
                predator.health,
                predator.hunger,
                predator.speed,
                transform.translation.x,
                transform.translation.y,
            )
        } else if let Ok((
            npc,
            lifecycle,
            needs,
            memory,
            intent,
            home,
            inventory,
            mana,
            transform,
            member,
            pregnancy,
            mind,
            psyche,
            programs,
        )) = npcs.get(entity)
        {
            let home_line = home
                .shelter
                .and_then(|home_entity| shelters.get(home_entity).ok())
                .map(|(shelter, stockpile, _, _)| {
                    let mut line = format!("Home shelter integrity: {:.2}", shelter.integrity);
                    if let Some(pile) = stockpile {
                        line.push_str(&format!(
                            " | stockpile F/W {:.1}/{:.1}",
                            pile.food, pile.wood
                        ));
                    }
                    line
                })
                .unwrap_or_else(|| "Home shelter: none".to_string());
            let faction_line = member
                .and_then(|member| factions.get(member.faction).ok())
                .map(|faction| faction.name.as_str())
                .unwrap_or("none");
            let governance_line = member
                .and_then(|member| {
                    faction_societies
                        .get(member.faction)
                        .ok()
                        .map(|society| (member.faction, society))
                })
                .map(|(faction, society)| {
                    let war_line = diplomacy
                        .relations
                        .iter()
                        .find(|((left, right), pair)| {
                            pair.at_war && (*left == faction || *right == faction)
                        })
                        .map(|(_, pair)| format!("at war {:.2}", pair.hostility))
                        .unwrap_or_else(|| "at peace".to_string());
                    format!(
                        "Governance: {} | cohesion {:.2} | care {:.2} | {}",
                        society.governance.label(),
                        society.cohesion,
                        society.care_drive,
                        war_line
                    )
                })
                .unwrap_or_else(|| "Governance: none".to_string());

            let coord = settings.tile_coord_for_position(transform.translation.truncate());
            let territory_line = regions
                .iter()
                .find(|(tile, _)| tile.coord == coord)
                .and_then(|(_, territory)| {
                    territory.owner.and_then(|owner| {
                        factions.get(owner).ok().map(|faction| {
                            format!(
                                "{} ({:.2}{})",
                                faction.name,
                                territory.control,
                                if territory.contested {
                                    ", contested"
                                } else {
                                    ""
                                }
                            )
                        })
                    })
                })
                .unwrap_or_else(|| "unclaimed".to_string());

            let climate_line = climates
                .iter()
                .find(|(tile, _)| tile.coord == coord)
                .map(|(tile, climate)| {
                    format!(
                        "Climate: temp {:.2} | pressure {:.2}\nTerrain: elevation {:.2} | moisture {:.2} | mana {:.2}",
                        tile.temperature,
                        climate.pressure,
                        tile.elevation,
                        tile.moisture,
                        tile.mana_density,
                    )
                })
                .unwrap_or_else(|| "Climate: n/a".to_string());

            let mind_line = mind
                .map(|mind| {
                    format!(
                        "Mind: {} | goal: {}\nPlan: {}\nBelief: {}\nMind pressure/conf: {:.2}/{:.2}",
                        mind.mood,
                        mind.goal,
                        mind.plan,
                        mind.belief,
                        mind.pressure,
                        mind.confidence
                    )
                })
                .unwrap_or_else(|| "Mind: not initialized".to_string());
            let programs_line = programs
                .map(|programs| {
                    format!(
                        "Programs: {} known | {}\nProgram note: {}\nWorld grants: {} | last: {}",
                        programs.known.len(),
                        programs.names(12),
                        programs.featured_summary(),
                        programs.granted_by_world.len(),
                        programs.last_grant_reason
                    )
                })
                .unwrap_or_else(|| "Programs: not initialized".to_string());
            let psyche_line = psyche
                .map(|psyche| {
                    format!(
                        "Psyche: {} | happiness {:.2}\nSins P/G/L/E/Gl/W/Sl: {:.2}/{:.2}/{:.2}/{:.2}/{:.2}/{:.2}/{:.2}",
                        psyche.personality.label(),
                        psyche.happiness,
                        psyche.pride,
                        psyche.greed,
                        psyche.lust,
                        psyche.envy,
                        psyche.gluttony,
                        psyche.wrath,
                        psyche.sloth
                    )
                })
                .unwrap_or_else(|| "Psyche: not initialized".to_string());

            format!(
                "Type: NPC\nName: {}\nSex/Gender: {} / {}\nAge: {:.0}y / mature {:.0}y\nFaction: {}\n{}\nTile: {},{}\nTerritory: {}\n{}\n{}\n{}\n{}\nHealth: {:.1}\nAction: {}\nTarget: {}\nHeading: {:.2},{:.2}\nTop needs: {}\nBlocked: {}\nNeeds H/T/F/S/Soc/C: {:.2}/{:.2}/{:.2}/{:.2}/{:.2}/{:.2}\nCooldowns: reproduction {:.1}d\nFertility: {:.2}\nReproduction: drive {:.2} | {}\nDrives D/A/Risk: {:.2}/{:.2}/{:.2}\nCarry F/W: {:.1}/{:.1}\nCraft S/Fi/Hi/O/M/C/W: {:.1}/{:.1}/{:.1}/{:.1}/{:.1}/{:.1}/{:.1}\nTools K/T: {:.2}/{:.2}\nExposure: {:.2}\nMana: {:.1}/{:.1}\n{}\nInsight: {}\nPos: {:.0}, {:.0}",
                npc.name,
                npc.sex.label(),
                npc.gender.label(),
                lifecycle.age_days / 365.0,
                lifecycle.maturity_age / 365.0,
                faction_line,
                governance_line,
                coord.x,
                coord.y,
                territory_line,
                climate_line,
                mind_line,
                psyche_line,
                programs_line,
                npc.health,
                intent.label,
                target_label(intent.target),
                intent.heading.x,
                intent.heading.y,
                top_npc_needs(needs),
                intent.blocked_reason,
                needs.hunger,
                needs.thirst,
                needs.fatigue,
                needs.safety,
                needs.social,
                needs.curiosity,
                lifecycle.reproduction_cooldown,
                lifecycle.fertility,
                npc.reproduction_drive,
                npc_reproduction_state(npc, lifecycle, needs, home, pregnancy),
                npc.discovery_drive,
                npc.aggression_drive,
                npc.risk_tolerance,
                inventory.food,
                inventory.wood,
                inventory.seeds,
                inventory.fiber,
                inventory.hides,
                inventory.ore,
                inventory.metal,
                inventory.clothing,
                inventory.weapons,
                npc.tool_knowledge,
                npc.woodcutting_tools,
                npc.exposure,
                mana.stored,
                mana.capacity,
                home_line,
                memory.last_mana_insight,
                transform.translation.x,
                transform.translation.y,
            )
        } else {
            "Selected entity no longer exists".to_string()
        }
    } else {
        "No entity selected\nPress Tab to cycle through trees, shelters, civic structures, animals, predators, and NPCs"
            .to_string()
    };

    for mut text in &mut query {
        *text = Text::new(format!("Inspector\n{}", body));
    }
}

fn tree_stage_label(stage: TreeStage) -> &'static str {
    match stage {
        TreeStage::Sapling => "Sapling",
        TreeStage::Young => "Young",
        TreeStage::Mature => "Mature",
    }
}

fn animal_life_stage_label(stage: AnimalLifeStage) -> &'static str {
    match stage {
        AnimalLifeStage::Juvenile => "Juvenile",
        AnimalLifeStage::Adult => "Adult",
        AnimalLifeStage::Elder => "Elder",
    }
}

fn animal_action_label(animal: &Animal, pregnancy: Option<&Pregnancy>) -> &'static str {
    if pregnancy.is_some() {
        "Gestating while foraging/wandering"
    } else if animal.hunger > 0.35 {
        "Forage"
    } else if animal.energy < 12.0 {
        "Tired wander"
    } else {
        "Wander"
    }
}

fn animal_blocked_reason(
    animal: &Animal,
    lifecycle: &Lifecycle,
    pregnancy: Option<&Pregnancy>,
) -> String {
    let mut blockers = Vec::new();
    if animal.life_stage != AnimalLifeStage::Adult {
        blockers.push("Reproduction blocked: not adult");
    }
    if lifecycle.age_days < lifecycle.maturity_age {
        blockers.push("Reproduction blocked: below maturity age");
    }
    if lifecycle.reproduction_cooldown > 0.0 {
        blockers.push("Reproduction blocked: cooldown active");
    }
    if animal.health < 30.0 {
        blockers.push("Reproduction blocked: health below 30");
    }
    if animal.energy < 28.0 {
        blockers.push("Reproduction blocked: energy below 28");
    }
    if pregnancy.is_some() {
        blockers.push("Reproduction blocked: already pregnant");
    }
    if animal.energy <= 0.0 {
        blockers.push("Movement blocked: no energy");
    }

    if blockers.is_empty() {
        "None".to_string()
    } else {
        blockers.join(" | ")
    }
}

fn top_animal_needs(animal: &Animal) -> String {
    let energy_need = (1.0 - animal.energy / 60.0).clamp(0.0, 1.0);
    let health_need = (1.0 - animal.health / 40.0).clamp(0.0, 1.0);
    let mut scores = [
        ("hunger", animal.hunger.clamp(0.0, 1.0)),
        ("energy", energy_need),
        ("health", health_need),
        ("repro", animal.reproduction_drive.clamp(0.0, 1.0)),
    ];
    scores.sort_by(|a, b| b.1.total_cmp(&a.1));
    format!(
        "{} {:.2}, {} {:.2}, {} {:.2}",
        scores[0].0, scores[0].1, scores[1].0, scores[1].1, scores[2].0, scores[2].1
    )
}

fn top_npc_needs(needs: &Needs) -> String {
    let mut scores = [
        ("hunger", needs.hunger),
        ("thirst", needs.thirst),
        ("fatigue", needs.fatigue),
        ("unsafe", 1.0 - needs.safety),
        ("social", 1.0 - needs.social),
        ("curiosity", needs.curiosity),
    ];
    scores.sort_by(|a, b| b.1.total_cmp(&a.1));
    format!(
        "{} {:.2}, {} {:.2}, {} {:.2}",
        scores[0].0, scores[0].1, scores[1].0, scores[1].1, scores[2].0, scores[2].1
    )
}

fn target_label(target: Option<Vec2>) -> String {
    target
        .map(|target| format!("{:.0}, {:.0}", target.x, target.y))
        .unwrap_or_else(|| "none".to_string())
}

fn npc_reproduction_state(
    npc: &Npc,
    lifecycle: &Lifecycle,
    needs: &Needs,
    home: &NpcHome,
    pregnancy: Option<&NpcPregnancy>,
) -> String {
    if let Some(pregnancy) = pregnancy {
        return format!("pregnant ({:.1}d left)", pregnancy.gestation_days);
    }

    let mut blockers = Vec::new();
    if lifecycle.age_days < lifecycle.maturity_age {
        blockers.push("immature");
    }
    if lifecycle.reproduction_cooldown > 0.0 {
        blockers.push("cooldown");
    }
    if npc.health < 32.0 {
        blockers.push("health < 32");
    }
    if needs.hunger > (0.74 - npc.reproduction_drive * 0.10).clamp(0.40, 0.82) {
        blockers.push("too hungry");
    }
    if needs.fatigue > (0.90 - npc.reproduction_drive * 0.08).clamp(0.55, 0.92) {
        blockers.push("too fatigued");
    }
    if needs.safety < (0.18 - npc.risk_tolerance * 0.04).clamp(0.05, 0.22) {
        blockers.push("unsafe");
    }
    let shelter_context = if npc.sex == crate::agents::npc::NpcSex::Female && home.shelter.is_none()
    {
        " | no shelter bonus"
    } else {
        ""
    };

    if blockers.is_empty() {
        format!(
            "eligible pending partner/resources/cycle{}",
            shelter_context
        )
    } else {
        format!("blocked: {}", blockers.join(", "))
    }
}
