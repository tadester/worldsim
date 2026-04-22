use bevy::prelude::*;

use crate::systems::logging::{LogEvent, LogEventKind};
use crate::systems::simulation::{SimulationClock, SimulationStep};
use crate::world::map::MapSettings;
use crate::world::resources::WorldStats;

#[derive(Resource, Debug, Clone)]
pub struct WorldMind {
    pub stance: String,
    pub intent: String,
    pub pressure: f32,
    pub nurture: f32,
    pub entropy: f32,
    pub climate_bias: f32,
    pub resource_bias: f32,
    pub focus_coord: IVec2,
    pub thought: String,
    pub reflection_timer_days: f32,
}

impl Default for WorldMind {
    fn default() -> Self {
        Self {
            stance: "Unformed".to_string(),
            intent: "Let the first patterns emerge".to_string(),
            pressure: 0.0,
            nurture: 0.5,
            entropy: 0.0,
            climate_bias: 0.0,
            resource_bias: 1.0,
            focus_coord: IVec2::ZERO,
            thought: "The world is waking up".to_string(),
            reflection_timer_days: 0.0,
        }
    }
}

pub struct WorldDirectorPlugin;

impl Plugin for WorldDirectorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldMind>()
            .add_systems(Update, update_world_mind);
    }
}

fn update_world_mind(
    clock: Res<SimulationClock>,
    step: Res<SimulationStep>,
    settings: Res<MapSettings>,
    stats: Res<WorldStats>,
    mut mind: ResMut<WorldMind>,
    mut writer: MessageWriter<LogEvent>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    mind.reflection_timer_days += delta_days;
    if mind.reflection_timer_days < 0.25 {
        return;
    }
    mind.reflection_timer_days = 0.0;

    let population = (stats.npcs + stats.animals) as f32;
    let scarcity = if population <= 0.0 {
        0.0
    } else {
        (1.0 - (stats.total_forage / (population * 0.9).max(1.0))).clamp(0.0, 1.0)
    };
    let shelter_gap = if stats.npcs == 0 {
        0.0
    } else {
        (1.0 - stats.shelters as f32 / (stats.npcs as f32 * 0.35).max(1.0)).clamp(0.0, 1.0)
    };
    let predation =
        (stats.predators as f32 / (stats.npcs + stats.animals).max(1) as f32 * 8.0).clamp(0.0, 1.0);
    let climate_pressure = stats.avg_climate_pressure.clamp(0.0, 1.0);
    let animal_load = (stats.animal_load_ratio - 0.8).clamp(0.0, 1.0);

    let next_pressure = (scarcity * 0.30
        + shelter_gap * 0.22
        + predation * 0.24
        + climate_pressure * 0.16
        + animal_load * 0.08)
        .clamp(0.0, 1.0);
    let next_nurture = (1.0 - next_pressure * 0.70
        + stats.avg_tree_capacity * 0.08
        + stats.avg_mana_density * 0.04)
        .clamp(0.0, 1.0);
    let next_entropy = (stats.avg_mana_density * 0.40 + climate_pressure * 0.35 + predation * 0.25)
        .clamp(0.0, 1.0);

    let previous_stance = mind.stance.clone();
    mind.pressure = mind.pressure * 0.70 + next_pressure * 0.30;
    mind.nurture = mind.nurture * 0.75 + next_nurture * 0.25;
    mind.entropy = mind.entropy * 0.80 + next_entropy * 0.20;
    mind.resource_bias = (1.08 - mind.pressure * 0.28 + mind.nurture * 0.16).clamp(0.70, 1.28);
    mind.climate_bias =
        (mind.pressure * 0.035 + mind.entropy * 0.025 - mind.nurture * 0.020).clamp(-0.04, 0.08);
    mind.focus_coord = focus_coord(&settings, step.tick);

    let (stance, intent) = if mind.pressure > 0.72 {
        ("Testing", "Push scarcity until agents adapt")
    } else if scarcity > 0.62 {
        (
            "Hungry",
            "Regrow food pockets and expose weak foraging plans",
        )
    } else if shelter_gap > 0.60 {
        ("Sheltering", "Pressure settlers toward homes and fire")
    } else if mind.entropy > 0.66 {
        ("Strange", "Let mana and weather disturb stable paths")
    } else if mind.nurture > 0.68 {
        ("Nurturing", "Give ecosystems room to recover")
    } else {
        ("Watching", "Let local minds negotiate the next story")
    };
    mind.stance = stance.to_string();
    mind.intent = intent.to_string();
    mind.thought = format!(
        "scarcity {:.2}, shelter gap {:.2}, predation {:.2}, climate {:.2}",
        scarcity, shelter_gap, predation, climate_pressure
    );

    if previous_stance != mind.stance {
        writer.write(LogEvent::new(
            LogEventKind::Discovery,
            format!("World mind shifted to {}: {}", mind.stance, mind.intent),
        ));
    }
}

fn focus_coord(settings: &MapSettings, tick: u64) -> IVec2 {
    let width = settings.width.max(1);
    let height = settings.height.max(1);
    let x = ((tick / 97) % width as u64) as i32;
    let y = ((tick / 131) % height as u64) as i32;
    IVec2::new(x, y)
}
