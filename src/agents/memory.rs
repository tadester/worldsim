use bevy::prelude::*;

#[derive(Component, Debug, Clone)]
pub struct Memory {
    pub notable_events: Vec<String>,
    pub last_forage_coord: Option<IVec2>,
    pub last_safe_position: Option<Vec2>,
    pub last_social_contact_days: f32,
    pub last_decision: String,
    pub last_mana_insight: String,
}

impl Default for Memory {
    fn default() -> Self {
        Self {
            notable_events: vec!["Spawned into the world".to_string()],
            last_forage_coord: None,
            last_safe_position: None,
            last_social_contact_days: 0.0,
            last_decision: "Settling in".to_string(),
            last_mana_insight: "No mana insight yet".to_string(),
        }
    }
}

pub struct MemoryPlugin;

impl Plugin for MemoryPlugin {
    fn build(&self, _app: &mut App) {}
}
