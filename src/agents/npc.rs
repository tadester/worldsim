use bevy::prelude::*;

use crate::agents::memory::Memory;
use crate::agents::needs::Needs;
use crate::agents::relationships::Relationships;
use crate::life::growth::Lifecycle;
use crate::magic::mana::ManaReservoir;
use crate::magic::storage::ManaStorageStyle;

#[derive(Component, Debug, Clone)]
pub struct Npc {
    pub name: String,
    pub health: f32,
    pub curiosity: f32,
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
    pub mana_reservoir: ManaReservoir,
    pub mana_style: ManaStorageStyle,
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
            sprite: Sprite::from_color(Color::srgb(0.28, 0.55, 0.94), Vec2::splat(14.0)),
            transform: Transform::from_xyz(position.x, position.y, 4.0),
            npc: Npc {
                name,
                health,
                curiosity: 0.6,
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
            mana_reservoir,
            mana_style,
        }
    }
}

pub struct NpcPlugin;

impl Plugin for NpcPlugin {
    fn build(&self, _app: &mut App) {}
}
