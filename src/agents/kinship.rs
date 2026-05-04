use bevy::prelude::*;

use crate::agents::factions::FactionMember;
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
        app.add_systems(Update, childhood_learning_from_kin);
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
