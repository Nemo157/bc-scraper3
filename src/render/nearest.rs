use bevy::{
    asset::Assets,
    color::Color,
    ecs::{
        component::Component,
        query::{QueryData, Without},
        system::{Commands, Res, ResMut, Single},
    },
    math::primitives::Rectangle,
    math::Vec2,
    math::{Quat, Vec3},
    picking::PickingBehavior,
    render::camera::Camera,
    render::mesh::{Mesh, Mesh2d},
    render::view::Visibility,
    sprite::{ColorMaterial, MeshMaterial2d},
    transform::components::{GlobalTransform, Transform},
    ui::Val,
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
struct NearestLineMarker;

pub fn setup(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
) {
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::new(1.0, 1.0))),
        MeshMaterial2d(materials.add(Color::hsl(30., 0.95, 0.7))),
        NearestLineMarker,
        PickingBehavior::IGNORE,
    ));
}

#[derive(QueryData)]
#[query_data(mutable)]
struct NearestLine {
    transform: &'static mut Transform,
    visibility: &'static mut Visibility,

    _marker: &'static NearestLineMarker,
}

fn update(
    mut line: Single<NearestLine>,
    nearest: Option<Res<Nearest>>,
    cursor: Option<Res<Cursor>>,
    menu: Single<crate::ui::menu::Menu, Without<NearestLineMarker>>,
    camera: Single<(&GlobalTransform, &Camera), ()>,
) {
    let Some(nearest) = nearest else { return };

    let target = if *menu.visibility == Visibility::Hidden {
        let Some(cursor) = cursor else {
            *line.visibility = Visibility::Hidden;
            return;
        };
        cursor.world_position
    } else {
        let (global_transform, camera) = camera.into_inner();

        let (Val::Px(left), Val::Px(top)) = (menu.node.left, menu.node.top) else {
            return;
        };
        let Ok(position) = camera.viewport_to_world_2d(&global_transform, Vec2::new(left, top))
        else {
            return;
        };

        position
    };

    let delta = target - nearest.position;

    *line.visibility = Visibility::Visible;
    line.transform.rotation = Quat::from_rotation_z(delta.to_angle());
    line.transform.scale = Vec3::new(delta.length(), 1.0, 1.0);
    line.transform.translation = target.midpoint(nearest.position).extend(-0.5);
}
