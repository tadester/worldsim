use bevy::prelude::*;

use crate::agents::npc::{Npc, NpcHome};
use crate::world::resources::Shelter;

#[derive(Component, Debug, Clone)]
pub struct Faction {
    pub name: String,
    pub color: Color,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct FactionMember {
    pub faction: Entity,
}

#[derive(Resource, Debug, Default, Clone)]
pub struct FactionRoster {
    pub factions: Vec<Entity>,
}

pub struct FactionPlugin;

impl Plugin for FactionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FactionRoster>()
            .add_systems(Startup, spawn_default_factions)
            .add_systems(
                Update,
                (
                    assign_unaligned_npcs,
                    attach_faction_marks,
                    sync_faction_marks,
                    assign_unaligned_shelters,
                )
                    .chain(),
            );
    }
}

#[derive(Component)]
struct NpcFactionMark;

#[derive(Component)]
struct FactionMarked;

fn spawn_default_factions(mut commands: Commands, mut roster: ResMut<FactionRoster>) {
    if !roster.factions.is_empty() {
        return;
    }

    let river_union = commands
        .spawn(Faction {
            name: "River Union".to_string(),
            color: Color::srgb(0.20, 0.46, 0.86),
        })
        .id();
    let ash_clan = commands
        .spawn(Faction {
            name: "Ash Clan".to_string(),
            color: Color::srgb(0.84, 0.42, 0.16),
        })
        .id();

    roster.factions = vec![river_union, ash_clan];
}

fn assign_unaligned_npcs(
    roster: Res<FactionRoster>,
    mut commands: Commands,
    npcs: Query<(Entity, &Transform), (With<Npc>, Without<FactionMember>)>,
) {
    if roster.factions.is_empty() {
        return;
    }

    let left = roster.factions[0];
    let right = roster.factions[roster.factions.len().min(2) - 1];

    for (entity, transform) in &npcs {
        let faction = if transform.translation.x < 0.0 {
            left
        } else {
            right
        };
        commands.entity(entity).insert(FactionMember { faction });
    }
}

fn attach_faction_marks(
    mut commands: Commands,
    npcs: Query<Entity, (With<Npc>, Added<FactionMember>, Without<FactionMarked>)>,
) {
    for entity in &npcs {
        commands
            .entity(entity)
            .insert(FactionMarked)
            .with_children(|parent| {
                parent.spawn((
                    Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.0), Vec2::new(8.0, 3.0)),
                    Transform::from_xyz(0.0, 14.0, 0.35),
                    NpcFactionMark,
                ));
            });
    }
}

fn sync_faction_marks(
    factions: Query<&Faction>,
    npcs: Query<
        (&FactionMember, &Children),
        (
            With<Npc>,
            Or<(Changed<FactionMember>, Added<FactionMember>)>,
        ),
    >,
    mut marks: Query<&mut Sprite, With<NpcFactionMark>>,
) {
    for (member, children) in &npcs {
        let Ok(faction) = factions.get(member.faction) else {
            continue;
        };
        for child in children.iter() {
            if let Ok(mut sprite) = marks.get_mut(child) {
                sprite.color = faction.color;
            }
        }
    }
}

fn assign_unaligned_shelters(
    mut commands: Commands,
    roster: Res<FactionRoster>,
    npcs: Query<(&NpcHome, &FactionMember), With<Npc>>,
    shelters: Query<Entity, (With<Shelter>, Without<FactionMember>)>,
) {
    if roster.factions.is_empty() {
        return;
    }

    let mut home_map = std::collections::HashMap::<Entity, Entity>::new();
    for (home, member) in &npcs {
        if let Some(shelter) = home.shelter {
            home_map.insert(shelter, member.faction);
        }
    }

    for shelter in &shelters {
        if let Some(&faction) = home_map.get(&shelter) {
            commands.entity(shelter).insert(FactionMember { faction });
        }
    }
}
