use bevy::prelude::*;

#[derive(Component, Debug, Clone)]
pub struct Relationships {
    pub social_drive: f32,
    pub trust_baseline: f32,
}

impl Default for Relationships {
    fn default() -> Self {
        Self {
            social_drive: 0.5,
            trust_baseline: 0.5,
        }
    }
}

pub struct RelationshipsPlugin;

impl Plugin for RelationshipsPlugin {
    fn build(&self, _app: &mut App) {}
}
