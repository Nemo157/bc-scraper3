use bevy::{
    app::{App, FixedUpdate, Plugin, Update},
    ecs::{
        bundle::Bundle,
        component::{Component, ComponentId},
        entity::Entity,
        query::Changed,
        schedule::IntoSystemConfigs,
        system::{Query, Res, Resource},
        world::DeferredWorld,
    },
    math::Vec2,
    time::{Fixed, Time},
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

#[derive(Component, Copy, Clone)]
pub struct Weight(pub f32);

fn increment_relation_count(mut world: DeferredWorld, entity: Entity, _id: ComponentId) {
    let Relationship { from, to, .. } = *world.get::<Relationship>(entity).unwrap();
    world.get_mut::<RelationCount>(from).unwrap().count += 1;
    world.get_mut::<RelationCount>(to).unwrap().count += 1;
}

#[derive(Default, Resource)]
pub struct Paused(pub bool);

pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (update_positions, repel, attract, update_velocities).chain(),
        );
        app.add_systems(Update, lock_pinned);
        app.insert_resource(Paused(false));
    }
}

fn lock_pinned(
    mut query: Query<(&mut Position, &mut Velocity, &Pinned), Changed<Pinned>>,
    time: Res<Time<Fixed>>,
) {
    for (mut position, mut velocity, pinned) in &mut query {
        if pinned.count > 0 {
            position.0 += velocity.0 * time.overstep_fraction();
            velocity.0 = Vec2::ZERO;
        }
    }
}

fn update_positions(
    paused: Res<Paused>,
    mut query: Query<(&mut Position, &Velocity, Option<&Pinned>)>,
) {
    if paused.0 {
        return;
    };

    query
        .iter_mut()
        .for_each(|(mut position, velocity, pinned)| {
            if pinned.map_or(0, |p| p.count) == 0 {
                position.0 = position.0 + velocity.0;
            }
        });
}

fn update_velocities(
    paused: Res<Paused>,
    mut query: Query<(&mut Velocity, &Acceleration, Option<&Pinned>)>,
) {
    if paused.0 {
        return;
    };

    query
        .iter_mut()
        .for_each(|(mut velocity, acceleration, pinned)| {
            if pinned.map_or(0, |p| p.count) == 0 {
                velocity.0 = (velocity.0 * 0.7 + acceleration.0 * 0.05).clamp_length_max(50.0);
            }
        });
}

fn repel(
    paused: Res<Paused>,
    mut entities: Query<(&mut Acceleration, &Position)>,
    positions: Query<&Position>,
) {
    if paused.0 {
        return;
    };

    entities
        .iter_mut()
        .for_each(|(mut acceleration, position)| {
            acceleration.0 = position.0 * -0.1;
            positions.iter().for_each(|other_position| {
                let dist = position.0 - other_position.0;
                let dsq = position.0.distance_squared(other_position.0).max(0.001);
                acceleration.0 += dist * 1000.0 / dsq;
            })
        });
}

fn attract(
    paused: Res<Paused>,
    relationships: Query<(&Relationship, &Weight)>,
    mut entities: Query<(&mut Acceleration, &Position, &RelationCount)>,
) {
    if paused.0 {
        return;
    };

    relationships.iter().for_each(|(rel, weight)| {
        let attraction = {
            let Ok((_, from, _)) = entities.get(rel.from) else {
                return;
            };
            let Ok((_, to, _)) = entities.get(rel.to) else {
                return;
            };
            (to.0 - from.0) * 2.0 * weight.0
        };
        if let Ok((mut from, _, relations)) = entities.get_mut(rel.from) {
            from.0 += attraction / (relations.count as f32).sqrt();
        }
        if let Ok((mut to, _, relations)) = entities.get_mut(rel.to) {
            to.0 -= attraction / (relations.count as f32).sqrt();
        }
    });
}
