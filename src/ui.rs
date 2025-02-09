use bevy::{
    app::{App, Plugin, Startup},
    color::Color,
    ecs::{
        component::{Component, ComponentId},
        entity::Entity,
        observer::Trigger,
        query::With,
        system::{Commands, Query, Single},
        world::DeferredWorld,
    },
    hierarchy::{BuildChildren, ChildBuild},
    picking::{
        events::{Down, Drag, Out, Over, Pointer, Up},
        PickingBehavior,
    },
    render::camera::Camera,
    text::TextFont,
    transform::components::Transform,
    ui::widget::{Label, Text},
    ui::{AlignItems, AlignSelf, BackgroundColor, FlexDirection, JustifyContent, Node, Val},
    utils::default,
};

use crate::{data::Url, sim::Pinned};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        app.add_observer(pointer_down);
        app.add_observer(pointer_drag);
        app.add_observer(pointer_up);
        app.add_observer(pointer_over);
        app.add_observer(pointer_out);
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

#[derive(Default, Component)]
#[require(Pinned)]
#[component(on_add = pin, on_remove = unpin)]
pub struct Dragged;

#[derive(Default, Component)]
#[require(Pinned)]
#[component(on_add = pin, on_remove = unpin)]
struct Hovered;

fn pointer_down(trigger: Trigger<Pointer<Down>>, mut commands: Commands) {
    commands.entity(trigger.entity()).insert_if_new(Dragged);
}

fn pointer_drag(
    trigger: Trigger<Pointer<Drag>>,
    camera_transform: Single<&mut Transform, With<Camera>>,
    mut positions: Query<&mut crate::sim::Position, With<Dragged>>,
) {
    if let Ok(mut position) = positions.get_mut(trigger.entity()) {
        let mut delta = trigger.delta * camera_transform.scale.x;
        delta.y *= -1.0;
        position.0 += delta;
    }
}

fn pointer_up(trigger: Trigger<Pointer<Up>>, mut commands: Commands) {
    commands.entity(trigger.entity()).remove::<Dragged>();
}

fn pointer_over(
    trigger: Trigger<Pointer<Over>>,
    urls: Query<&Url>,
    mut span: Single<&mut Text, With<HoverDetails>>,
    mut commands: Commands,
) {
    if let Ok(Url(url)) = urls.get(trigger.entity()) {
        ***span = url.clone()
    }
    commands.entity(trigger.entity()).insert_if_new(Hovered);
}

fn pointer_out(
    trigger: Trigger<Pointer<Out>>,
    mut span: Single<&mut Text, With<HoverDetails>>,
    mut commands: Commands,
) {
    ***span = String::new();
    commands.entity(trigger.entity()).remove::<Hovered>();
}

fn pin(mut world: DeferredWorld, entity: Entity, _id: ComponentId) {
    if let Some(mut pinned) = world.get_mut::<Pinned>(entity) {
        pinned.count += 1;
    }
}

fn unpin(mut world: DeferredWorld, entity: Entity, _id: ComponentId) {
    if let Some(mut pinned) = world.get_mut::<Pinned>(entity) {
        pinned.count -= 1;
    }
}
