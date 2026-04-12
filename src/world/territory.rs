use bevy::prelude::*;

use crate::agents::factions::{Faction, FactionMember, FactionRoster};
use crate::agents::npc::Npc;
use crate::systems::logging::{LogEvent, LogEventKind};
use crate::systems::simulation::SimulationClock;
use crate::world::map::{MapSettings, RegionTile};
use crate::world::resources::Shelter;

#[derive(Component, Debug, Clone, Copy)]
pub struct Territory {
    pub owner: Option<Entity>,
    pub control: f32,
    pub contested: bool,
}

impl Default for Territory {
    fn default() -> Self {
        Self {
            owner: None,
            control: 0.0,
            contested: false,
        }
    }
}

#[derive(Component)]
struct TerritoryOverlay;

#[derive(Resource, Default)]
struct TerritoryMilestones {
    progress: std::collections::HashMap<Entity, usize>,
}

pub struct TerritoryPlugin;

impl Plugin for TerritoryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerritoryMilestones>()
            .add_systems(
                PreUpdate,
                (attach_territory_overlays, update_territory_control).chain(),
            )
            .add_systems(
                PostUpdate,
                (sync_territory_overlays, announce_territory_milestones).chain(),
            );
    }
}

fn attach_territory_overlays(
    mut commands: Commands,
    settings: Res<MapSettings>,
    regions: Query<Entity, (With<RegionTile>, Without<Territory>)>,
) {
    let overlay_size = Vec2::splat((settings.tile_size - 6.0).max(4.0));

    for entity in &regions {
        commands
            .entity(entity)
            .insert(Territory::default())
            .with_children(|parent| {
                parent.spawn((
                    Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.0), overlay_size),
                    Transform::from_xyz(0.0, 0.0, 0.05),
                    TerritoryOverlay,
                ));
            });
    }
}

pub fn update_territory_control(
    clock: Res<SimulationClock>,
    settings: Res<MapSettings>,
    roster: Res<FactionRoster>,
    npcs: Query<(&Transform, &FactionMember), With<Npc>>,
    shelters: Query<(&Transform, &FactionMember), With<Shelter>>,
    mut regions: Query<(&RegionTile, &mut Territory)>,
) {
    if roster.factions.is_empty() {
        return;
    }

    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    let mut faction_index = std::collections::HashMap::<Entity, usize>::new();
    for (idx, faction) in roster.factions.iter().copied().enumerate() {
        faction_index.insert(faction, idx);
    }

    let npc_presences: Vec<(IVec2, usize, f32)> = npcs
        .iter()
        .filter_map(|(transform, member)| {
            let idx = *faction_index.get(&member.faction)?;
            let coord = settings.tile_coord_for_position(transform.translation.truncate());
            Some((coord, idx, 1.0))
        })
        .collect();

    let shelter_presences: Vec<(IVec2, usize, f32)> = shelters
        .iter()
        .filter_map(|(transform, member)| {
            let idx = *faction_index.get(&member.faction)?;
            let coord = settings.tile_coord_for_position(transform.translation.truncate());
            Some((coord, idx, 1.6))
        })
        .collect();

    let npc_radius = 3.6f32;
    let shelter_radius = 5.2f32;
    let gain = (delta_days * 0.55).clamp(0.0, 0.25);

    for (tile, mut territory) in &mut regions {
        let mut scores = vec![0.0f32; roster.factions.len()];

        for (coord, idx, weight) in npc_presences.iter().copied() {
            let delta = tile.coord - coord;
            let d = Vec2::new(delta.x as f32, delta.y as f32).length();
            if d > npc_radius {
                continue;
            }
            scores[idx] += (npc_radius + 0.4 - d).max(0.0) * weight;
        }

        for (coord, idx, weight) in shelter_presences.iter().copied() {
            let delta = tile.coord - coord;
            let d = Vec2::new(delta.x as f32, delta.y as f32).length();
            if d > shelter_radius {
                continue;
            }
            scores[idx] += (shelter_radius + 0.5 - d).max(0.0) * weight;
        }

        let mut best_idx = None;
        let mut best_score = 0.0;
        let mut second_score = 0.0;
        for (idx, score) in scores.iter().copied().enumerate() {
            if score > best_score {
                second_score = best_score;
                best_score = score;
                best_idx = Some(idx);
            } else if score > second_score {
                second_score = score;
            }
        }

        let desired_owner = if best_score >= 1.2 {
            best_idx.map(|idx| roster.factions[idx])
        } else {
            None
        };
        let contested = best_score >= 1.2 && second_score >= (best_score * 0.75).max(0.9);

        if territory.owner == desired_owner {
            territory.control =
                (territory.control + gain * if contested { 0.6 } else { 1.0 }).clamp(0.0, 1.0);
        } else {
            territory.control = (territory.control - gain).clamp(0.0, 1.0);
            if territory.control <= 0.05 {
                territory.owner = desired_owner;
                territory.control = if territory.owner.is_some() { 0.22 } else { 0.0 };
            }
        }

        territory.contested = contested;
    }
}

fn sync_territory_overlays(
    factions: Query<&Faction>,
    territories: Query<&Territory>,
    mut overlays: Query<(&ChildOf, &mut Sprite), With<TerritoryOverlay>>,
) {
    for (parent, mut sprite) in &mut overlays {
        let Ok(territory) = territories.get(parent.parent()) else {
            continue;
        };

        let Some(owner) = territory.owner else {
            sprite.color = Color::srgba(0.0, 0.0, 0.0, 0.0);
            continue;
        };

        let Ok(faction) = factions.get(owner) else {
            sprite.color = Color::srgba(0.0, 0.0, 0.0, 0.0);
            continue;
        };

        let base = faction.color.to_srgba();
        let alpha =
            (0.08 + territory.control * 0.18 + if territory.contested { 0.06 } else { 0.0 })
                .clamp(0.0, 0.34);
        let (r, g, b) = if territory.contested {
            (
                (base.red + 0.95) * 0.5,
                (base.green + 0.90) * 0.5,
                (base.blue + 0.35) * 0.5,
            )
        } else {
            (base.red, base.green, base.blue)
        };
        sprite.color = Color::srgba(r, g, b, alpha);
    }
}

fn announce_territory_milestones(
    factions: Query<&Faction>,
    territories: Query<&Territory>,
    mut milestones: ResMut<TerritoryMilestones>,
    mut writer: MessageWriter<LogEvent>,
) {
    const STEPS: [usize; 5] = [10, 25, 50, 80, 120];

    let mut counts = std::collections::HashMap::<Entity, usize>::new();
    for territory in &territories {
        if let Some(owner) = territory.owner {
            *counts.entry(owner).or_insert(0) += 1;
        }
    }

    for (faction_entity, count) in counts {
        let Ok(faction) = factions.get(faction_entity) else {
            continue;
        };

        let idx = milestones.progress.entry(faction_entity).or_insert(0);
        while *idx < STEPS.len() && count >= STEPS[*idx] {
            writer.write(LogEvent::new(
                LogEventKind::Territory,
                format!("{} holds {} tiles of territory", faction.name, count),
            ));
            *idx += 1;
        }
    }
}
