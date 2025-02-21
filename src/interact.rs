use bevy::{
    ecs::{
        change_detection::DetectChangesMut,
        entity::Entity,
        observer::Trigger,
        query::{With, Without},
        system::{Commands, Query, Res, ResMut, Resource, Single},
    },
    input::{keyboard::KeyCode, ButtonInput},
    math::Vec2,
    picking::events::{Click, Down, Drag, Out, Over, Pointer, Up},
    render::camera::Camera,
    transform::components::Transform,
};

use crate::{
    background::Request,
    camera::Cursor,
    data::{EntityType, Url},
    sim::{Pinned, PredictedPosition, Relationship},
};

#[derive(Default, Resource)]
pub struct Dragged(pub Option<Entity>);

#[derive(Default, Resource)]
pub struct Hovered(pub Option<Entity>);

#[derive(Debug, Resource, PartialEq)]
pub struct Nearest {
    pub entity: Entity,
    pub position: Vec2,
}

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.init_resource::<Dragged>();
        app.init_resource::<Hovered>();

        app.add_systems(bevy::app::PreUpdate, update_nearest);

        app.add_observer(pointer_down);
        app.add_observer(pointer_drag);
        app.add_observer(pointer_up);
        app.add_observer(pointer_click);
        app.add_observer(pointer_over);
        app.add_observer(pointer_out);
    }
}

fn update_nearest(
    cursor: Option<Res<Cursor>>,
    positions: Query<(Entity, &PredictedPosition)>,
    nearest: Option<ResMut<Nearest>>,
    mut commands: Commands,
) {
    let Some(cursor) = cursor else { return };

    let Some((entity, position)) = positions.iter().min_by_key(|(_, position)| {
        // positive floats have the same order when viewed as bits
        (position.0 - cursor.world_position)
            .length_squared()
            .to_bits()
    }) else {
        commands.remove_resource::<Nearest>();
        return;
    };

    let new = Nearest {
        entity,
        position: position.0,
    };

    if let Some(mut nearest) = nearest {
        nearest.set_if_neq(new);
    } else {
        commands.insert_resource(new);
    }
}

fn pointer_down(
    trigger: Trigger<Pointer<Down>>,
    mut dragged: ResMut<Dragged>,
    mut pinned: Query<&mut Pinned>,
) {
    if let Some(mut pinned) = dragged.0.and_then(|entity| pinned.get_mut(entity).ok()) {
        pinned.count -= 1;
    }

    dragged.0 = Some(trigger.entity());

    if let Some(mut pinned) = pinned.get_mut(trigger.entity()).ok() {
        pinned.count += 1;
    }
}

fn pointer_up(
    _trigger: Trigger<Pointer<Up>>,
    mut dragged: ResMut<Dragged>,
    mut pinned: Query<&mut Pinned>,
) {
    if let Some(mut pinned) = dragged.0.and_then(|entity| pinned.get_mut(entity).ok()) {
        pinned.count -= 1;
    }

    dragged.0 = None;
}

fn pointer_over(
    trigger: Trigger<Pointer<Over>>,
    mut hovered: ResMut<Hovered>,
    mut pinned: Query<&mut Pinned>,
) {
    if let Some(mut pinned) = hovered.0.and_then(|entity| pinned.get_mut(entity).ok()) {
        pinned.count -= 1;
    }

    hovered.0 = Some(trigger.entity());

    if let Some(mut pinned) = pinned.get_mut(trigger.entity()).ok() {
        pinned.count += 1;
    }
}

fn pointer_out(
    trigger: Trigger<Pointer<Out>>,
    mut hovered: ResMut<Hovered>,
    mut pinned: Query<&mut Pinned>,
) {
    if hovered.0 != Some(trigger.entity()) {
        return;
    }

    if let Some(mut pinned) = hovered.0.and_then(|entity| pinned.get_mut(entity).ok()) {
        pinned.count -= 1;
    }

    hovered.0 = None;
}

fn pointer_drag(
    trigger: Trigger<Pointer<Drag>>,
    dragged: Res<Dragged>,
    camera_transform: Single<&mut Transform, With<Camera>>,
    mut positions: Query<(&mut crate::sim::Position, &mut Transform), Without<Camera>>,
) {
    if dragged.0 != Some(trigger.entity()) {
        return;
    }

    if let Ok((mut position, mut transform)) = positions.get_mut(trigger.entity()) {
        let mut delta = trigger.delta * camera_transform.scale.x;
        delta.y *= -1.0;
        position.0 += delta;
        transform.translation += delta.extend(0.0);
    }
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
            Ok((Url(url), EntityType::Release)) => {
                scraper.send(Request::Release { url: url.clone() }).unwrap();
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
