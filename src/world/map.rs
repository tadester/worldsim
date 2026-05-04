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
    pub biome: BiomeKind,
    pub elevation: f32,
    pub moisture: f32,
    pub mana_density: f32,
    pub soil_fertility: f32,
    pub animal_capacity: f32,
    pub tree_capacity: f32,
    pub base_temperature: f32,
    pub temperature: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiomeKind {
    Water,
    Highland,
    Dryland,
    Wetland,
    Forest,
    Meadow,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct RegionState {
    pub forage: f32,
    pub forage_capacity: f32,
    pub tree_biomass: f32,
    pub tree_biomass_capacity: f32,
    pub path_wear: f32,
    pub settlement_clearance: f32,
}

#[derive(Component)]
struct PathWearOverlay;

#[derive(Component)]
struct SettlementClearOverlay;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (spawn_camera, generate_map).chain())
            .add_systems(Update, sync_region_detail_visuals);
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
            let broad_noise = layered_noise(x, y, 0.19, 0.13, 0.08);
            let ridge_noise = layered_noise(x + 17, y - 11, 0.41, 0.31, 0.12);
            let moisture_noise = layered_noise(x - 9, y + 23, 0.27, 0.17, 0.09);
            let mana_noise = layered_noise(x + 41, y + 5, 0.33, 0.29, 0.10);

            let coast_falloff =
                (1.0 - ((xf - 0.5).abs() * 1.35 + (yf - 0.5).abs() * 1.1)).clamp(0.0, 1.0);
            let elevation = (0.18 + coast_falloff * 0.38 + broad_noise * 0.32 + ridge_noise * 0.18)
                .clamp(0.0, 1.0);
            let moisture = (0.24 + (1.0 - yf) * 0.18 + moisture_noise * 0.44 - ridge_noise * 0.10)
                .clamp(0.0, 1.0);
            let mana_density =
                (0.10 + xf * 0.22 + mana_noise * 0.58 + elevation * 0.08).clamp(0.05, 1.0);
            let soil_fertility =
                (0.12 + moisture * 0.54 + coast_falloff * 0.12 - elevation * 0.08).clamp(0.05, 1.0);
            let animal_capacity =
                (1.4 + soil_fertility * 5.0 + moisture * 1.6 - elevation * 0.7).clamp(0.8, 8.5);
            let tree_capacity =
                (0.6 + soil_fertility * 7.2 + moisture * 2.4 - elevation * 0.5).clamp(0.4, 12.0);
            let temperature = (0.48 + xf * 0.12 - yf * 0.20 - elevation * 0.18
                + mana_density * 0.06)
                .clamp(0.0, 1.0);
            let biome = biome_for(elevation, moisture, soil_fertility, tree_capacity);

            let pos_x = x as f32 * settings.tile_size - half_width + settings.tile_size * 0.5;
            let pos_y = y as f32 * settings.tile_size - half_height + settings.tile_size * 0.5;
            let tint = tile_base_color(elevation, moisture, mana_density, temperature);
            let accent = tile_accent_color(tint, mana_density, elevation);
            let shadow = tile_shadow_color(tint, elevation);
            let detail_offset = detail_offset(x, y);

            let tile_size = settings.tile_size + 3.5;
            commands
                .spawn((
                    Sprite::from_color(tint, Vec2::splat(tile_size)),
                    Transform::from_xyz(pos_x, pos_y, 0.0),
                    RegionTile {
                        coord: IVec2::new(x, y),
                        biome,
                        elevation,
                        moisture,
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
                        path_wear: 0.0,
                        settlement_clearance: 0.0,
                    },
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Sprite::from_color(
                            tile_highlight_color(tint, moisture, temperature),
                            Vec2::new(settings.tile_size * 0.88, settings.tile_size * 0.52),
                        ),
                        Transform::from_xyz(-detail_offset.x * 0.45, detail_offset.y * 0.35, 0.006)
                            .with_rotation(Quat::from_rotation_z(
                                ((x * 31 + y * 17) as f32).sin() * 0.18,
                            )),
                    ));
                    parent.spawn((
                        Sprite::from_color(
                            shadow,
                            Vec2::new(settings.tile_size * 1.05, settings.tile_size * 0.20),
                        ),
                        Transform::from_xyz(0.0, -settings.tile_size * 0.22, 0.01),
                    ));
                    parent.spawn((
                        Sprite::from_color(
                            accent,
                            Vec2::new(settings.tile_size * 0.42, settings.tile_size * 0.30),
                        ),
                        Transform::from_xyz(detail_offset.x, detail_offset.y, 0.02).with_rotation(
                            Quat::from_rotation_z(((x * 13 - y * 19) as f32).cos() * 0.22),
                        ),
                    ));
                    parent.spawn((
                        Sprite::from_color(
                            Color::srgba(0.56, 0.46, 0.32, 0.0),
                            Vec2::new(settings.tile_size * 1.05, settings.tile_size * 0.12),
                        ),
                        Transform::from_xyz(0.0, 0.0, 0.04).with_rotation(Quat::from_rotation_z(
                            ((x * 7 + y * 23) as f32).sin() * 0.35,
                        )),
                        PathWearOverlay,
                    ));
                    parent.spawn((
                        Sprite::from_color(
                            Color::srgba(0.60, 0.52, 0.38, 0.0),
                            Vec2::new(settings.tile_size * 0.90, settings.tile_size * 0.70),
                        ),
                        Transform::from_xyz(0.0, 0.0, 0.035),
                        SettlementClearOverlay,
                    ));
                });
        }
    }
}

fn sync_region_detail_visuals(
    regions: Query<(&RegionState, &Children), Changed<RegionState>>,
    mut paths: Query<&mut Sprite, With<PathWearOverlay>>,
    mut clearings: Query<&mut Sprite, (With<SettlementClearOverlay>, Without<PathWearOverlay>)>,
) {
    for (state, children) in &regions {
        for child in children.iter() {
            if let Ok(mut sprite) = paths.get_mut(child) {
                let wear = state.path_wear.clamp(0.0, 1.0);
                sprite.color = Color::srgba(0.58, 0.49, 0.35, wear * 0.42);
                sprite.custom_size = Some(Vec2::new(32.0 + wear * 18.0, 3.0 + wear * 5.0));
            }
            if let Ok(mut sprite) = clearings.get_mut(child) {
                let clearance = state.settlement_clearance.clamp(0.0, 1.0);
                sprite.color = Color::srgba(
                    0.58 + clearance * 0.08,
                    0.52 + clearance * 0.05,
                    0.38,
                    clearance * 0.30,
                );
            }
        }
    }
}

fn biome_for(elevation: f32, moisture: f32, fertility: f32, tree_capacity: f32) -> BiomeKind {
    if elevation < 0.24 {
        BiomeKind::Water
    } else if elevation > 0.74 {
        BiomeKind::Highland
    } else if moisture < 0.24 {
        BiomeKind::Dryland
    } else if moisture > 0.66 {
        BiomeKind::Wetland
    } else if tree_capacity > 6.0 && fertility > 0.36 {
        BiomeKind::Forest
    } else {
        BiomeKind::Meadow
    }
}

fn layered_noise(x: i32, y: i32, freq_x: f32, freq_y: f32, sway: f32) -> f32 {
    let xf = x as f32;
    let yf = y as f32;
    let major = ((xf * freq_x).sin() + (yf * freq_y).cos()) * 0.5;
    let minor = (((xf + yf) * (freq_x + sway)).sin() + ((xf - yf) * (freq_y + sway)).cos()) * 0.25;
    (0.5 + major * 0.32 + minor * 0.28).clamp(0.0, 1.0)
}

fn detail_offset(x: i32, y: i32) -> Vec2 {
    let a = ((x * 17 + y * 9) as f32 * 0.37).sin();
    let b = ((x * 11 - y * 13) as f32 * 0.29).cos();
    Vec2::new(a * 7.0, b * 5.0)
}

fn tile_base_color(elevation: f32, moisture: f32, mana_density: f32, temperature: f32) -> Color {
    if elevation < 0.24 {
        Color::srgb(
            0.05 + mana_density * 0.05,
            0.16 + mana_density * 0.14,
            0.24 + moisture * 0.22,
        )
    } else if elevation > 0.74 {
        Color::srgb(
            0.30 + elevation * 0.18,
            0.30 + elevation * 0.14,
            0.34 + mana_density * 0.12,
        )
    } else if moisture < 0.24 {
        Color::srgb(
            0.34 + temperature * 0.20,
            0.28 + temperature * 0.12,
            0.15 + mana_density * 0.08,
        )
    } else if moisture > 0.66 {
        Color::srgb(
            0.08 + moisture * 0.08,
            0.26 + moisture * 0.34,
            0.14 + mana_density * 0.12,
        )
    } else {
        Color::srgb(
            0.12 + elevation * 0.12 + temperature * 0.04,
            0.25 + moisture * 0.28,
            0.10 + mana_density * 0.18,
        )
    }
}

fn tile_accent_color(base: Color, mana_density: f32, elevation: f32) -> Color {
    let rgba = base.to_srgba();
    Color::srgba(
        (rgba.red + mana_density * 0.18).clamp(0.0, 1.0),
        (rgba.green + mana_density * 0.10 + elevation * 0.05).clamp(0.0, 1.0),
        (rgba.blue + mana_density * 0.22).clamp(0.0, 1.0),
        0.38,
    )
}

fn tile_highlight_color(base: Color, moisture: f32, temperature: f32) -> Color {
    let rgba = base.to_srgba();
    Color::srgba(
        (rgba.red + 0.04 + temperature * 0.03).clamp(0.0, 1.0),
        (rgba.green + 0.05 + moisture * 0.06).clamp(0.0, 1.0),
        (rgba.blue + moisture * 0.03).clamp(0.0, 1.0),
        0.24,
    )
}

fn tile_shadow_color(base: Color, elevation: f32) -> Color {
    let rgba = base.to_srgba();
    Color::srgba(
        (rgba.red * 0.52).clamp(0.0, 1.0),
        (rgba.green * 0.46).clamp(0.0, 1.0),
        (rgba.blue * (0.42 + elevation * 0.08)).clamp(0.0, 1.0),
        0.28,
    )
}
