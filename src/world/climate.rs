use bevy::prelude::*;

use crate::systems::logging::{LogEvent, LogEventKind};
use crate::systems::simulation::SimulationClock;
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
    pub day_length_days: f32,
    pub seasonal_amplitude: f32,
    pub drift_period_days: f32,
    pub drift_amplitude: f32,
    pub comfort_temp: f32,
    pub comfort_band: f32,
    pub solar_warmth: f32,
    pub lunar_cold: f32,
    pub current_offset: f32,
    pub current_season_phase: f32,
    pub current_day_phase: f32,
}

impl Default for ClimateModel {
    fn default() -> Self {
        Self {
            year_length_days: 365.0,
            day_length_days: 1.0,
            seasonal_amplitude: 0.18,
            drift_period_days: 3650.0,
            drift_amplitude: 0.06,
            comfort_temp: 0.55,
            comfort_band: 0.14,
            solar_warmth: 0.18,
            lunar_cold: 0.12,
            current_offset: 0.0,
            current_season_phase: 0.0,
            current_day_phase: 0.0,
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

    pub fn solar_factor(&self) -> f32 {
        let cycle = self.current_day_phase.rem_euclid(1.0) * std::f32::consts::TAU;
        (cycle.sin() * 0.5 + 0.5 - 0.15).clamp(0.0, 1.0) / 0.85
    }

    pub fn lunar_factor(&self) -> f32 {
        (1.0 - self.solar_factor()).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ClimateEventKind {
    Heatwave,
    ColdSnap,
}

impl ClimateEventKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Heatwave => "Heatwave",
            Self::ColdSnap => "Cold snap",
        }
    }

    fn base_delta(self) -> f32 {
        match self {
            Self::Heatwave => 1.0,
            Self::ColdSnap => -1.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ActiveClimateEvent {
    pub kind: ClimateEventKind,
    pub center: IVec2,
    pub radius: f32,
    pub intensity: f32,
    pub remaining_days: f32,
}

impl ActiveClimateEvent {
    pub fn local_temp_delta(&self, coord: IVec2) -> f32 {
        if self.radius <= 0.1 || self.intensity.abs() <= 0.0001 {
            return 0.0;
        }
        let dx = (coord.x - self.center.x) as f32;
        let dy = (coord.y - self.center.y) as f32;
        let d = (dx * dx + dy * dy).sqrt();
        let falloff = (1.0 - d / self.radius).clamp(0.0, 1.0);
        self.kind.base_delta() * self.intensity * falloff
    }
}

#[derive(Resource, Debug, Clone)]
pub struct ClimateEventState {
    rng: SimpleRng,
    pub active: Option<ActiveClimateEvent>,
    pub cooldown_days: f32,
}

impl Default for ClimateEventState {
    fn default() -> Self {
        Self {
            rng: SimpleRng::new(0x8f3b_12d9_b5f0_28c1),
            active: None,
            cooldown_days: 6.0,
        }
    }
}

impl ClimateEventState {
    pub fn local_temp_delta(&self, coord: IVec2) -> f32 {
        self.active
            .as_ref()
            .map(|event| event.local_temp_delta(coord))
            .unwrap_or(0.0)
    }
}

pub struct ClimatePlugin;

impl Plugin for ClimatePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClimateModel>()
            .init_resource::<ClimateEventState>()
            .add_systems(
                PreUpdate,
                (update_climate_events, update_region_climate).chain(),
            );
    }
}

fn update_climate_events(
    clock: Res<SimulationClock>,
    step: Res<SimulationStep>,
    settings: Res<MapSettings>,
    mut state: ResMut<ClimateEventState>,
    mut writer: MessageWriter<LogEvent>,
) {
    let delta_days = clock.delta_days();
    if delta_days <= 0.0 {
        return;
    }

    if let Some(active) = state.active.as_mut() {
        active.remaining_days -= delta_days;
        if active.remaining_days <= 0.0 {
            writer.write(LogEvent::new(
                LogEventKind::Climate,
                format!(
                    "{} ends near {},{} on day {:.1}",
                    active.kind.label(),
                    active.center.x,
                    active.center.y,
                    step.elapsed_days
                ),
            ));
            state.active = None;
            state.cooldown_days = 6.0 + state.rng.next_f32() * 8.0;
        }
        return;
    }

    state.cooldown_days = (state.cooldown_days - delta_days).max(0.0);
    if state.cooldown_days > 0.0 {
        return;
    }

    let start_chance = (0.04 * delta_days).clamp(0.0, 0.4);
    if state.rng.next_f32() >= start_chance {
        return;
    }

    let kind = if state.rng.next_f32() < 0.5 {
        ClimateEventKind::Heatwave
    } else {
        ClimateEventKind::ColdSnap
    };

    let width = settings.width.max(1);
    let height = settings.height.max(1);
    let center = IVec2::new(
        state.rng.range_i32(0, width - 1),
        state.rng.range_i32(0, height - 1),
    );
    let radius = 3.0 + state.rng.next_f32() * 7.0;
    let intensity = 0.08 + state.rng.next_f32() * 0.14;
    let duration = 2.0 + state.rng.next_f32() * 6.0;

    state.active = Some(ActiveClimateEvent {
        kind,
        center,
        radius,
        intensity,
        remaining_days: duration,
    });

    writer.write(LogEvent::new(
        LogEventKind::Climate,
        format!(
            "{} begins near {},{} (radius {:.0}, {:.1}d) on day {:.1}",
            kind.label(),
            center.x,
            center.y,
            radius,
            duration,
            step.elapsed_days
        ),
    ));
}

fn update_region_climate(
    step: Res<SimulationStep>,
    settings: Res<MapSettings>,
    mut climate: ResMut<ClimateModel>,
    events: Res<ClimateEventState>,
    mut regions: Query<(&mut RegionTile, &mut Sprite, &mut RegionClimate)>,
) {
    if climate.year_length_days <= 0.0 {
        return;
    }

    let elapsed_days = step.elapsed_days;
    let season_phase = (elapsed_days / climate.year_length_days).rem_euclid(1.0);
    climate.current_season_phase = season_phase;
    let day_phase = if climate.day_length_days > 0.0 {
        (elapsed_days / climate.day_length_days).rem_euclid(1.0)
    } else {
        0.0
    };
    climate.current_day_phase = day_phase;

    let seasonal = (season_phase * std::f32::consts::TAU).sin();
    let drift = if climate.drift_period_days > 0.0 {
        ((elapsed_days / climate.drift_period_days).rem_euclid(1.0) * std::f32::consts::TAU).sin()
    } else {
        0.0
    };

    let daylight_cycle = (day_phase * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2).sin();
    let sun_warmth = daylight_cycle.max(0.0) * climate.solar_warmth;
    let moon_cold = (-daylight_cycle).max(0.0) * climate.lunar_cold;
    let global_offset =
        seasonal * climate.seasonal_amplitude + drift * climate.drift_amplitude + sun_warmth - moon_cold;
    climate.current_offset = global_offset;

    let height = (settings.height.max(2) - 1) as f32;
    for (mut tile, mut sprite, mut region_climate) in &mut regions {
        let y_norm = tile.coord.y as f32 / height;
        let latitude_strength = 0.8 + y_norm * 0.6;
        let event_delta = events.local_temp_delta(tile.coord);
        let temp = (tile.base_temperature + global_offset * latitude_strength + event_delta)
            .clamp(0.0, 1.0);
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

#[derive(Debug, Clone, Copy)]
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    fn next_f32(&mut self) -> f32 {
        let bits = (self.next_u64() >> 40) as u32;
        bits as f32 / (u32::MAX as f32)
    }

    fn range_i32(&mut self, min: i32, max_inclusive: i32) -> i32 {
        if max_inclusive <= min {
            return min;
        }
        let span = (max_inclusive - min) as u32 + 1;
        let value = (self.next_u64() as u32) % span;
        min + value as i32
    }
}
