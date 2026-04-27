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
use crate::systems::simulation::SimulationStep;

#[derive(Component)]
struct NpcTorso;

#[derive(Component)]
struct NpcHead;

#[derive(Component)]
struct NpcLeg;

#[derive(Component)]
struct NpcAura;

#[derive(Component)]
struct NpcCloak;

#[derive(Component)]
struct NpcHandItem;

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

#[derive(Component, Debug, Clone)]
pub struct NpcCondition {
    pub last_damage_reason: String,
    pub last_damage_day: f32,
}

impl Default for NpcCondition {
    fn default() -> Self {
        Self {
            last_damage_reason: "none".to_string(),
            last_damage_day: -999.0,
        }
    }
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
    pub condition: NpcCondition,
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
            condition: NpcCondition::default(),
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
                Sprite::from_color(Color::srgba(0.25, 0.20, 0.16, 0.0), Vec2::new(12.0, 13.0)),
                Transform::from_xyz(0.0, -1.0, 0.05),
                NpcCloak,
            ));
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
            parent.spawn((
                Sprite::from_color(Color::srgba(0.68, 0.62, 0.52, 0.0), Vec2::new(3.0, 11.0)),
                Transform::from_xyz(7.0, -2.0, 0.28).with_rotation(Quat::from_rotation_z(-0.25)),
                NpcHandItem,
            ));
        });
    }
}

fn sync_npc_visuals(
    step: Res<SimulationStep>,
    npcs: Query<
        (
            &Npc,
            &Lifecycle,
            &Inventory,
            &NpcIntent,
            &NpcCondition,
            &ManaReservoir,
            &ManaPractice,
            &Children,
        ),
    >,
    mut torsos: Query<
        (&mut Sprite, &mut Transform),
        (
            With<NpcTorso>,
            Without<NpcHead>,
            Without<NpcLeg>,
            Without<NpcCloak>,
            Without<NpcHandItem>,
            Without<NpcAura>,
        ),
    >,
    mut heads: Query<
        (&mut Sprite, &mut Transform),
        (
            With<NpcHead>,
            Without<NpcTorso>,
            Without<NpcLeg>,
            Without<NpcCloak>,
            Without<NpcHandItem>,
            Without<NpcAura>,
        ),
    >,
    mut legs: Query<
        &mut Transform,
        (
            With<NpcLeg>,
            Without<NpcTorso>,
            Without<NpcHead>,
            Without<NpcCloak>,
            Without<NpcHandItem>,
            Without<NpcAura>,
        ),
    >,
    mut cloaks: Query<
        (&mut Sprite, &mut Transform),
        (
            With<NpcCloak>,
            Without<NpcTorso>,
            Without<NpcHead>,
            Without<NpcLeg>,
            Without<NpcHandItem>,
            Without<NpcAura>,
        ),
    >,
    mut items: Query<
        (&mut Sprite, &mut Transform),
        (
            With<NpcHandItem>,
            Without<NpcTorso>,
            Without<NpcHead>,
            Without<NpcLeg>,
            Without<NpcCloak>,
            Without<NpcAura>,
        ),
    >,
    mut auras: Query<
        (&mut Sprite, &mut Transform),
        (
            With<NpcAura>,
            Without<NpcTorso>,
            Without<NpcHead>,
            Without<NpcLeg>,
            Without<NpcCloak>,
            Without<NpcHandItem>,
        ),
    >,
) {
    let phase = step.elapsed_days * 28.0;
    for (npc, lifecycle, inventory, intent, condition, reservoir, practice, children) in &npcs {
        let fill_ratio = if reservoir.capacity <= 0.0 {
            0.0
        } else {
            reservoir.stored / reservoir.capacity
        };
        let age_ratio = (lifecycle.age_days / lifecycle.maturity_age.max(1.0)).clamp(0.0, 1.35);
        let body_scale = if age_ratio < 1.0 {
            0.45 + age_ratio * 0.55
        } else {
            1.0 + (age_ratio - 1.0) * 0.06
        };
        let child_tint = if age_ratio < 1.0 { 0.12 } else { 0.0 };
        let clothing_alpha = (inventory.clothing * 0.85).clamp(0.0, 0.95);
        let weapon_alpha =
            (inventory.weapons * 0.95 + npc.woodcutting_tools * 0.55).clamp(0.0, 1.0);
        let mana_glow = (fill_ratio * 0.25 + reservoir.stability * 0.08).clamp(0.0, 0.45);
        let carry_ratio = inventory.carry_ratio();
        let work_label = intent.label.as_str();
        let work_cycle = match work_label {
            "Gather Wood" => (phase * 1.5).sin(),
            "Stockpile" => (phase * 1.3).sin(),
            "Build Shelter" | "Repair Shelter" => (phase * 1.2).sin(),
            "Build Fire" | "Tend Fire" => (phase * 2.4).sin(),
            "Hunt Predator" | "Raid" | "Flee" | "Retreat" => (phase * 2.1).sin(),
            _ => phase.sin() * 0.35,
        };
        let is_hearth_work = matches!(work_label, "Build Fire" | "Tend Fire");
        let is_combat = matches!(work_label, "Hunt Predator" | "Raid" | "Flee" | "Retreat");
        let is_telekinetic_work = practice.telekinesis >= 0.35
            && matches!(
                work_label,
                "Gather Wood" | "Stockpile" | "Build Shelter" | "Repair Shelter" | "Forage"
            );
        let ward_pulse = if practice.warding >= 0.35 && is_combat {
            0.25 + ((phase * 3.6).sin() * 0.5 + 0.5) * 0.75
        } else {
            0.0
        };
        let raid_ward_flash = if practice.warding >= 0.35
            && condition.last_damage_reason.contains("raid")
            && step.elapsed_days - condition.last_damage_day < 0.25
        {
            0.45 + ((phase * 8.0).sin() * 0.5 + 0.5) * 0.55
        } else {
            0.0
        };

        for child in children.iter() {
            if let Ok((mut sprite, mut transform)) = torsos.get_mut(child) {
                sprite.color = if fill_ratio > 0.65 {
                    Color::srgb(0.20 + child_tint, 0.35 + child_tint * 0.5, 0.76)
                } else {
                    Color::srgb(0.17 + child_tint, 0.28 + child_tint * 0.4, 0.58)
                };
                sprite.custom_size = Some(Vec2::new(10.0, 14.0) * body_scale);
                transform.translation.y = -1.0 * body_scale + work_cycle * 0.55;
            }

            if let Ok((mut sprite, mut transform)) = heads.get_mut(child) {
                sprite.color = if npc.health < 35.0 {
                    Color::srgb(0.84, 0.72, 0.60)
                } else {
                    Color::srgb(0.92, 0.82, 0.68)
                };
                sprite.custom_size = Some(Vec2::splat(8.0 * body_scale));
                transform.translation.x = if is_combat {
                    0.4 * work_cycle
                } else {
                    0.0
                };
                transform.translation.y = 10.0 * body_scale + work_cycle.abs() * 0.25;
            }

            if let Ok(mut transform) = legs.get_mut(child) {
                transform.scale = Vec3::new(1.0, body_scale, 1.0);
                transform.translation.y = -11.0 * body_scale - work_cycle.abs() * 0.45;
            }

            if let Ok((mut sprite, mut transform)) = cloaks.get_mut(child) {
                sprite.color = Color::srgba(
                    0.32 + fill_ratio * 0.18,
                    0.24 + mana_glow * 0.35,
                    0.16 + mana_glow * 0.55,
                    clothing_alpha,
                );
                sprite.custom_size = Some(Vec2::new(12.0, 13.0) * body_scale);
                transform.translation.y = -1.0 * body_scale;
            }

            if let Ok((mut sprite, mut transform)) = items.get_mut(child) {
                let has_weapon = inventory.weapons > 0.08;
                let has_tool = npc.woodcutting_tools > 0.08;
                let telekinetic_tool = practice.telekinesis >= 0.35 && (has_weapon || has_tool);
                let is_load = is_telekinetic_work && carry_ratio > 0.18;
                sprite.color = if is_load {
                    Color::srgba(
                        0.58 + carry_ratio * 0.12,
                        0.48 + practice.telekinesis * 0.14,
                        0.36 + mana_glow * 0.25,
                        0.82,
                    )
                } else if is_hearth_work {
                    Color::srgba(
                        0.96,
                        0.56 + practice.hearthspark * 0.12,
                        0.18 + mana_glow * 0.18,
                        0.88,
                    )
                } else if has_weapon {
                    Color::srgba(
                        0.74 + practice.hunter_focus * 0.10,
                        0.74,
                        0.80 + mana_glow * 0.3,
                        weapon_alpha,
                    )
                } else if has_tool {
                    Color::srgba(
                        0.58,
                        0.46 + practice.hearthspark * 0.06,
                        0.30 + practice.telekinesis * 0.08,
                        weapon_alpha * 0.85,
                    )
                } else {
                    Color::srgba(0.0, 0.0, 0.0, 0.0)
                };
                sprite.custom_size = Some(if is_load {
                    Vec2::new(8.0 + carry_ratio * 6.0, 6.0 + carry_ratio * 5.0)
                } else if is_hearth_work {
                    Vec2::new(4.0 + practice.hearthspark * 3.0, 8.0 + practice.hearthspark * 5.0)
                } else if has_weapon {
                    Vec2::new(3.0, 12.0 * body_scale)
                } else {
                    Vec2::new(4.0, 9.0 * body_scale)
                });
                transform.translation = if is_load {
                    Vec3::new(
                        0.0,
                        8.0 * body_scale + 2.0 + work_cycle * 1.8,
                        0.34 + practice.telekinesis * 0.10,
                    )
                } else if is_hearth_work {
                    Vec3::new(
                        6.2 * body_scale,
                        0.2 * body_scale + work_cycle * 1.0,
                        0.32,
                    )
                } else {
                    Vec3::new(7.0 * body_scale, -2.0 * body_scale, 0.28)
                };
                transform.rotation = if is_load {
                    Quat::from_rotation_z(work_cycle * 0.12)
                } else if has_weapon {
                    Quat::from_rotation_z(-0.45)
                } else {
                    Quat::from_rotation_z(-0.20)
                };
                if telekinetic_tool {
                    transform.translation.x += 1.5;
                    transform.translation.y += 1.2;
                }
            }

            if let Ok((mut sprite, mut transform)) = auras.get_mut(child) {
                let alpha = (0.08 + fill_ratio * 0.22).min(0.35);
                sprite.color = if reservoir.stability < 0.45 || practice.backlash > 0.0 {
                    Color::srgba(0.92, 0.42, 0.28, alpha)
                } else if practice.telekinesis >= 0.35 {
                    Color::srgba(0.50, 0.72, 0.98, alpha + 0.05)
                } else if practice.hearthspark >= 0.35 {
                    Color::srgba(0.94, 0.54, 0.24, alpha + 0.05)
                } else if practice.warding >= 0.35 {
                    Color::srgba(0.34, 0.88, 0.70, alpha + 0.05)
                } else if practice.hunter_focus >= 0.35 {
                    Color::srgba(0.78, 0.32, 0.90, alpha + 0.05)
                } else if practice.verdant_touch >= 0.35 {
                    Color::srgba(0.42, 0.84, 0.42, alpha + 0.05)
                } else {
                    Color::srgba(0.35, 0.72, 0.92, alpha)
                };
                transform.scale = Vec3::splat(
                    (0.7
                        + fill_ratio * 0.45
                        + practice.discovered_count() as f32 * 0.05
                        + ward_pulse
                        + if is_hearth_work {
                            practice.hearthspark * 0.12
                        } else {
                            0.0
                        })
                        * body_scale.max(0.7),
                );
                transform.translation.y = 2.0 + if is_hearth_work { work_cycle.abs() * 1.2 } else { 0.0 };
                if practice.warding >= 0.35 && is_combat {
                    sprite.color = Color::srgba(
                        0.34,
                        0.90,
                        0.72,
                        (alpha + 0.10 + ward_pulse * 0.18 + raid_ward_flash * 0.20)
                            .clamp(0.0, 0.85),
                    );
                } else if raid_ward_flash > 0.0 {
                    sprite.color = Color::srgba(
                        0.68,
                        0.96,
                        0.86,
                        (alpha + raid_ward_flash * 0.24).clamp(0.0, 0.82),
                    );
                }
            }
        }
    }
}
