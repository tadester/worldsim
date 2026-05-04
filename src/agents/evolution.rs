use bevy::prelude::*;

use crate::agents::npc::Npc;
use crate::agents::personality::NpcPsyche;
use crate::life::growth::Lifecycle;
use crate::life::population::PopulationStats;
use crate::systems::simulation::SimulationClock;
use crate::world::director::WorldMind;
use crate::world::resources::WorldStats;

#[derive(Resource, Debug, Clone)]
pub struct EvolutionPressure {
    pub survival_fitness: f32,
    pub reproduction_fitness: f32,
    pub teaching_fitness: f32,
    pub shelter_fitness: f32,
    pub community_fitness: f32,
    pub happiness_fitness: f32,
    pub mutation_rate: f32,
    pub generation_estimate: f32,
}

impl Default for EvolutionPressure {
    fn default() -> Self {
        Self {
            survival_fitness: 0.5,
            reproduction_fitness: 0.5,
            teaching_fitness: 0.5,
            shelter_fitness: 0.5,
            community_fitness: 0.5,
            happiness_fitness: 0.5,
            mutation_rate: 0.08,
            generation_estimate: 0.0,
        }
    }
}

pub struct EvolutionPlugin;

impl Plugin for EvolutionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EvolutionPressure>()
            .add_systems(Update, update_evolution_pressure);
    }
}

fn update_evolution_pressure(
    clock: Res<SimulationClock>,
    stats: Res<WorldStats>,
    population: Res<PopulationStats>,
    world_mind: Option<Res<WorldMind>>,
    mut pressure: ResMut<EvolutionPressure>,
    npcs: Query<(&Npc, &Lifecycle, Option<&NpcPsyche>)>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    let count = stats.npcs.max(1) as f32;
    let adults = npcs
        .iter()
        .filter(|(_, lifecycle, _)| lifecycle.age_days >= lifecycle.maturity_age)
        .count() as f32;
    let avg_happiness = npcs
        .iter()
        .map(|(_, _, psyche)| psyche.map(|p| p.happiness).unwrap_or(0.5))
        .sum::<f32>()
        / count;
    let avg_reproduction_drive = npcs
        .iter()
        .map(|(npc, _, _)| npc.reproduction_drive)
        .sum::<f32>()
        / count;
    let avg_discovery_drive = npcs
        .iter()
        .map(|(npc, _, _)| npc.discovery_drive)
        .sum::<f32>()
        / count;

    let death_rate = population.npc_deaths as f32 / population.total_deaths.max(1) as f32;
    let birth_rate = population.npc_births as f32 / population.total_births.max(1) as f32;
    let shelter_ratio = stats.shelters as f32 / (stats.npcs.max(1) as f32 * 0.45).max(1.0);
    let food_ratio = (stats.total_forage + stats.total_food_stockpiled + stats.total_food_carried)
        / (stats.npcs.max(1) as f32 * 1.2);
    let community_ratio = ((stats.shelters + stats.civic_structures) as f32
        / stats.npcs.max(1) as f32)
        .clamp(0.0, 1.0);
    let world_stress = world_mind.as_ref().map(|mind| mind.pressure).unwrap_or(0.0);

    let target_survival = (food_ratio * 0.35
        + (1.0 - stats.avg_npc_exposure).clamp(0.0, 1.0) * 0.35
        + (1.0 - death_rate) * 0.30)
        .clamp(0.0, 1.0);
    let target_reproduction = (birth_rate * 0.40
        + avg_reproduction_drive * 0.22
        + adults / count * 0.18
        + avg_happiness * 0.20)
        .clamp(0.0, 1.0);
    let target_teaching =
        (avg_discovery_drive * 0.34 + avg_happiness * 0.22 + stats.civic_structures as f32 * 0.04)
            .clamp(0.0, 1.0);
    let target_shelter = (shelter_ratio * 0.60
        + (1.0 - stats.avg_npc_exposure).clamp(0.0, 1.0) * 0.40)
        .clamp(0.0, 1.0);
    let target_community =
        (community_ratio * 0.40 + avg_happiness * 0.34 + birth_rate * 0.26).clamp(0.0, 1.0);

    let blend = (delta_days * 0.035).clamp(0.0, 0.20);
    pressure.survival_fitness = pressure.survival_fitness.lerp(target_survival, blend);
    pressure.reproduction_fitness = pressure
        .reproduction_fitness
        .lerp(target_reproduction, blend);
    pressure.teaching_fitness = pressure.teaching_fitness.lerp(target_teaching, blend);
    pressure.shelter_fitness = pressure.shelter_fitness.lerp(target_shelter, blend);
    pressure.community_fitness = pressure.community_fitness.lerp(target_community, blend);
    pressure.happiness_fitness = pressure.happiness_fitness.lerp(avg_happiness, blend);
    pressure.mutation_rate =
        (0.04 + world_stress * 0.10 + (1.0 - target_survival) * 0.05).clamp(0.03, 0.18);
    pressure.generation_estimate = population.npc_births as f32 / count.max(1.0);
}
