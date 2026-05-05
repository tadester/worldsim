use bevy::prelude::*;

use crate::life::growth::Lifecycle;
use crate::systems::simulation::SimulationClock;

#[derive(Component, Debug, Clone, Copy)]
pub struct Needs {
    pub hunger: f32,
    pub thirst: f32,
    pub fatigue: f32,
    pub safety: f32,
    pub social: f32,
    pub curiosity: f32,
}

impl Needs {
    pub fn default_humanoid() -> Self {
        Self {
            hunger: 0.2,
            thirst: 0.2,
            fatigue: 0.1,
            safety: 0.8,
            social: 0.45,
            curiosity: 0.55,
        }
    }
}

pub struct NeedsPlugin;

impl Plugin for NeedsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, decay_needs);
    }
}

fn decay_needs(
    clock: Res<SimulationClock>,
    mut needs_query: Query<(&mut Needs, Option<&Lifecycle>)>,
) {
    let delta_days = clock.delta_days();

    for (mut needs, lifecycle) in &mut needs_query {
        let childhood = lifecycle
            .map(|life| (1.0 - life.age_days / life.maturity_age.max(1.0)).clamp(0.0, 1.0))
            .unwrap_or(0.0);
        let dependency_buffer = 1.0 - childhood * 0.45;
        needs.hunger = (needs.hunger + delta_days * 0.05 * dependency_buffer).min(1.0);
        needs.thirst = (needs.thirst + delta_days * 0.07 * dependency_buffer).min(1.0);
        needs.fatigue = (needs.fatigue + delta_days * 0.03 * (1.0 - childhood * 0.35)).min(1.0);
        needs.social = (needs.social - delta_days * 0.01 * (1.0 - childhood * 0.50)).max(0.0);
        needs.safety =
            (needs.safety - delta_days * 0.005 * (1.0 - childhood * 0.55)).clamp(0.0, 1.0);
        needs.curiosity = (needs.curiosity + delta_days * 0.006).clamp(0.1, 1.0);
    }
}
