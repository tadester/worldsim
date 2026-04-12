use bevy::prelude::*;

use crate::world::climate::RegionClimate;

#[derive(Resource, Debug, Clone)]
pub struct MapSettings {
    pub width: i32,
    pub height: i32,
    pub tile_size: f32,
}

impl Default for MapSettings {
    fn default() -> Self {
        Self {
            width: 24,
            height: 14,
            tile_size: 40.0,
        }
    }
}

impl MapSettings {
    pub fn world_bounds(&self) -> Vec2 {
        Vec2::new(
            self.width as f32 * self.tile_size * 0.5,
            self.height as f32 * self.tile_size * 0.5,
        )
    }

    pub fn tile_coord_for_position(&self, position: Vec2) -> IVec2 {
        let bounds = self.world_bounds();
        let local = position + bounds;
        let x = (local.x / self.tile_size).floor() as i32;
        let y = (local.y / self.tile_size).floor() as i32;

        IVec2::new(x.clamp(0, self.width - 1), y.clamp(0, self.height - 1))
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct RegionTile {
    pub coord: IVec2,
    pub mana_density: f32,
    pub soil_fertility: f32,
    pub animal_capacity: f32,
    pub tree_capacity: f32,
    pub base_temperature: f32,
    pub temperature: f32,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct RegionState {
    pub forage: f32,
    pub forage_capacity: f32,
    pub tree_biomass: f32,
    pub tree_biomass_capacity: f32,
}

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (spawn_camera, generate_map));
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn generate_map(mut commands: Commands, settings: Res<MapSettings>) {
    let half_width = settings.width as f32 * settings.tile_size * 0.5;
    let half_height = settings.height as f32 * settings.tile_size * 0.5;

    for y in 0..settings.height {
        for x in 0..settings.width {
            let xf = x as f32 / settings.width as f32;
            let yf = y as f32 / settings.height as f32;

            let mana_density = 0.2 + xf * 0.6;
            let soil_fertility = 0.3 + (1.0 - yf) * 0.5;
            let animal_capacity = 2.0 + soil_fertility * 6.0;
            let tree_capacity = 1.0 + soil_fertility * 10.0;
            let temperature = 0.35 + xf * 0.25 - yf * 0.15;

            let pos_x = x as f32 * settings.tile_size - half_width + settings.tile_size * 0.5;
            let pos_y = y as f32 * settings.tile_size - half_height + settings.tile_size * 0.5;
            let tint = Color::srgb(
                0.08 + soil_fertility * 0.18,
                0.20 + soil_fertility * 0.35,
                0.10 + mana_density * 0.25,
            );

            commands.spawn((
                Sprite::from_color(tint, Vec2::splat(settings.tile_size - 1.0)),
                Transform::from_xyz(pos_x, pos_y, 0.0),
                RegionTile {
                    coord: IVec2::new(x, y),
                    mana_density,
                    soil_fertility,
                    animal_capacity,
                    tree_capacity,
                    base_temperature: temperature,
                    temperature,
                },
                RegionClimate::default(),
                RegionState {
                    forage: animal_capacity * 0.55,
                    forage_capacity: animal_capacity,
                    tree_biomass: tree_capacity * 0.45,
                    tree_biomass_capacity: tree_capacity,
                },
            ));
        }
    }
}
