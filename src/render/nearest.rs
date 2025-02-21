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
    mut transform: Single<&mut Transform, With<NearestLine>>,
    nearest: Res<Nearest>,
    cursor: Res<Cursor>,
) {
    let from = cursor.world_position;
    let Some(to) = nearest.position else { return };

    let delta = from - to;

    transform.rotation = Quat::from_rotation_z(delta.to_angle());
    transform.scale = Vec3::new(delta.length(), 1.0, 1.0);
    transform.translation = from.midpoint(to).extend(-0.5);
}
