use bevy::{
    app::{App, Startup, Update},
    asset::{Assets, Handle},
    color::Color,
    ecs::{
        query::With,
        system::{Query, Res, ResMut, Single},
    },
    math::primitives::{Annulus, Circle, Rectangle},
    math::Quat,
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
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_meshes);

        app.add_systems(
            Update,
            (update_entity_transforms, update_relationship_transforms),
        );

        app.register_required_components_with::<AlbumId, _>(|| Mesh2d(ALBUM_MESH_HANDLE.clone()));
        app.register_required_components_with::<AlbumId, _>(|| {
            MeshMaterial2d(ALBUM_COLOR_MATERIAL_HANDLE.clone())
        });

        app.register_required_components_with::<ArtistId, _>(|| Mesh2d(ARTIST_MESH_HANDLE.clone()));
        app.register_required_components_with::<ArtistId, _>(|| {
            MeshMaterial2d(ARTIST_COLOR_MATERIAL_HANDLE.clone())
        });

        app.register_required_components_with::<UserId, _>(|| Mesh2d(USER_MESH_HANDLE.clone()));
        app.register_required_components_with::<UserId, _>(|| {
            MeshMaterial2d(USER_COLOR_MATERIAL_HANDLE.clone())
        });

        app.register_required_components_with::<Relationship, _>(|| {
            Mesh2d(LINK_MESH_HANDLE.clone())
        });
        app.register_required_components_with::<Relationship, _>(|| {
            MeshMaterial2d(LINK_COLOR_MATERIAL_HANDLE.clone())
        });

        app.register_required_components::<Position, Transform>();
    }
}

pub fn init_meshes(mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<ColorMaterial>>) {
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

fn update_entity_transforms(
    paused: Res<Paused>,
    mut query: Query<(&mut Transform, &Position, &Velocity)>,
    time: Res<Time<Fixed>>,
) {
    if paused.0 {
        return;
    };

    query
        .iter_mut()
        .for_each(|(mut transform, position, velocity)| {
            transform.translation =
                (position.0 + velocity.0 * time.overstep_fraction()).extend(0.0);
        })
}

fn update_relationship_transforms(
    paused: Res<Paused>,
    relationship_parent: Single<&Visibility, With<RelationshipParent>>,
    mut relationships: Query<(&Relationship, &mut Transform)>,
    entities: Query<(&Position, &Velocity)>,
    time: Res<Time<Fixed>>,
) {
    if *relationship_parent == Visibility::Hidden || paused.0 {
        return;
    }

    relationships.iter_mut().for_each(|(rel, mut transform)| {
        let Ok((from_pos, from_vel)) = entities.get(rel.from) else {
            return;
        };
        let from_pos = from_pos.0 + from_vel.0 * time.overstep_fraction();
        let Ok((to_pos, to_vel)) = entities.get(rel.to) else {
            return;
        };
        let to_pos = to_pos.0 + to_vel.0 * time.overstep_fraction();
        let delta = to_pos - from_pos;
        transform.rotation = Quat::from_rotation_z((to_pos - from_pos).to_angle());
        transform.scale.x = delta.length();
        transform.translation = from_pos.midpoint(to_pos).extend(-1.0);
    });
}
