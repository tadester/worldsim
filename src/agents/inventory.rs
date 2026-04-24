use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy)]
pub struct Inventory {
    pub food: f32,
    pub wood: f32,
    pub seeds: f32,
    pub fiber: f32,
    pub hides: f32,
    pub ore: f32,
    pub metal: f32,
    pub clothing: f32,
    pub weapons: f32,
    pub max_food: f32,
    pub max_wood: f32,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            food: 0.0,
            wood: 0.0,
            seeds: 0.0,
            fiber: 0.0,
            hides: 0.0,
            ore: 0.0,
            metal: 0.0,
            clothing: 0.0,
            weapons: 0.0,
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

    pub fn food_ratio(&self) -> f32 {
        if self.max_food <= 0.0 {
            0.0
        } else {
            (self.food / self.max_food).clamp(0.0, 1.0)
        }
    }

    pub fn wood_ratio(&self) -> f32 {
        if self.max_wood <= 0.0 {
            0.0
        } else {
            (self.wood / self.max_wood).clamp(0.0, 1.0)
        }
    }

    pub fn carry_ratio(&self) -> f32 {
        (self.food_ratio() + self.wood_ratio()) * 0.5
    }
}
