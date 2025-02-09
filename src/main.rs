use bevy::{
    app::{App, Startup},
    asset::Assets,
    color::Color,
    ecs::{change_detection::ResMut, system::Commands},
    math::{
        primitives::{Circle, Rectangle},
        Vec2,
    },
    picking::{mesh_picking::MeshPickingPlugin, PickingBehavior},
    render::mesh::{Mesh, Mesh2d},
    sprite::{ColorMaterial, MeshMaterial2d},
    time::{Fixed, Time},
    transform::components::Transform,
    DefaultPlugins,
};

mod camera;
mod data;
mod sim;
mod ui;

use crate::{
    data::Url,
    sim::{Acceleration, Position, Relationship, Velocity},
};
use rand::{
    distr::{Distribution, Uniform},
    seq::IndexedRandom,
};
use rand_distr::Poisson;

fn main() {
    App::new()
        .insert_resource(Time::<Fixed>::from_hz(100.0))
        .add_plugins((
            DefaultPlugins,
            MeshPickingPlugin,
            camera::CameraPlugin,
            sim::SimPlugin,
            ui::UiPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let circle = meshes.add(Circle::new(5.0));
    let square = meshes.add(Rectangle::new(5.0, 5.0));
    let unit = meshes.add(Rectangle::new(1.0, 1.0));
    let album_mat = materials.add(Color::hsl(0., 0.95, 0.7));
    let user_mat = materials.add(Color::hsl(180., 0.95, 0.7));
    let link_mat = materials.add(Color::hsl(90., 0.95, 0.7));

    let mut rng = rand::rng();

    let positions = Uniform::new(200.0, 400.0).unwrap();
    let velocities = Uniform::new(-10.0, 10.0).unwrap();

    let mut albums = Vec::new();
    for i in 0..100 {
        let position = Vec2::new(positions.sample(&mut rng), positions.sample(&mut rng));
        albums.push(
            commands
                .spawn((
                    Mesh2d(circle.clone()),
                    MeshMaterial2d(album_mat.clone()),
                    Transform::from_translation(position.extend(0.0)),
                    Position(position),
                    Velocity(Vec2::new(
                        velocities.sample(&mut rng),
                        velocities.sample(&mut rng),
                    )),
                    Acceleration(Vec2::ZERO),
                    Url(format!("rand:album:{i}")),
                ))
                .id(),
        );
    }

    let mut users = Vec::new();
    for i in 0..5 {
        let position = Vec2::new(positions.sample(&mut rng), positions.sample(&mut rng));
        users.push(
            commands
                .spawn((
                    Mesh2d(square.clone()),
                    MeshMaterial2d(user_mat.clone()),
                    Transform::from_translation(position.extend(0.0)),
                    Position(position),
                    Velocity(Vec2::new(
                        velocities.sample(&mut rng),
                        velocities.sample(&mut rng),
                    )),
                    Acceleration(Vec2::ZERO),
                    Url(format!("rand:user:{i}")),
                ))
                .id(),
        );
    }

    let mut linked_albums = Vec::new();

    for from in &users {
        let count: f64 = Poisson::new(20.0).unwrap().sample(&mut rng);
        for to in albums.drain(..(count as usize).min(albums.len())) {
            linked_albums.push(to);
            commands.spawn((
                Relationship { from: *from, to },
                Mesh2d(unit.clone()),
                MeshMaterial2d(link_mat.clone()),
                Transform::IDENTITY,
                PickingBehavior::IGNORE,
            ));
        }
    }

    for from in &users {
        let count: f64 = Poisson::new(6.0).unwrap().sample(&mut rng);
        for to in linked_albums.choose_multiple(&mut rng, count as usize) {
            commands.spawn((
                Relationship {
                    from: *from,
                    to: *to,
                },
                Mesh2d(unit.clone()),
                MeshMaterial2d(link_mat.clone()),
                Transform::IDENTITY,
                PickingBehavior::IGNORE,
            ));
        }
    }

    for from in &albums {
        let to = users.choose(&mut rng).unwrap();
        commands.spawn((
            Relationship {
                from: *from,
                to: *to,
            },
            Mesh2d(unit.clone()),
            MeshMaterial2d(link_mat.clone()),
            Transform::IDENTITY,
            PickingBehavior::IGNORE,
        ));
    }
}
