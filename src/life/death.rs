use bevy::prelude::*;

use crate::agents::animal::Animal;
use crate::agents::npc::Npc;
use crate::life::growth::Lifecycle;
use crate::life::population::{PopulationKind, PopulationStats};
use crate::systems::logging::{LogEvent, LogEventKind};
use crate::systems::simulation::SimulationStep;

pub struct DeathPlugin;

impl Plugin for DeathPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, cleanup_dead_entities);
    }
}

fn cleanup_dead_entities(
    mut commands: Commands,
    step: Res<SimulationStep>,
    mut population: ResMut<PopulationStats>,
    mut writer: MessageWriter<LogEvent>,
    animals: Query<(Entity, &Lifecycle, &Animal)>,
    npcs: Query<(Entity, &Lifecycle, &Npc)>,
) {
    for (entity, lifecycle, animal) in &animals {
        let reason = if lifecycle.age_days >= lifecycle.max_age {
            Some("old age")
        } else if animal.health <= 0.0 {
            Some("health collapse")
        } else if animal.energy <= 0.0 && animal.hunger >= 0.95 {
            Some("starvation")
        } else {
            None
        };

        if let Some(reason) = reason {
            commands.entity(entity).despawn();
            population.record_death(PopulationKind::Animal, step.elapsed_days);
            writer.write(LogEvent::new(
                LogEventKind::Death,
                format!("An animal died from {reason}"),
            ));
        }
    }

    for (entity, lifecycle, npc) in &npcs {
        if lifecycle.age_days >= lifecycle.max_age || npc.health <= 0.0 {
            commands.entity(entity).despawn();
            population.record_death(PopulationKind::Npc, step.elapsed_days);
            writer.write(LogEvent::new(
                LogEventKind::Death,
                format!("{} died", npc.name),
            ));
        }
    }
}
