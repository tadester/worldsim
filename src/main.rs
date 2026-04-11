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
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.04, 0.04, 0.06)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "WorldSim Prototype".into(),
                resolution: (1280u32, 720u32).into(),
                present_mode: bevy::window::PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        }))
        .add_plugins((
            WorldPlugin,
            LifePlugin,
            AgentsPlugin,
            MagicPlugin,
            SimulationPlugin,
            UiPlugin,
        ))
        .run();
}
