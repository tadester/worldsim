use bevy::prelude::*;

use crate::systems::simulation::SimulationStep;

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub day: f32,
    pub kind: LogEventKind,
    pub message: String,
}

#[derive(Message, Debug, Clone)]
pub struct LogEvent {
    pub kind: LogEventKind,
    pub message: String,
}

impl LogEvent {
    pub fn new(kind: LogEventKind, message: String) -> Self {
        Self { kind, message }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LogEventKind {
    Birth,
    Death,
    Discovery,
    Construction,
    Territory,
    Threat,
}

#[derive(Resource, Default)]
pub struct EventLog {
    pub entries: Vec<LogEntry>,
    pub max_entries: usize,
}

pub struct LoggingPlugin;

impl Plugin for LoggingPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<LogEvent>()
            .insert_resource(EventLog {
                entries: Vec::new(),
                max_entries: 32,
            })
            .add_systems(Update, collect_log_events);
    }
}

fn collect_log_events(
    step: Res<SimulationStep>,
    mut events: MessageReader<LogEvent>,
    mut log: ResMut<EventLog>,
) {
    for event in events.read() {
        log.entries.push(LogEntry {
            day: step.elapsed_days,
            kind: event.kind,
            message: event.message.clone(),
        });
    }

    if log.entries.len() > log.max_entries {
        let overflow = log.entries.len() - log.max_entries;
        log.entries.drain(0..overflow);
    }
}
