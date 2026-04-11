use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy)]
pub struct Inventory {
    pub food: f32,
    pub wood: f32,
    pub max_food: f32,
    pub max_wood: f32,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            food: 0.0,
            wood: 0.0,
            max_food: 3.0,
            max_wood: 3.0,
        }
    }
}

impl Inventory {
    pub fn food_space(&self) -> f32 {
        (self.max_food - self.food).max(0.0)
    }

    pub fn wood_space(&self) -> f32 {
        (self.max_wood - self.wood).max(0.0)
    }
}
