use bevy::prelude::*;

use crate::agents::factions::FactionMember;
use crate::agents::inventory::Inventory;
use crate::agents::needs::Needs;
use crate::agents::npc::Npc;
use crate::agents::programs::{KnownPrograms, ProgramId};
use crate::life::growth::Lifecycle;
use crate::systems::simulation::SimulationClock;

#[derive(Component, Debug, Clone)]
pub struct Kinship {
    pub mother: Option<Entity>,
    pub father: Option<Entity>,
    pub generation: u32,
    pub home_culture: Option<Entity>,
    pub birth_home: Option<Entity>,
    pub siblings_at_birth: u32,
}

impl Default for Kinship {
    fn default() -> Self {
        Self {
            mother: None,
            father: None,
            generation: 0,
            home_culture: None,
            birth_home: None,
            siblings_at_birth: 0,
        }
    }
}

pub struct KinshipPlugin;

impl Plugin for KinshipPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (family_care_sharing, childhood_learning_from_kin).chain(),
        );
    }
}

fn family_care_sharing(
    clock: Res<SimulationClock>,
    mut npcs: ParamSet<(
        Query<
            (
                Entity,
                &Transform,
                &Lifecycle,
                &FactionMember,
                &mut Inventory,
            ),
            With<Npc>,
        >,
        Query<
            (
                Entity,
                &Transform,
                &Lifecycle,
                &Kinship,
                &FactionMember,
                &mut Needs,
                &mut Inventory,
            ),
            With<Npc>,
        >,
    )>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    let child_needs = npcs
        .p1()
        .iter()
        .filter(|(_, _, lifecycle, _, _, needs, _)| {
            lifecycle.age_days < lifecycle.maturity_age
                && (needs.hunger > 0.42 || needs.thirst > 0.50 || needs.safety < 0.45)
        })
        .map(
            |(entity, transform, _, kinship, member, needs, inventory)| {
                (
                    entity,
                    transform.translation.truncate(),
                    kinship.mother,
                    kinship.father,
                    member.faction,
                    needs.hunger,
                    needs.thirst,
                    needs.safety,
                    inventory.food_space(),
                )
            },
        )
        .collect::<Vec<_>>();

    let mut transfers = Vec::<(Entity, Entity, f32, f32)>::new();
    for (adult_entity, adult_transform, lifecycle, member, mut inventory) in &mut npcs.p0() {
        if lifecycle.age_days < lifecycle.maturity_age || inventory.food <= 0.08 {
            continue;
        }
        let adult_pos = adult_transform.translation.truncate();
        let Some((child_entity, _, hunger, thirst, safety, food_space)) = child_needs
            .iter()
            .filter(
                |(child_entity, child_pos, mother, father, faction, _, _, _, _)| {
                    *child_entity != adult_entity
                        && *faction == member.faction
                        && (adult_pos.distance(*child_pos) < 82.0
                            || Some(adult_entity) == *mother
                            || Some(adult_entity) == *father)
                },
            )
            .map(
                |(
                    child_entity,
                    child_pos,
                    mother,
                    father,
                    _,
                    hunger,
                    thirst,
                    safety,
                    food_space,
                )| {
                    let parent_bonus =
                        if Some(adult_entity) == *mother || Some(adult_entity) == *father {
                            -42.0
                        } else {
                            0.0
                        };
                    (
                        *child_entity,
                        adult_pos.distance(*child_pos) + parent_bonus,
                        *hunger,
                        *thirst,
                        *safety,
                        *food_space,
                    )
                },
            )
            .min_by(|a, b| a.1.total_cmp(&b.1))
        else {
            continue;
        };

        let distress = hunger.max(thirst).max(1.0 - safety);
        let moved = (delta_days * (0.18 + distress * 0.34))
            .min(inventory.food)
            .min(food_space.max(0.0));
        if moved <= 0.0 {
            continue;
        }
        inventory.food -= moved;
        transfers.push((adult_entity, child_entity, moved, distress));
    }

    if transfers.is_empty() {
        return;
    }

    for (child_entity, _, lifecycle, _, _, mut needs, mut inventory) in &mut npcs.p1() {
        if lifecycle.age_days >= lifecycle.maturity_age {
            continue;
        }
        let received = transfers
            .iter()
            .filter(|(_, target, _, _)| *target == child_entity)
            .map(|(_, _, moved, _)| *moved)
            .sum::<f32>();
        if received <= 0.0 {
            continue;
        }
        inventory.food = (inventory.food + received).min(inventory.max_food);
        needs.hunger = (needs.hunger - received * 0.95).max(0.0);
        needs.thirst = (needs.thirst - received * 0.22).max(0.0);
        needs.safety = (needs.safety + received * 0.16).min(1.0);
        needs.social = (needs.social + received * 0.22).min(1.0);
    }
}

fn childhood_learning_from_kin(
    clock: Res<SimulationClock>,
    mut npcs: ParamSet<(
        Query<
            (
                Entity,
                &Transform,
                &Lifecycle,
                &FactionMember,
                Option<&KnownPrograms>,
            ),
            With<Npc>,
        >,
        Query<
            (
                Entity,
                &Transform,
                &Lifecycle,
                &Kinship,
                &FactionMember,
                &mut KnownPrograms,
            ),
            With<Npc>,
        >,
    )>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    let adult_snapshots = npcs
        .p0()
        .iter()
        .filter(|(_, _, lifecycle, _, _)| lifecycle.age_days >= lifecycle.maturity_age)
        .map(|(entity, transform, _, member, programs)| {
            (
                entity,
                transform.translation.truncate(),
                member.faction,
                programs.cloned(),
            )
        })
        .collect::<Vec<_>>();

    for (child_entity, transform, lifecycle, kinship, member, mut child_programs) in &mut npcs.p1()
    {
        if lifecycle.age_days >= lifecycle.maturity_age {
            continue;
        }

        let child_pos = transform.translation.truncate();
        let age_ratio = (lifecycle.age_days / lifecycle.maturity_age.max(1.0)).clamp(0.0, 1.0);
        let kin_culture_bonus = if kinship.home_culture == Some(member.faction) {
            0.08
        } else {
            0.0
        };
        let home_stability_bonus = if kinship.birth_home.is_some() {
            0.06
        } else {
            0.0
        };
        let sibling_bonus = (kinship.siblings_at_birth as f32 * 0.002).min(0.06);
        let generation_bonus = (kinship.generation as f32 * 0.004).min(0.08);
        let learning_window = delta_days
            * (0.16
                + age_ratio * 0.42
                + kin_culture_bonus
                + home_stability_bonus
                + sibling_bonus
                + generation_bonus);
        let seed = ((child_entity.to_bits() + lifecycle.age_days as u64 * 13) % 997) as f32 / 997.0;
        if seed > learning_window {
            continue;
        }

        let teacher = adult_snapshots
            .iter()
            .filter(|(adult_entity, adult_pos, adult_faction, _)| {
                *adult_entity != child_entity
                    && *adult_faction == member.faction
                    && child_pos.distance(*adult_pos) < 82.0
            })
            .min_by(|(a_entity, a_pos, _, _), (b_entity, b_pos, _, _)| {
                let a_bonus =
                    if Some(*a_entity) == kinship.mother || Some(*a_entity) == kinship.father {
                        -28.0
                    } else {
                        0.0
                    };
                let b_bonus =
                    if Some(*b_entity) == kinship.mother || Some(*b_entity) == kinship.father {
                        -28.0
                    } else {
                        0.0
                    };
                (child_pos.distance(*a_pos) + a_bonus)
                    .total_cmp(&(child_pos.distance(*b_pos) + b_bonus))
            });

        let Some((_, _, _, Some(teacher_programs))) = teacher else {
            continue;
        };

        for program in teacher_programs.known.iter().copied() {
            if matches!(
                program,
                ProgramId::Foraging
                    | ProgramId::WaterFinding
                    | ProgramId::Firemaking
                    | ProgramId::ShelterBuilding
                    | ProgramId::Toolmaking
                    | ProgramId::Storykeeping
                    | ProgramId::Childcare
                    | ProgramId::Teaching
                    | ProgramId::FoodStorage
            ) && child_programs.learn(program)
            {
                child_programs.last_grant_reason = "learned from nearby kin and elders".to_string();
                break;
            }
        }
    }
}
