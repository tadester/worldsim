use bevy::prelude::*;

use crate::agents::programs::{SocietyProgress, WorldProgramState};
use crate::systems::logging::{LogEvent, LogEventKind, NpcDeathLog};
use crate::systems::simulation::{SimulationClock, SimulationStep};
use crate::world::resources::WorldStats;

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
            .add_systems(Update, propose_world_solutions);
    }
}

fn propose_world_solutions(
    clock: Res<SimulationClock>,
    step: Res<SimulationStep>,
    stats: Res<WorldStats>,
    deaths: Res<NpcDeathLog>,
    programs: Res<WorldProgramState>,
    society: Res<SocietyProgress>,
    mut queue: ResMut<WorldProposalQueue>,
    mut writer: MessageWriter<LogEvent>,
) {
    if clock.delta_days() <= 0.0 || step.elapsed_days - queue.last_proposal_day < 3.0 {
        return;
    }

    let recent_cold_deaths = deaths
        .entries
        .iter()
        .rev()
        .take_while(|entry| step.elapsed_days - entry.day <= 12.0)
        .filter(|entry| entry.reason.contains("cold"))
        .count();

    let candidate = if recent_cold_deaths >= 2 && stats.cold_stressed_npcs > 0 {
        Some(DeveloperProposal {
            day: step.elapsed_days,
            title: "Add winter clothing production".to_string(),
            problem: format!(
                "{} recent cold deaths and {} cold-stressed NPCs remain after world grants.",
                recent_cold_deaths, stats.cold_stressed_npcs
            ),
            proposed_solution:
                "Create a clothing/leather/fiber resource loop where NPCs craft coats before winter."
                    .to_string(),
            request:
                "Add wearable insulation items, a clothing workshop job, and visual clothing changes."
                    .to_string(),
        })
    } else if stats.shelters == 0 && step.elapsed_days > 8.0 {
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
    } else if stats.npcs >= 6 && stats.civic_structures == 0 && society.stage == "Band" {
        Some(DeveloperProposal {
            day: step.elapsed_days,
            title: "Add settlement planning".to_string(),
            problem: "Population has grown but no civic structures mark a village center."
                .to_string(),
            proposed_solution:
                "Create a planning program that chooses a plaza, paths, fences, and role buildings."
                    .to_string(),
            request: "Add a village planner system with roads, zoning, and expansion rings."
                .to_string(),
        })
    } else if stats.predators > 0 && programs.unlocked.len() > 8 && stats.npcs <= 5 {
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
    } else if stats.total_food_carried + stats.total_food_stockpiled < 1.0
        && step.elapsed_days > 10.0
    {
        Some(DeveloperProposal {
            day: step.elapsed_days,
            title: "Add farming plots".to_string(),
            problem: "Food remains scarce after foraging and food-storage programs.".to_string(),
            proposed_solution:
                "Let NPCs clear farm plots, plant saved seeds, irrigate, and harvest seasonally."
                    .to_string(),
            request: "Add crop entities, farm work actions, seed inventory, and seasonal yields."
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
