use bevy::prelude::*;

use crate::agents::animal::{Animal, AnimalBundle, AnimalLifeStage, Pregnancy};
use crate::agents::evolution::EvolutionPressure;
use crate::agents::factions::FactionMember;
use crate::agents::inventory::Inventory;
use crate::agents::kinship::Kinship;
use crate::agents::needs::Needs;
use crate::agents::npc::{Npc, NpcBundle, NpcGender, NpcHome, NpcSex};
use crate::agents::personality::{NpcPsyche, PersonalityType};
use crate::agents::programs::{KnownPrograms, ProgramId};
use crate::life::growth::Lifecycle;
use crate::life::population::{PopulationKind, PopulationStats};
use crate::magic::mana::ManaReservoir;
use crate::magic::storage::ManaStorageStyle;
use crate::systems::logging::{LogEvent, LogEventKind};
use crate::systems::simulation::{SimulationClock, SimulationStep};
use crate::world::director::WorldMind;
use crate::world::map::{MapSettings, RegionState, RegionTile};
use crate::world::resources::{ShelterStockpile, Tree, TreeStage};

pub struct ReproductionPlugin;

#[derive(Component, Debug, Clone, Copy)]
pub struct NpcPregnancy {
    pub gestation_days: f32,
    pub father: Option<Entity>,
    pub home_culture: Option<Entity>,
    pub birth_home: Option<Entity>,
}

impl Plugin for ReproductionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                tree_seed_spread,
                animal_reproduction,
                resolve_animal_births.after(animal_reproduction),
                npc_reproduction.after(resolve_animal_births),
                resolve_npc_births.after(npc_reproduction),
            ),
        );
    }
}

fn tree_seed_spread(
    mut commands: Commands,
    clock: Res<SimulationClock>,
    settings: Res<MapSettings>,
    mut trees: Query<(&Transform, &mut Tree)>,
    mut regions: Query<(&RegionTile, &mut RegionState)>,
) {
    let delta_days = clock.delta_days();

    for (transform, mut tree) in &mut trees {
        if tree.stage != TreeStage::Mature {
            continue;
        }

        let mut biomass_ratio = 0.0;
        for (tile, state) in &mut regions {
            if tile.coord != tree.root_coord {
                continue;
            }

            biomass_ratio = if state.tree_biomass_capacity <= 0.0 {
                0.0
            } else {
                state.tree_biomass / state.tree_biomass_capacity
            };
            break;
        }

        tree.spread_progress += delta_days * 0.00045 * biomass_ratio.max(0.20);

        if tree.spread_progress < 1.0 || biomass_ratio <= 0.25 {
            continue;
        }

        tree.spread_progress = 0.0;
        let spawn_offset =
            Vec2::new(transform.translation.x.sin(), transform.translation.y.cos()) * 18.0;
        let spawn_position = transform.translation.truncate() + spawn_offset;
        let spawn_coord = settings.tile_coord_for_position(spawn_position);

        for (tile, mut state) in &mut regions {
            if tile.coord != spawn_coord {
                continue;
            }

            if state.tree_biomass < 0.85 || tile.tree_capacity < 2.2 {
                break;
            }

            state.tree_biomass = (state.tree_biomass - 0.55).max(0.0);
            commands.spawn((
                Sprite::from_color(Color::srgba(0.0, 0.0, 0.0, 0.0), Vec2::splat(1.0)),
                Transform::from_xyz(spawn_position.x, spawn_position.y, 2.0),
                Tree {
                    root_coord: spawn_coord,
                    stage: TreeStage::Sapling,
                    growth: 0.1,
                    chop_progress: 0.0,
                    spread_progress: 0.0,
                },
                ManaReservoir {
                    capacity: 10.0 + tile.mana_density * 10.0,
                    stored: tile.mana_density * 2.0,
                    stability: 0.85,
                },
            ));
            break;
        }
    }
}

fn animal_reproduction(
    mut commands: Commands,
    clock: Res<SimulationClock>,
    settings: Res<MapSettings>,
    mut writer: MessageWriter<LogEvent>,
    regions: Query<(&RegionTile, &RegionState)>,
    mut animals: ParamSet<(
        Query<(&Transform, &Animal)>,
        Query<(
            Entity,
            &Transform,
            &mut Animal,
            &mut Lifecycle,
            Option<&Pregnancy>,
        )>,
    )>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    let mut animal_counts = std::collections::HashMap::<IVec2, usize>::new();
    for (transform, animal) in &animals.p0() {
        if animal.life_stage == AnimalLifeStage::Juvenile {
            continue;
        }
        let coord = settings.tile_coord_for_position(transform.translation.truncate());
        *animal_counts.entry(coord).or_insert(0) += 1;
    }

    let region_state_by_coord: std::collections::HashMap<IVec2, (f32, f32, f32)> = regions
        .iter()
        .map(|(tile, state)| {
            (
                tile.coord,
                (
                    tile.animal_capacity.max(0.1),
                    state.forage,
                    state.forage_capacity.max(0.1),
                ),
            )
        })
        .collect();

    for (entity, transform, mut animal, mut lifecycle, pregnancy) in &mut animals.p1() {
        let mature = lifecycle.age_days >= lifecycle.maturity_age;
        let fertile = lifecycle.reproduction_cooldown <= 0.0;
        let healthy = animal.health >= 30.0 && animal.energy >= 28.0;
        let adult = animal.life_stage == AnimalLifeStage::Adult;

        if !(mature && fertile && healthy && adult && pregnancy.is_none()) {
            continue;
        }

        let coord = settings.tile_coord_for_position(transform.translation.truncate());
        let (capacity, forage, forage_capacity) = region_state_by_coord
            .get(&coord)
            .copied()
            .unwrap_or((1.0, 0.0, 1.0));
        let local_animals = animal_counts.get(&coord).copied().unwrap_or(0) as f32;
        let crowding_ratio = local_animals / capacity.max(1.0);
        let forage_ratio = (forage / forage_capacity).clamp(0.0, 1.0);
        let ecological_headroom =
            (1.0 - crowding_ratio * 0.85).clamp(0.05, 1.0) * (0.35 + forage_ratio * 0.65);

        if ecological_headroom < 0.18 {
            continue;
        }

        animal.reproduction_drive +=
            delta_days * 0.0045 * lifecycle.fertility.max(0.16) * ecological_headroom;

        if animal.reproduction_drive < 1.0 {
            continue;
        }

        animal.reproduction_drive = 0.0;
        animal.energy = (animal.energy - 12.0).max(0.0);
        lifecycle.reproduction_cooldown = 260.0;
        commands.entity(entity).insert(Pregnancy {
            gestation_days: 160.0,
            offspring_health: 22.0,
            offspring_speed: animal.speed * 0.95,
        });

        writer.write(LogEvent::new(
            LogEventKind::Discovery,
            format!(
                "Animal pregnancy began near {:.0},{:.0}",
                transform.translation.x, transform.translation.y
            ),
        ));
    }
}

fn npc_reproduction(
    mut commands: Commands,
    clock: Res<SimulationClock>,
    step: Res<SimulationStep>,
    mut writer: MessageWriter<LogEvent>,
    shelters: Query<(&crate::world::resources::Shelter, Option<&ShelterStockpile>)>,
    mut npcs: ParamSet<(
        Query<(
            Entity,
            &Npc,
            &Transform,
            &Needs,
            &NpcHome,
            &Lifecycle,
            Option<&NpcPsyche>,
            Option<&NpcPregnancy>,
        )>,
        Query<(
            Entity,
            &Npc,
            &Transform,
            &Needs,
            &Inventory,
            &NpcHome,
            &mut Lifecycle,
            Option<&FactionMember>,
            Option<&NpcPsyche>,
            Option<&NpcPregnancy>,
        )>,
    )>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    let partner_candidates: Vec<(Entity, Vec2, Option<Entity>)> = npcs
        .p0()
        .iter()
        .filter_map(
            |(entity, npc, transform, needs, home, lifecycle, psyche, pregnancy)| {
                let happiness = psyche.map(|psyche| psyche.happiness).unwrap_or(0.5);
                if npc.sex != NpcSex::Male
                    || pregnancy.is_some()
                    || lifecycle.age_days < lifecycle.maturity_age
                    || lifecycle.reproduction_cooldown > 0.0
                    || npc.health < 28.0
                    || needs.hunger > 0.84
                    || needs.fatigue > 0.92
                    || needs.safety < 0.12
                    || happiness < 0.18
                {
                    None
                } else {
                    Some((entity, transform.translation.truncate(), home.shelter))
                }
            },
        )
        .collect();

    for (
        entity,
        npc,
        transform,
        needs,
        inventory,
        home,
        mut lifecycle,
        member,
        psyche,
        pregnancy,
    ) in &mut npcs.p1()
    {
        let happiness = psyche.map(|psyche| psyche.happiness).unwrap_or(0.5);
        if npc.sex != NpcSex::Female {
            continue;
        }
        if pregnancy.is_some()
            || lifecycle.age_days < lifecycle.maturity_age
            || lifecycle.reproduction_cooldown > 0.0
            || npc.health < 28.0
            || needs.hunger > (0.82 - npc.reproduction_drive * 0.10).clamp(0.48, 0.88)
            || needs.fatigue > (0.94 - npc.reproduction_drive * 0.08).clamp(0.62, 0.94)
            || needs.safety < (0.13 - npc.risk_tolerance * 0.04).clamp(0.03, 0.18)
            || happiness < 0.18
        {
            continue;
        }

        let home_security = home
            .shelter
            .and_then(|home_entity| shelters.get(home_entity).ok())
            .map(|(shelter, stockpile)| {
                let stockpile = stockpile.copied().unwrap_or_default();
                stockpile.food + inventory.food + stockpile.wood * 0.20 + shelter.integrity * 1.2
            })
            .unwrap_or(inventory.food);
        let social_resilience = npc.reproduction_drive * 0.22 + npc.risk_tolerance * 0.10;
        if home_security + social_resilience
            < (0.48 - npc.reproduction_drive * 0.10).clamp(0.22, 0.50)
            && needs.safety < 0.32
        {
            continue;
        }

        let mother_pos = transform.translation.truncate();
        let partner_match =
            partner_candidates
                .iter()
                .find(|(partner_entity, pos, partner_home)| {
                    if *partner_entity == entity {
                        return false;
                    }
                    let distance = mother_pos.distance(*pos);
                    (home.shelter.is_some() && *partner_home == home.shelter && distance < 108.0)
                        || distance < 74.0
                });
        let Some((father, _, _)) = partner_match else {
            continue;
        };

        let entity_seed = entity.to_bits() as f32;
        let cycle_days = (22.0 - npc.reproduction_drive * 6.0).clamp(10.0, 24.0);
        let phase = (step.elapsed_days + entity_seed * 0.37) % cycle_days;
        let conception_window = delta_days
            * (12.0 + lifecycle.fertility * 5.5 + npc.reproduction_drive * 4.0 + happiness * 4.0);
        if phase > conception_window {
            continue;
        }

        lifecycle.reproduction_cooldown =
            (120.0 - npc.reproduction_drive * 28.0).clamp(72.0, 120.0);
        commands.entity(entity).insert(NpcPregnancy {
            gestation_days: 280.0,
            father: Some(*father),
            home_culture: member.map(|member| member.faction),
            birth_home: home.shelter,
        });
        writer.write(LogEvent::new(
            LogEventKind::Birth,
            format!(
                "{} is expecting a child near {:.0},{:.0}",
                npc.name, transform.translation.x, transform.translation.y
            ),
        ));
    }
}

fn resolve_npc_births(
    mut commands: Commands,
    clock: Res<SimulationClock>,
    step: Res<SimulationStep>,
    world_mind: Option<Res<WorldMind>>,
    evolution: Option<Res<EvolutionPressure>>,
    mut population: ResMut<PopulationStats>,
    mut writer: MessageWriter<LogEvent>,
    mut npcs: Query<(
        Entity,
        &Transform,
        &Npc,
        Option<&mut NpcPsyche>,
        &ManaReservoir,
        &ManaStorageStyle,
        Option<&KnownPrograms>,
        &mut NpcPregnancy,
    )>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    for (
        entity,
        transform,
        npc,
        mut psyche,
        reservoir,
        mana_style,
        known_programs,
        mut pregnancy,
    ) in &mut npcs
    {
        pregnancy.gestation_days -= delta_days;
        if pregnancy.gestation_days > 0.0 {
            continue;
        }

        let offset = Vec2::new(transform.translation.y.sin(), transform.translation.x.cos()) * 16.0;
        let child_name = format!("{} Kin {}", npc.name, step.tick % 10_000);
        let birth_seed = step.tick + entity.to_bits();
        let child_sex = if birth_seed.is_multiple_of(2) {
            NpcSex::Female
        } else {
            NpcSex::Male
        };
        let child_gender = if birth_seed.is_multiple_of(7) {
            NpcGender::Nonbinary
        } else if child_sex == NpcSex::Female {
            NpcGender::Woman
        } else {
            NpcGender::Man
        };
        let child_seed = (birth_seed % 17) as f32 / 16.0;
        let pressure = world_mind.as_ref().map(|mind| mind.pressure).unwrap_or(0.0);
        let nurture = world_mind.as_ref().map(|mind| mind.nurture).unwrap_or(0.5);
        let entropy = world_mind.as_ref().map(|mind| mind.entropy).unwrap_or(0.0);
        let food_security_selection = (nurture - pressure * 0.35).clamp(-0.25, 0.35);
        let shelter_selection = (pressure * 0.22 + nurture * 0.18).clamp(0.0, 0.42);
        let curiosity_selection = (entropy * 0.26 + nurture * 0.10).clamp(0.0, 0.36);
        let peace_selection = (nurture * 0.28 - pressure * 0.12).clamp(-0.10, 0.30);
        let evo = evolution.as_ref();
        let survival_selection = evo
            .map(|pressure| pressure.survival_fitness - 0.5)
            .unwrap_or(0.0);
        let reproduction_selection = evo
            .map(|pressure| pressure.reproduction_fitness - 0.5)
            .unwrap_or(0.0);
        let teaching_selection = evo
            .map(|pressure| pressure.teaching_fitness - 0.5)
            .unwrap_or(0.0);
        let shelter_fitness_selection = evo
            .map(|pressure| pressure.shelter_fitness - 0.5)
            .unwrap_or(0.0);
        let community_selection = evo
            .map(|pressure| pressure.community_fitness - 0.5)
            .unwrap_or(0.0);
        let mutation = evo
            .map(|pressure| pressure.mutation_rate * (child_seed - 0.5))
            .unwrap_or(0.0);
        let inherited_personality = psyche
            .as_ref()
            .map(|psyche| psyche.personality)
            .unwrap_or(PersonalityType::Builder);
        let mut child_programs = KnownPrograms::default();
        if let Some(parent_programs) = known_programs {
            child_programs.known.clear();
            for program in parent_programs.known.iter().copied() {
                if inherited_program(program, child_seed) {
                    child_programs.learn(program);
                }
            }
            for starter in [
                ProgramId::Foraging,
                ProgramId::WaterFinding,
                ProgramId::Storykeeping,
            ] {
                child_programs.learn(starter);
            }
            child_programs.last_grant_reason = format!("Inherited from {}", npc.name);
        }

        let child_bundle = NpcBundle::new(
            transform.translation.truncate() + offset,
            child_name,
            (npc.health * 0.72).clamp(34.0, 60.0),
            ManaReservoir {
                capacity: reservoir.capacity,
                stored: (reservoir.stored * 0.35).min(reservoir.capacity),
                stability: reservoir.stability,
            },
            *mana_style,
        )
        .with_identity(child_sex, child_gender)
        .with_tooling(0.1, 0.0)
        .with_drives(
            (npc.reproduction_drive * 0.82
                + 0.25
                + child_seed * 0.22
                + food_security_selection
                + reproduction_selection * 0.30
                + community_selection * 0.16
                + mutation)
                .clamp(0.1, 1.6),
            (npc.discovery_drive * 0.78
                + 0.20
                + child_seed * 0.18
                + curiosity_selection
                + shelter_selection * 0.20
                + teaching_selection * 0.34
                + survival_selection * 0.14
                + mutation * 0.8)
                .clamp(0.1, 1.6),
            (npc.aggression_drive * 0.72 + child_seed * 0.30
                - peace_selection
                - community_selection * 0.20
                + mutation * 0.5)
                .clamp(0.0, 1.6),
            (npc.risk_tolerance * 0.80
                + 0.15
                + child_seed * 0.16
                + shelter_selection * 0.35
                + shelter_fitness_selection * 0.28
                + survival_selection * 0.18
                + mutation * 0.7)
                .clamp(0.0, 1.4),
        )
        .with_personality(
            inherited_personality,
            0.25 + child_seed * 0.55,
            0.18 + child_seed * 0.42,
            0.22 + child_seed * 0.50,
            0.12 + child_seed * 0.40,
            0.15 + child_seed * 0.48,
            0.14 + child_seed * 0.52,
            0.10 + child_seed * 0.38,
        )
        .with_age_days(0.0);
        let sibling_count = population.npc_births as u32;
        commands.spawn((
            child_bundle,
            child_programs,
            Kinship {
                mother: Some(entity),
                father: pregnancy.father,
                generation: evo
                    .map(|pressure| pressure.generation_estimate.ceil() as u32)
                    .unwrap_or(0),
                home_culture: pregnancy.home_culture,
                birth_home: pregnancy.birth_home,
                siblings_at_birth: sibling_count,
            },
        ));
        if let Some(psyche) = psyche.as_mut() {
            psyche.reward_reproduction();
        }
        commands.entity(entity).remove::<NpcPregnancy>();
        population.record_birth(PopulationKind::Npc, step.elapsed_days);
        writer.write(LogEvent::new(
            LogEventKind::Birth,
            format!("A child was born to {}", npc.name),
        ));
    }
}

fn inherited_program(program: ProgramId, seed: f32) -> bool {
    matches!(
        program,
        ProgramId::Foraging
            | ProgramId::WaterFinding
            | ProgramId::Firemaking
            | ProgramId::ShelterBuilding
            | ProgramId::Toolmaking
            | ProgramId::Storykeeping
            | ProgramId::Childcare
            | ProgramId::Teaching
    ) || seed > 0.28
}

fn resolve_animal_births(
    mut commands: Commands,
    clock: Res<SimulationClock>,
    step: Res<SimulationStep>,
    mut writer: MessageWriter<LogEvent>,
    mut population: ResMut<PopulationStats>,
    mut animals: Query<(Entity, &Transform, &mut Pregnancy)>,
) {
    let delta_days = clock.delta_days();

    for (entity, transform, mut pregnancy) in &mut animals {
        pregnancy.gestation_days -= delta_days;

        if pregnancy.gestation_days > 0.0 {
            continue;
        }

        let offset = Vec2::new(transform.translation.y.cos(), transform.translation.x.sin()) * 14.0;
        commands.spawn(AnimalBundle::new(
            transform.translation.truncate() + offset,
            pregnancy.offspring_health,
            pregnancy.offspring_speed,
        ));
        commands.entity(entity).remove::<Pregnancy>();
        population.record_birth(PopulationKind::Animal, step.elapsed_days);

        writer.write(LogEvent::new(
            LogEventKind::Birth,
            format!(
                "Animal offspring born near {:.0},{:.0}",
                transform.translation.x, transform.translation.y
            ),
        ));
    }
}
