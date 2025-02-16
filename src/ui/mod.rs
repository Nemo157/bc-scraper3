use bevy::{
    app::{App, Plugin, Startup},
    color::Color,
    ecs::{
        component::{Component, ComponentId},
        entity::Entity,
        observer::Trigger,
        query::{With, Without},
        system::{Commands, Query, Res, Single},
        world::DeferredWorld,
    },
    hierarchy::{BuildChildren, ChildBuild},
    input::{keyboard::KeyCode, ButtonInput},
    picking::{
        events::{Click, Down, Drag, Out, Over, Pointer, Up},
        PickingBehavior,
    },
    render::camera::Camera,
    text::TextFont,
    time::{Time, Virtual},
    transform::components::Transform,
    ui::widget::{Label, Text},
    ui::{AlignItems, BackgroundColor, FlexDirection, JustifyContent, Node},
    utils::default,
};

use crate::{
    background::Request,
    data::{EntityType, Url},
    sim::{Pinned, Relationship},
};

mod diagnostic;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        app.add_systems(bevy::app::Update, update_time);
        app.add_observer(pointer_down);
        app.add_observer(pointer_drag);
        app.add_observer(pointer_up);
        app.add_observer(pointer_click);
        app.add_observer(pointer_over);
        app.add_observer(pointer_out);

        app.add_plugins(self::diagnostic::Plugin);
    }
}

#[derive(Default, Component)]
struct HoverDetails;

#[derive(Default, Component)]
struct TimeText;

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
                Text::default(),
                TextFont::default(),
                Label,
                TimeText,
                PickingBehavior::IGNORE,
            ));
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

fn update_time(time: Res<Time<Virtual>>, mut text: Single<&mut Text, With<TimeText>>) {
    ***text = format!("speed: {}", time.relative_speed());
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
    mut positions: Query<
        (&mut crate::sim::Position, &mut Transform),
        (With<Dragged>, Without<Camera>),
    >,
) {
    if let Ok((mut position, mut transform)) = positions.get_mut(trigger.entity()) {
        let mut delta = trigger.delta * camera_transform.scale.x;
        delta.y *= -1.0;
        position.0 += delta;
        transform.translation += delta.extend(0.0);
    }
}

fn pointer_up(trigger: Trigger<Pointer<Up>>, mut commands: Commands) {
    commands.entity(trigger.entity()).remove::<Dragged>();
}

fn pointer_over(
    trigger: Trigger<Pointer<Over>>,
    url: Query<&Url>,
    mut span: Single<&mut Text, With<HoverDetails>>,
    mut commands: Commands,
) {
    if let Ok(url) = url.get(trigger.entity()) {
        ***span = url.0.clone();
    }
    commands.entity(trigger.entity()).insert_if_new(Hovered);
}

fn pointer_click(
    trigger: Trigger<Pointer<Click>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    scraper: Res<crate::background::Thread>,
    data: Query<(&Url, &EntityType)>,
    relationships: Query<&Relationship>,
) {
    if trigger.duration.as_millis() < 100 {
        let request = |entity| match data.get(entity) {
            Ok((Url(url), EntityType::Album)) => {
                scraper.send(Request::Album { url: url.clone() }).unwrap();
            }
            Ok((Url(url), EntityType::Artist)) => {
                scraper.send(Request::Artist { url: url.clone() }).unwrap();
            }
            Ok((Url(url), EntityType::User)) => {
                scraper.send(Request::User { url: url.clone() }).unwrap();
            }
            Err(_) => {}
        };

        if keyboard.pressed(KeyCode::ShiftLeft) {
            let next_level = |entity| {
                relationships.iter().filter_map(move |rel| {
                    (rel.from == entity)
                        .then_some(rel.to)
                        .or((rel.to == entity).then_some(rel.from))
                })
            };
            if keyboard.pressed(KeyCode::ControlLeft) {
                next_level(trigger.entity())
                    .flat_map(|entity| next_level(entity))
                    .for_each(|entity| request(entity));
            } else {
                next_level(trigger.entity()).for_each(|entity| request(entity));
            }
        } else {
            request(trigger.entity());
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
