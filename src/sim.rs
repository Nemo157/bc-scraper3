use bevy::{
    app::{App, FixedUpdate, Plugin, Update},
    ecs::{
        component::Component,
        entity::Entity,
        query::Without,
        schedule::IntoSystemConfigs,
        system::{Query, Res},
    },
    math::{Quat, Vec2},
    time::{Fixed, Time},
    transform::components::Transform,
};

#[derive(Default, Component)]
pub struct Position(pub Vec2);

#[derive(Default, Component)]
pub struct Velocity(pub Vec2);

#[derive(Default, Component)]
pub struct Acceleration(pub Vec2);

#[derive(Default, Component)]
pub struct Pinned {
    pub count: u32,
}

#[derive(Component)]
pub struct Relationship {
    pub from: Entity,
    pub to: Entity,
}

pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (update_positions, repel, attract, update_velocities).chain(),
        );
        app.add_systems(
            Update,
            (update_entity_transforms, update_relationship_transforms),
        );
    }
}

fn update_entity_transforms(
    mut query: Query<(&mut Transform, &Position, &Velocity)>,
    time: Res<Time<Fixed>>,
) {
    for (mut transform, position, velocity) in &mut query {
        transform.translation =
            (position.0 + velocity.0 * time.overstep().as_secs_f32()).extend(0.0);
    }
}

fn update_relationship_transforms(
    mut relationships: Query<
        (&Relationship, &mut Transform),
        (Without<Position>, Without<Velocity>),
    >,
    entities: Query<(&Position, &Velocity), Without<Relationship>>,
    time: Res<Time<Fixed>>,
) {
    for (rel, mut transform) in &mut relationships {
        let Ok((from_pos, from_vel)) = entities.get(rel.from) else {
            continue;
        };
        let from_pos = from_pos.0 + from_vel.0 * time.overstep().as_secs_f32();
        let Ok((to_pos, to_vel)) = entities.get(rel.to) else {
            continue;
        };
        let to_pos = to_pos.0 + to_vel.0 * time.overstep().as_secs_f32();
        let delta = to_pos - from_pos;
        transform.rotation = Quat::from_rotation_z((to_pos - from_pos).to_angle());
        transform.scale.x = delta.length();
        transform.translation = from_pos.midpoint(to_pos).extend(-1.0);
    }
}

fn update_positions(
    mut query: Query<(&mut Position, &Velocity, Option<&Pinned>)>,
    time: Res<Time>,
) {
    for (mut position, velocity, pinned) in &mut query {
        if pinned.map_or(0, |p| p.count) == 0 {
            position.0 = position.0 + velocity.0 * time.delta().as_secs_f32();
        }
    }
}

fn update_velocities(
    mut query: Query<(&mut Velocity, &Acceleration, Option<&Pinned>)>,
    time: Res<Time>,
) {
    for (mut velocity, acceleration, pinned) in &mut query {
        if pinned.map_or(0, |p| p.count) == 0 {
            velocity.0 = velocity.0 * 0.7 + acceleration.0 * time.delta().as_secs_f32();
        }
    }
}

fn repel(mut entities: Query<(&mut Acceleration, &Position)>, positions: Query<&Position>) {
    for (mut acceleration, position) in &mut entities {
        acceleration.0 = Vec2::ZERO;
        for other_position in &positions {
            let dist = position.0 - other_position.0;
            let dsq = (dist.x * dist.x + dist.y * dist.y).max(0.001);
            acceleration.0 += dist * 50000.0 / dsq;
        }
    }
}

fn attract(
    relationships: Query<&Relationship, (Without<Position>, Without<Acceleration>)>,
    mut entities: Query<(&mut Acceleration, &Position), Without<Relationship>>,
) {
    for rel in &relationships {
        let attraction = {
            let Ok((_, from)) = entities.get(rel.from) else {
                continue;
            };
            let Ok((_, to)) = entities.get(rel.to) else {
                continue;
            };
            (to.0 - from.0) * 100.0
        };
        if let Ok((mut from, _)) = entities.get_mut(rel.from) {
            from.0 += attraction;
        }
        if let Ok((mut to, _)) = entities.get_mut(rel.to) {
            to.0 -= attraction;
        }
    }
}
