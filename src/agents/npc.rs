use bevy::prelude::*;

use crate::agents::decisions::NpcIntent;
use crate::agents::memory::Memory;
use crate::agents::needs::Needs;
use crate::agents::relationships::Relationships;
use crate::life::growth::Lifecycle;
use crate::magic::mana::ManaReservoir;
use crate::magic::storage::{ManaPractice, ManaStorageStyle};

#[derive(Component)]
struct NpcTorso;

#[derive(Component)]
struct NpcHead;

#[derive(Component)]
struct NpcLeg;

#[derive(Component)]
struct NpcAura;

#[derive(Component, Debug, Clone)]
pub struct Npc {
    pub name: String,
    pub health: f32,
    pub curiosity: f32,
    pub speed: f32,
}

#[derive(Bundle)]
pub struct NpcBundle {
    pub sprite: Sprite,
    pub transform: Transform,
    pub npc: Npc,
    pub lifecycle: Lifecycle,
    pub needs: Needs,
    pub memory: Memory,
    pub relationships: Relationships,
    pub intent: NpcIntent,
    pub mana_reservoir: ManaReservoir,
    pub mana_style: ManaStorageStyle,
    pub mana_practice: ManaPractice,
}

impl NpcBundle {
    pub fn new(
        position: Vec2,
        name: String,
        health: f32,
        mana_reservoir: ManaReservoir,
        mana_style: ManaStorageStyle,
    ) -> Self {
        Self {
            sprite: Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.0), Vec2::splat(1.0)),
            transform: Transform::from_xyz(position.x, position.y, 4.0),
            npc: Npc {
                name,
                health,
                curiosity: 0.6,
                speed: 28.0,
            },
            lifecycle: Lifecycle {
                age_days: 0.0,
                maturity_age: 120.0,
                max_age: 24_000.0,
                fertility: 0.25,
                reproduction_cooldown: 0.0,
            },
            needs: Needs::default_humanoid(),
            memory: Memory::default(),
            relationships: Relationships::default(),
            intent: NpcIntent::default(),
            mana_reservoir,
            mana_style,
            mana_practice: ManaPractice::default(),
        }
    }
}

pub struct NpcPlugin;

impl Plugin for NpcPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (attach_npc_visuals, sync_npc_visuals));
    }
}

fn attach_npc_visuals(mut commands: Commands, npcs: Query<Entity, Added<Npc>>) {
    for entity in &npcs {
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                Sprite::from_color(Color::srgb(0.17, 0.28, 0.58), Vec2::new(10.0, 14.0)),
                Transform::from_xyz(0.0, -1.0, 0.1),
                NpcTorso,
            ));
            parent.spawn((
                Sprite::from_color(Color::srgb(0.92, 0.82, 0.68), Vec2::new(8.0, 8.0)),
                Transform::from_xyz(0.0, 10.0, 0.2),
                NpcHead,
            ));
            parent.spawn((
                Sprite::from_color(Color::srgb(0.23, 0.18, 0.14), Vec2::new(2.0, 8.0)),
                Transform::from_xyz(-3.0, -11.0, 0.0),
                NpcLeg,
            ));
            parent.spawn((
                Sprite::from_color(Color::srgb(0.23, 0.18, 0.14), Vec2::new(2.0, 8.0)),
                Transform::from_xyz(3.0, -11.0, 0.0),
                NpcLeg,
            ));
            parent.spawn((
                Sprite::from_color(Color::srgba(0.35, 0.72, 0.92, 0.18), Vec2::new(20.0, 24.0)),
                Transform::from_xyz(0.0, 2.0, -0.1),
                NpcAura,
            ));
        });
    }
}

fn sync_npc_visuals(
    npcs: Query<(&Npc, &ManaReservoir, &ManaPractice, &Children), Changed<ManaReservoir>>,
    mut torsos: Query<&mut Sprite, With<NpcTorso>>,
    mut heads: Query<&mut Sprite, (With<NpcHead>, Without<NpcTorso>)>,
    mut auras: Query<
        (&mut Sprite, &mut Transform),
        (With<NpcAura>, Without<NpcTorso>, Without<NpcHead>),
    >,
) {
    for (npc, reservoir, practice, children) in &npcs {
        let fill_ratio = if reservoir.capacity <= 0.0 {
            0.0
        } else {
            reservoir.stored / reservoir.capacity
        };

        for child in children.iter() {
            if let Ok(mut sprite) = torsos.get_mut(child) {
                sprite.color = if fill_ratio > 0.65 {
                    Color::srgb(0.20, 0.35, 0.76)
                } else {
                    Color::srgb(0.17, 0.28, 0.58)
                };
            }

            if let Ok(mut sprite) = heads.get_mut(child) {
                sprite.color = if npc.health < 35.0 {
                    Color::srgb(0.84, 0.72, 0.60)
                } else {
                    Color::srgb(0.92, 0.82, 0.68)
                };
            }

            if let Ok((mut sprite, mut transform)) = auras.get_mut(child) {
                let alpha = (0.08 + fill_ratio * 0.22).min(0.35);
                sprite.color = if reservoir.stability < 0.45 || practice.backlash > 0.0 {
                    Color::srgba(0.92, 0.42, 0.28, alpha)
                } else {
                    Color::srgba(0.35, 0.72, 0.92, alpha)
                };
                transform.scale = Vec3::splat(0.9 + fill_ratio * 0.35);
            }
        }
    }
}
