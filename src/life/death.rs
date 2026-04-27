use bevy::prelude::*;

use crate::agents::animal::Animal;
use crate::agents::needs::Needs;
use crate::agents::npc::{Npc, NpcCondition};
use crate::life::growth::Lifecycle;
use crate::life::population::{PopulationKind, PopulationStats};
use crate::magic::storage::ManaPractice;
use crate::systems::logging::{LogEvent, LogEventKind, NpcDeathEvent};
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
    mut npc_death_writer: MessageWriter<NpcDeathEvent>,
    animals: Query<(Entity, &Lifecycle, &Animal)>,
    npcs: Query<(
        Entity,
        &Lifecycle,
        &Npc,
        &Needs,
        Option<&NpcCondition>,
        Option<&ManaPractice>,
    )>,
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

    for (entity, lifecycle, npc, needs, condition, mana_practice) in &npcs {
        let reason = if lifecycle.age_days >= lifecycle.max_age {
            Some("old age")
        } else if npc.health <= 0.0 && needs.hunger > 0.98 && needs.thirst > 0.92 {
            Some("starved and dehydrated after prolonged deprivation")
        } else if npc.health <= 0.0 && needs.hunger > 0.98 {
            Some("starved after food exhaustion")
        } else if npc.health <= 0.0 && npc.exposure > 1.25 {
            Some("froze from severe night exposure")
        } else if npc.health <= 0.0
            && mana_practice.is_some_and(|practice| practice.backlash > 0.015)
        {
            Some("died from uncontrolled mana backlash")
        } else if npc.health <= 0.0 {
            condition
                .filter(|condition| !condition.last_damage_reason.is_empty())
                .map(|condition| condition.last_damage_reason.as_str())
                .or(Some("succumbed to untreated wounds"))
        } else {
            None
        };

        if let Some(reason) = reason {
            commands.entity(entity).despawn();
            population.record_death(PopulationKind::Npc, step.elapsed_days);
            npc_death_writer.write(NpcDeathEvent::new(
                step.elapsed_days,
                npc.name.clone(),
                reason.to_string(),
            ));
            writer.write(LogEvent::new(
                LogEventKind::Death,
                format!("{} died from {reason}", npc.name),
            ));
        }
    }
}
