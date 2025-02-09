use bevy::{
    app::{App, Plugin, Startup},
    color::Color,
    ecs::{
        component::Component,
        observer::Trigger,
        query::With,
        system::{Commands, Query, Single},
    },
    hierarchy::{BuildChildren, ChildBuild},
    picking::{
        events::{Out, Over, Pointer},
        PickingBehavior,
    },
    text::TextFont,
    ui::widget::{Label, Text},
    ui::{AlignItems, AlignSelf, BackgroundColor, FlexDirection, JustifyContent, Node, Val},
    utils::default,
};

use crate::data::Url;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        app.add_observer(update_hover_over);
        app.add_observer(update_hover_out);
    }
}

#[derive(Default, Component)]
struct HoverDetails;

fn setup(mut commands: Commands) {
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Start,
            ..default()
        })
        .insert(PickingBehavior::IGNORE)
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Start,
                        align_items: AlignItems::Start,
                        align_self: AlignSelf::Stretch,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.10, 0.10, 0.10, 0.98)),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Hovered Entity"),
                        TextFont::default().with_font_size(21.0),
                        Label,
                    ));
                    parent.spawn((Text::default(), TextFont::default(), Label, HoverDetails));
                });
        });
}

fn update_hover_over(
    trigger: Trigger<Pointer<Over>>,
    urls: Query<&Url>,
    mut span: Single<&mut Text, With<HoverDetails>>,
) {
    if let Ok(Url(url)) = urls.get(trigger.entity()) {
        ***span = url.clone()
    }
}

fn update_hover_out(
    _trigger: Trigger<Pointer<Out>>,
    mut span: Single<&mut Text, With<HoverDetails>>,
) {
    ***span = String::new();
}
