use bevy::prelude::*;

use crate::agents::decisions::NpcIntent;
use crate::agents::inventory::Inventory;
use crate::agents::memory::Memory;
use crate::agents::needs::Needs;
use crate::agents::personality::{NpcPsyche, PersonalityType};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcSex {
    Female,
    Male,
}

impl NpcSex {
    pub fn label(self) -> &'static str {
        match self {
            Self::Female => "Female",
            Self::Male => "Male",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcGender {
    Woman,
    Man,
    Nonbinary,
}

impl NpcGender {
    pub fn label(self) -> &'static str {
        match self {
            Self::Woman => "Woman",
            Self::Man => "Man",
            Self::Nonbinary => "Nonbinary",
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct Npc {
    pub name: String,
    pub health: f32,
    pub curiosity: f32,
    pub speed: f32,
    pub sex: NpcSex,
    pub gender: NpcGender,
    pub tool_knowledge: f32,
    pub woodcutting_tools: f32,
    pub exposure: f32,
    pub reproduction_drive: f32,
    pub discovery_drive: f32,
    pub aggression_drive: f32,
    pub risk_tolerance: f32,
    pub personality: PersonalityType,
}

#[derive(Component, Debug, Clone, Default)]
pub struct NpcHome {
    pub shelter: Option<Entity>,
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
    pub home: NpcHome,
    pub inventory: Inventory,
    pub psyche: NpcPsyche,
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
                sex: NpcSex::Female,
                gender: NpcGender::Woman,
                tool_knowledge: 0.0,
                woodcutting_tools: 0.0,
                exposure: 0.0,
                reproduction_drive: 0.9,
                discovery_drive: 0.9,
                aggression_drive: 0.3,
                risk_tolerance: 0.5,
                personality: PersonalityType::Builder,
            },
            lifecycle: Lifecycle {
                age_days: 0.0,
                maturity_age: 16.0 * 365.0,
                max_age: 86.0 * 365.0,
                fertility: 0.9,
                reproduction_cooldown: 0.0,
            },
            needs: Needs::default_humanoid(),
            memory: Memory::default(),
            relationships: Relationships::default(),
            intent: NpcIntent::default(),
            home: NpcHome::default(),
            inventory: Inventory::default(),
            psyche: NpcPsyche::default(),
            mana_reservoir,
            mana_style,
            mana_practice: ManaPractice::default(),
        }
    }

    pub fn with_age_days(mut self, age_days: f32) -> Self {
        self.lifecycle.age_days = age_days.max(0.0);
        self
    }

    pub fn with_identity(mut self, sex: NpcSex, gender: NpcGender) -> Self {
        self.npc.sex = sex;
        self.npc.gender = gender;
        self
    }

    pub fn with_tooling(mut self, knowledge: f32, tools: f32) -> Self {
        self.npc.tool_knowledge = knowledge.clamp(0.0, 1.0);
        self.npc.woodcutting_tools = tools.clamp(0.0, 1.0);
        self
    }

    pub fn with_drives(
        mut self,
        reproduction_drive: f32,
        discovery_drive: f32,
        aggression_drive: f32,
        risk_tolerance: f32,
    ) -> Self {
        self.npc.reproduction_drive = reproduction_drive.clamp(0.1, 1.6);
        self.npc.discovery_drive = discovery_drive.clamp(0.1, 1.6);
        self.npc.aggression_drive = aggression_drive.clamp(0.0, 1.6);
        self.npc.risk_tolerance = risk_tolerance.clamp(0.0, 1.4);
        self.psyche.personality = if aggression_drive > 1.0 {
            PersonalityType::Raider
        } else if discovery_drive > 1.20 && aggression_drive < 0.35 {
            PersonalityType::Mystic
        } else if discovery_drive > 1.15 {
            PersonalityType::Scholar
        } else if reproduction_drive > 1.15 {
            PersonalityType::Caregiver
        } else if risk_tolerance > 0.9 {
            PersonalityType::Sovereign
        } else {
            PersonalityType::Builder
        };
        self.npc.personality = self.psyche.personality;
        self
    }

    pub fn with_personality(
        mut self,
        personality: PersonalityType,
        pride: f32,
        greed: f32,
        lust: f32,
        envy: f32,
        gluttony: f32,
        wrath: f32,
        sloth: f32,
    ) -> Self {
        self.npc.personality = personality;
        self.psyche.personality = personality;
        self.psyche.pride = pride.clamp(0.0, 1.0);
        self.psyche.greed = greed.clamp(0.0, 1.0);
        self.psyche.lust = lust.clamp(0.0, 1.0);
        self.psyche.envy = envy.clamp(0.0, 1.0);
        self.psyche.gluttony = gluttony.clamp(0.0, 1.0);
        self.psyche.wrath = wrath.clamp(0.0, 1.0);
        self.psyche.sloth = sloth.clamp(0.0, 1.0);
        self
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
    npcs: Query<
        (&Npc, &ManaReservoir, &ManaPractice, &Children),
        Or<(Changed<Npc>, Changed<ManaReservoir>, Changed<ManaPractice>)>,
    >,
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
