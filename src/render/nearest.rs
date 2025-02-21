use bevy::{
    asset::Assets,
    color::Color,
    ecs::{
        component::Component,
        query::With,
        system::{Commands, Res, ResMut, Single},
    },
    math::primitives::Rectangle,
    math::{Quat, Vec3},
    picking::PickingBehavior,
    render::mesh::{Mesh, Mesh2d},
    render::view::Visibility,
    sprite::{ColorMaterial, MeshMaterial2d},
    transform::components::Transform,
};

use crate::{camera::Cursor, interact::Nearest};

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(bevy::app::Startup, setup);
        app.add_systems(bevy::app::Update, update);
    }
}

#[derive(Default, Component)]
struct NearestLine;

pub fn setup(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
) {
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(1.0, 1.0))),
        MeshMaterial2d(materials.add(Color::hsl(30., 0.95, 0.7))),
        NearestLine,
        PickingBehavior::IGNORE,
    ));
}

fn update(
    line: Single<(&mut Transform, &mut Visibility), With<NearestLine>>,
    nearest: Option<Res<Nearest>>,
    cursor: Option<Res<Cursor>>,
) {
    let (mut transform, mut visibility) = line.into_inner();

    let Some((cursor, nearest)) = cursor.zip(nearest) else {
        *visibility = Visibility::Hidden;
        return;
    };

    let delta = cursor.world_position - nearest.position;

    *visibility = Visibility::Visible;
    transform.rotation = Quat::from_rotation_z(delta.to_angle());
    transform.scale = Vec3::new(delta.length(), 1.0, 1.0);
    transform.translation = cursor
        .world_position
        .midpoint(nearest.position)
        .extend(-0.5);
}
