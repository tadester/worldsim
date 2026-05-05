mod agents;
mod life;
mod magic;
mod systems;
mod ui;
mod world;

use agents::AgentsPlugin;
use bevy::prelude::*;
use life::LifePlugin;
use magic::MagicPlugin;
use systems::SimulationPlugin;
use ui::UiPlugin;
use world::WorldPlugin;

fn main() {
    build_app().run();
}

fn build_app() -> App {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb(0.04, 0.04, 0.06)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "WorldSim Prototype".into(),
                resolution: (1280u32, 720u32).into(),
                present_mode: bevy::window::PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        }));
    add_game_plugins(&mut app);
    app
}

fn add_game_plugins(app: &mut App) {
    app.add_plugins((
        WorldPlugin,
        LifePlugin,
        AgentsPlugin,
        MagicPlugin,
        SimulationPlugin,
        UiPlugin,
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::{animal::Animal, npc::Npc, predator::Predator};
    use crate::life::population::PopulationStats;
    use crate::systems::logging::NpcDeathLog;
    use crate::systems::simulation::{SimulationClock, SimulationStep};
    use crate::world::resources::Tree;
    use crate::world::settlement::Settlement;

    #[test]
    fn headless_startup_runs_multiple_updates_without_panicking() {
        let mut app = App::new();
        app.insert_resource(ButtonInput::<KeyCode>::default());
        add_game_plugins(&mut app);

        for _ in 0..5 {
            app.update();
        }

        let world = app.world_mut();
        assert!(world.query::<&Tree>().iter(world).count() > 0);
        assert!(world.query::<&Animal>().iter(world).count() > 0);
        assert!(world.query::<&Npc>().iter(world).count() > 0);
        assert!(world.query::<&Predator>().iter(world).count() > 0);
    }

    #[test]
    #[ignore = "long horizon smoke test: run manually before major sim releases"]
    fn five_year_society_does_not_stall() {
        let mut app = headless_app_for_long_run();
        run_sim_days(&mut app, 5.0 * 365.0);
        assert_society_is_alive(&mut app, 2);
    }

    #[test]
    #[ignore = "long horizon smoke test: run manually before major sim releases"]
    fn twenty_five_year_society_keeps_developing() {
        let mut app = headless_app_for_long_run();
        run_sim_days(&mut app, 25.0 * 365.0);
        assert_society_is_alive(&mut app, 3);
        let world = app.world_mut();
        let population = world.resource::<PopulationStats>();
        assert!(
            population.npc_births > 0 || world.query::<&Npc>().iter(world).count() >= 4,
            "expected births or a stable founding population after 25 years"
        );
    }

    #[test]
    #[ignore = "long horizon smoke test: run manually before major sim releases"]
    fn hundred_year_society_keeps_a_trace_of_civilization() {
        let mut app = headless_app_for_long_run();
        run_sim_days(&mut app, 100.0 * 365.0);
        assert_society_is_alive(&mut app, 1);
        let world = app.world_mut();
        let settlements = world.query::<&Settlement>().iter(world).count();
        assert!(settlements > 0, "expected settlement identity to persist");
    }

    fn headless_app_for_long_run() -> App {
        let mut app = App::new();
        app.insert_resource(ButtonInput::<KeyCode>::default());
        add_game_plugins(&mut app);
        {
            let mut clock = app.world_mut().resource_mut::<SimulationClock>();
            clock.seconds_per_day = 12.0;
            clock.step_seconds = 1.0 / 60.0;
            clock.steps_per_frame = 300;
        }
        app
    }

    fn run_sim_days(app: &mut App, days: f32) {
        loop {
            app.update();
            let elapsed = app.world().resource::<SimulationStep>().elapsed_days;
            if elapsed >= days {
                break;
            }
        }
    }

    fn assert_society_is_alive(app: &mut App, minimum_npcs: usize) {
        let world = app.world_mut();
        let trees = world.query::<&Tree>().iter(world).count();
        let animals = world.query::<&Animal>().iter(world).count();
        let npcs = world.query::<&Npc>().iter(world).count();
        let settlements = world.query::<&Settlement>().iter(world).count();
        let population = world.resource::<PopulationStats>();
        let death_reasons = world
            .resource::<NpcDeathLog>()
            .entries
            .iter()
            .rev()
            .take(6)
            .map(|entry| format!("{}: {}", entry.npc_name, entry.reason))
            .collect::<Vec<_>>()
            .join(" | ");
        assert!(trees > 0, "expected trees to persist");
        assert!(animals > 0, "expected animals to persist");
        assert!(
            npcs >= minimum_npcs,
            "expected society to retain at least {minimum_npcs} NPCs, got {npcs}; births {}, deaths {}; recent deaths: {}",
            population.npc_births,
            population.npc_deaths,
            death_reasons
        );
        assert!(settlements > 0, "expected at least one settlement center");
    }
}
