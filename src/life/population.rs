use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopulationKind {
    Animal,
    Npc,
}

#[derive(Resource, Debug, Default, Clone)]
pub struct PopulationStats {
    pub total_births: usize,
    pub total_deaths: usize,
    pub animal_births: usize,
    pub animal_deaths: usize,
    pub npc_births: usize,
    pub npc_deaths: usize,
    pub last_birth_day: Option<f32>,
    pub last_death_day: Option<f32>,
}

impl PopulationStats {
    pub fn record_birth(&mut self, kind: PopulationKind, day: f32) {
        self.total_births += 1;
        self.last_birth_day = Some(day);
        match kind {
            PopulationKind::Animal => self.animal_births += 1,
            PopulationKind::Npc => self.npc_births += 1,
        }
    }

    pub fn record_death(&mut self, kind: PopulationKind, day: f32) {
        self.total_deaths += 1;
        self.last_death_day = Some(day);
        match kind {
            PopulationKind::Animal => self.animal_deaths += 1,
            PopulationKind::Npc => self.npc_deaths += 1,
        }
    }

    pub fn net_growth(&self) -> isize {
        self.total_births as isize - self.total_deaths as isize
    }
}

pub struct PopulationPlugin;

impl Plugin for PopulationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PopulationStats>();
    }
}
