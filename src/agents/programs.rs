use bevy::prelude::*;

use crate::agents::animal::AnimalBundle;
use crate::agents::decisions::NpcIntent;
use crate::agents::inventory::Inventory;
use crate::agents::npc::{Npc, NpcHome};
use crate::agents::personality::PersonalityType;
use crate::agents::society::FactionSociety;
use crate::life::population::PopulationStats;
use crate::systems::logging::{LogEvent, LogEventKind, NpcDeathLog};
use crate::systems::simulation::{SimulationClock, SimulationStep};
use crate::world::director::WorldMind;
use crate::world::map::{MapSettings, RegionTile};
use crate::world::proposals::{WorldActionLog, push_world_action};
use crate::world::resources::{
    CivicStructure, CivicStructureKind, Shelter, ShelterStockpile, WorldStats,
    spawn_transient_effect,
};
use crate::world::settlement::Settlement;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProgramId {
    Foraging,
    WaterFinding,
    Firemaking,
    HearthKeeping,
    ShelterBuilding,
    ShelterRepair,
    WarmClothing,
    FoodStorage,
    Childcare,
    Midwifery,
    FirstAid,
    HerbalMedicine,
    Sanitation,
    WasteManagement,
    Toolmaking,
    Woodworking,
    Blacksmithing,
    CharcoalMaking,
    Mining,
    Stoneworking,
    Pottery,
    Weaving,
    Leatherworking,
    Cooking,
    Agriculture,
    SeedSaving,
    AnimalHusbandry,
    Fishing,
    Irrigation,
    GranaryManagement,
    Trade,
    Accounting,
    Lawkeeping,
    ConflictMediation,
    Teaching,
    Apprenticeship,
    Storykeeping,
    Ritual,
    Surveying,
    RoadBuilding,
    BridgeBuilding,
    Watchkeeping,
    PredatorDefense,
    Migration,
    WeatherReading,
    ManaSensing,
    ManaCirculation,
    ManaHealing,
    ManaWarding,
    ManaAgriculture,
    ManaSmithing,
    ManaStorage,
    ManaCommunication,
    ManaTransit,
    ResearchMethod,
    Governance,
    FestivalMaking,
    Artistry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgramDomain {
    Survival,
    Care,
    Craft,
    Food,
    Society,
    Infrastructure,
    Defense,
    Mana,
    Culture,
}

#[derive(Debug, Clone, Copy)]
pub struct ProgramDef {
    pub id: ProgramId,
    pub name: &'static str,
    pub domain: ProgramDomain,
    pub tier: u8,
    pub summary: &'static str,
}

#[derive(Component, Debug, Clone)]
pub struct KnownPrograms {
    pub known: Vec<ProgramId>,
    pub granted_by_world: Vec<ProgramId>,
    pub last_grant_reason: String,
}

impl Default for KnownPrograms {
    fn default() -> Self {
        Self {
            known: vec![
                ProgramId::Foraging,
                ProgramId::WaterFinding,
                ProgramId::Firemaking,
                ProgramId::ShelterBuilding,
                ProgramId::ShelterRepair,
                ProgramId::Toolmaking,
                ProgramId::Storykeeping,
            ],
            granted_by_world: Vec::new(),
            last_grant_reason: "Inherited starter knowledge".to_string(),
        }
    }
}

#[derive(Resource, Debug, Clone)]
pub struct SocietyProgress {
    pub stage: String,
    pub last_project_day: f32,
    pub last_project: String,
}

impl Default for SocietyProgress {
    fn default() -> Self {
        Self {
            stage: "Band".to_string(),
            last_project_day: -999.0,
            last_project: "No civic projects yet".to_string(),
        }
    }
}

impl KnownPrograms {
    pub fn knows(&self, id: ProgramId) -> bool {
        self.known.contains(&id)
    }

    pub fn learn(&mut self, id: ProgramId) -> bool {
        if self.knows(id) {
            false
        } else {
            self.known.push(id);
            true
        }
    }

    pub fn grant(&mut self, id: ProgramId, reason: &str) -> bool {
        if self.learn(id) {
            self.granted_by_world.push(id);
            self.last_grant_reason = reason.to_string();
            true
        } else {
            false
        }
    }

    pub fn names(&self, limit: usize) -> String {
        self.known
            .iter()
            .take(limit)
            .filter_map(|id| {
                program_def(*id).map(|def| format!("{} T{} {:?}", def.name, def.tier, def.domain))
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn featured_summary(&self) -> &'static str {
        self.known
            .last()
            .and_then(|id| program_def(*id))
            .map(|def| def.summary)
            .unwrap_or("No program details available")
    }
}

#[derive(Resource, Debug, Clone)]
pub struct WorldProgramState {
    pub unlocked: Vec<ProgramId>,
    pub last_grant_day: f32,
    pub last_spawn_day: f32,
    pub last_nurture_day: f32,
    pub cold_discovery_pressure: f32,
    pub hunger_discovery_pressure: f32,
    pub care_discovery_pressure: f32,
    pub defense_discovery_pressure: f32,
    pub culture_pressure: f32,
    pub last_reason: String,
}

impl Default for WorldProgramState {
    fn default() -> Self {
        Self {
            unlocked: Vec::new(),
            last_grant_day: -999.0,
            last_spawn_day: -999.0,
            last_nurture_day: -999.0,
            cold_discovery_pressure: 0.0,
            hunger_discovery_pressure: 0.0,
            care_discovery_pressure: 0.0,
            defense_discovery_pressure: 0.0,
            culture_pressure: 0.0,
            last_reason: "No world grants yet".to_string(),
        }
    }
}

pub struct ProgramPlugin;

impl Plugin for ProgramPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldProgramState>()
            .init_resource::<SocietyProgress>()
            .add_systems(
                Update,
                (
                    attach_known_programs,
                    npc_self_discover_programs,
                    cultural_learning_from_deaths,
                    update_world_cultural_pressure,
                    update_migration_pressure.after(update_world_cultural_pressure),
                    world_nurture_thriving_society.after(update_migration_pressure),
                    build_society_projects,
                    advance_civic_projects.after(build_society_projects),
                    apply_known_program_effects,
                    materialize_resource_chains.after(apply_known_program_effects),
                ),
            );
    }
}

pub fn programs_for_death_reason(reason: &str) -> &'static [ProgramId] {
    if reason.contains("cold") || reason.contains("exposure") {
        &[
            ProgramId::Firemaking,
            ProgramId::HearthKeeping,
            ProgramId::WarmClothing,
            ProgramId::ShelterBuilding,
            ProgramId::WeatherReading,
            ProgramId::ManaWarding,
        ]
    } else if reason.contains("starvation") {
        &[
            ProgramId::Foraging,
            ProgramId::FoodStorage,
            ProgramId::Cooking,
            ProgramId::Agriculture,
            ProgramId::SeedSaving,
        ]
    } else if reason.contains("injury") {
        &[
            ProgramId::FirstAid,
            ProgramId::HerbalMedicine,
            ProgramId::Watchkeeping,
            ProgramId::PredatorDefense,
        ]
    } else {
        &[
            ProgramId::Storykeeping,
            ProgramId::Teaching,
            ProgramId::ResearchMethod,
        ]
    }
}

fn attach_known_programs(
    mut commands: Commands,
    npcs: Query<Entity, (Added<Npc>, Without<KnownPrograms>)>,
) {
    for entity in &npcs {
        commands.entity(entity).insert(KnownPrograms::default());
    }
}

fn npc_self_discover_programs(
    clock: Res<SimulationClock>,
    step: Res<SimulationStep>,
    mut writer: MessageWriter<LogEvent>,
    mut npcs: Query<(
        Entity,
        &Npc,
        &crate::agents::needs::Needs,
        &crate::agents::inventory::Inventory,
        &mut KnownPrograms,
    )>,
    state: Res<WorldProgramState>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    for (entity, npc, needs, inventory, mut programs) in &mut npcs {
        let seed = ((entity.to_bits() ^ step.tick) % 997) as f32 / 997.0;
        let discovery = npc.discovery_drive
            * (needs.curiosity + (1.0 - needs.safety) * 0.35 + npc.exposure * 0.45)
            * delta_days
            * (0.42 + state.culture_pressure * 0.30);

        let candidates = [
            (
                ProgramId::Firemaking,
                (needs.safety < 0.55 && npc.exposure > 0.25 && inventory.wood >= 0.35)
                    || state.cold_discovery_pressure > 0.30,
            ),
            (
                ProgramId::ShelterBuilding,
                (needs.safety < 0.58 && inventory.wood >= 0.8)
                    || state.cold_discovery_pressure > 0.46,
            ),
            (
                ProgramId::Toolmaking,
                npc.tool_knowledge > 0.65 || inventory.wood >= 0.4,
            ),
            (
                ProgramId::WarmClothing,
                (npc.exposure > 0.45 && programs.knows(ProgramId::ShelterBuilding))
                    || state.cold_discovery_pressure > 0.55,
            ),
            (
                ProgramId::FoodStorage,
                inventory.food > 0.6
                    || inventory.wood > 0.8
                    || state.hunger_discovery_pressure > 0.32,
            ),
            (
                ProgramId::Woodworking,
                programs.knows(ProgramId::Toolmaking) && inventory.wood > 0.7,
            ),
            (
                ProgramId::Childcare,
                (npc.reproduction_drive > 1.0 && needs.safety > 0.45)
                    || state.care_discovery_pressure > 0.28,
            ),
            (
                ProgramId::Watchkeeping,
                needs.safety < 0.45
                    || npc.risk_tolerance > 0.8
                    || state.defense_discovery_pressure > 0.34,
            ),
            (
                ProgramId::PredatorDefense,
                (programs.knows(ProgramId::Watchkeeping) && npc.aggression_drive > 0.65)
                    || state.defense_discovery_pressure > 0.58,
            ),
            (
                ProgramId::Blacksmithing,
                programs.knows(ProgramId::Toolmaking)
                    && programs.knows(ProgramId::Firemaking)
                    && npc.tool_knowledge > 0.90,
            ),
            (
                ProgramId::Teaching,
                (programs.known.len() >= 8 && needs.social > 0.5)
                    || state.care_discovery_pressure > 0.50,
            ),
            (
                ProgramId::Governance,
                programs.known.len() >= 12 && programs.knows(ProgramId::Teaching),
            ),
            (
                ProgramId::ManaSensing,
                npc.curiosity > 0.55 && npc.discovery_drive > 0.8,
            ),
        ];

        for (program, eligible) in candidates {
            let pressure_bonus = program_pressure_bonus(program, &state);
            if eligible && !programs.knows(program) && seed < discovery + pressure_bonus {
                programs.learn(program);
                let name = program_def(program)
                    .map(|def| def.name)
                    .unwrap_or("Unknown");
                writer.write(LogEvent::new(
                    LogEventKind::Discovery,
                    format!("{} figured out {name}", npc.name),
                ));
                break;
            }
        }
    }
}

fn program_pressure_bonus(program: ProgramId, state: &WorldProgramState) -> f32 {
    match program {
        ProgramId::Firemaking
        | ProgramId::HearthKeeping
        | ProgramId::WarmClothing
        | ProgramId::ShelterBuilding
        | ProgramId::ShelterRepair
        | ProgramId::WeatherReading
        | ProgramId::ManaWarding => state.cold_discovery_pressure * 0.08,
        ProgramId::Foraging
        | ProgramId::FoodStorage
        | ProgramId::Cooking
        | ProgramId::Agriculture
        | ProgramId::SeedSaving
        | ProgramId::AnimalHusbandry
        | ProgramId::ManaAgriculture => state.hunger_discovery_pressure * 0.08,
        ProgramId::Childcare
        | ProgramId::Midwifery
        | ProgramId::Teaching
        | ProgramId::ConflictMediation
        | ProgramId::Governance => state.care_discovery_pressure * 0.08,
        ProgramId::Watchkeeping | ProgramId::PredatorDefense | ProgramId::FirstAid => {
            state.defense_discovery_pressure * 0.08
        }
        _ => state.culture_pressure * 0.03,
    }
}

fn cultural_learning_from_deaths(
    step: Res<SimulationStep>,
    deaths: Res<NpcDeathLog>,
    mut state: ResMut<WorldProgramState>,
    mut world_actions: ResMut<WorldActionLog>,
    mut writer: MessageWriter<LogEvent>,
) {
    let Some(death) = deaths.entries.last() else {
        return;
    };
    if step.elapsed_days - death.day > 0.25 {
        return;
    }

    let programs = programs_for_death_reason(&death.reason);
    for program in programs.iter().copied() {
        if !state.unlocked.contains(&program) {
            state.unlocked.push(program);
        }
    }
    if death.reason.contains("cold") || death.reason.contains("exposure") {
        state.cold_discovery_pressure = (state.cold_discovery_pressure + 0.22).clamp(0.0, 1.0);
    } else if death.reason.contains("starv") {
        state.hunger_discovery_pressure = (state.hunger_discovery_pressure + 0.22).clamp(0.0, 1.0);
    } else if death.reason.contains("predator")
        || death.reason.contains("wound")
        || death.reason.contains("injury")
    {
        state.defense_discovery_pressure =
            (state.defense_discovery_pressure + 0.22).clamp(0.0, 1.0);
    } else {
        state.culture_pressure = (state.culture_pressure + 0.10).clamp(0.0, 1.0);
    }

    push_world_action(
        &mut world_actions,
        step.elapsed_days,
        "Cultural pressure rose",
        format!(
            "{}'s death from {} changed what future minds are likely to discover",
            death.npc_name, death.reason
        ),
    );
    writer.write(LogEvent::new(
        LogEventKind::Discovery,
        format!(
            "{}'s death raised cultural discovery pressure",
            death.npc_name
        ),
    ));
}

fn update_world_cultural_pressure(
    step: Res<SimulationStep>,
    stats: Res<WorldStats>,
    population: Res<PopulationStats>,
    deaths: Res<NpcDeathLog>,
    mut state: ResMut<WorldProgramState>,
    mut world_mind: ResMut<WorldMind>,
    mut world_actions: ResMut<WorldActionLog>,
    mut writer: MessageWriter<LogEvent>,
) {
    let recent_cold_deaths = deaths
        .entries
        .iter()
        .rev()
        .take_while(|entry| step.elapsed_days - entry.day <= 8.0)
        .filter(|entry| entry.reason.contains("cold"))
        .count();
    let generations_elapsed = step.elapsed_days / (22.0 * 365.0);
    let thriving_stalled = population
        .last_birth_day
        .map(|day| step.elapsed_days - day > 365.0 * 2.0)
        .unwrap_or(true);
    let cold_emergency = (recent_cold_deaths >= 4 || stats.cold_stressed_npcs >= 5)
        && generations_elapsed > 2.0
        && thriving_stalled;
    let population_emergency = stats.npcs > 0 && stats.npcs <= 2 && generations_elapsed > 2.5;
    let hunger_emergency = stats.npcs > 0
        && stats.total_food_carried + stats.total_food_stockpiled < 1.0
        && generations_elapsed > 2.0
        && thriving_stalled;

    let mut reason = None;
    if cold_emergency {
        state.cold_discovery_pressure = (state.cold_discovery_pressure + 0.18).clamp(0.0, 1.0);
        reason = Some("cold deaths or exposure stress");
    }
    if population_emergency {
        state.care_discovery_pressure = (state.care_discovery_pressure + 0.16).clamp(0.0, 1.0);
        reason = Some("population survival threshold");
    }
    if hunger_emergency {
        state.hunger_discovery_pressure = (state.hunger_discovery_pressure + 0.18).clamp(0.0, 1.0);
        reason = Some("food security threshold");
    }

    state.cold_discovery_pressure = (state.cold_discovery_pressure * 0.996).clamp(0.0, 1.0);
    state.hunger_discovery_pressure = (state.hunger_discovery_pressure * 0.996).clamp(0.0, 1.0);
    state.care_discovery_pressure = (state.care_discovery_pressure * 0.997).clamp(0.0, 1.0);
    state.defense_discovery_pressure = (state.defense_discovery_pressure * 0.997).clamp(0.0, 1.0);
    state.culture_pressure = (state.culture_pressure * 0.998
        + (state.cold_discovery_pressure
            + state.hunger_discovery_pressure
            + state.care_discovery_pressure
            + state.defense_discovery_pressure)
            * 0.001)
        .clamp(0.0, 1.0);

    if let Some(reason) = reason {
        state.last_grant_day = step.elapsed_days;
        state.last_reason = reason.to_string();
        world_mind.intent = format!("Pressure culture toward solutions for {reason}");
        push_world_action(
            &mut world_actions,
            step.elapsed_days,
            "Discovery pressure shifted",
            format!("Raised discovery odds for {reason} without granting knowledge directly"),
        );
        writer.write(LogEvent::new(
            LogEventKind::Discovery,
            format!("World pressure now favors discoveries for {reason}"),
        ));
    }
}

fn world_nurture_thriving_society(
    step: Res<SimulationStep>,
    stats: Res<WorldStats>,
    mut state: ResMut<WorldProgramState>,
    mut world_mind: ResMut<WorldMind>,
    mut world_actions: ResMut<WorldActionLog>,
    mut writer: MessageWriter<LogEvent>,
    mut regions: Query<&mut crate::world::map::RegionState>,
) {
    if stats.npcs == 0 || step.elapsed_days - state.last_nurture_day < 3.0 {
        return;
    }

    let npc_count = stats.npcs.max(1) as f32;
    let food_stores = stats.total_food_carried + stats.total_food_stockpiled;
    let wood_stores = stats.total_wood_carried + stats.total_wood_stockpiled;
    let food_gap = (npc_count * 0.85 - food_stores).max(0.0);
    let wood_gap = (npc_count * 0.55 - wood_stores).max(0.0);
    let housing_gap = stats.npcs.saturating_sub(stats.shelters * 2);
    let climate_gap = stats.cold_stressed_npcs as f32 + stats.avg_npc_exposure * npc_count;
    let danger_gap = stats.predators.saturating_sub((stats.npcs / 3).max(1)) as f32;
    let should_nurture = food_gap > 0.0
        || wood_gap > 0.0
        || housing_gap > 0
        || climate_gap > 1.2
        || danger_gap > 0.0;

    if !should_nurture {
        return;
    }

    state.last_nurture_day = step.elapsed_days;
    world_mind.stance = "Selecting".to_string();
    world_mind.intent =
        "Shift ecology and social incentives so better-adapted generations emerge".to_string();
    world_mind.resource_bias =
        (world_mind.resource_bias + food_gap * 0.015 + wood_gap * 0.010).clamp(0.70, 1.45);
    world_mind.climate_bias = (world_mind.climate_bias - climate_gap * 0.002).clamp(-0.08, 0.08);
    world_mind.nurture = (world_mind.nurture + 0.04).clamp(0.0, 1.0);

    let forage_adaptation = (0.04 + food_gap * 0.012).clamp(0.0, 0.22);
    let biomass_adaptation =
        (0.020 + wood_gap * 0.010 + housing_gap as f32 * 0.008).clamp(0.0, 0.16);
    for mut region in &mut regions {
        if food_gap > 0.0 {
            region.forage = (region.forage + forage_adaptation).min(region.forage_capacity);
        }
        if wood_gap > 0.0 || housing_gap > 0 {
            region.tree_biomass =
                (region.tree_biomass + biomass_adaptation).min(region.tree_biomass_capacity);
        }
    }

    for program in [
        ProgramId::FoodStorage,
        ProgramId::Agriculture,
        ProgramId::SeedSaving,
        ProgramId::ShelterRepair,
        ProgramId::Childcare,
        ProgramId::Teaching,
        ProgramId::ConflictMediation,
    ] {
        if !state.unlocked.contains(&program) {
            state.unlocked.push(program);
        }
    }

    push_world_action(
        &mut world_actions,
        step.elapsed_days,
        "Selection pressure adjusted",
        format!(
            "Ecosystem and culture pressures shifted: food gap {:.1}, wood gap {:.1}, housing gap {}",
            food_gap, wood_gap, housing_gap
        ),
    );
    writer.write(LogEvent::new(
        LogEventKind::Discovery,
        "The world changed survival pressures for future generations".to_string(),
    ));
}

fn update_migration_pressure(
    step: Res<SimulationStep>,
    stats: Res<WorldStats>,
    mut state: ResMut<WorldProgramState>,
    mut world_actions: ResMut<WorldActionLog>,
    mut writer: MessageWriter<LogEvent>,
) {
    let generations_elapsed = step.elapsed_days / (22.0 * 365.0);
    if stats.npcs >= 4
        || step.elapsed_days - state.last_spawn_day < 365.0
        || (generations_elapsed < 3.0 && stats.npcs > 0)
    {
        return;
    }

    state.last_spawn_day = step.elapsed_days;
    state.care_discovery_pressure = (state.care_discovery_pressure + 0.24).clamp(0.0, 1.0);
    state.hunger_discovery_pressure = (state.hunger_discovery_pressure + 0.14).clamp(0.0, 1.0);
    state.culture_pressure = (state.culture_pressure + 0.18).clamp(0.0, 1.0);
    push_world_action(
        &mut world_actions,
        step.elapsed_days,
        "Migration pressure rose",
        "The world made family, teaching, and food-security adaptations more likely",
    );
    writer.write(LogEvent::new(
        LogEventKind::Discovery,
        "Population stress increased migration and culture pressure".to_string(),
    ));
}

fn build_society_projects(
    mut commands: Commands,
    step: Res<SimulationStep>,
    stats: Res<WorldStats>,
    deaths: Res<NpcDeathLog>,
    mut society: ResMut<SocietyProgress>,
    mut world_actions: ResMut<WorldActionLog>,
    mut writer: MessageWriter<LogEvent>,
    mut npcs: Query<
        (
            &Transform,
            &KnownPrograms,
            &crate::magic::storage::ManaPractice,
            &mut NpcHome,
            &mut Inventory,
            Option<&mut crate::agents::personality::NpcPsyche>,
            Option<&crate::agents::factions::FactionMember>,
        ),
        With<Npc>,
    >,
    mut shelters: ParamSet<(
        Query<
            (
                Entity,
                &Transform,
                Option<&crate::agents::factions::FactionMember>,
            ),
            With<Shelter>,
        >,
        Query<
            (
                Entity,
                &Transform,
                Option<&crate::agents::factions::FactionMember>,
                &mut ShelterStockpile,
            ),
            With<Shelter>,
        >,
    )>,
    structures: Query<&CivicStructure>,
    faction_societies: Query<&FactionSociety>,
    settlements: Query<&Settlement>,
) {
    society.stage =
        settlement_stage(stats.npcs, stats.shelters, stats.civic_structures).to_string();

    let shelter_positions = shelters
        .p0()
        .iter()
        .map(|(entity, transform, faction)| {
            (
                entity,
                transform.translation.truncate(),
                faction.map(|member| member.faction),
            )
        })
        .collect::<Vec<_>>();
    if shelter_positions.is_empty() {
        return;
    }

    for (transform, _, _, mut home, _, _, member) in &mut npcs {
        if home.shelter.is_some() {
            continue;
        }
        let pos = transform.translation.truncate();
        let faction = member.map(|member| member.faction);
        home.shelter = shelter_positions
            .iter()
            .filter(|(_, _, shelter_faction)| {
                faction.is_none() || *shelter_faction == faction || shelter_faction.is_none()
            })
            .min_by(|(_, a, _), (_, b, _)| pos.distance(*a).total_cmp(&pos.distance(*b)))
            .filter(|(_, shelter_pos, _)| pos.distance(*shelter_pos) < 90.0)
            .map(|(entity, _, _)| *entity);
    }

    if step.elapsed_days - society.last_project_day < 2.0 {
        return;
    }

    let known_any = |program: ProgramId| {
        npcs.iter()
            .any(|(_, known, _, _, _, _, _)| known.knows(program))
    };
    let existing =
        |kind: CivicStructureKind| structures.iter().any(|structure| structure.kind == kind);
    let society_ready = faction_societies
        .iter()
        .any(|faction| faction.settlement_drive > 0.40 && faction.cohesion > 0.36);
    let recent_predator_deaths = deaths
        .entries
        .iter()
        .rev()
        .take_while(|entry| step.elapsed_days - entry.day <= 28.0)
        .filter(|entry| entry.reason.contains("predator") || entry.reason.contains("mauled"))
        .count();
    let urgent_defense = recent_predator_deaths >= 2 || stats.predators >= (stats.npcs / 2).max(2);
    let (telekinesis_bias, hearth_bias, warding_bias, hunt_bias, verdant_bias) = npcs.iter().fold(
        (0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32),
        |acc, (_, _, practice, _, _, _, _)| {
            (
                acc.0 + practice.telekinesis,
                acc.1 + practice.hearthspark,
                acc.2 + practice.warding,
                acc.3 + practice.hunter_focus,
                acc.4 + practice.verdant_touch,
            )
        },
    );
    let shelter_center = shelter_positions
        .iter()
        .fold(Vec2::ZERO, |sum, (_, pos, _)| sum + *pos)
        / shelter_positions.len().max(1) as f32;
    let center = settlements
        .iter()
        .max_by(|a, b| a.population.cmp(&b.population))
        .map(|settlement| settlement.center)
        .unwrap_or(shelter_center);
    let layout_push = 12.0 + telekinesis_bias * 1.2;
    let defense_ring = 44.0 + warding_bias * 2.4 + hunt_bias * 2.0;
    let farm_reach = 50.0 + verdant_bias * 2.6;
    let forge_pull = 34.0 + hearth_bias * 2.2;

    let next_project = if !society_ready {
        None
    } else if urgent_defense
        && stats.shelters >= 2
        && known_any(ProgramId::Woodworking)
        && stats.total_wood_carried + stats.total_wood_stockpiled > 2.8
        && !existing(CivicStructureKind::Fence)
    {
        Some((
            CivicStructureKind::Fence,
            center + Vec2::new(0.0, -defense_ring),
        ))
    } else if urgent_defense
        && stats.npcs >= 4
        && known_any(ProgramId::Watchkeeping)
        && stats.total_wood_carried + stats.total_wood_stockpiled > 2.6
        && !existing(CivicStructureKind::WatchPost)
    {
        Some((
            CivicStructureKind::WatchPost,
            center + Vec2::new(layout_push * 0.25, defense_ring + 6.0),
        ))
    } else if stats.shelters >= 3
        && !existing(CivicStructureKind::Plaza)
        && faction_societies
            .iter()
            .any(|society| society.cohesion > 0.40)
    {
        Some((CivicStructureKind::Plaza, center))
    } else if stats.shelters >= 3
        && !existing(CivicStructureKind::Road)
        && faction_societies
            .iter()
            .any(|society| society.settlement_drive > 0.45)
    {
        Some((
            CivicStructureKind::Road,
            center + Vec2::new(0.0, -22.0 - telekinesis_bias.min(10.0)),
        ))
    } else if stats.shelters >= 2
        && known_any(ProgramId::Woodworking)
        && stats.total_wood_carried + stats.total_wood_stockpiled > 2.8
        && !existing(CivicStructureKind::Fence)
    {
        Some((
            CivicStructureKind::Fence,
            center + Vec2::new(0.0, -defense_ring),
        ))
    } else if stats.shelters >= 2
        && known_any(ProgramId::Agriculture)
        && verdant_bias > 0.7
        && stats.total_food_stockpiled + stats.total_food_carried > 1.8
        && !existing(CivicStructureKind::Farm)
    {
        Some((
            CivicStructureKind::Farm,
            center + Vec2::new(-farm_reach, 26.0),
        ))
    } else if stats.shelters >= 2
        && known_any(ProgramId::Agriculture)
        && stats.total_food_stockpiled + stats.total_food_carried > 2.0
        && !existing(CivicStructureKind::Farm)
    {
        Some((
            CivicStructureKind::Farm,
            center + Vec2::new(-farm_reach, 26.0),
        ))
    } else if stats.shelters >= 2
        && known_any(ProgramId::AnimalHusbandry)
        && stats.animals >= 2
        && !existing(CivicStructureKind::Pasture)
    {
        Some((
            CivicStructureKind::Pasture,
            center + Vec2::new(farm_reach + 2.0, 28.0),
        ))
    } else if stats.shelters >= 2
        && known_any(ProgramId::Toolmaking)
        && (telekinesis_bias > 0.65 || hearth_bias > 0.65)
        && stats.total_wood_carried + stats.total_wood_stockpiled > 2.8
        && !existing(CivicStructureKind::Workshop)
    {
        Some((
            CivicStructureKind::Workshop,
            center + Vec2::new(forge_pull, 10.0),
        ))
    } else if stats.shelters >= 2
        && known_any(ProgramId::Toolmaking)
        && stats.total_wood_carried + stats.total_wood_stockpiled > 3.2
        && !existing(CivicStructureKind::Workshop)
    {
        Some((
            CivicStructureKind::Workshop,
            center + Vec2::new(forge_pull, 8.0),
        ))
    } else if stats.npcs >= 4
        && known_any(ProgramId::Childcare)
        && stats.total_food_stockpiled + stats.total_food_carried > 2.4
        && !existing(CivicStructureKind::Nursery)
    {
        Some((
            CivicStructureKind::Nursery,
            center + Vec2::new(-28.0 + warding_bias * 0.8, 10.0),
        ))
    } else if stats.npcs >= 4
        && known_any(ProgramId::Watchkeeping)
        && stats.total_wood_carried + stats.total_wood_stockpiled > 2.6
        && !existing(CivicStructureKind::WatchPost)
    {
        Some((
            CivicStructureKind::WatchPost,
            center + Vec2::new(layout_push * 0.25, defense_ring + 6.0),
        ))
    } else if stats.shelters >= 3
        && known_any(ProgramId::FoodStorage)
        && stats.total_food_stockpiled + stats.total_food_carried > 4.2
        && !existing(CivicStructureKind::Granary)
    {
        Some((
            CivicStructureKind::Granary,
            center + Vec2::new(-44.0, -16.0),
        ))
    } else if stats.shelters >= 3
        && known_any(ProgramId::Blacksmithing)
        && stats.total_metal > 1.2
        && stats.total_ore > 1.6
        && !existing(CivicStructureKind::Forge)
    {
        Some((
            CivicStructureKind::Forge,
            center + Vec2::new(forge_pull + 10.0, -18.0 - hearth_bias),
        ))
    } else if stats.shelters >= 5
        && known_any(ProgramId::Governance)
        && faction_societies.iter().any(|society| {
            society.cohesion > 0.52
                && society.settlement_drive > 0.54
                && !matches!(
                    society.governance,
                    crate::agents::society::GovernanceKind::KinCircle
                )
        })
        && !existing(CivicStructureKind::TownHall)
    {
        Some((
            CivicStructureKind::TownHall,
            center + Vec2::new(telekinesis_bias * 0.5, 0.0),
        ))
    } else {
        None
    };

    let Some((kind, position)) = next_project else {
        if stats.npcs > stats.shelters.saturating_mul(2)
            && stats.total_wood_carried + stats.total_wood_stockpiled > 1.8
            && spend_project_resources(&mut shelters, &mut npcs, 1.2, 0.0, 0.0, 0.0, 0.0, 0.0)
        {
            let shelter_index = stats.shelters as f32;
            let angle = shelter_index * 0.92 + step.elapsed_days * 0.01;
            let radius = 48.0 + (shelter_index % 4.0) * 16.0;
            let shelter_pos = center + Vec2::new(angle.cos(), angle.sin()) * radius;
            commands.spawn((
                Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.0), Vec2::splat(1.0)),
                Transform::from_xyz(shelter_pos.x, shelter_pos.y, 1.8),
                Shelter {
                    integrity: 1.0,
                    safety_bonus: 0.25,
                    insulation: 0.42,
                },
                ShelterStockpile::default(),
            ));
            society.last_project_day = step.elapsed_days;
            society.last_project = "Clustered shelter".to_string();
            push_world_action(
                &mut world_actions,
                step.elapsed_days,
                "Housing cluster expanded",
                "The settlement added another visible home near the village center",
            );
            writer.write(LogEvent::new(
                LogEventKind::Construction,
                "The settlement expanded with another clustered shelter".to_string(),
            ));
        }
        return;
    };
    let (wood_cost, food_cost, ore_cost, metal_cost, fiber_cost, hides_cost) = project_cost(kind);
    if !spend_project_resources(
        &mut shelters,
        &mut npcs,
        wood_cost,
        food_cost,
        ore_cost,
        metal_cost,
        fiber_cost,
        hides_cost,
    ) {
        return;
    }

    commands.spawn((
        Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.0), Vec2::splat(1.0)),
        Transform::from_xyz(position.x, position.y, 1.9).with_rotation(project_rotation(kind)),
        CivicStructure {
            kind,
            progress: 0.18,
        },
    ));
    society.last_project_day = step.elapsed_days;
    society.last_project = format!("Planning {}", kind.label());
    for (_, _, _, _, _, psyche, _) in &mut npcs {
        if let Some(mut psyche) = psyche {
            let bonus = match psyche.personality {
                PersonalityType::Builder => 0.10,
                PersonalityType::Caregiver => {
                    if matches!(kind, CivicStructureKind::Nursery | CivicStructureKind::Farm) {
                        0.12
                    } else {
                        0.05
                    }
                }
                PersonalityType::Sovereign => {
                    if kind == CivicStructureKind::TownHall {
                        0.12
                    } else {
                        0.06
                    }
                }
                PersonalityType::Scholar => {
                    if matches!(
                        kind,
                        CivicStructureKind::Workshop
                            | CivicStructureKind::Forge
                            | CivicStructureKind::Farm
                    ) {
                        0.09
                    } else {
                        0.04
                    }
                }
                PersonalityType::Raider => {
                    if kind == CivicStructureKind::WatchPost {
                        0.08
                    } else {
                        0.02
                    }
                }
                PersonalityType::Mystic | PersonalityType::Hedonist => 0.03,
            };
            psyche.happiness = (psyche.happiness + bonus).clamp(0.0, 1.0);
            if matches!(
                psyche.personality,
                PersonalityType::Builder | PersonalityType::Sovereign
            ) {
                psyche.reward_building();
            }
        }
    }
    push_world_action(
        &mut world_actions,
        step.elapsed_days,
        "Civic project started",
        format!(
            "Started a {} during {} stage using W {:.1} F {:.1} O {:.1} M {:.1}",
            kind.label(),
            society.stage,
            wood_cost,
            food_cost,
            ore_cost,
            metal_cost
        ),
    );
    writer.write(LogEvent::new(
        LogEventKind::Construction,
        format!("The settlement began building a {}", kind.label()),
    ));
}

fn advance_civic_projects(
    mut commands: Commands,
    clock: Res<SimulationClock>,
    mut writer: MessageWriter<LogEvent>,
    npcs: Query<
        (
            &Transform,
            &Npc,
            &crate::magic::storage::ManaPractice,
            Option<&crate::agents::factions::FactionMember>,
        ),
        With<Npc>,
    >,
    animals: Query<Entity, With<crate::agents::animal::Animal>>,
    mut structures: Query<(
        Entity,
        &mut CivicStructure,
        &Transform,
        Option<&crate::agents::factions::FactionMember>,
    )>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    let mana_workers = npcs
        .iter()
        .map(|(transform, _, practice, member)| {
            (
                transform.translation.truncate(),
                practice.telekinesis + practice.hearthspark * 0.35,
                practice.warding,
                member.map(|member| member.faction),
            )
        })
        .collect::<Vec<_>>();
    let npc_snapshots = npcs
        .iter()
        .map(|(transform, npc, _, member)| {
            (
                transform.translation.truncate(),
                npc.discovery_drive,
                npc.tool_knowledge,
                member.map(|member| member.faction),
            )
        })
        .collect::<Vec<_>>();

    for (entity, mut structure, transform, member) in &mut structures {
        if structure.progress >= 1.0 {
            continue;
        }
        let pos = transform.translation.truncate();
        let faction = member.map(|member| member.faction);
        let workers = npc_snapshots
            .iter()
            .filter(|(npc_pos, _, _, npc_faction)| {
                pos.distance(*npc_pos) < 96.0
                    && (faction.is_none() || *npc_faction == faction || npc_faction.is_none())
            })
            .count() as f32;
        let curiosity = npc_snapshots
            .iter()
            .filter(|(npc_pos, _, _, npc_faction)| {
                pos.distance(*npc_pos) < 96.0
                    && (faction.is_none() || *npc_faction == faction || npc_faction.is_none())
            })
            .map(|(_, discovery, _, _)| *discovery)
            .sum::<f32>();
        let tools = npc_snapshots
            .iter()
            .filter(|(npc_pos, _, _, npc_faction)| {
                pos.distance(*npc_pos) < 96.0
                    && (faction.is_none() || *npc_faction == faction || npc_faction.is_none())
            })
            .map(|(_, _, tools, _)| *tools)
            .sum::<f32>();
        let mana_support = mana_workers
            .iter()
            .filter(|(npc_pos, _, _, npc_faction)| {
                pos.distance(*npc_pos) < 96.0
                    && (faction.is_none() || *npc_faction == faction || npc_faction.is_none())
            })
            .map(|(_, telekinesis, warding, _)| *telekinesis * 0.8 + *warding * 0.3)
            .sum::<f32>();
        let progress_gain =
            (0.03 + workers * 0.018 + curiosity * 0.006 + tools * 0.004 + mana_support * 0.010)
                * delta_days;
        let before = structure.progress;
        if progress_gain > 0.001 && workers > 0.0 {
            for (idx, (worker_pos, _, _, npc_faction)) in npc_snapshots
                .iter()
                .filter(|(npc_pos, _, _, worker_faction)| {
                    pos.distance(*npc_pos) < 64.0
                        && (faction.is_none()
                            || *worker_faction == faction
                            || worker_faction.is_none())
                })
                .take(3)
                .enumerate()
            {
                let _ = npc_faction;
                let material_color = match structure.kind {
                    CivicStructureKind::Farm | CivicStructureKind::Pasture => {
                        Color::srgba(0.72, 0.86, 0.38, 0.64)
                    }
                    CivicStructureKind::Forge | CivicStructureKind::Workshop => {
                        Color::srgba(0.82, 0.64, 0.36, 0.66)
                    }
                    _ => Color::srgba(0.90, 0.78, 0.48, 0.62),
                };
                let midpoint = worker_pos.lerp(pos, 0.42 + idx as f32 * 0.10);
                spawn_transient_effect(
                    &mut commands,
                    midpoint,
                    material_color,
                    Vec2::new(6.0, 3.5),
                    (pos - *worker_pos).normalize_or_zero() * 10.0,
                    0.22,
                    5.9,
                );
            }
        }
        structure.progress = (structure.progress + progress_gain).clamp(0.0, 1.0);
        if before < 1.0 && structure.progress >= 1.0 {
            if structure.kind == CivicStructureKind::Pasture && animals.iter().count() < 20 {
                for idx in 0..2 {
                    let offset = Vec2::new(
                        if idx == 0 { -8.0 } else { 9.0 },
                        if idx == 0 { -4.0 } else { 6.0 },
                    );
                    commands.spawn(
                        AnimalBundle::new(pos + offset, 26.0, 0.78)
                            .with_age_days(240.0 + idx as f32 * 70.0),
                    );
                }
            }
            writer.write(LogEvent::new(
                LogEventKind::Construction,
                format!("The settlement finished a {}", structure.kind.label()),
            ));
        } else if structure.progress > before + 0.10 {
            writer.write(LogEvent::new(
                LogEventKind::Construction,
                format!(
                    "Work advanced on the {} ({:.0}%)",
                    structure.kind.label(),
                    structure.progress * 100.0
                ),
            ));
        }
        if structure.progress <= 0.02 {
            commands.entity(entity).despawn();
        }
    }
}

fn project_cost(kind: CivicStructureKind) -> (f32, f32, f32, f32, f32, f32) {
    match kind {
        CivicStructureKind::Road => (0.8, 0.0, 0.0, 0.0, 0.0, 0.0),
        CivicStructureKind::Plaza => (1.4, 0.2, 0.0, 0.2, 0.0, 0.0),
        CivicStructureKind::Fence => (1.8, 0.0, 0.0, 0.0, 0.0, 0.0),
        CivicStructureKind::Farm => (1.5, 0.6, 0.0, 0.0, 0.4, 0.0),
        CivicStructureKind::Pasture => (1.7, 0.8, 0.0, 0.0, 0.2, 0.4),
        CivicStructureKind::Workshop => (2.4, 0.2, 0.0, 0.0, 0.0, 0.0),
        CivicStructureKind::Nursery => (1.6, 1.0, 0.0, 0.0, 0.4, 0.0),
        CivicStructureKind::WatchPost => (2.2, 0.2, 0.0, 0.2, 0.0, 0.0),
        CivicStructureKind::Granary => (2.6, 1.4, 0.0, 0.0, 0.0, 0.0),
        CivicStructureKind::Forge => (1.8, 0.4, 1.0, 0.8, 0.0, 0.0),
        CivicStructureKind::TownHall => (3.4, 1.2, 0.8, 0.8, 0.6, 0.0),
    }
}

fn project_rotation(kind: CivicStructureKind) -> Quat {
    match kind {
        CivicStructureKind::Road => Quat::from_rotation_z(0.0),
        CivicStructureKind::Fence => Quat::from_rotation_z(0.0),
        CivicStructureKind::Farm => Quat::from_rotation_z(0.04),
        CivicStructureKind::Pasture => Quat::from_rotation_z(-0.03),
        CivicStructureKind::WatchPost => Quat::from_rotation_z(0.02),
        _ => Quat::IDENTITY,
    }
}

fn spend_project_resources(
    shelters: &mut ParamSet<(
        Query<
            (
                Entity,
                &Transform,
                Option<&crate::agents::factions::FactionMember>,
            ),
            With<Shelter>,
        >,
        Query<
            (
                Entity,
                &Transform,
                Option<&crate::agents::factions::FactionMember>,
                &mut ShelterStockpile,
            ),
            With<Shelter>,
        >,
    )>,
    npcs: &mut Query<
        (
            &Transform,
            &KnownPrograms,
            &crate::magic::storage::ManaPractice,
            &mut NpcHome,
            &mut Inventory,
            Option<&mut crate::agents::personality::NpcPsyche>,
            Option<&crate::agents::factions::FactionMember>,
        ),
        With<Npc>,
    >,
    wood_cost: f32,
    food_cost: f32,
    ore_cost: f32,
    metal_cost: f32,
    fiber_cost: f32,
    hides_cost: f32,
) -> bool {
    let total_wood = shelters
        .p1()
        .iter()
        .map(|(_, _, _, pile)| pile.wood)
        .sum::<f32>()
        + npcs
            .iter()
            .map(|(_, _, _, _, inv, _, _)| inv.wood)
            .sum::<f32>();
    let total_food = shelters
        .p1()
        .iter()
        .map(|(_, _, _, pile)| pile.food)
        .sum::<f32>()
        + npcs
            .iter()
            .map(|(_, _, _, _, inv, _, _)| inv.food)
            .sum::<f32>();
    let total_ore = shelters
        .p1()
        .iter()
        .map(|(_, _, _, pile)| pile.ore)
        .sum::<f32>()
        + npcs
            .iter()
            .map(|(_, _, _, _, inv, _, _)| inv.ore)
            .sum::<f32>();
    let total_metal = shelters
        .p1()
        .iter()
        .map(|(_, _, _, pile)| pile.metal)
        .sum::<f32>()
        + npcs
            .iter()
            .map(|(_, _, _, _, inv, _, _)| inv.metal)
            .sum::<f32>();
    let total_fiber = shelters
        .p1()
        .iter()
        .map(|(_, _, _, pile)| pile.fiber)
        .sum::<f32>()
        + npcs
            .iter()
            .map(|(_, _, _, _, inv, _, _)| inv.fiber)
            .sum::<f32>();
    let total_hides = shelters
        .p1()
        .iter()
        .map(|(_, _, _, pile)| pile.hides)
        .sum::<f32>()
        + npcs
            .iter()
            .map(|(_, _, _, _, inv, _, _)| inv.hides)
            .sum::<f32>();
    if total_wood + 0.001 < wood_cost
        || total_food + 0.001 < food_cost
        || total_ore + 0.001 < ore_cost
        || total_metal + 0.001 < metal_cost
        || total_fiber + 0.001 < fiber_cost
        || total_hides + 0.001 < hides_cost
    {
        return false;
    }

    let mut remaining_wood = wood_cost;
    let mut remaining_food = food_cost;
    let mut remaining_ore = ore_cost;
    let mut remaining_metal = metal_cost;
    let mut remaining_fiber = fiber_cost;
    let mut remaining_hides = hides_cost;
    {
        for (_, _, _, mut pile) in &mut shelters.p1() {
            drain_resource(&mut pile.wood, &mut remaining_wood);
            drain_resource(&mut pile.food, &mut remaining_food);
            drain_resource(&mut pile.ore, &mut remaining_ore);
            drain_resource(&mut pile.metal, &mut remaining_metal);
            drain_resource(&mut pile.fiber, &mut remaining_fiber);
            drain_resource(&mut pile.hides, &mut remaining_hides);
        }
    }
    for (_, _, _, _, mut inventory, _, _) in npcs.iter_mut() {
        drain_resource(&mut inventory.wood, &mut remaining_wood);
        drain_resource(&mut inventory.food, &mut remaining_food);
        drain_resource(&mut inventory.ore, &mut remaining_ore);
        drain_resource(&mut inventory.metal, &mut remaining_metal);
        drain_resource(&mut inventory.fiber, &mut remaining_fiber);
        drain_resource(&mut inventory.hides, &mut remaining_hides);
    }
    true
}

fn drain_resource(store: &mut f32, remaining: &mut f32) {
    if *remaining <= 0.0 || *store <= 0.0 {
        return;
    }
    let used = store.min(*remaining);
    *store -= used;
    *remaining -= used;
}

fn settlement_stage(npcs: usize, shelters: usize, civic_structures: usize) -> &'static str {
    if shelters >= 10 && civic_structures >= 6 && npcs >= 18 {
        "City"
    } else if shelters >= 7 && civic_structures >= 5 && npcs >= 12 {
        "Town"
    } else if shelters >= 4 && civic_structures >= 3 && npcs >= 7 {
        "Village"
    } else if shelters >= 2 && npcs >= 4 {
        "Hamlet"
    } else {
        "Band"
    }
}

fn apply_known_program_effects(
    clock: Res<SimulationClock>,
    mut npcs: Query<(
        &KnownPrograms,
        &mut Npc,
        &mut crate::agents::needs::Needs,
        &mut crate::agents::inventory::Inventory,
        &mut crate::magic::mana::ManaReservoir,
        &mut crate::magic::storage::ManaPractice,
    )>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    for (programs, mut npc, mut needs, mut inventory, mut mana, mut practice) in &mut npcs {
        if programs.knows(ProgramId::WarmClothing) {
            npc.exposure = (npc.exposure - delta_days * 0.035).max(0.0);
        }
        if programs.knows(ProgramId::HearthKeeping) && inventory.wood > 0.15 {
            needs.safety = (needs.safety + delta_days * 0.010).min(1.0);
        }
        if programs.knows(ProgramId::FirstAid) && npc.health < 55.0 {
            npc.health = (npc.health + delta_days * 0.08).min(55.0);
        }
        if programs.knows(ProgramId::Sanitation) {
            needs.safety = (needs.safety + delta_days * 0.004).min(1.0);
        }
        if programs.knows(ProgramId::FoodStorage) {
            inventory.max_food = inventory.max_food.max(3.4);
        }
        if programs.knows(ProgramId::Weaving) || programs.knows(ProgramId::Leatherworking) {
            inventory.max_wood = inventory.max_wood.max(3.4);
        }
        if programs.knows(ProgramId::Toolmaking) {
            npc.tool_knowledge = (npc.tool_knowledge + delta_days * 0.004).min(1.0);
        }
        if programs.knows(ProgramId::Teaching) {
            needs.social = (needs.social + delta_days * 0.006).min(1.0);
        }
        if programs.knows(ProgramId::ConflictMediation) {
            npc.aggression_drive = (npc.aggression_drive - delta_days * 0.003).max(0.05);
        }
        if programs.knows(ProgramId::ManaCirculation) {
            practice.control = (practice.control + delta_days * 0.006).min(1.0);
            needs.fatigue = (needs.fatigue - delta_days * 0.006).max(0.0);
        }
        if programs.knows(ProgramId::ManaWarding) && mana.stored > 0.05 {
            let spend = mana.stored.min(delta_days * 0.03);
            mana.stored -= spend;
            npc.exposure = (npc.exposure - spend * 0.8).max(0.0);
            needs.safety = (needs.safety + spend * 0.5).min(1.0);
        }
        if programs.knows(ProgramId::ManaStorage) {
            mana.capacity = mana.capacity.max(32.0);
        }
        if programs.knows(ProgramId::WarmClothing) && inventory.clothing > 0.08 {
            npc.exposure =
                (npc.exposure - delta_days * (0.020 + inventory.clothing * 0.025)).max(0.0);
            needs.safety = (needs.safety + delta_days * 0.010).min(1.0);
        }
    }
}

fn materialize_resource_chains(
    clock: Res<SimulationClock>,
    settings: Res<MapSettings>,
    regions: Query<(&RegionTile, &crate::world::map::RegionState)>,
    campfires: Query<&Transform, With<crate::world::resources::Campfire>>,
    structures: Query<(&CivicStructure, &Transform)>,
    mut npcs: Query<
        (
            &Transform,
            &NpcIntent,
            &KnownPrograms,
            &mut crate::agents::inventory::Inventory,
            &mut Npc,
            &mut crate::agents::needs::Needs,
        ),
        With<Npc>,
    >,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    let region_index = regions
        .iter()
        .map(|(tile, state)| {
            (
                tile.coord,
                (
                    tile.soil_fertility,
                    tile.elevation,
                    tile.mana_density,
                    state.forage,
                ),
            )
        })
        .collect::<std::collections::HashMap<_, _>>();

    for (transform, intent, programs, mut inventory, mut npc, mut needs) in &mut npcs {
        let pos = transform.translation.truncate();
        let coord = settings.tile_coord_for_position(pos);
        let (fertility, elevation, mana_density, forage) = region_index
            .get(&coord)
            .copied()
            .unwrap_or((0.0, 0.0, 0.0, 0.0));
        let near_fire = campfires
            .iter()
            .any(|fire| fire.translation.truncate().distance(pos) < 40.0);
        let near_forge = structures.iter().any(|(structure, structure_transform)| {
            structure.kind == CivicStructureKind::Forge
                && structure_transform.translation.truncate().distance(pos) < 58.0
        });
        let near_granary = structures.iter().any(|(structure, structure_transform)| {
            structure.kind == CivicStructureKind::Granary
                && structure_transform.translation.truncate().distance(pos) < 58.0
        });

        if intent.label == "Forage" {
            if programs.knows(ProgramId::SeedSaving) || programs.knows(ProgramId::Agriculture) {
                inventory.seeds += delta_days * (0.05 + fertility * 0.08);
            }
            if programs.knows(ProgramId::Weaving) {
                inventory.fiber += delta_days * (0.04 + forage * 0.05);
            }
        }

        if (intent.label == "Explore" || intent.label == "Gather Wood")
            && programs.knows(ProgramId::Mining)
            && (elevation > 0.48 || mana_density > 0.58)
        {
            inventory.ore += delta_days * (0.05 + elevation * 0.05 + mana_density * 0.03);
        }

        if programs.knows(ProgramId::Agriculture)
            && inventory.seeds > 0.06
            && fertility > 0.42
            && (intent.label == "Forage" || intent.label == "Stockpile")
        {
            let planted = (delta_days * 0.04).min(inventory.seeds);
            inventory.seeds -= planted * 0.55;
            inventory.food += planted * (1.5 + fertility);
        }

        if programs.knows(ProgramId::Weaving) && inventory.fiber > 0.16 {
            let spun = (delta_days * 0.05).min(inventory.fiber);
            inventory.fiber -= spun;
            inventory.clothing += spun * 0.75;
        }
        if programs.knows(ProgramId::Leatherworking) && inventory.hides > 0.12 {
            let cured = (delta_days * 0.05).min(inventory.hides);
            inventory.hides -= cured;
            inventory.clothing += cured * 0.95;
        }
        if programs.knows(ProgramId::CharcoalMaking) && near_fire && inventory.wood > 0.20 {
            let charred = (delta_days * 0.04).min(inventory.wood);
            inventory.wood -= charred * 0.45;
            inventory.ore += charred * 0.08;
        }
        if programs.knows(ProgramId::Blacksmithing)
            && near_fire
            && (near_forge || inventory.ore > 0.30)
            && inventory.ore > 0.10
            && inventory.wood > 0.10
        {
            let smelted = (delta_days * 0.05).min(inventory.ore).min(inventory.wood);
            inventory.ore -= smelted;
            inventory.wood -= smelted * 0.55;
            inventory.metal += smelted * 0.85;
            npc.tool_knowledge = (npc.tool_knowledge + smelted * 0.08).min(1.0);
        }
        if programs.knows(ProgramId::PredatorDefense) || programs.knows(ProgramId::Blacksmithing) {
            if inventory.metal > 0.12 && near_fire {
                let forged = (delta_days * 0.04).min(inventory.metal);
                inventory.metal -= forged;
                inventory.weapons += forged * 0.75;
            }
        }
        if near_granary && programs.knows(ProgramId::GranaryManagement) {
            needs.safety = (needs.safety + delta_days * 0.01).min(1.0);
        }
    }
}

pub fn program_def(id: ProgramId) -> Option<&'static ProgramDef> {
    ALL_PROGRAMS.iter().find(|def| def.id == id)
}

pub const ALL_PROGRAMS: &[ProgramDef] = &[
    ProgramDef {
        id: ProgramId::Foraging,
        name: "Foraging",
        domain: ProgramDomain::Survival,
        tier: 1,
        summary: "Find edible wild resources.",
    },
    ProgramDef {
        id: ProgramId::WaterFinding,
        name: "Water Finding",
        domain: ProgramDomain::Survival,
        tier: 1,
        summary: "Locate drinkable water and wet terrain.",
    },
    ProgramDef {
        id: ProgramId::Firemaking,
        name: "Firemaking",
        domain: ProgramDomain::Survival,
        tier: 1,
        summary: "Turn wood into warmth and light.",
    },
    ProgramDef {
        id: ProgramId::HearthKeeping,
        name: "Hearth Keeping",
        domain: ProgramDomain::Survival,
        tier: 1,
        summary: "Maintain shared fires and ember stores.",
    },
    ProgramDef {
        id: ProgramId::ShelterBuilding,
        name: "Shelter Building",
        domain: ProgramDomain::Survival,
        tier: 1,
        summary: "Build basic protection from weather.",
    },
    ProgramDef {
        id: ProgramId::ShelterRepair,
        name: "Shelter Repair",
        domain: ProgramDomain::Survival,
        tier: 1,
        summary: "Restore damaged homes.",
    },
    ProgramDef {
        id: ProgramId::WarmClothing,
        name: "Warm Clothing",
        domain: ProgramDomain::Craft,
        tier: 2,
        summary: "Reduce cold exposure with worn insulation.",
    },
    ProgramDef {
        id: ProgramId::FoodStorage,
        name: "Food Storage",
        domain: ProgramDomain::Food,
        tier: 1,
        summary: "Preserve and stockpile food.",
    },
    ProgramDef {
        id: ProgramId::Childcare,
        name: "Childcare",
        domain: ProgramDomain::Care,
        tier: 1,
        summary: "Protect children and parents.",
    },
    ProgramDef {
        id: ProgramId::Midwifery,
        name: "Midwifery",
        domain: ProgramDomain::Care,
        tier: 2,
        summary: "Improve pregnancy and birth outcomes.",
    },
    ProgramDef {
        id: ProgramId::FirstAid,
        name: "First Aid",
        domain: ProgramDomain::Care,
        tier: 1,
        summary: "Treat injuries before they become fatal.",
    },
    ProgramDef {
        id: ProgramId::HerbalMedicine,
        name: "Herbal Medicine",
        domain: ProgramDomain::Care,
        tier: 2,
        summary: "Use plants and mana-rich herbs for recovery.",
    },
    ProgramDef {
        id: ProgramId::Sanitation,
        name: "Sanitation",
        domain: ProgramDomain::Care,
        tier: 2,
        summary: "Keep waste away from food and shelter.",
    },
    ProgramDef {
        id: ProgramId::WasteManagement,
        name: "Waste Management",
        domain: ProgramDomain::Infrastructure,
        tier: 2,
        summary: "Organize refuse, ash, and compost.",
    },
    ProgramDef {
        id: ProgramId::Toolmaking,
        name: "Toolmaking",
        domain: ProgramDomain::Craft,
        tier: 1,
        summary: "Create simple tools from wood, stone, and bone.",
    },
    ProgramDef {
        id: ProgramId::Woodworking,
        name: "Woodworking",
        domain: ProgramDomain::Craft,
        tier: 2,
        summary: "Build durable wooden objects.",
    },
    ProgramDef {
        id: ProgramId::Blacksmithing,
        name: "Blacksmithing",
        domain: ProgramDomain::Craft,
        tier: 3,
        summary: "Forge metal tools, fasteners, and weapons.",
    },
    ProgramDef {
        id: ProgramId::CharcoalMaking,
        name: "Charcoal Making",
        domain: ProgramDomain::Craft,
        tier: 2,
        summary: "Make hotter fuel for craft and smithing.",
    },
    ProgramDef {
        id: ProgramId::Mining,
        name: "Mining",
        domain: ProgramDomain::Craft,
        tier: 3,
        summary: "Extract ore, stone, and mana crystals.",
    },
    ProgramDef {
        id: ProgramId::Stoneworking,
        name: "Stoneworking",
        domain: ProgramDomain::Craft,
        tier: 2,
        summary: "Shape stone for shelter and infrastructure.",
    },
    ProgramDef {
        id: ProgramId::Pottery,
        name: "Pottery",
        domain: ProgramDomain::Craft,
        tier: 2,
        summary: "Make vessels for food, water, and mana reagents.",
    },
    ProgramDef {
        id: ProgramId::Weaving,
        name: "Weaving",
        domain: ProgramDomain::Craft,
        tier: 2,
        summary: "Make cloth, nets, baskets, and straps.",
    },
    ProgramDef {
        id: ProgramId::Leatherworking,
        name: "Leatherworking",
        domain: ProgramDomain::Craft,
        tier: 2,
        summary: "Make durable clothing and containers.",
    },
    ProgramDef {
        id: ProgramId::Cooking,
        name: "Cooking",
        domain: ProgramDomain::Food,
        tier: 1,
        summary: "Improve nutrition and safety of food.",
    },
    ProgramDef {
        id: ProgramId::Agriculture,
        name: "Agriculture",
        domain: ProgramDomain::Food,
        tier: 2,
        summary: "Cultivate reliable food sources.",
    },
    ProgramDef {
        id: ProgramId::SeedSaving,
        name: "Seed Saving",
        domain: ProgramDomain::Food,
        tier: 2,
        summary: "Preserve future crops.",
    },
    ProgramDef {
        id: ProgramId::AnimalHusbandry,
        name: "Animal Husbandry",
        domain: ProgramDomain::Food,
        tier: 2,
        summary: "Manage animals for food, labor, and warmth.",
    },
    ProgramDef {
        id: ProgramId::Fishing,
        name: "Fishing",
        domain: ProgramDomain::Food,
        tier: 1,
        summary: "Harvest rivers and lakes.",
    },
    ProgramDef {
        id: ProgramId::Irrigation,
        name: "Irrigation",
        domain: ProgramDomain::Infrastructure,
        tier: 3,
        summary: "Move water to farms and settlements.",
    },
    ProgramDef {
        id: ProgramId::GranaryManagement,
        name: "Granary Management",
        domain: ProgramDomain::Food,
        tier: 2,
        summary: "Protect reserves from spoilage and theft.",
    },
    ProgramDef {
        id: ProgramId::Trade,
        name: "Trade",
        domain: ProgramDomain::Society,
        tier: 2,
        summary: "Exchange surpluses and specialties.",
    },
    ProgramDef {
        id: ProgramId::Accounting,
        name: "Accounting",
        domain: ProgramDomain::Society,
        tier: 3,
        summary: "Track stores, debts, and public works.",
    },
    ProgramDef {
        id: ProgramId::Lawkeeping,
        name: "Lawkeeping",
        domain: ProgramDomain::Society,
        tier: 2,
        summary: "Stabilize expectations and obligations.",
    },
    ProgramDef {
        id: ProgramId::ConflictMediation,
        name: "Conflict Mediation",
        domain: ProgramDomain::Society,
        tier: 2,
        summary: "Prevent disputes from becoming violence.",
    },
    ProgramDef {
        id: ProgramId::Teaching,
        name: "Teaching",
        domain: ProgramDomain::Society,
        tier: 2,
        summary: "Transfer knowledge between generations.",
    },
    ProgramDef {
        id: ProgramId::Apprenticeship,
        name: "Apprenticeship",
        domain: ProgramDomain::Society,
        tier: 2,
        summary: "Train specialists through work.",
    },
    ProgramDef {
        id: ProgramId::Storykeeping,
        name: "Storykeeping",
        domain: ProgramDomain::Culture,
        tier: 1,
        summary: "Remember dangers, places, and identity.",
    },
    ProgramDef {
        id: ProgramId::Ritual,
        name: "Ritual",
        domain: ProgramDomain::Culture,
        tier: 2,
        summary: "Coordinate meaning, grief, and commitment.",
    },
    ProgramDef {
        id: ProgramId::Surveying,
        name: "Surveying",
        domain: ProgramDomain::Infrastructure,
        tier: 2,
        summary: "Map terrain and resources.",
    },
    ProgramDef {
        id: ProgramId::RoadBuilding,
        name: "Road Building",
        domain: ProgramDomain::Infrastructure,
        tier: 3,
        summary: "Make travel predictable.",
    },
    ProgramDef {
        id: ProgramId::BridgeBuilding,
        name: "Bridge Building",
        domain: ProgramDomain::Infrastructure,
        tier: 3,
        summary: "Cross rivers, ravines, and marsh.",
    },
    ProgramDef {
        id: ProgramId::Watchkeeping,
        name: "Watchkeeping",
        domain: ProgramDomain::Defense,
        tier: 1,
        summary: "Notice threats before they arrive.",
    },
    ProgramDef {
        id: ProgramId::PredatorDefense,
        name: "Predator Defense",
        domain: ProgramDomain::Defense,
        tier: 2,
        summary: "Organize defense against predators.",
    },
    ProgramDef {
        id: ProgramId::Migration,
        name: "Migration",
        domain: ProgramDomain::Survival,
        tier: 2,
        summary: "Relocate when the local world fails.",
    },
    ProgramDef {
        id: ProgramId::WeatherReading,
        name: "Weather Reading",
        domain: ProgramDomain::Survival,
        tier: 2,
        summary: "Anticipate cold, heat, and dangerous pressure.",
    },
    ProgramDef {
        id: ProgramId::ManaSensing,
        name: "Mana Sensing",
        domain: ProgramDomain::Mana,
        tier: 1,
        summary: "Feel local mana density and instability.",
    },
    ProgramDef {
        id: ProgramId::ManaCirculation,
        name: "Mana Circulation",
        domain: ProgramDomain::Mana,
        tier: 2,
        summary: "Move mana through the body safely.",
    },
    ProgramDef {
        id: ProgramId::ManaHealing,
        name: "Mana Healing",
        domain: ProgramDomain::Mana,
        tier: 3,
        summary: "Use mana to support recovery.",
    },
    ProgramDef {
        id: ProgramId::ManaWarding,
        name: "Mana Warding",
        domain: ProgramDomain::Mana,
        tier: 3,
        summary: "Reduce exposure and hostile pressure.",
    },
    ProgramDef {
        id: ProgramId::ManaAgriculture,
        name: "Mana Agriculture",
        domain: ProgramDomain::Mana,
        tier: 3,
        summary: "Nudge crops and forage with mana.",
    },
    ProgramDef {
        id: ProgramId::ManaSmithing,
        name: "Mana Smithing",
        domain: ProgramDomain::Mana,
        tier: 4,
        summary: "Forge tools that hold magical charge.",
    },
    ProgramDef {
        id: ProgramId::ManaStorage,
        name: "Mana Storage",
        domain: ProgramDomain::Mana,
        tier: 3,
        summary: "Store ambient mana in vessels and places.",
    },
    ProgramDef {
        id: ProgramId::ManaCommunication,
        name: "Mana Communication",
        domain: ProgramDomain::Mana,
        tier: 4,
        summary: "Send simple signals through mana fields.",
    },
    ProgramDef {
        id: ProgramId::ManaTransit,
        name: "Mana Transit",
        domain: ProgramDomain::Mana,
        tier: 4,
        summary: "Reduce travel cost through charged routes.",
    },
    ProgramDef {
        id: ProgramId::ResearchMethod,
        name: "Research Method",
        domain: ProgramDomain::Society,
        tier: 3,
        summary: "Experiment systematically instead of randomly.",
    },
    ProgramDef {
        id: ProgramId::Governance,
        name: "Governance",
        domain: ProgramDomain::Society,
        tier: 3,
        summary: "Coordinate shared decisions and roles.",
    },
    ProgramDef {
        id: ProgramId::FestivalMaking,
        name: "Festival Making",
        domain: ProgramDomain::Culture,
        tier: 2,
        summary: "Restore morale and social trust.",
    },
    ProgramDef {
        id: ProgramId::Artistry,
        name: "Artistry",
        domain: ProgramDomain::Culture,
        tier: 2,
        summary: "Create beauty, identity, and esteem.",
    },
];
