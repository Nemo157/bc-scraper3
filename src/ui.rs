use bevy::{
    app::{App, Plugin, Startup},
    color::Color,
    ecs::{
        component::{Component, ComponentId},
        entity::Entity,
        observer::Trigger,
        query::With,
        system::{Commands, Query, Res, Single},
        world::DeferredWorld,
    },
    hierarchy::{BuildChildren, ChildBuild},
    picking::{
        events::{Click, Down, Drag, Out, Over, Pointer, Up},
        PickingBehavior,
    },
    render::camera::Camera,
    text::TextFont,
    transform::components::Transform,
    ui::widget::{Label, Text},
    ui::{AlignItems, BackgroundColor, FlexDirection, JustifyContent, Node},
    utils::default,
};

use crate::{background::Request, data::EntityData, sim::Pinned};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        app.add_observer(pointer_down);
        app.add_observer(pointer_drag);
        app.add_observer(pointer_up);
        app.add_observer(pointer_click);
        app.add_observer(pointer_over);
        app.add_observer(pointer_out);
    }
}

#[derive(Default, Component)]
struct HoverDetails;

fn setup(mut commands: Commands) {
    commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Start,
                align_items: AlignItems::Start,
                ..default()
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
                HoverDetails,
                PickingBehavior::IGNORE,
            ));
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
    data: Query<&EntityData>,
    mut span: Single<&mut Text, With<HoverDetails>>,
    mut commands: Commands,
) {
    if let Ok(data) = data.get(trigger.entity()) {
        ***span = data.url().to_owned();
    }
    commands.entity(trigger.entity()).insert_if_new(Hovered);
}

fn pointer_click(
    trigger: Trigger<Pointer<Click>>,
    scraper: Res<crate::background::Thread>,
    data: Query<&EntityData>,
) {
    if trigger.duration.as_millis() < 100 {
        match data.get(trigger.entity()) {
            Ok(EntityData::Album(album)) => {
                scraper
                    .send(Request::Album {
                        url: album.url.clone(),
                    })
                    .unwrap();
            }
            Ok(EntityData::User(user)) => {
                scraper
                    .send(Request::User {
                        url: user.url.clone(),
                    })
                    .unwrap();
            }
            Err(_) => {}
        }
    }
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
