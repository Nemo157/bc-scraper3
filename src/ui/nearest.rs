use bevy::{
    color::Color,
    ecs::{
        change_detection::DetectChanges,
        component::Component,
        entity::Entity,
        query::{QueryData, With},
        system::{Commands, Query, Res, Single},
    },
    hierarchy::{BuildChildren, ChildBuild, DespawnRecursiveExt},
    picking::PickingBehavior,
    text::TextFont,
    ui::widget::{Label, Text},
    ui::{
        AlignItems, BackgroundColor, Display, FlexDirection, JustifyContent, Node, PositionType,
        Val,
    },
};

use crate::{
    data::{EntityType, Url},
    interact::Nearest,
};

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(bevy::app::Startup, setup);
        app.add_systems(bevy::app::Update, update);
    }
}

#[derive(Default, Component)]
struct NodeUi;

fn setup(mut commands: Commands) {
    commands.spawn((
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Start,
            align_items: AlignItems::Start,
            position_type: PositionType::Absolute,
            left: Val::Px(0.),
            top: Val::Px(0.),
            ..Node::default()
        },
        BackgroundColor(Color::srgba(0.10, 0.10, 0.10, 0.98)),
        PickingBehavior::IGNORE,
        NodeUi,
    ));
}

#[derive(QueryData)]
struct NodeDetails {
    ty: &'static EntityType,
    url: &'static Url,
}

fn update(
    nearest: Option<Res<Nearest>>,
    details: Query<NodeDetails>,
    ui: Single<Entity, With<NodeUi>>,
    mut commands: Commands,
) {
    let Some(nearest) = nearest else { return };

    if nearest.is_changed() {
        commands.entity(*ui).despawn_descendants();

        let Ok(details) = details.get(nearest.entity) else {
            // nothing to show
            return;
        };

        commands.entity(*ui).with_children(|ui| {
            ui.spawn((
                Text::new(format!("{:?}", &details.ty)),
                TextFont::default().with_font_size(21.0),
                Label,
                PickingBehavior::IGNORE,
            ));

            ui.spawn((
                Text::new(&details.url.0),
                TextFont::default(),
                Label,
                PickingBehavior::IGNORE,
            ));
        });
    }
}
