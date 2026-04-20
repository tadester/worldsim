use bevy::prelude::*;

#[derive(Resource, Debug, Clone, Copy)]
pub struct SimulationClock {
    pub seconds_per_day: f32,
    pub step_seconds: f32,
    pub steps_per_frame: u32,
    pub paused: bool,
}

impl Default for SimulationClock {
    fn default() -> Self {
        Self {
            seconds_per_day: 12.0,
            step_seconds: 1.0 / 60.0,
            steps_per_frame: 1,
            paused: false,
        }
    }
}

impl SimulationClock {
    pub fn delta_seconds(&self) -> f32 {
        if self.paused {
            0.0
        } else {
            self.step_seconds * self.steps_per_frame as f32
        }
    }

    pub fn delta_days(&self) -> f32 {
        self.delta_seconds() / self.seconds_per_day
    }

    pub fn speed_label(&self) -> &'static str {
        match self.steps_per_frame {
            1 => "1x",
            5 => "5x",
            20 => "20x",
            120 => "120x",
            300 => "300x",
            900 => "900x",
            _ => "custom",
        }
    }
}

#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct SimulationStep {
    pub tick: u64,
    pub elapsed_days: f32,
}

pub struct SimulationCorePlugin;

impl Plugin for SimulationCorePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimulationClock>()
            .init_resource::<SimulationStep>()
            .add_systems(Update, advance_simulation_step);
    }
}

fn advance_simulation_step(clock: Res<SimulationClock>, mut step: ResMut<SimulationStep>) {
    if clock.paused {
        return;
    }

    step.tick += clock.steps_per_frame as u64;
    step.elapsed_days += clock.delta_days();
}
