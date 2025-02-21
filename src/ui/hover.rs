use bevy::{
    color::Color,
    ecs::{
        change_detection::DetectChanges,
        component::Component,
        query::With,
        system::{Commands, Query, Res, Single},
    },
    hierarchy::{BuildChildren, ChildBuild},
    picking::PickingBehavior,
    text::TextFont,
    ui::widget::{Label, Text},
    ui::{
        AlignItems, BackgroundColor, Display, FlexDirection, JustifyContent, Node, PositionType,
        Val,
    },
};

use crate::{data::Url, interact::Hovered};

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(bevy::app::Startup, setup);
        app.add_systems(bevy::app::Update, update);
    }
}

#[derive(Default, Component)]
pub struct NodeDetails;

fn setup(mut commands: Commands) {
    commands
        .spawn((
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
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Hovered Entity"),
                TextFont::default().with_font_size(21.0),
                Label,
                PickingBehavior::IGNORE,
            ));
            parent.spawn((
                Text::default(),
                TextFont::default(),
                Label,
                NodeDetails,
                PickingBehavior::IGNORE,
            ));
        });
}

fn update(hovered: Res<Hovered>, url: Query<&Url>, mut span: Single<&mut Text, With<NodeDetails>>) {
    if hovered.is_changed() {
        if let Some(url) = hovered.0.and_then(|entity| url.get(entity).ok()) {
            ***span = url.0.clone();
        } else {
            ***span = "".into();
        }
    }
}
