use bevy::{
    asset::{Assets, Handle},
    color::Color,
    diagnostic::Diagnostics,
    ecs::{
        entity::Entity,
        query::{With, Without},
        system::{Commands, Query, Res, ResMut, Single},
    },
    math::primitives::{Annulus, Circle, Rectangle},
    math::{Quat, Vec3},
    render::mesh::{Mesh, Mesh2d},
    render::view::Visibility,
    sprite::{ColorMaterial, MeshMaterial2d},
    time::{Fixed, Time},
    transform::components::Transform,
};

use crate::{
    data::{AlbumId, ArtistId, UserId},
    sim::{Paused, Position, Relationship, Velocity},
    RelationshipParent,
};

use std::time::Instant;

mod diagnostic;
mod nearest;

static ALBUM_MESH_HANDLE: Handle<Mesh> = Handle::weak_from_u128(0xe7233fda8e904a2f8cff6638b3bc5e7f);
static ALBUM_COLOR_MATERIAL_HANDLE: Handle<ColorMaterial> =
    Handle::weak_from_u128(0x3d3b3dfff39b42a39e7af2d5f1f80ad6);

static ARTIST_MESH_HANDLE: Handle<Mesh> =
    Handle::weak_from_u128(0x3fc46e8efa014a19808ae833b2a2b5bd);
static ARTIST_COLOR_MATERIAL_HANDLE: Handle<ColorMaterial> =
    Handle::weak_from_u128(0x7253624dfd34415b9a309cc0c289fe6f);

static USER_MESH_HANDLE: Handle<Mesh> = Handle::weak_from_u128(0x48daf856c5c742eeaf609e4ad20bc5fc);
static USER_COLOR_MATERIAL_HANDLE: Handle<ColorMaterial> =
    Handle::weak_from_u128(0x531591f539514109bd0aa36c2231ded4);

static LINK_MESH_HANDLE: Handle<Mesh> = Handle::weak_from_u128(0x003550e416a740c886de78b65200b0f6);
static LINK_COLOR_MATERIAL_HANDLE: Handle<ColorMaterial> =
    Handle::weak_from_u128(0x4d9f259f1e2841a0988b14dce5b76f91);

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(bevy::app::Startup, setup_meshes);

        app.add_systems(
            bevy::app::Update,
            (
                init_meshes,
                init_node_transforms,
                update_node_transforms,
                init_relationship_transforms,
                update_relationship_transforms,
            ),
        );

        app.add_plugins(self::diagnostic::Plugin);
        app.add_plugins(self::nearest::Plugin);
    }
}

pub fn setup_meshes(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    meshes.insert(&ALBUM_MESH_HANDLE, Circle::new(10.0).into());
    materials.insert(
        &ALBUM_COLOR_MATERIAL_HANDLE,
        Color::hsl(0., 0.95, 0.7).into(),
    );

    meshes.insert(&ARTIST_MESH_HANDLE, Annulus::new(10.0, 6.0).into());
    materials.insert(
        &ARTIST_COLOR_MATERIAL_HANDLE,
        Color::hsl(270., 0.95, 0.7).into(),
    );

    meshes.insert(&USER_MESH_HANDLE, Rectangle::new(10.0, 10.0).into());
    materials.insert(
        &USER_COLOR_MATERIAL_HANDLE,
        Color::hsl(180., 0.95, 0.7).into(),
    );

    meshes.insert(&LINK_MESH_HANDLE, Rectangle::new(1.0, 1.0).into());
    materials.insert(
        &LINK_COLOR_MATERIAL_HANDLE,
        Color::hsl(90., 0.95, 0.7).into(),
    );
}

fn init_meshes(
    albums: Query<Entity, (With<AlbumId>, Without<Mesh2d>)>,
    artists: Query<Entity, (With<ArtistId>, Without<Mesh2d>)>,
    users: Query<Entity, (With<UserId>, Without<Mesh2d>)>,
    relationships: Query<Entity, (With<Relationship>, Without<Mesh2d>)>,
    mut commands: Commands,
) {
    for entity in &albums {
        commands.entity(entity).insert((
            Mesh2d(ALBUM_MESH_HANDLE.clone()),
            MeshMaterial2d(ALBUM_COLOR_MATERIAL_HANDLE.clone()),
        ));
    }

    for entity in &artists {
        commands.entity(entity).insert((
            Mesh2d(ARTIST_MESH_HANDLE.clone()),
            MeshMaterial2d(ARTIST_COLOR_MATERIAL_HANDLE.clone()),
        ));
    }

    for entity in &users {
        commands.entity(entity).insert((
            Mesh2d(USER_MESH_HANDLE.clone()),
            MeshMaterial2d(USER_COLOR_MATERIAL_HANDLE.clone()),
        ));
    }

    for entity in &relationships {
        commands.entity(entity).insert((
            Mesh2d(LINK_MESH_HANDLE.clone()),
            MeshMaterial2d(LINK_COLOR_MATERIAL_HANDLE.clone()),
        ));
    }
}

fn node_translation(position: &Position, velocity: &Velocity, time: &Time<Fixed>) -> Vec3 {
    (position.0 + velocity.0 * time.overstep_fraction()).extend(0.0)
}

fn init_node_transforms(
    query: Query<(Entity, &Position, &Velocity), Without<Transform>>,
    time: Res<Time<Fixed>>,
    mut commands: Commands,
) {
    for (entity, position, velocity) in &query {
        commands
            .entity(entity)
            .insert(Transform::from_translation(node_translation(
                &position, &velocity, &time,
            )));
    }
}

fn update_node_transforms(
    paused: Res<Paused>,
    mut query: Query<(&mut Transform, &Position, &Velocity)>,
    time: Res<Time<Fixed>>,
    mut diagnostics: Diagnostics,
) {
    if paused.0 {
        return;
    }

    let start = Instant::now();

    for (mut transform, position, velocity) in &mut query {
        transform.translation = node_translation(&position, &velocity, &time);
    }

    diagnostics.add_measurement(&self::diagnostic::NODES, || {
        start.elapsed().as_secs_f64() * 1000.
    });
}

fn relationship_transform(
    (from_pos, from_vel): (&Position, &Velocity),
    (to_pos, to_vel): (&Position, &Velocity),
    time: &Time<Fixed>,
) -> Transform {
    let from = from_pos.0 + from_vel.0 * time.overstep_fraction();
    let to = to_pos.0 + to_vel.0 * time.overstep_fraction();
    let delta = to - from;
    Transform {
        rotation: Quat::from_rotation_z(delta.to_angle()),
        scale: Vec3::new(delta.length(), 1.0, 1.0),
        translation: from.midpoint(to).extend(-1.0),
    }
}

fn init_relationship_transforms(
    relationships: Query<(Entity, &Relationship), Without<Transform>>,
    nodes: Query<(&Position, &Velocity)>,
    time: Res<Time<Fixed>>,
    mut commands: Commands,
) {
    for (entity, rel) in &relationships {
        let Ok(from) = nodes.get(rel.from) else {
            continue;
        };
        let Ok(to) = nodes.get(rel.to) else {
            continue;
        };

        commands
            .entity(entity)
            .insert(relationship_transform(from, to, &time));
    }
}

fn update_relationship_transforms(
    paused: Res<Paused>,
    relationship_parent: Single<&Visibility, With<RelationshipParent>>,
    mut relationships: Query<(&Relationship, &mut Transform)>,
    nodes: Query<(&Position, &Velocity)>,
    time: Res<Time<Fixed>>,
    mut diagnostics: Diagnostics,
) {
    if *relationship_parent == Visibility::Hidden || paused.0 {
        return;
    }

    let start = Instant::now();

    for (rel, mut transform) in &mut relationships {
        let Ok(from) = nodes.get(rel.from) else {
            continue;
        };
        let Ok(to) = nodes.get(rel.to) else {
            continue;
        };

        *transform = relationship_transform(from, to, &time);
    }

    diagnostics.add_measurement(&self::diagnostic::RELATIONS, || {
        start.elapsed().as_secs_f64() * 1000.
    });
}
