use bevy::prelude::*;

use crate::agents::memory::Memory;
use crate::magic::mana::ManaReservoir;
use crate::magic::storage::ManaStorageStyle;
use crate::systems::logging::{LogEvent, LogEventKind};

pub struct ExperimentsPlugin;

impl Plugin for ExperimentsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, record_mana_bias);
    }
}

fn record_mana_bias(
    mut query: Query<(&ManaReservoir, &ManaStorageStyle, &mut Memory), Added<ManaReservoir>>,
    mut writer: MessageWriter<LogEvent>,
) {
    for (reservoir, style, mut memory) in &mut query {
        let pattern = if style.concentration >= style.circulation
            && style.concentration >= style.distribution
        {
            "concentrated"
        } else if style.circulation >= style.distribution {
            "circulating"
        } else {
            "distributed"
        };

        let note = if reservoir.stored > reservoir.capacity * 0.4 {
            format!("Spawned with notable mana affinity and a {pattern} storage bias")
        } else {
            format!("Spawned with low internal mana and a {pattern} storage bias")
        };

        writer.write(LogEvent::new(LogEventKind::Discovery, note.clone()));
        memory.notable_events.push(note);
    }
}
