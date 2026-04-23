use bevy::prelude::*;

use crate::agents::npc::{Npc, NpcBundle, NpcGender, NpcHome, NpcSex};
use crate::life::population::{PopulationKind, PopulationStats};
use crate::magic::mana::ManaReservoir;
use crate::magic::storage::ManaStorageStyle;
use crate::systems::logging::{LogEvent, LogEventKind, NpcDeathLog};
use crate::systems::simulation::{SimulationClock, SimulationStep};
use crate::world::director::WorldMind;
use crate::world::map::{MapSettings, RegionTile};
use crate::world::resources::{CivicStructure, CivicStructureKind, Shelter, WorldStats};

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
    pub last_reason: String,
}

impl Default for WorldProgramState {
    fn default() -> Self {
        Self {
            unlocked: Vec::new(),
            last_grant_day: -999.0,
            last_spawn_day: -999.0,
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
                    world_grant_emergency_programs,
                    world_spawn_rescue_settlers.after(world_grant_emergency_programs),
                    build_society_projects,
                    apply_known_program_effects,
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
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    for (entity, npc, needs, inventory, mut programs) in &mut npcs {
        let seed = ((entity.to_bits() ^ step.tick) % 997) as f32 / 997.0;
        let discovery = npc.discovery_drive * needs.curiosity * delta_days * 0.18;

        let candidates = [
            (
                ProgramId::Firemaking,
                needs.safety < 0.55 && npc.exposure > 0.25 && inventory.wood >= 0.35,
            ),
            (
                ProgramId::ShelterBuilding,
                needs.safety < 0.58 && inventory.wood >= 0.8,
            ),
            (
                ProgramId::Toolmaking,
                npc.tool_knowledge > 0.65 || inventory.wood >= 0.4,
            ),
            (
                ProgramId::WarmClothing,
                npc.exposure > 0.45 && programs.knows(ProgramId::ShelterBuilding),
            ),
            (
                ProgramId::FoodStorage,
                inventory.food > 0.6 || inventory.wood > 0.8,
            ),
            (
                ProgramId::Woodworking,
                programs.knows(ProgramId::Toolmaking) && inventory.wood > 0.7,
            ),
            (
                ProgramId::Childcare,
                npc.reproduction_drive > 1.0 && needs.safety > 0.45,
            ),
            (
                ProgramId::Watchkeeping,
                needs.safety < 0.45 || npc.risk_tolerance > 0.8,
            ),
            (
                ProgramId::PredatorDefense,
                programs.knows(ProgramId::Watchkeeping) && npc.aggression_drive > 0.65,
            ),
            (
                ProgramId::Blacksmithing,
                programs.knows(ProgramId::Toolmaking)
                    && programs.knows(ProgramId::Firemaking)
                    && npc.tool_knowledge > 0.90,
            ),
            (
                ProgramId::Teaching,
                programs.known.len() >= 8 && needs.social > 0.5,
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
            if eligible && !programs.knows(program) && seed < discovery {
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

fn cultural_learning_from_deaths(
    step: Res<SimulationStep>,
    deaths: Res<NpcDeathLog>,
    mut writer: MessageWriter<LogEvent>,
    mut npcs: Query<(&Npc, &mut KnownPrograms)>,
) {
    let Some(death) = deaths.entries.last() else {
        return;
    };
    if step.elapsed_days - death.day > 0.25 {
        return;
    }

    let programs = programs_for_death_reason(&death.reason);
    let mut learned_count = 0usize;
    for (npc, mut known) in &mut npcs {
        if npc.name == death.npc_name {
            continue;
        }
        for program in programs.iter().copied() {
            if known.grant(
                program,
                &format!(
                    "learned after {} died from {}",
                    death.npc_name, death.reason
                ),
            ) {
                learned_count += 1;
                break;
            }
        }
    }

    if learned_count > 0 {
        writer.write(LogEvent::new(
            LogEventKind::Discovery,
            format!(
                "{}'s death taught {} survivors new survival knowledge",
                death.npc_name, learned_count
            ),
        ));
    }
}

fn world_grant_emergency_programs(
    step: Res<SimulationStep>,
    stats: Res<WorldStats>,
    deaths: Res<NpcDeathLog>,
    mut state: ResMut<WorldProgramState>,
    mut world_mind: ResMut<WorldMind>,
    mut writer: MessageWriter<LogEvent>,
    mut npcs: Query<(&mut KnownPrograms, &Npc)>,
) {
    let recent_cold_deaths = deaths
        .entries
        .iter()
        .rev()
        .take_while(|entry| step.elapsed_days - entry.day <= 8.0)
        .filter(|entry| entry.reason.contains("cold"))
        .count();
    let cold_emergency = recent_cold_deaths > 0 || stats.cold_stressed_npcs >= 2;
    let population_emergency = stats.npcs > 0 && stats.npcs <= 3;
    let hunger_emergency =
        stats.npcs > 0 && stats.total_food_carried + stats.total_food_stockpiled < 1.0;

    let mut grants = Vec::new();
    let mut reason = None;
    if cold_emergency {
        grants.extend([
            ProgramId::Firemaking,
            ProgramId::HearthKeeping,
            ProgramId::WarmClothing,
            ProgramId::ShelterBuilding,
            ProgramId::ShelterRepair,
            ProgramId::WeatherReading,
            ProgramId::ManaWarding,
        ]);
        reason = Some("cold deaths or exposure stress");
    }
    if population_emergency {
        grants.extend([
            ProgramId::Childcare,
            ProgramId::Midwifery,
            ProgramId::FirstAid,
            ProgramId::ConflictMediation,
            ProgramId::Teaching,
        ]);
        reason = Some("population survival threshold");
    }
    if hunger_emergency {
        grants.extend([
            ProgramId::FoodStorage,
            ProgramId::Cooking,
            ProgramId::Agriculture,
            ProgramId::SeedSaving,
            ProgramId::AnimalHusbandry,
            ProgramId::ManaAgriculture,
        ]);
        reason = Some("food security threshold");
    }

    let Some(reason) = reason else {
        return;
    };
    if step.elapsed_days - state.last_grant_day < 1.0 {
        return;
    }

    grants.sort_by_key(|id| *id as u8);
    grants.dedup();
    let mut granted_any = false;
    for program in grants {
        if !state.unlocked.contains(&program) {
            state.unlocked.push(program);
        }
        for (mut known, _) in &mut npcs {
            granted_any |= known.grant(program, reason);
        }
    }

    if granted_any {
        state.last_grant_day = step.elapsed_days;
        state.last_reason = reason.to_string();
        world_mind.intent = format!("Program society for {reason}");
        writer.write(LogEvent::new(
            LogEventKind::Discovery,
            format!("World granted emergency programs for {reason}"),
        ));
    }
}

fn world_spawn_rescue_settlers(
    mut commands: Commands,
    step: Res<SimulationStep>,
    settings: Res<MapSettings>,
    stats: Res<WorldStats>,
    mut state: ResMut<WorldProgramState>,
    mut population: ResMut<PopulationStats>,
    mut writer: MessageWriter<LogEvent>,
    tiles: Query<(&RegionTile, &Transform)>,
) {
    if stats.npcs >= 4 || step.elapsed_days - state.last_spawn_day < 12.0 {
        return;
    }

    let mut candidates = tiles
        .iter()
        .filter(|(tile, _)| tile.soil_fertility > 0.45 && tile.temperature > 0.35)
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return;
    }
    candidates.sort_by(|(a, _), (b, _)| {
        (b.soil_fertility + b.temperature + b.mana_density * 0.5)
            .total_cmp(&(a.soil_fertility + a.temperature + a.mana_density * 0.5))
    });

    let spawn_count = (4usize).saturating_sub(stats.npcs).clamp(1, 3);
    for idx in 0..spawn_count {
        let (tile, transform) = candidates[idx % candidates.len()];
        let offset = Vec2::new((idx as f32 * 2.41).cos(), (idx as f32 * 2.41).sin())
            * settings.tile_size
            * 0.18;
        let sex = if (step.tick + idx as u64).is_multiple_of(2) {
            NpcSex::Female
        } else {
            NpcSex::Male
        };
        let gender = if sex == NpcSex::Female {
            NpcGender::Woman
        } else {
            NpcGender::Man
        };
        let entity = commands
            .spawn(
                NpcBundle::new(
                    transform.translation.truncate() + offset,
                    format!("Rescue Settler {}", step.tick % 10_000 + idx as u64),
                    70.0,
                    ManaReservoir {
                        capacity: 28.0 + tile.mana_density * 20.0,
                        stored: 8.0 + tile.mana_density * 8.0,
                        stability: 0.92,
                    },
                    ManaStorageStyle {
                        concentration: 0.28 + tile.mana_density * 0.12,
                        circulation: 0.42,
                        distribution: 0.36,
                    },
                )
                .with_identity(sex, gender)
                .with_tooling(0.75, 0.45)
                .with_drives(1.2, 1.1, 0.35, 0.72)
                .with_age_days((22.0 + idx as f32 * 4.0) * 365.0),
            )
            .id();
        let mut known = KnownPrograms::default();
        for program in [
            ProgramId::Firemaking,
            ProgramId::HearthKeeping,
            ProgramId::ShelterBuilding,
            ProgramId::ShelterRepair,
            ProgramId::Toolmaking,
            ProgramId::FoodStorage,
            ProgramId::Childcare,
            ProgramId::FirstAid,
            ProgramId::Teaching,
            ProgramId::ManaSensing,
        ] {
            known.grant(program, "rescue settler starting knowledge");
        }
        commands.entity(entity).insert(known);
        population.record_birth(PopulationKind::Npc, step.elapsed_days);
    }

    state.last_spawn_day = step.elapsed_days;
    writer.write(LogEvent::new(
        LogEventKind::Birth,
        format!("World mind sent {spawn_count} rescue settlers"),
    ));
}

fn build_society_projects(
    mut commands: Commands,
    step: Res<SimulationStep>,
    stats: Res<WorldStats>,
    mut society: ResMut<SocietyProgress>,
    mut writer: MessageWriter<LogEvent>,
    mut npcs: Query<
        (
            &Transform,
            &KnownPrograms,
            &mut NpcHome,
            Option<&crate::agents::factions::FactionMember>,
        ),
        With<Npc>,
    >,
    shelters: Query<
        (
            Entity,
            &Transform,
            Option<&crate::agents::factions::FactionMember>,
        ),
        With<Shelter>,
    >,
    structures: Query<&CivicStructure>,
) {
    society.stage =
        settlement_stage(stats.npcs, stats.shelters, stats.civic_structures).to_string();

    let shelter_positions = shelters
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

    for (transform, _, mut home, member) in &mut npcs {
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

    let known_any = |program: ProgramId| npcs.iter().any(|(_, known, _, _)| known.knows(program));
    let existing =
        |kind: CivicStructureKind| structures.iter().any(|structure| structure.kind == kind);
    let center = shelter_positions
        .iter()
        .fold(Vec2::ZERO, |sum, (_, pos, _)| sum + *pos)
        / shelter_positions.len().max(1) as f32;

    let next_project = if stats.shelters >= 2
        && known_any(ProgramId::Woodworking)
        && !existing(CivicStructureKind::Fence)
    {
        Some((CivicStructureKind::Fence, center + Vec2::new(0.0, -42.0)))
    } else if stats.shelters >= 2
        && known_any(ProgramId::Toolmaking)
        && !existing(CivicStructureKind::Workshop)
    {
        Some((CivicStructureKind::Workshop, center + Vec2::new(38.0, 8.0)))
    } else if stats.npcs >= 4
        && known_any(ProgramId::Childcare)
        && !existing(CivicStructureKind::Nursery)
    {
        Some((CivicStructureKind::Nursery, center + Vec2::new(-36.0, 10.0)))
    } else if stats.npcs >= 4
        && known_any(ProgramId::Watchkeeping)
        && !existing(CivicStructureKind::WatchPost)
    {
        Some((CivicStructureKind::WatchPost, center + Vec2::new(0.0, 46.0)))
    } else if stats.shelters >= 3
        && known_any(ProgramId::FoodStorage)
        && !existing(CivicStructureKind::Granary)
    {
        Some((
            CivicStructureKind::Granary,
            center + Vec2::new(-44.0, -16.0),
        ))
    } else if stats.shelters >= 3
        && known_any(ProgramId::Blacksmithing)
        && !existing(CivicStructureKind::Forge)
    {
        Some((CivicStructureKind::Forge, center + Vec2::new(44.0, -18.0)))
    } else if stats.shelters >= 5
        && known_any(ProgramId::Governance)
        && !existing(CivicStructureKind::TownHall)
    {
        Some((CivicStructureKind::TownHall, center + Vec2::new(0.0, 0.0)))
    } else {
        None
    };

    let Some((kind, position)) = next_project else {
        return;
    };

    commands.spawn((
        Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.0), Vec2::splat(1.0)),
        Transform::from_xyz(position.x, position.y, 1.9),
        CivicStructure {
            kind,
            progress: 1.0,
        },
    ));
    society.last_project_day = step.elapsed_days;
    society.last_project = kind.label().to_string();
    writer.write(LogEvent::new(
        LogEventKind::Construction,
        format!("The settlement built a {}", kind.label()),
    ));
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
