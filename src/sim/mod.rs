use bevy::{
    diagnostic::Diagnostics,
    ecs::{
        bundle::Bundle,
        change_detection::{DetectChanges, Mut},
        component::{Component, ComponentId},
        entity::Entity,
        query::{Changed, Without},
        schedule::IntoSystemConfigs,
        system::{Commands, Query, Res, ResMut, Resource},
        world::DeferredWorld,
    },
    math::{I64Vec2, Vec2},
    time::{Fixed, Time},
    utils::{AHasher, PassHash},
};

use std::{
    collections::{hash_map, HashMap, HashSet},
    hash::BuildHasherDefault,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

use rand::distr::{Distribution, Uniform};

mod diagnostic;

#[derive(Debug, Default, Component, Copy, Clone)]
pub struct Position(pub Vec2);

#[derive(Debug, Default, Component, Copy, Clone)]
pub struct PredictedPosition(pub Vec2);

#[derive(Debug, Default, Component)]
pub struct Velocity(pub Vec2);

#[derive(Debug, Default, Component)]
pub struct Acceleration(pub Vec2);

#[derive(Debug, Default, Component)]
pub struct Pinned {
    pub count: u32,
}

#[derive(Debug, Default, Bundle)]
pub struct MotionBundle {
    pub position: Position,
    pub velocity: Velocity,
    pub acceleration: Acceleration,
    pinned: Pinned,
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
            pinned: Pinned::default(),
            relation_count: RelationCount::default(),
        }
    }

    pub fn random_near(position: Vec2) -> Self {
        let mut rng = rand::rng();
        let positions = Uniform::new(-100.0, 100.0).unwrap();
        let velocities = Uniform::new(-10.0, 10.0).unwrap();

        let position = position + Vec2::new(positions.sample(&mut rng), positions.sample(&mut rng));
        let velocity = Vec2::new(velocities.sample(&mut rng), velocities.sample(&mut rng));

        Self {
            position: Position(position),
            velocity: Velocity(velocity),
            acceleration: Acceleration(Vec2::ZERO),
            pinned: Pinned::default(),
            relation_count: RelationCount::default(),
        }
    }
}

#[derive(Debug, Default, Component)]
pub struct RelationCount {
    pub count: u32,
}

#[derive(Debug, Component, Copy, Clone, Eq, PartialEq, Hash)]
#[component(on_add = increment_relation_count)]
pub struct Relationship {
    pub from: Entity,
    pub to: Entity,
}

#[derive(Debug, Component, Copy, Clone)]
pub struct Weight(pub f32);

fn increment_relation_count(mut world: DeferredWorld, entity: Entity, _id: ComponentId) {
    let Relationship { from, to, .. } = *world.get::<Relationship>(entity).unwrap();
    world.get_mut::<RelationCount>(from).unwrap().count += 1;
    world.get_mut::<RelationCount>(to).unwrap().count += 1;
}

#[derive(Debug, Default, Resource, Copy, Clone)]
pub enum OriginForceMode {
    #[default]
    Unit,
    Square,
    Cube,
}

impl OriginForceMode {
    pub fn go_to_next(&mut self) {
        *self = match *self {
            OriginForceMode::Unit => OriginForceMode::Square,
            OriginForceMode::Square => OriginForceMode::Cube,
            OriginForceMode::Cube => OriginForceMode::Unit,
        }
    }
}

#[derive(Default, Resource)]
pub struct Paused(pub bool);

#[derive(Default, Resource)]
pub struct Partitions(HashMap<I64Vec2, HashSet<Entity, PassHash>, BuildHasherDefault<AHasher>>);

impl Partitions {
    pub const SIZE: f32 = 400.;

    fn key(point: Vec2) -> I64Vec2 {
        (point / Self::SIZE).floor().as_i64vec2()
    }

    fn update(&mut self, from: Vec2, to: Vec2, entity: Entity) {
        let from = Self::key(from);
        let to = Self::key(to);

        if from != to {
            if let hash_map::Entry::Occupied(mut partition) = self.0.entry(from) {
                partition.get_mut().remove(&entity);
                if partition.get().is_empty() {
                    partition.remove();
                }
            }
            self.0.entry(to).or_default().insert(entity);
        }
    }

    fn add(&mut self, point: Vec2, entity: Entity) {
        self.0.entry(Self::key(point)).or_default().insert(entity);
    }

    fn iter(
        &self,
    ) -> impl Iterator<Item = (I64Vec2, impl Iterator<Item = Entity> + use<'_>)> + use<'_> {
        self.0.iter().map(|(&key, set)| (key, set.iter().copied()))
    }

    fn nearby_keys(point: Vec2) -> [I64Vec2; 4] {
        let key = Self::key(point);
        let center = (key.as_vec2() * Self::SIZE) + Vec2::new(Self::SIZE / 2., Self::SIZE / 2.);
        let (x, y) = (
            if center.x < point.x { 1 } else { -1 },
            if center.y < point.y { 1 } else { -1 },
        );
        [
            key,
            key + I64Vec2::new(0, y),
            key + I64Vec2::new(x, 0),
            key + I64Vec2::new(x, y),
        ]
    }

    fn nearby(&self, point: Vec2) -> impl Iterator<Item = Entity> + use<'_> {
        Self::nearby_keys(point)
            .into_iter()
            .filter_map(|key| self.0.get(&key))
            .flatten()
            .copied()
    }

    fn distant_keys(&self, point: Vec2) -> impl Iterator<Item = I64Vec2> + use<'_> {
        let nearby_keys = Self::nearby_keys(point);
        self.0
            .keys()
            .copied()
            .filter(move |key| !nearby_keys.contains(key))
    }
}

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(
            bevy::app::FixedUpdate,
            (
                update_positions,
                check_yeet,
                repel,
                attract,
                update_velocities,
            )
                .chain(),
        );
        app.add_systems(
            bevy::app::PreUpdate,
            (lock_pinned, init_predicted_position, predict_positions),
        );
        app.insert_resource(Paused(false));
        app.insert_resource(Partitions::default());
        app.insert_resource(OriginForceMode::default());
        app.add_plugins(self::diagnostic::Plugin);
    }
}

fn init_predicted_position(
    query: Query<(Entity, &Position), Without<PredictedPosition>>,
    mut commands: Commands,
) {
    for (entity, position) in &query {
        commands
            .entity(entity)
            .insert(PredictedPosition(position.0));
    }
}

fn lock_pinned(
    mut query: Query<(&mut Position, &mut Velocity, &PredictedPosition, &Pinned), Changed<Pinned>>,
) {
    for (mut position, mut velocity, predicted, pinned) in &mut query {
        if pinned.count > 0 {
            position.0 = predicted.0;
            velocity.0 = Vec2::ZERO;
        }
    }
}

fn predict_positions(
    paused: Res<Paused>,
    mut query: Query<(
        &mut PredictedPosition,
        &Position,
        &Velocity,
        Option<&Pinned>,
    )>,
    time: Res<Time<Fixed>>,
) {
    if paused.0 {
        return;
    }

    query
        .par_iter_mut()
        .for_each(|(mut predicted, position, velocity, pinned)| {
            if pinned.map_or(0, |p| p.count) == 0 {
                predicted.0 = position.0 + velocity.0 * time.overstep_fraction();
            } else {
                predicted.0 = position.0;
            }
        });
}

fn check_yeet(query: Query<(&Position, &Velocity, &Acceleration)>) {
    let yeet = std::sync::atomic::AtomicBool::new(false);

    query.par_iter().for_each(|(position, _, _)| {
        if position.0.length() > 20000. {
            yeet.store(true, Ordering::Relaxed);
        }
    });

    if yeet.load(Ordering::Relaxed) {
        eprintln!();
        eprintln!();
        for ((pos, vel, acc), i) in query.iter().zip(0..) {
            eprint!(
                "{:>15} {:>15} {:>15}    ",
                format!("{:.0},{:.0}", pos.0.x, pos.0.y),
                format!("{:.0},{:.0}", vel.0.x, vel.0.y),
                format!("{:.0},{:.0}", acc.0.x, acc.0.y),
            );
            if i % 7 == 0 {
                eprintln!();
            }
        }
        eprintln!();
        eprintln!("sim yeeted itself, hopefully the above shows something useful");
        std::process::exit(0);
    }
}

fn update_positions(
    paused: Res<Paused>,
    mut partitions: ResMut<Partitions>,
    mut query: Query<(Mut<Position>, &Velocity, Option<&Pinned>, Entity)>,
    mut diagnostics: Diagnostics,
) {
    if paused.0 {
        return;
    };

    let start = Instant::now();

    query
        .iter_mut()
        .for_each(|(mut position, velocity, pinned, entity)| {
            if pinned.map_or(0, |p| p.count) == 0 {
                let old_position = position.0;
                let new_position = position.0 + velocity.0;
                position.0 = new_position;
                partitions.update(old_position, new_position, entity);
            }
            if position.is_added() {
                partitions.add(position.0, entity);
            }
        });

    diagnostics.add_measurement(&self::diagnostic::update::POSITIONS, || {
        start.elapsed().as_secs_f64() * 1000.
    });
}

fn update_velocities(
    paused: Res<Paused>,
    mut query: Query<(&mut Velocity, &Acceleration, Option<&Pinned>)>,
    mut diagnostics: Diagnostics,
) {
    if paused.0 {
        return;
    };

    let start = Instant::now();

    query
        .par_iter_mut()
        .for_each(|(mut velocity, acceleration, pinned)| {
            if pinned.map_or(0, |p| p.count) == 0 {
                velocity.0 = (velocity.0 * 0.7 + acceleration.0).clamp_length_max(50.0);
            }
        });

    diagnostics.add_measurement(&self::diagnostic::update::VELOCITIES, || {
        start.elapsed().as_secs_f64() * 1000.
    });
}

fn repel(
    paused: Res<Paused>,
    origin_force_mode: Res<OriginForceMode>,
    mut nodes: Query<(&mut Acceleration, &Position)>,
    partitions: Res<Partitions>,
    positions: Query<&Position>,
    mut diagnostics: Diagnostics,
) {
    if paused.0 {
        return;
    };

    let start = Instant::now();

    let partition_start = Instant::now();

    let averages = HashMap::<_, _, BuildHasherDefault<AHasher>>::from_iter(partitions.iter().map(
        |(key, entities)| {
            (key, {
                let (sum, count) = entities
                    .filter_map(|entity| positions.get(entity).ok())
                    .fold((Vec2::ZERO, 0), |(average, count), position| {
                        (average + position.0, count + 1)
                    });
                let position = sum / (count as f32);
                // Note: because of floats and rounding the position might be just outside the
                // partition if all entities are on the border.
                (position, count)
            })
        },
    ));

    diagnostics.add_measurement(&self::diagnostic::update::repel::PARTITIONS, || {
        partition_start.elapsed().as_secs_f64() * 1000.
    });

    let nearby_us = AtomicU64::new(0);
    let distant_us = AtomicU64::new(0);

    nodes
        .par_iter_mut()
        .for_each(|(mut acceleration, position)| {
            acceleration.0 = match *origin_force_mode {
                OriginForceMode::Unit => position.0 * -0.005,
                OriginForceMode::Square => position.0 * position.0.length() * -0.00005,
                OriginForceMode::Cube => position.0 * position.0.length_squared() * -0.0000005,
            };

            let nearby_start = Instant::now();
            partitions
                .nearby(position.0)
                .filter_map(|entity| positions.get(entity).ok())
                .for_each(|other_position| {
                    let dist = position.0 - other_position.0;
                    let dsq = position.0.distance_squared(other_position.0);
                    if dsq < 0.001 {
                        acceleration.0 +=
                            Vec2::new(rand::random::<f32>() - 0.5, rand::random::<f32>() - 0.5);
                    } else {
                        acceleration.0 += dist * 50.0 / dsq;
                    }
                });
            nearby_us.fetch_add(nearby_start.elapsed().as_micros() as u64, Ordering::Relaxed);

            let distant_start = Instant::now();
            partitions
                .distant_keys(position.0)
                .filter_map(|key| averages.get(&key))
                .for_each(|&(other_position, count)| {
                    let dist = position.0 - other_position;
                    let dsq = position.0.distance_squared(other_position);
                    acceleration.0 += dist * 50.0 * (count as f32) / dsq;
                });
            distant_us.fetch_add(
                distant_start.elapsed().as_micros() as u64,
                Ordering::Relaxed,
            );
        });

    diagnostics.add_measurement(&self::diagnostic::update::repel::NEARBY, || {
        nearby_us.load(Ordering::Relaxed) as f64 / 1000.
    });

    diagnostics.add_measurement(&self::diagnostic::update::repel::DISTANT, || {
        distant_us.load(Ordering::Relaxed) as f64 / 1000.
    });

    diagnostics.add_measurement(&self::diagnostic::update::REPEL, || {
        start.elapsed().as_secs_f64() * 1000.
    });
}

fn attract(
    paused: Res<Paused>,
    relationships: Query<(&Relationship, &Weight)>,
    mut nodes: Query<(&mut Acceleration, &Position, &RelationCount)>,
    mut diagnostics: Diagnostics,
) {
    if paused.0 {
        return;
    };

    let start = Instant::now();

    relationships.iter().for_each(|(rel, weight)| {
        let attraction = {
            let Ok((_, from, _)) = nodes.get(rel.from) else {
                return;
            };
            let Ok((_, to, _)) = nodes.get(rel.to) else {
                return;
            };
            (to.0 - from.0) * weight.0 / 10.
        };
        if let Ok((mut from, _, relations)) = nodes.get_mut(rel.from) {
            from.0 += attraction / (relations.count as f32);
        }
        if let Ok((mut to, _, relations)) = nodes.get_mut(rel.to) {
            to.0 -= attraction / (relations.count as f32);
        }
    });

    diagnostics.add_measurement(&self::diagnostic::update::ATTRACT, || {
        start.elapsed().as_secs_f64() * 1000.
    });
}
