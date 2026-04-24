use bevy::prelude::*;

use crate::agents::programs::{SocietyProgress, WorldProgramState};
use crate::life::population::PopulationStats;
use crate::systems::logging::{LogEvent, LogEventKind, NpcDeathLog};
use crate::systems::simulation::{SimulationClock, SimulationStep};
use crate::world::resources::WorldStats;

#[derive(Debug, Clone)]
pub struct WorldActionEntry {
    pub day: f32,
    pub title: String,
    pub detail: String,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct WorldActionLog {
    pub entries: Vec<WorldActionEntry>,
}

#[derive(Debug, Clone)]
pub struct DeveloperProposal {
    pub day: f32,
    pub title: String,
    pub problem: String,
    pub proposed_solution: String,
    pub request: String,
}

#[derive(Resource, Debug, Clone)]
pub struct WorldProposalQueue {
    pub proposals: Vec<DeveloperProposal>,
    pub last_proposal_day: f32,
}

impl Default for WorldProposalQueue {
    fn default() -> Self {
        Self {
            proposals: Vec::new(),
            last_proposal_day: -999.0,
        }
    }
}

pub struct WorldProposalPlugin;

impl Plugin for WorldProposalPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldProposalQueue>()
            .init_resource::<WorldActionLog>()
            .add_systems(Update, propose_world_solutions);
    }
}

fn propose_world_solutions(
    clock: Res<SimulationClock>,
    step: Res<SimulationStep>,
    stats: Res<WorldStats>,
    population: Res<PopulationStats>,
    _deaths: Res<NpcDeathLog>,
    programs: Res<WorldProgramState>,
    society: Res<SocietyProgress>,
    mut queue: ResMut<WorldProposalQueue>,
    mut writer: MessageWriter<LogEvent>,
) {
    let generations_elapsed = step.elapsed_days / (22.0 * 365.0);
    let thriving_stalled = population
        .last_birth_day
        .map(|day| step.elapsed_days - day > 365.0 * 3.0)
        .unwrap_or(true);
    if clock.delta_days() <= 0.0
        || step.elapsed_days - queue.last_proposal_day < 365.0
        || generations_elapsed < 4.0
        || !thriving_stalled
    {
        return;
    }

    let candidate = if stats.shelters == 0 && step.elapsed_days > 365.0 * 4.0 {
        Some(DeveloperProposal {
            day: step.elapsed_days,
            title: "Add coordinated construction crews".to_string(),
            problem: "NPCs know shelter programs but no shelters exist after several days."
                .to_string(),
            proposed_solution:
                "Let groups reserve a build site, haul wood together, and complete a shared home."
                    .to_string(),
            request:
                "Add construction job reservations, work parties, and partially built shelter ghosts."
                    .to_string(),
        })
    } else if stats.npcs >= 12 && stats.civic_structures <= 1 && society.stage == "Band" {
        Some(DeveloperProposal {
            day: step.elapsed_days,
            title: "Add road and zoning planning".to_string(),
            problem: "Population has grown but town layout is still loose and structure placement is ad hoc."
                .to_string(),
            proposed_solution:
                "Create a planner that establishes roads, districts, plazas, and expansion rings."
                    .to_string(),
            request: "Add a village planner system with roads, zoning, district claims, and expansion rings."
                .to_string(),
        })
    } else if stats.predators > 0 && programs.unlocked.len() > 12 && stats.npcs <= 5 {
        Some(DeveloperProposal {
            day: step.elapsed_days,
            title: "Add guard patrols and weapons".to_string(),
            problem: "The society has knowledge but remains vulnerable to predators.".to_string(),
            proposed_solution:
                "Promote watch posts into patrol routes and craft spears/bows from tool programs."
                    .to_string(),
            request: "Add weapon items, guard roles, patrol behavior, and combat readiness UI."
                .to_string(),
        })
    } else if stats.total_clothing < 1.0
        && stats.cold_stressed_npcs > 4
        && step.elapsed_days > 365.0 * 5.0
    {
        Some(DeveloperProposal {
            day: step.elapsed_days,
            title: "Add visible equipment and clothing".to_string(),
            problem: "Clothing and weapon resources exist, but NPC visuals and role readability do not reflect them.".to_string(),
            proposed_solution:
                "Show coats, tools, and weapons on NPCs so society specialization is legible."
                    .to_string(),
            request: "Add visual equipment layers, clothing states, and held-tool sprites."
                .to_string(),
        })
    } else {
        None
    };

    let Some(proposal) = candidate else {
        return;
    };
    if queue
        .proposals
        .iter()
        .any(|item| item.title == proposal.title)
    {
        return;
    }

    writer.write(LogEvent::new(
        LogEventKind::Proposal,
        format!("{}: {}", proposal.title, proposal.request),
    ));
    queue.last_proposal_day = step.elapsed_days;
    queue.proposals.push(proposal);
    if queue.proposals.len() > 16 {
        let overflow = queue.proposals.len() - 16;
        queue.proposals.drain(0..overflow);
    }
}

pub fn push_world_action(
    log: &mut WorldActionLog,
    day: f32,
    title: impl Into<String>,
    detail: impl Into<String>,
) {
    log.entries.push(WorldActionEntry {
        day,
        title: title.into(),
        detail: detail.into(),
    });
    if log.entries.len() > 32 {
        let overflow = log.entries.len() - 32;
        log.entries.drain(0..overflow);
    }
}
