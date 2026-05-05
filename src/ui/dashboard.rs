use bevy::prelude::*;

use crate::agents::evolution::EvolutionPressure;
use crate::agents::factions::Faction;
use crate::agents::programs::{SocietyProgress, WorldProgramState};
use crate::agents::society::{DiplomacyState, FactionSociety};
use crate::life::population::PopulationStats;
use crate::systems::logging::{EventLog, LogEventKind};
use crate::systems::simulation::{SimulationClock, SimulationStep};
use crate::ui::DiagnosticsSettingsPane;
use crate::world::climate::{ClimateEventState, ClimateModel};
use crate::world::director::WorldMind;
use crate::world::proposals::WorldProposalQueue;
use crate::world::resources::WorldStats;
use crate::world::settlement::Settlement;
use crate::world::territory::Territory;

#[derive(Component)]
struct DashboardText;

#[derive(Resource, Default)]
pub struct TrendHistory {
    pub samples: Vec<TrendSample>,
    pub sample_timer_days: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct TrendSample {
    pub day: f32,
    pub trees: usize,
    pub animals: usize,
    pub npcs: usize,
    pub births: usize,
    pub deaths: usize,
    pub happiness: f32,
    pub food_security: f32,
    pub civic_level: f32,
}

pub struct DashboardPlugin;

impl Plugin for DashboardPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TrendHistory>()
            .add_systems(PostStartup, spawn_dashboard)
            .add_systems(Update, (update_trend_history, update_dashboard_text));
    }
}

fn spawn_dashboard(mut commands: Commands, settings_pane: Res<DiagnosticsSettingsPane>) {
    commands.entity(settings_pane.0).with_children(|parent| {
        parent
            .spawn((
                Node {
                    width: percent(100.0),
                    padding: UiRect::axes(px(14.0), px(12.0)),
                    border: UiRect::all(px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.06, 0.08, 0.12, 0.92)),
                BorderColor::all(Color::srgba(0.24, 0.34, 0.44, 0.85)),
            ))
            .with_child((
                Text::new("Settings"),
                TextFont::from_font_size(16.0),
                TextColor(Color::WHITE),
                DashboardText,
            ));
    });
}

fn update_dashboard_text(
    stats: Res<WorldStats>,
    climate: Res<ClimateModel>,
    climate_events: Res<ClimateEventState>,
    clock: Res<SimulationClock>,
    step: Res<SimulationStep>,
    population: Res<PopulationStats>,
    log: Res<EventLog>,
    trends: Res<TrendHistory>,
    world_mind: Res<WorldMind>,
    programs: Res<WorldProgramState>,
    evolution: Res<EvolutionPressure>,
    society: Res<SocietyProgress>,
    proposals: Res<WorldProposalQueue>,
    mut world_queries: ParamSet<(
        Query<(Entity, &Faction, Option<&FactionSociety>)>,
        Query<&Settlement>,
        Query<&Territory>,
    )>,
    diplomacy: Res<DiplomacyState>,
    mut text_query: Query<&mut Text, With<DashboardText>>,
) {
    let latest = log
        .entries
        .last()
        .map(|entry| {
            format!(
                "[day {:.2}] {}: {}",
                entry.day,
                event_label(entry.kind),
                entry.message
            )
        })
        .unwrap_or_else(|| "No events yet".to_string());
    let population_trend_line = {
        let sample_count = trends.samples.len();
        let start = sample_count.saturating_sub(6);
        let mut line = String::new();
        for (index, sample) in trends.samples[start..].iter().enumerate() {
            if index > 0 {
                line.push_str("  ");
            }
            use std::fmt::Write;
            let _ = write!(
                line,
                "{:.0}:{}/{}/{}",
                sample.day, sample.trees, sample.animals, sample.npcs
            );
        }
        line
    };
    let birth_graph = sparkline(trends.samples.iter().map(|sample| sample.births as f32));
    let death_graph = sparkline(trends.samples.iter().map(|sample| sample.deaths as f32));
    let happiness_graph = sparkline(trends.samples.iter().map(|sample| sample.happiness));
    let food_graph = sparkline(trends.samples.iter().map(|sample| sample.food_security));
    let civic_graph = sparkline(trends.samples.iter().map(|sample| sample.civic_level));
    let recent_births = trends
        .samples
        .last()
        .map(|sample| sample.births)
        .unwrap_or(0);
    let recent_deaths = trends
        .samples
        .last()
        .map(|sample| sample.deaths)
        .unwrap_or(0);

    let climate_event_line = climate_events
        .active
        .as_ref()
        .map(|event| {
            format!(
                "{} near {},{} ({:.1}d left)",
                event.kind.label(),
                event.center.x,
                event.center.y,
                event.remaining_days.max(0.0)
            )
        })
        .unwrap_or_else(|| "none".to_string());

    let mut territory_counts = std::collections::HashMap::<Entity, usize>::new();
    let mut claimed_tiles = 0usize;
    let mut contested_tiles = 0usize;
    let mut total_tiles = 0usize;
    for territory in &world_queries.p2() {
        total_tiles += 1;
        if territory.contested {
            contested_tiles += 1;
        }
        if let Some(owner) = territory.owner {
            claimed_tiles += 1;
            *territory_counts.entry(owner).or_insert(0) += 1;
        }
    }
    let factions = world_queries.p0();
    let mut faction_split = territory_counts
        .into_iter()
        .filter_map(|(entity, count)| {
            factions
                .get(entity)
                .ok()
                .map(|(_, faction, society)| (faction, society, count))
        })
        .map(|(faction, society, count)| {
            let governance = society
                .map(|society| society.governance.label())
                .unwrap_or("Kin Circle");
            format!("{} {} ({})", faction.name, count, governance)
        })
        .collect::<Vec<_>>();
    faction_split.sort();
    let active_wars = diplomacy
        .relations
        .values()
        .filter(|pair| pair.at_war)
        .count();
    let active_feuds = diplomacy
        .relations
        .values()
        .filter(|pair| pair.hostility > 0.55 && !pair.at_war)
        .count();
    let mut settlement_lines = world_queries
        .p1()
        .iter()
        .map(|settlement| {
            format!(
                "{} pop {} L{} happy {:.2} food {:.2} house {:.2}",
                settlement.name,
                settlement.population,
                settlement.civic_level,
                settlement.happiness,
                settlement.food_security,
                settlement.housing_security
            )
        })
        .collect::<Vec<_>>();
    settlement_lines.sort();

    for mut text in &mut text_query {
        *text = Text::new(format!(
            "Ticks: {}\nDays: {:.2}\nSpeed: {}{}\nTrees: {}\nAnimals: {}\nPredators: {}\nNPCs: {}\nShelters: {}\nTerritory: {}/{} ({} contested){}\nAvg mana: {:.2}\nAvg animal cap: {:.2}\nAnimal load: {:.2}x\nAvg tree cap: {:.2}\nAvg temp: {:.2}\nSeason: {} day {:.1}/{:.0} (offset {:+.2})\nClimate event: {}\nAvg pressure: {:.2}\nForage: {:.1}\nTree biomass: {:.1}\nFood carried: {:.1}\nWood carried: {:.1}\nFood stockpiled: {:.1}\nWood stockpiled: {:.1}\nLive births: {} (animals {} | npcs {})\nLive deaths: {} (animals {} | npcs {})\nNet growth: {:+}\nRecent births: {}\nRecent deaths: {}\nTrend T/A/N: {}\nGraphs B/D/H/F/C: {} / {} / {} / {} / {}\nLatest: {}",
            step.tick,
            step.elapsed_days,
            clock.speed_label(),
            if clock.paused { " (paused)" } else { "" },
            stats.trees,
            stats.animals,
            stats.predators,
            stats.npcs,
            stats.shelters,
            claimed_tiles,
            total_tiles,
            contested_tiles,
            if faction_split.is_empty() {
                "".to_string()
            } else {
                format!("\nFactions: {}", faction_split.join(" | "))
            },
            stats.avg_mana_density,
            stats.avg_animal_capacity,
            stats.animal_load_ratio,
            stats.avg_tree_capacity,
            stats.avg_temperature,
            climate.season_label(),
            climate.year_day(step.elapsed_days),
            climate.year_length_days,
            climate.current_offset,
            climate_event_line,
            stats.avg_climate_pressure,
            stats.total_forage,
            stats.total_tree_biomass,
            stats.total_food_carried,
            stats.total_wood_carried,
            stats.total_food_stockpiled,
            stats.total_wood_stockpiled,
            population.total_births,
            population.animal_births,
            population.npc_births,
            population.total_deaths,
            population.animal_deaths,
            population.npc_deaths,
            population.net_growth(),
            recent_births,
            recent_deaths,
            if population_trend_line.is_empty() {
                "No samples yet"
            } else {
                &population_trend_line
            },
            birth_graph,
            death_graph,
            happiness_graph,
            food_graph,
            civic_graph,
            latest
        ));
        text.0.push_str(&format!(
            "\nWorld mind: {} | {}\nWorld pressure/nurture/entropy: {:.2}/{:.2}/{:.2}\nWorld focus: {},{} | {}\nEvolution S/R/T/Sh/C/H mut: {:.2}/{:.2}/{:.2}/{:.2}/{:.2}/{:.2} {:.2}\nDiscovery pressure cold/hunger/care/def/culture: {:.2}/{:.2}/{:.2}/{:.2}/{:.2}\nSettlements: {}\nNPC exposure: avg {:.2}, cold stressed {}\nSociety: {} | civic {} | last project: {}\nEconomy: ore {:.1} | metal {:.1} | clothing {:.1} | weapons {:.1}\nFood web: carried {:.1} stockpiled {:.1} | wood carried {:.1} stockpiled {:.1}\nUI parity: inspector click-select | civic/farm/pasture visible | clothing/tools visible\nDiplomacy: wars {} | feuds {}\nWorld programs: {} pressure-unlocked | last: {}\nDeveloper proposals: {}{}",
            world_mind.stance,
            world_mind.intent,
            world_mind.pressure,
            world_mind.nurture,
            world_mind.entropy,
            world_mind.focus_coord.x,
            world_mind.focus_coord.y,
            world_mind.thought,
            evolution.survival_fitness,
            evolution.reproduction_fitness,
            evolution.teaching_fitness,
            evolution.shelter_fitness,
            evolution.community_fitness,
            evolution.happiness_fitness,
            evolution.mutation_rate,
            programs.cold_discovery_pressure,
            programs.hunger_discovery_pressure,
            programs.care_discovery_pressure,
            programs.defense_discovery_pressure,
            programs.culture_pressure,
            if settlement_lines.is_empty() {
                "none".to_string()
            } else {
                settlement_lines.join(" | ")
            },
            stats.avg_npc_exposure,
            stats.cold_stressed_npcs,
            society.stage,
            stats.civic_structures,
            society.last_project,
            stats.total_ore,
            stats.total_metal,
            stats.total_clothing,
            stats.total_weapons,
            stats.total_food_carried,
            stats.total_food_stockpiled,
            stats.total_wood_carried,
            stats.total_wood_stockpiled,
            active_wars,
            active_feuds,
            programs.unlocked.len(),
            programs.last_reason,
            proposals.proposals.len(),
            proposals
                .proposals
                .last()
                .map(|proposal| format!(" | latest: {}", proposal.title))
                .unwrap_or_default()
        ));
    }
}

fn update_trend_history(
    clock: Res<SimulationClock>,
    step: Res<SimulationStep>,
    stats: Res<WorldStats>,
    log: Res<EventLog>,
    settlements: Query<&Settlement>,
    mut trends: ResMut<TrendHistory>,
) {
    if clock.paused {
        return;
    }

    trends.sample_timer_days += clock.delta_days();
    if trends.sample_timer_days < 0.75 {
        return;
    }
    trends.sample_timer_days = 0.0;

    let mut births = 0usize;
    let mut deaths = 0usize;
    for entry in log.entries.iter().rev() {
        if step.elapsed_days - entry.day > 1.0 {
            break;
        }
        match entry.kind {
            LogEventKind::Birth => births += 1,
            LogEventKind::Death => deaths += 1,
            LogEventKind::Discovery
            | LogEventKind::Construction
            | LogEventKind::Territory
            | LogEventKind::Threat
            | LogEventKind::Climate
            | LogEventKind::Proposal => {}
        }
    }

    let settlement_count = settlements.iter().count().max(1) as f32;
    let (happiness, food_security, civic_level) =
        settlements
            .iter()
            .fold((0.0, 0.0, 0.0), |(happiness, food, civic), settlement| {
                (
                    happiness + settlement.happiness,
                    food + settlement.food_security,
                    civic + settlement.civic_level as f32,
                )
            });

    trends.samples.push(TrendSample {
        day: step.elapsed_days,
        trees: stats.trees,
        animals: stats.animals,
        npcs: stats.npcs,
        births,
        deaths,
        happiness: happiness / settlement_count,
        food_security: food_security / settlement_count,
        civic_level: civic_level / settlement_count,
    });

    if trends.samples.len() > 48 {
        let overflow = trends.samples.len() - 48;
        trends.samples.drain(0..overflow);
    }
}

fn sparkline(values: impl Iterator<Item = f32>) -> String {
    const BARS: [&str; 8] = ["_", "-", "=", "+", "*", "#", "%", "@"];
    let values = values.collect::<Vec<_>>();
    if values.is_empty() {
        return "none".to_string();
    }
    let start = values.len().saturating_sub(18);
    let window = &values[start..];
    let min = window.iter().copied().fold(f32::INFINITY, f32::min);
    let max = window.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let span = (max - min).max(0.001);
    window
        .iter()
        .map(|value| {
            let index = (((*value - min) / span) * (BARS.len() - 1) as f32)
                .round()
                .clamp(0.0, (BARS.len() - 1) as f32) as usize;
            BARS[index]
        })
        .collect::<Vec<_>>()
        .join("")
}

fn event_label(kind: LogEventKind) -> &'static str {
    match kind {
        LogEventKind::Birth => "Birth",
        LogEventKind::Death => "Death",
        LogEventKind::Discovery => "Discovery",
        LogEventKind::Construction => "Build",
        LogEventKind::Territory => "Territory",
        LogEventKind::Threat => "Threat",
        LogEventKind::Climate => "Climate",
        LogEventKind::Proposal => "Proposal",
    }
}
