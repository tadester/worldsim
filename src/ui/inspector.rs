use bevy::prelude::*;

use crate::agents::animal::Animal;
use crate::agents::decisions::NpcIntent;
use crate::agents::memory::Memory;
use crate::agents::needs::Needs;
use crate::agents::npc::Npc;
use crate::life::growth::Lifecycle;
use crate::magic::mana::ManaReservoir;
use crate::world::resources::{Shelter, Tree, TreeStage};

#[derive(Resource, Default)]
struct SelectedEntity {
    entity: Option<Entity>,
    index: usize,
}

#[derive(Component)]
struct InspectorText;

pub struct InspectorPlugin;

impl Plugin for InspectorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SelectedEntity>()
            .add_systems(Startup, spawn_inspector)
            .add_systems(Update, (cycle_selected_entity, update_inspector));
    }
}

fn spawn_inspector(mut commands: Commands) {
    commands.spawn((
        Text::new("Inspector"),
        TextFont::from_font_size(14.0),
        TextColor(Color::srgb(0.96, 0.92, 0.84)),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(44.0),
            left: px(12.0),
            width: px(340.0),
            ..default()
        },
        InspectorText,
    ));
}

fn cycle_selected_entity(
    keys: Res<ButtonInput<KeyCode>>,
    trees: Query<(Entity, &Transform), With<Tree>>,
    animals: Query<(Entity, &Transform), With<Animal>>,
    npcs: Query<(Entity, &Transform), With<Npc>>,
    mut selected: ResMut<SelectedEntity>,
) {
    if !keys.just_pressed(KeyCode::Tab) {
        return;
    }

    let mut entities: Vec<(Entity, f32, f32)> =
        trees
            .iter()
            .map(|(entity, transform)| (entity, transform.translation.x, transform.translation.y))
            .chain(animals.iter().map(|(entity, transform)| {
                (entity, transform.translation.x, transform.translation.y)
            }))
            .chain(npcs.iter().map(|(entity, transform)| {
                (entity, transform.translation.x, transform.translation.y)
            }))
            .collect();

    entities.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.2.total_cmp(&b.2)));

    if entities.is_empty() {
        selected.entity = None;
        selected.index = 0;
        return;
    }

    selected.index = (selected.index + 1) % entities.len();
    selected.entity = Some(entities[selected.index].0);
}

fn update_inspector(
    selected: Res<SelectedEntity>,
    trees: Query<(&Tree, &Transform, Option<&ManaReservoir>)>,
    shelters: Query<(&Shelter, &Transform)>,
    animals: Query<(&Animal, &Lifecycle, &Transform)>,
    npcs: Query<(
        &Npc,
        &Needs,
        &Memory,
        &NpcIntent,
        &ManaReservoir,
        &Transform,
    )>,
    mut query: Query<&mut Text, With<InspectorText>>,
) {
    let body = if let Some(entity) = selected.entity {
        if let Ok((tree, transform, mana)) = trees.get(entity) {
            format!(
                "Type: Tree\nStage: {}\nGrowth: {:.2}\nPos: {:.0}, {:.0}\nMana: {:.1}",
                tree_stage_label(tree.stage),
                tree.growth,
                transform.translation.x,
                transform.translation.y,
                mana.map(|m| m.stored).unwrap_or(0.0),
            )
        } else if let Ok((shelter, transform)) = shelters.get(entity) {
            format!(
                "Type: Shelter\nIntegrity: {:.2}\nSafety bonus: {:.2}\nPos: {:.0}, {:.0}",
                shelter.integrity,
                shelter.safety_bonus,
                transform.translation.x,
                transform.translation.y,
            )
        } else if let Ok((animal, lifecycle, transform)) = animals.get(entity) {
            format!(
                "Type: Animal\nAge: {:.1}\nHealth: {:.1}\nEnergy: {:.1}\nHunger: {:.2}\nPos: {:.0}, {:.0}",
                lifecycle.age_days,
                animal.health,
                animal.energy,
                animal.hunger,
                transform.translation.x,
                transform.translation.y,
            )
        } else if let Ok((npc, needs, memory, intent, mana, transform)) = npcs.get(entity) {
            format!(
                "Type: NPC\nName: {}\nHealth: {:.1}\nIntent: {}\nNeeds H/S/C: {:.2}/{:.2}/{:.2}\nMana: {:.1}/{:.1}\nInsight: {}\nPos: {:.0}, {:.0}",
                npc.name,
                npc.health,
                intent.label,
                needs.hunger,
                needs.safety,
                needs.curiosity,
                mana.stored,
                mana.capacity,
                memory.last_mana_insight,
                transform.translation.x,
                transform.translation.y,
            )
        } else {
            "Selected entity no longer exists".to_string()
        }
    } else {
        "No entity selected\nPress Tab to cycle through trees, animals, and NPCs".to_string()
    };

    for mut text in &mut query {
        *text = Text::new(format!("Inspector\n{}", body));
    }
}

fn tree_stage_label(stage: TreeStage) -> &'static str {
    match stage {
        TreeStage::Sapling => "Sapling",
        TreeStage::Young => "Young",
        TreeStage::Mature => "Mature",
    }
}
