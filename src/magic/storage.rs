use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy)]
pub struct ManaStorageStyle {
    pub concentration: f32,
    pub circulation: f32,
    pub distribution: f32,
}

impl Default for ManaStorageStyle {
    fn default() -> Self {
        Self {
            concentration: 0.3,
            circulation: 0.4,
            distribution: 0.3,
        }
    }
}

pub struct StoragePlugin;

impl Plugin for StoragePlugin {
    fn build(&self, _app: &mut App) {}
}
