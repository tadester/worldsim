use bevy::prelude::*;

use crate::agents::animal::Animal;
use crate::agents::decisions::NpcIntent;
use crate::agents::factions::{Faction, FactionMember};
use crate::agents::inventory::Inventory;
use crate::agents::memory::Memory;
use crate::agents::needs::Needs;
use crate::agents::npc::{Npc, NpcHome};
use crate::agents::predator::Predator;
use crate::life::growth::Lifecycle;
use crate::magic::mana::ManaReservoir;
use crate::ui::DiagnosticsSettingsPane;
use crate::world::climate::RegionClimate;
use crate::world::map::{MapSettings, RegionTile};
use crate::world::resources::{Shelter, ShelterStockpile, Tree, TreeStage};
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
    animals: Query<(&Animal, &Lifecycle, &Transform)>,
    predators: Query<(&Predator, &Transform)>,
    factions: Query<&Faction>,
    regions: Query<(&RegionTile, &Territory)>,
    climates: Query<(&RegionTile, &RegionClimate)>,
    npcs: Query<(
        &Npc,
        &Needs,
        &Memory,
        &NpcIntent,
        &NpcHome,
        &Inventory,
        &ManaReservoir,
        &Transform,
        Option<&FactionMember>,
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
        } else if let Ok((animal, lifecycle, transform)) = animals.get(entity) {
            format!(
                "Type: Animal\nAge: {:.1}\nHealth: {:.1}\nEnergy: {:.1}\nHunger: {:.2}\nPos: {:.0}, {:.0}",
                lifecycle.age_days,
                animal.health,
                animal.energy,
                animal.hunger,
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
        } else if let Ok((npc, needs, memory, intent, home, inventory, mana, transform, member)) =
            npcs.get(entity)
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

            format!(
                "Type: NPC\nName: {}\nFaction: {}\nTile: {},{}\nTerritory: {}\n{}\nHealth: {:.1}\nIntent: {}\nNeeds H/S/C: {:.2}/{:.2}/{:.2}\nCarry F/W: {:.1}/{:.1}\nMana: {:.1}/{:.1}\n{}\nInsight: {}\nPos: {:.0}, {:.0}",
                npc.name,
                faction_line,
                coord.x,
                coord.y,
                territory_line,
                climate_line,
                npc.health,
                intent.label,
                needs.hunger,
                needs.safety,
                needs.curiosity,
                inventory.food,
                inventory.wood,
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
        "No entity selected\nPress Tab to cycle through trees, shelters, animals, predators, and NPCs"
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
