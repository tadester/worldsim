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
    use crate::world::resources::Tree;

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
}
