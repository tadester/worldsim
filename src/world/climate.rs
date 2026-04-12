use bevy::prelude::*;

use crate::systems::simulation::SimulationStep;
use crate::world::map::{MapSettings, RegionTile};

#[derive(Component, Debug, Clone, Copy)]
pub struct RegionClimate {
    pub pressure: f32,
}

impl Default for RegionClimate {
    fn default() -> Self {
        Self { pressure: 0.0 }
    }
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct ClimateModel {
    pub year_length_days: f32,
    pub seasonal_amplitude: f32,
    pub drift_period_days: f32,
    pub drift_amplitude: f32,
    pub comfort_temp: f32,
    pub comfort_band: f32,
    pub current_offset: f32,
    pub current_season_phase: f32,
}

impl Default for ClimateModel {
    fn default() -> Self {
        Self {
            year_length_days: 60.0,
            seasonal_amplitude: 0.18,
            drift_period_days: 420.0,
            drift_amplitude: 0.06,
            comfort_temp: 0.55,
            comfort_band: 0.14,
            current_offset: 0.0,
            current_season_phase: 0.0,
        }
    }
}

impl ClimateModel {
    pub fn year_day(&self, elapsed_days: f32) -> f32 {
        if self.year_length_days <= 0.0 {
            return 0.0;
        }
        elapsed_days.rem_euclid(self.year_length_days)
    }

    pub fn season_label(&self) -> &'static str {
        let phase = self.current_season_phase.rem_euclid(1.0);
        match (phase * 4.0).floor() as i32 {
            0 => "Spring",
            1 => "Summer",
            2 => "Autumn",
            _ => "Winter",
        }
    }
}

pub struct ClimatePlugin;

impl Plugin for ClimatePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClimateModel>()
            .add_systems(PreUpdate, update_region_climate);
    }
}

fn update_region_climate(
    step: Res<SimulationStep>,
    settings: Res<MapSettings>,
    mut climate: ResMut<ClimateModel>,
    mut regions: Query<(&mut RegionTile, &mut Sprite, &mut RegionClimate)>,
) {
    if climate.year_length_days <= 0.0 {
        return;
    }

    let elapsed_days = step.elapsed_days;
    let season_phase = (elapsed_days / climate.year_length_days).rem_euclid(1.0);
    climate.current_season_phase = season_phase;

    let seasonal = (season_phase * std::f32::consts::TAU).sin();
    let drift = if climate.drift_period_days > 0.0 {
        ((elapsed_days / climate.drift_period_days).rem_euclid(1.0) * std::f32::consts::TAU).sin()
    } else {
        0.0
    };

    let global_offset = seasonal * climate.seasonal_amplitude + drift * climate.drift_amplitude;
    climate.current_offset = global_offset;

    let height = (settings.height.max(2) - 1) as f32;
    for (mut tile, mut sprite, mut region_climate) in &mut regions {
        let y_norm = tile.coord.y as f32 / height;
        let latitude_strength = 0.8 + y_norm * 0.6;
        let temp = (tile.base_temperature + global_offset * latitude_strength).clamp(0.0, 1.0);
        tile.temperature = temp;

        region_climate.pressure =
            temperature_pressure(temp, climate.comfort_temp, climate.comfort_band);

        sprite.color = tile_color(&tile, temp, region_climate.pressure, climate.comfort_temp);
    }
}

fn temperature_pressure(temp: f32, comfort: f32, band: f32) -> f32 {
    let delta = (temp - comfort).abs();
    if delta <= band {
        0.0
    } else {
        let max_delta = (comfort.max(1.0 - comfort) - band).max(0.05);
        ((delta - band) / max_delta).clamp(0.0, 1.0)
    }
}

fn tile_color(tile: &RegionTile, temp: f32, pressure: f32, comfort: f32) -> Color {
    let mut r = 0.08 + tile.soil_fertility * 0.18;
    let mut g = 0.20 + tile.soil_fertility * 0.35;
    let mut b = 0.10 + tile.mana_density * 0.25;

    let warm = ((temp - comfort).max(0.0) / (1.0 - comfort).max(0.05)).clamp(0.0, 1.0);
    let cold = ((comfort - temp).max(0.0) / comfort.max(0.05)).clamp(0.0, 1.0);

    r += warm * 0.08;
    g += warm * 0.03;
    b += cold * 0.10;
    g -= cold * 0.02;

    let dim = 1.0 - pressure * 0.12;
    r = (r * dim).clamp(0.0, 1.0);
    g = (g * dim).clamp(0.0, 1.0);
    b = (b * dim).clamp(0.0, 1.0);

    Color::srgb(r, g, b)
}
