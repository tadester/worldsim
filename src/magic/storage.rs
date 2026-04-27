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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManaDiscipline {
    Kinesis,
    Hearth,
    Warding,
    Hunt,
    Verdant,
}

impl ManaDiscipline {
    pub fn label(self) -> &'static str {
        match self {
            Self::Kinesis => "Kinesis",
            Self::Hearth => "Hearth",
            Self::Warding => "Warding",
            Self::Hunt => "Hunt",
            Self::Verdant => "Verdant",
        }
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct ManaPractice {
    pub current_action: ManaAction,
    pub last_action: ManaAction,
    pub control: f32,
    pub experimentation_drive: f32,
    pub backlash: f32,
    pub spell_cooldown: f32,
    pub discipline: ManaDiscipline,
    pub telekinesis: f32,
    pub hearthspark: f32,
    pub warding: f32,
    pub hunter_focus: f32,
    pub verdant_touch: f32,
    pub gravity_well: f32,
    pub fireball: f32,
    pub windstep: f32,
    pub healing_pulse: f32,
    pub stone_skin: f32,
    pub root_snare: f32,
    pub mana_bolt: f32,
}

impl Default for ManaPractice {
    fn default() -> Self {
        Self {
            current_action: ManaAction::Hold,
            last_action: ManaAction::Hold,
            control: 0.35,
            experimentation_drive: 0.5,
            backlash: 0.0,
            spell_cooldown: 0.0,
            discipline: ManaDiscipline::Kinesis,
            telekinesis: 0.0,
            hearthspark: 0.0,
            warding: 0.0,
            hunter_focus: 0.0,
            verdant_touch: 0.0,
            gravity_well: 0.0,
            fireball: 0.0,
            windstep: 0.0,
            healing_pulse: 0.0,
            stone_skin: 0.0,
            root_snare: 0.0,
            mana_bolt: 0.0,
        }
    }
}

impl ManaPractice {
    pub fn dominant_ability_label(self) -> &'static str {
        let mut best = ("None", 0.0f32);
        for (label, value) in [
            ("Telekinesis", self.telekinesis),
            ("Hearthspark", self.hearthspark),
            ("Warding", self.warding),
            ("Hunter Focus", self.hunter_focus),
            ("Verdant Touch", self.verdant_touch),
            ("Gravity Well", self.gravity_well),
            ("Fireball", self.fireball),
            ("Windstep", self.windstep),
            ("Healing Pulse", self.healing_pulse),
            ("Stone Skin", self.stone_skin),
            ("Root Snare", self.root_snare),
            ("Mana Bolt", self.mana_bolt),
        ] {
            if value > best.1 {
                best = (label, value);
            }
        }
        best.0
    }

    pub fn discovered_count(self) -> usize {
        [
            self.telekinesis,
            self.hearthspark,
            self.warding,
            self.hunter_focus,
            self.verdant_touch,
            self.gravity_well,
            self.fireball,
            self.windstep,
            self.healing_pulse,
            self.stone_skin,
            self.root_snare,
            self.mana_bolt,
        ]
        .into_iter()
        .filter(|value| *value >= 0.35)
        .count()
    }

    pub fn abilities_summary(self) -> String {
        let mut names = Vec::new();
        if self.telekinesis >= 0.35 {
            names.push("Telekinesis");
        }
        if self.hearthspark >= 0.35 {
            names.push("Hearthspark");
        }
        if self.warding >= 0.35 {
            names.push("Warding");
        }
        if self.hunter_focus >= 0.35 {
            names.push("Hunter Focus");
        }
        if self.verdant_touch >= 0.35 {
            names.push("Verdant Touch");
        }
        if self.gravity_well >= 0.35 {
            names.push("Gravity Well");
        }
        if self.fireball >= 0.35 {
            names.push("Fireball");
        }
        if self.windstep >= 0.35 {
            names.push("Windstep");
        }
        if self.healing_pulse >= 0.35 {
            names.push("Healing Pulse");
        }
        if self.stone_skin >= 0.35 {
            names.push("Stone Skin");
        }
        if self.root_snare >= 0.35 {
            names.push("Root Snare");
        }
        if self.mana_bolt >= 0.35 {
            names.push("Mana Bolt");
        }
        if names.is_empty() {
            "none".to_string()
        } else {
            names.join(", ")
        }
    }

    pub fn spell_summary(self) -> String {
        let mut names = Vec::new();
        if self.gravity_well >= 0.35 {
            names.push("Gravity Well");
        }
        if self.fireball >= 0.35 {
            names.push("Fireball");
        }
        if self.windstep >= 0.35 {
            names.push("Windstep");
        }
        if self.healing_pulse >= 0.35 {
            names.push("Healing Pulse");
        }
        if self.stone_skin >= 0.35 {
            names.push("Stone Skin");
        }
        if self.root_snare >= 0.35 {
            names.push("Root Snare");
        }
        if self.mana_bolt >= 0.35 {
            names.push("Mana Bolt");
        }
        if names.is_empty() {
            "none".to_string()
        } else {
            names.join(", ")
        }
    }
}

pub struct StoragePlugin;

impl Plugin for StoragePlugin {
    fn build(&self, _app: &mut App) {}
}
