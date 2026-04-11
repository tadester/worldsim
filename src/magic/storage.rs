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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManaAction {
    Absorb,
    Hold,
    Circulate,
    Concentrate,
    Distribute,
    Release,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct ManaPractice {
    pub current_action: ManaAction,
    pub last_action: ManaAction,
    pub control: f32,
    pub experimentation_drive: f32,
    pub backlash: f32,
}

impl Default for ManaPractice {
    fn default() -> Self {
        Self {
            current_action: ManaAction::Hold,
            last_action: ManaAction::Hold,
            control: 0.35,
            experimentation_drive: 0.5,
            backlash: 0.0,
        }
    }
}

pub struct StoragePlugin;

impl Plugin for StoragePlugin {
    fn build(&self, _app: &mut App) {}
}
