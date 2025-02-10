use bevy::{
    app::{App, FixedUpdate, Plugin, Update},
    ecs::{
        bundle::Bundle,
        component::{Component, ComponentId},
        entity::Entity,
        query::Without,
        schedule::IntoSystemConfigs,
        system::{Query, Res},
        world::DeferredWorld,
    },
    math::{Quat, Vec2},
    time::{Fixed, Time},
    transform::components::Transform,
};

use rand::distr::{Distribution, Uniform};

#[derive(Default, Component, Copy, Clone)]
pub struct Position(pub Vec2);

#[derive(Default, Component)]
pub struct Velocity(pub Vec2);

#[derive(Default, Component)]
pub struct Acceleration(pub Vec2);

#[derive(Default, Component)]
pub struct Pinned {
    pub count: u32,
}

#[derive(Default, Bundle)]
pub struct MotionBundle {
    pub transform: Transform,
    pub position: Position,
    pub velocity: Velocity,
    pub acceleration: Acceleration,
    relation_count: RelationCount,
}

impl MotionBundle {
    pub fn random() -> Self {
        let mut rng = rand::rng();
        let positions = Uniform::new(-300.0, 300.0).unwrap();
        let velocities = Uniform::new(-10.0, 10.0).unwrap();

        let position = Vec2::new(positions.sample(&mut rng), positions.sample(&mut rng));
        let velocity = Vec2::new(velocities.sample(&mut rng), velocities.sample(&mut rng));

        Self {
            transform: Transform::from_translation(position.extend(0.0)),
            position: Position(position),
            velocity: Velocity(velocity),
            acceleration: Acceleration(Vec2::ZERO),
            relation_count: RelationCount::default(),
        }
    }

    pub fn random_near(position: Position) -> Self {
        let mut rng = rand::rng();
        let positions = Uniform::new(-100.0, 100.0).unwrap();
        let velocities = Uniform::new(-10.0, 10.0).unwrap();

        let position =
            position.0 + Vec2::new(positions.sample(&mut rng), positions.sample(&mut rng));
        let velocity = Vec2::new(velocities.sample(&mut rng), velocities.sample(&mut rng));

        Self {
            transform: Transform::from_translation(position.extend(0.0)),
            position: Position(position),
            velocity: Velocity(velocity),
            acceleration: Acceleration(Vec2::ZERO),
            relation_count: RelationCount::default(),
        }
    }
}

#[derive(Default, Component)]
pub struct RelationCount {
    pub count: u32,
}

#[derive(Component, Copy, Clone, Eq, PartialEq, Hash)]
#[component(on_add = increment_relation_count)]
pub struct Relationship {
    pub from: Entity,
    pub to: Entity,
}

fn increment_relation_count(mut world: DeferredWorld, entity: Entity, _id: ComponentId) {
    let Relationship { from, to } = *world.get::<Relationship>(entity).unwrap();
    world.get_mut::<RelationCount>(from).unwrap().count += 1;
    world.get_mut::<RelationCount>(to).unwrap().count += 1;
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
        transform.translation = (position.0 + velocity.0 * time.overstep_fraction()).extend(0.0);
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
        let from_pos = from_pos.0 + from_vel.0 * time.overstep_fraction();
        let Ok((to_pos, to_vel)) = entities.get(rel.to) else {
            continue;
        };
        let to_pos = to_pos.0 + to_vel.0 * time.overstep_fraction();
        let delta = to_pos - from_pos;
        transform.rotation = Quat::from_rotation_z((to_pos - from_pos).to_angle());
        transform.scale.x = delta.length();
        transform.translation = from_pos.midpoint(to_pos).extend(-1.0);
    }
}

fn update_positions(mut query: Query<(&mut Position, &Velocity, Option<&Pinned>)>) {
    for (mut position, velocity, pinned) in &mut query {
        if pinned.map_or(0, |p| p.count) == 0 {
            position.0 = position.0 + velocity.0;
        }
    }
}

fn update_velocities(mut query: Query<(&mut Velocity, &Acceleration, Option<&Pinned>)>) {
    for (mut velocity, acceleration, pinned) in &mut query {
        if pinned.map_or(0, |p| p.count) == 0 {
            velocity.0 = (velocity.0 * 0.7 + acceleration.0 * 0.05).clamp_length_max(50.0);
        }
    }
}

fn repel(mut entities: Query<(&mut Acceleration, &Position)>, positions: Query<&Position>) {
    for (mut acceleration, position) in &mut entities {
        acceleration.0 = Vec2::ZERO;
        for other_position in &positions {
            let dist = position.0 - other_position.0;
            let dsq = position.0.distance_squared(other_position.0).max(0.001);
            acceleration.0 += dist * 1000.0 / dsq;
        }
    }
}

fn attract(
    relationships: Query<&Relationship, (Without<Position>, Without<Acceleration>)>,
    mut entities: Query<(&mut Acceleration, &Position, &RelationCount), Without<Relationship>>,
) {
    for rel in &relationships {
        let attraction = {
            let Ok((_, from, _)) = entities.get(rel.from) else {
                continue;
            };
            let Ok((_, to, _)) = entities.get(rel.to) else {
                continue;
            };
            (to.0 - from.0) * 2.0
        };
        if let Ok((mut from, _, relations)) = entities.get_mut(rel.from) {
            from.0 += attraction / (relations.count as f32).sqrt();
        }
        if let Ok((mut to, _, relations)) = entities.get_mut(rel.to) {
            to.0 -= attraction / (relations.count as f32).sqrt();
        }
    }
}
