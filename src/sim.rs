use bevy::{
    app::{App, FixedUpdate, Plugin, Update},
    ecs::{
        component::Component,
        entity::Entity,
        query::Without,
        schedule::IntoSystemConfigs,
        system::{Query, Res},
    },
    math::Vec2,
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
pub struct Pinned;

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
        app.add_systems(Update, update_transforms);
    }
}

fn update_transforms(
    mut query: Query<(&mut Transform, &Position, &Velocity)>,
    time: Res<Time<Fixed>>,
) {
    for (mut transform, position, velocity) in &mut query {
        transform.translation =
            (position.0 + velocity.0 * time.overstep().as_secs_f32()).extend(0.0);
    }
}

fn update_positions(
    mut query: Query<(&mut Position, &Velocity), Without<Pinned>>,
    time: Res<Time>,
) {
    for (mut position, velocity) in &mut query {
        position.0 = position.0 + velocity.0 * time.delta().as_secs_f32();
    }
}

fn update_velocities(
    mut query: Query<(&mut Velocity, &Acceleration), Without<Pinned>>,
    time: Res<Time>,
) {
    for (mut velocity, acceleration) in &mut query {
        velocity.0 = velocity.0 * 0.7 + acceleration.0 * time.delta().as_secs_f32();
    }
}

fn repel(mut movables: Query<(&mut Acceleration, &Position)>, positions: Query<&Position>) {
    for (mut acceleration, position) in &mut movables {
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
    mut movables: Query<(&mut Acceleration, &Position), Without<Relationship>>,
) {
    for rel in &relationships {
        let attraction = {
            let Ok((_, from)) = movables.get(rel.from) else {
                continue;
            };
            let Ok((_, to)) = movables.get(rel.to) else {
                continue;
            };
            (to.0 - from.0) * 100.0
        };
        if let Ok((mut from, _)) = movables.get_mut(rel.from) {
            from.0 += attraction;
        }
        if let Ok((mut to, _)) = movables.get_mut(rel.to) {
            to.0 -= attraction;
        }
    }
}
