use bevy::prelude::*;

#[derive(Component, Debug, Clone)]
pub struct Memory {
    pub notable_events: Vec<String>,
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            notable_events: vec!["Spawned into the world".to_string()],
        }
    }
}

pub struct MemoryPlugin;

impl Plugin for MemoryPlugin {
    fn build(&self, _app: &mut App) {}
}
