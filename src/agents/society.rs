use bevy::prelude::*;

use std::collections::HashMap;

use crate::agents::decisions::NpcIntent;
use crate::agents::factions::{Faction, FactionMember};
use crate::agents::needs::Needs;
use crate::agents::npc::Npc;
use crate::agents::personality::{NpcPsyche, PersonalityType};
use crate::agents::programs::{KnownPrograms, ProgramId};
use crate::agents::relationships::Relationships;
use crate::life::growth::Lifecycle;
use crate::systems::simulation::{SimulationClock, SimulationStep};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GovernanceKind {
    KinCircle,
    Leader,
    Council,
    Democracy,
}

impl GovernanceKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::KinCircle => "Kin Circle",
            Self::Leader => "Leader",
            Self::Council => "Council",
            Self::Democracy => "Democracy",
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct FactionSociety {
    pub governance: GovernanceKind,
    pub leader: Option<Entity>,
    pub cohesion: f32,
    pub care_drive: f32,
    pub peace_bias: f32,
    pub war_pressure: f32,
    pub settlement_drive: f32,
    pub last_change_day: f32,
    pub last_policy: String,
}

impl Default for FactionSociety {
    fn default() -> Self {
        Self {
            governance: GovernanceKind::KinCircle,
            leader: None,
            cohesion: 0.35,
            care_drive: 0.40,
            peace_bias: 0.55,
            war_pressure: 0.18,
            settlement_drive: 0.24,
            last_change_day: -999.0,
            last_policy: "Survive together".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PairRelation {
    pub hostility: f32,
    pub feud_days: f32,
    pub at_war: bool,
    pub last_raid_day: f32,
}

#[derive(Resource, Debug, Default, Clone)]
pub struct DiplomacyState {
    pub relations: HashMap<(Entity, Entity), PairRelation>,
}

impl DiplomacyState {
    pub fn relation(&self, a: Entity, b: Entity) -> Option<&PairRelation> {
        self.relations.get(&relation_key(a, b))
    }

    pub fn hostility_between(&self, a: Entity, b: Entity) -> f32 {
        self.relation(a, b)
            .map(|pair| pair.hostility)
            .unwrap_or(0.0)
    }

    pub fn at_war(&self, a: Entity, b: Entity) -> bool {
        self.relation(a, b).map(|pair| pair.at_war).unwrap_or(false)
    }
}

#[derive(Debug, Clone)]
struct FactionSnapshot {
    entity: Entity,
    adults: usize,
    avg_hunger: f32,
    avg_safety: f32,
    avg_aggression: f32,
    avg_wrath: f32,
    care_bias: f32,
    mediation_share: f32,
    governance_share: f32,
    teaching_share: f32,
    armed_share: f32,
    raid_share: f32,
}

pub struct SocietyPlugin;

impl Plugin for SocietyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DiplomacyState>().add_systems(
            Update,
            (
                attach_faction_societies,
                update_faction_societies,
                update_faction_diplomacy.after(update_faction_societies),
                stabilize_peaceful_societies.after(update_faction_diplomacy),
            ),
        );
    }
}

fn attach_faction_societies(
    mut commands: Commands,
    factions: Query<Entity, (With<Faction>, Without<FactionSociety>)>,
) {
    for entity in &factions {
        commands.entity(entity).insert(FactionSociety::default());
    }
}

fn update_faction_societies(
    step: Res<SimulationStep>,
    factions: Query<Entity, With<Faction>>,
    mut societies: Query<&mut FactionSociety>,
    npcs: Query<
        (
            Entity,
            &FactionMember,
            &Npc,
            &Lifecycle,
            &Needs,
            &Relationships,
            Option<&KnownPrograms>,
            Option<&NpcPsyche>,
        ),
        With<Npc>,
    >,
) {
    let mut members_by_faction = HashMap::<Entity, Vec<_>>::new();
    for npc in &npcs {
        members_by_faction
            .entry(npc.1.faction)
            .or_default()
            .push(npc);
    }

    for faction in &factions {
        let Ok(mut society) = societies.get_mut(faction) else {
            continue;
        };
        let Some(members) = members_by_faction.get(&faction) else {
            society.governance = GovernanceKind::KinCircle;
            society.leader = None;
            society.cohesion = (society.cohesion - 0.002).max(0.15);
            society.last_policy = "Survive scattered".to_string();
            continue;
        };

        let adults = members
            .iter()
            .filter(|(_, _, _, life, _, _, _, _)| life.age_days >= life.maturity_age)
            .count();
        let count = members.len().max(1) as f32;
        let avg_social = members
            .iter()
            .map(|(_, _, _, _, needs, _, _, _)| needs.social)
            .sum::<f32>()
            / count;
        let avg_trust = members
            .iter()
            .map(|(_, _, _, _, _, rel, _, _)| rel.trust_baseline)
            .sum::<f32>()
            / count;
        let avg_affinity = members
            .iter()
            .map(|(_, _, _, _, _, rel, _, _)| rel.affinity)
            .sum::<f32>()
            / count;
        let avg_hunger = members
            .iter()
            .map(|(_, _, _, _, needs, _, _, _)| needs.hunger)
            .sum::<f32>()
            / count;
        let avg_safety = members
            .iter()
            .map(|(_, _, _, _, needs, _, _, _)| needs.safety)
            .sum::<f32>()
            / count;
        let avg_aggression = members
            .iter()
            .map(|(_, _, npc, _, _, _, _, _)| npc.aggression_drive)
            .sum::<f32>()
            / count;
        let avg_wrath = members
            .iter()
            .map(|(_, _, _, _, _, _, _, psyche)| psyche.map(|p| p.wrath).unwrap_or(0.2))
            .sum::<f32>()
            / count;
        let avg_happiness = members
            .iter()
            .map(|(_, _, _, _, _, _, _, psyche)| psyche.map(|p| p.happiness).unwrap_or(0.5))
            .sum::<f32>()
            / count;
        let care_bias = members
            .iter()
            .map(|(_, _, npc, _, needs, rel, _, psyche)| {
                let personality = psyche.map(|p| p.personality).unwrap_or(npc.personality);
                let personality_bonus = if personality == PersonalityType::Caregiver {
                    0.18
                } else {
                    0.0
                };
                npc.reproduction_drive * 0.15
                    + needs.social * 0.18
                    + rel.trust_baseline * 0.20
                    + personality_bonus
            })
            .sum::<f32>()
            / count;
        let sovereignty_bias = members
            .iter()
            .map(|(_, _, npc, _, _, _, _, psyche)| {
                psyche.map(|p| p.pride + p.envy * 0.35).unwrap_or(0.30) + npc.risk_tolerance * 0.25
            })
            .sum::<f32>()
            / count;
        let builder_bias = members
            .iter()
            .map(|(_, _, npc, _, _, _, _, psyche)| {
                let personality = psyche.map(|p| p.personality).unwrap_or(npc.personality);
                if matches!(
                    personality,
                    PersonalityType::Builder | PersonalityType::Scholar
                ) {
                    1.0
                } else {
                    0.0
                }
            })
            .sum::<f32>()
            / count;
        let scholar_bias = members
            .iter()
            .map(|(_, _, npc, _, _, _, _, psyche)| {
                let personality = psyche.map(|p| p.personality).unwrap_or(npc.personality);
                if matches!(
                    personality,
                    PersonalityType::Scholar | PersonalityType::Mystic
                ) {
                    1.0
                } else {
                    0.0
                }
            })
            .sum::<f32>()
            / count;
        let mediation_share = members
            .iter()
            .filter(|(_, _, _, _, _, _, programs, _)| {
                programs.is_some_and(|p| p.knows(ProgramId::ConflictMediation))
            })
            .count() as f32
            / count;
        let governance_share = members
            .iter()
            .filter(|(_, _, _, _, _, _, programs, _)| {
                programs.is_some_and(|p| p.knows(ProgramId::Governance))
            })
            .count() as f32
            / count;
        let teaching_share = members
            .iter()
            .filter(|(_, _, _, _, _, _, programs, _)| {
                programs.is_some_and(|p| p.knows(ProgramId::Teaching))
            })
            .count() as f32
            / count;
        let armed_share = members
            .iter()
            .filter(|(_, _, _, _, _, _, programs, _)| {
                programs.is_some_and(|p| {
                    p.knows(ProgramId::Blacksmithing) || p.knows(ProgramId::PredatorDefense)
                })
            })
            .count() as f32
            / count;

        society.cohesion =
            (avg_trust * 0.35 + avg_affinity * 0.30 + avg_social * 0.15 + avg_happiness * 0.20)
                .clamp(0.0, 1.0);
        society.care_drive =
            (care_bias * 0.55 + avg_safety * 0.15 + avg_social * 0.15 + teaching_share * 0.15)
                .clamp(0.0, 1.0);
        society.peace_bias = ((1.0 - avg_aggression) * 0.28
            + (1.0 - avg_wrath) * 0.25
            + mediation_share * 0.20
            + avg_trust * 0.17
            + care_bias * 0.10)
            .clamp(0.0, 1.0);
        society.war_pressure = (avg_aggression * 0.33
            + avg_wrath * 0.27
            + armed_share * 0.14
            + avg_hunger * 0.16
            + (1.0 - avg_safety) * 0.10)
            .clamp(0.0, 1.0);
        society.settlement_drive = (builder_bias * 0.28
            + scholar_bias * 0.10
            + governance_share * 0.12
            + teaching_share * 0.08
            + society.care_drive * 0.20
            + society.cohesion * 0.22)
            .clamp(0.0, 1.0);

        let next_governance = if adults < 3 || society.cohesion < 0.32 {
            GovernanceKind::KinCircle
        } else if governance_share > 0.45
            && teaching_share > 0.30
            && society.peace_bias > 0.56
            && care_bias >= sovereignty_bias * 0.95
        {
            GovernanceKind::Democracy
        } else if governance_share > 0.22
            && builder_bias + scholar_bias > 0.80
            && society.cohesion > 0.46
        {
            GovernanceKind::Council
        } else if sovereignty_bias > care_bias * 0.95 && society.cohesion > 0.44 {
            GovernanceKind::Leader
        } else {
            GovernanceKind::KinCircle
        };

        if next_governance != society.governance {
            society.governance = next_governance;
            society.last_change_day = step.elapsed_days;
        }

        society.leader = if society.governance == GovernanceKind::Leader {
            members
                .iter()
                .max_by(|a, b| {
                    leadership_score(a.2, a.4, a.5, a.7)
                        .total_cmp(&leadership_score(b.2, b.4, b.5, b.7))
                })
                .map(|(entity, _, _, _, _, _, _, _)| *entity)
        } else {
            None
        };

        society.last_policy = match society.governance {
            GovernanceKind::KinCircle => "Keep kin alive".to_string(),
            GovernanceKind::Leader => "Follow the strongest organizer".to_string(),
            GovernanceKind::Council => "Specialists negotiate shared work".to_string(),
            GovernanceKind::Democracy => "Vote for peace, food, and public works".to_string(),
        };
    }
}

fn update_faction_diplomacy(
    clock: Res<SimulationClock>,
    factions: Query<Entity, With<Faction>>,
    societies: Query<&FactionSociety>,
    npcs: Query<
        (
            &FactionMember,
            &Npc,
            &Needs,
            &Relationships,
            &NpcIntent,
            Option<&NpcPsyche>,
            Option<&KnownPrograms>,
        ),
        With<Npc>,
    >,
    mut diplomacy: ResMut<DiplomacyState>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    let mut snapshots = HashMap::<Entity, FactionSnapshot>::new();
    for faction in &factions {
        let mut adults = 0usize;
        let mut avg_hunger = 0.0;
        let mut avg_safety = 0.0;
        let mut avg_aggression = 0.0;
        let mut avg_wrath = 0.0;
        let mut care_bias = 0.0;
        let mut mediation_share = 0.0;
        let mut governance_share = 0.0;
        let mut teaching_share = 0.0;
        let mut armed_share = 0.0;
        let mut raid_share = 0.0;
        let mut count = 0.0;

        for (member, npc, needs, _relationships, intent, psyche, programs) in &npcs {
            if member.faction != faction {
                continue;
            }
            count += 1.0;
            adults += 1;
            let personality = psyche.map(|p| p.personality).unwrap_or(npc.personality);
            avg_hunger += needs.hunger;
            avg_safety += needs.safety;
            avg_aggression += npc.aggression_drive;
            avg_wrath += psyche.map(|p| p.wrath).unwrap_or(0.2);
            if personality == PersonalityType::Caregiver {
                care_bias += 1.0;
            }
            if intent.label == "Raid" {
                raid_share += 1.0;
            }
            if programs.is_some_and(|p| p.knows(ProgramId::ConflictMediation)) {
                mediation_share += 1.0;
            }
            if programs.is_some_and(|p| p.knows(ProgramId::Governance)) {
                governance_share += 1.0;
            }
            if programs.is_some_and(|p| p.knows(ProgramId::Teaching)) {
                teaching_share += 1.0;
            }
            if programs.is_some_and(|p| {
                p.knows(ProgramId::Blacksmithing) || p.knows(ProgramId::PredatorDefense)
            }) {
                armed_share += 1.0;
            }
        }

        if count <= 0.0 {
            continue;
        }
        snapshots.insert(
            faction,
            FactionSnapshot {
                entity: faction,
                adults,
                avg_hunger: avg_hunger / count,
                avg_safety: avg_safety / count,
                avg_aggression: avg_aggression / count,
                avg_wrath: avg_wrath / count,
                care_bias: care_bias / count,
                mediation_share: mediation_share / count,
                governance_share: governance_share / count,
                teaching_share: teaching_share / count,
                armed_share: armed_share / count,
                raid_share: raid_share / count,
            },
        );
    }

    let factions = snapshots.values().cloned().collect::<Vec<_>>();
    for (index, left) in factions.iter().enumerate() {
        for right in factions.iter().skip(index + 1) {
            let Some(left_society) = societies.get(left.entity).ok() else {
                continue;
            };
            let Some(right_society) = societies.get(right.entity).ok() else {
                continue;
            };
            let key = relation_key(left.entity, right.entity);
            let pair = diplomacy.relations.entry(key).or_default();

            let scarcity = ((left.avg_hunger + right.avg_hunger) * 0.5
                + ((1.0 - left.avg_safety) + (1.0 - right.avg_safety)) * 0.25)
                .clamp(0.0, 1.0);
            let militant = ((left.avg_aggression + right.avg_aggression) * 0.28
                + (left.avg_wrath + right.avg_wrath) * 0.26
                + (left.armed_share + right.armed_share) * 0.12
                + (left.raid_share + right.raid_share) * 0.34)
                .clamp(0.0, 1.0);
            let peace = ((left_society.peace_bias + right_society.peace_bias) * 0.32
                + (left.mediation_share + right.mediation_share) * 0.20
                + (left.governance_share + right.governance_share) * 0.18
                + (left.teaching_share + right.teaching_share) * 0.10
                + (left.care_bias + right.care_bias) * 0.20)
                .clamp(0.0, 1.0);

            let pressure = (scarcity * 0.30 + militant * 0.46 - peace * 0.28).clamp(-0.2, 1.0);
            pair.hostility = (pair.hostility + pressure * delta_days * 0.12
                - peace * delta_days * 0.05)
                .clamp(0.0, 1.0);

            if left.raid_share > 0.0 || right.raid_share > 0.0 {
                pair.last_raid_day += delta_days;
            } else {
                pair.last_raid_day = (pair.last_raid_day - delta_days).max(0.0);
            }

            if pair.hostility > 0.55 {
                pair.feud_days += delta_days;
            } else {
                pair.feud_days = (pair.feud_days - delta_days * 1.5).max(0.0);
            }

            if !pair.at_war
                && pair.hostility > 0.74
                && pair.feud_days > 45.0
                && left.adults >= 4
                && right.adults >= 4
            {
                pair.at_war = true;
            }
            if pair.at_war && pair.hostility < 0.34 && peace > 0.46 {
                pair.at_war = false;
                pair.feud_days = 0.0;
            }
        }
    }
}

fn stabilize_peaceful_societies(
    clock: Res<SimulationClock>,
    societies: Query<&FactionSociety>,
    diplomacy: Res<DiplomacyState>,
    mut npcs: Query<(&FactionMember, &mut NpcPsyche, &Npc), With<Npc>>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    for (member, mut psyche, npc) in &mut npcs {
        let Some(society) = societies.get(member.faction).ok() else {
            continue;
        };
        let at_war = diplomacy.relations.iter().any(|((left, right), pair)| {
            pair.at_war && (*left == member.faction || *right == member.faction)
        });

        let peaceful = matches!(
            psyche.personality,
            PersonalityType::Caregiver
                | PersonalityType::Builder
                | PersonalityType::Scholar
                | PersonalityType::Mystic
        ) && npc.aggression_drive < 0.65;

        if peaceful && !at_war {
            psyche.happiness = (psyche.happiness
                + (society.peace_bias * 0.05 + society.care_drive * 0.03) * delta_days)
                .clamp(0.0, 1.0);
        } else if at_war && peaceful {
            psyche.happiness = (psyche.happiness - delta_days * 0.03).max(0.0);
        }
    }
}

fn leadership_score(
    npc: &Npc,
    needs: &Needs,
    relationships: &Relationships,
    psyche: Option<&NpcPsyche>,
) -> f32 {
    let pride = psyche.map(|p| p.pride).unwrap_or(0.3);
    let happiness = psyche.map(|p| p.happiness).unwrap_or(0.5);
    npc.health * 0.01
        + needs.safety * 0.20
        + relationships.trust_baseline * 0.30
        + relationships.affinity * 0.18
        + pride * 0.18
        + happiness * 0.14
}

fn relation_key(a: Entity, b: Entity) -> (Entity, Entity) {
    if a.to_bits() <= b.to_bits() {
        (a, b)
    } else {
        (b, a)
    }
}
