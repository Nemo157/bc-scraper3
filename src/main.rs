use bevy::{
    app::{App, Startup},
    asset::Assets,
    color::Color,
    core_pipeline::core_2d::Camera2d,
    ecs::{change_detection::ResMut, system::Commands},
    math::primitives::Circle,
    render::mesh::{Mesh, Mesh2d},
    sprite::{ColorMaterial, MeshMaterial2d},
    transform::components::Transform,
    DefaultPlugins,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

const X_EXTENT: f32 = 900.;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);

    let shapes = [
        meshes.add(Circle::new(50.0)),
        meshes.add(Circle::new(50.0)),
        meshes.add(Circle::new(50.0)),
        meshes.add(Circle::new(50.0)),
        meshes.add(Circle::new(50.0)),
        meshes.add(Circle::new(50.0)),
    ];
    let num_shapes = shapes.len();

    for (i, shape) in shapes.into_iter().enumerate() {
        let color = Color::hsl(360. * i as f32 / num_shapes as f32, 0.95, 0.7);

        commands.spawn((
            Mesh2d(shape),
            MeshMaterial2d(materials.add(color)),
            Transform::from_xyz(
                -X_EXTENT / 2. + i as f32 / (num_shapes - 1) as f32 * X_EXTENT,
                0.0,
                0.0,
            ),
        ));
    }
}
