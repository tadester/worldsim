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
    Climate,
    Proposal,
}

#[derive(Message, Debug, Clone)]
pub struct NpcDeathEvent {
    pub day: f32,
    pub npc_name: String,
    pub reason: String,
}

impl NpcDeathEvent {
    pub fn new(day: f32, npc_name: String, reason: String) -> Self {
        Self {
            day,
            npc_name,
            reason,
        }
    }
}

#[derive(Resource, Default)]
pub struct EventLog {
    pub entries: Vec<LogEntry>,
    pub max_entries: usize,
}

#[derive(Debug, Clone)]
pub struct NpcDeathEntry {
    pub day: f32,
    pub npc_name: String,
    pub reason: String,
}

#[derive(Resource, Default)]
pub struct NpcDeathLog {
    pub entries: Vec<NpcDeathEntry>,
    pub max_entries: usize,
}

pub struct LoggingPlugin;

impl Plugin for LoggingPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<LogEvent>()
            .add_message::<NpcDeathEvent>()
            .insert_resource(EventLog {
                entries: Vec::new(),
                max_entries: 32,
            })
            .insert_resource(NpcDeathLog {
                entries: Vec::new(),
                max_entries: 128,
            })
            .add_systems(Update, (collect_log_events, collect_npc_death_events));
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

fn collect_npc_death_events(
    mut events: MessageReader<NpcDeathEvent>,
    mut log: ResMut<NpcDeathLog>,
) {
    for event in events.read() {
        log.entries.push(NpcDeathEntry {
            day: event.day,
            npc_name: event.npc_name.clone(),
            reason: event.reason.clone(),
        });
    }

    if log.entries.len() > log.max_entries {
        let overflow = log.entries.len() - log.max_entries;
        log.entries.drain(0..overflow);
    }
}
