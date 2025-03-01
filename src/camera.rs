use bevy::{
    core_pipeline::core_2d::Camera2d,
    ecs::{
        change_detection::DetectChangesMut,
        change_detection::{Res, ResMut},
        query::With,
        schedule::IntoSystemConfigs,
        system::{Commands, Resource, Single},
    },
    input::keyboard::KeyCode,
    input::{
        mouse::{AccumulatedMouseScroll, MouseButton, MouseScrollUnit},
        ButtonInput,
    },
    math::Vec2,
    render::camera::Camera,
    time::{Time, Virtual},
    transform::components::{GlobalTransform, Transform},
    window::{PrimaryWindow, Window},
};

#[derive(Default, Resource, PartialEq)]
pub struct Cursor {
    pub screen_delta: Vec2,
    pub screen_position: Vec2,
    pub world_position: Vec2,
}

pub struct CameraPlugin;

impl bevy::app::Plugin for CameraPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(bevy::app::Startup, setup).add_systems(
            bevy::app::PreUpdate,
            (update_cursor_position, drag, zoom).chain(),
        );
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn update_cursor_position(
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&GlobalTransform, &Camera), ()>,
    cursor: Option<ResMut<Cursor>>,
    mut commands: Commands,
) {
    let Some(screen_position) = window.cursor_position() else {
        commands.remove_resource::<Cursor>();
        return;
    };

    let (global_transform, camera) = camera.into_inner();

    let Ok(world_position) = camera.viewport_to_world_2d(&global_transform, screen_position) else {
        commands.remove_resource::<Cursor>();
        return;
    };

    if let Some(mut cursor) = cursor {
        cursor.set_if_neq(Cursor {
            // Can't use `AccumulatedMouseMotion` because that has truncation issues.
            screen_delta: cursor.screen_position - screen_position,
            screen_position,
            world_position,
        });
    } else {
        commands.insert_resource(Cursor {
            screen_delta: Vec2::ZERO,
            screen_position,
            world_position,
        });
    }
}

fn drag(
    button: Res<ButtonInput<MouseButton>>,
    cursor: Option<Res<Cursor>>,
    camera: Single<(&mut Transform, &mut GlobalTransform), With<Camera>>,
    dragged: Res<crate::interact::Dragged>,
) {
    let (mut transform, mut global_transform) = camera.into_inner();

    if dragged.0.is_some() {
        return;
    }

    let Some(cursor) = cursor else { return };

    if button.pressed(MouseButton::Left)
        && !button.just_pressed(MouseButton::Left)
        && cursor.screen_delta != Vec2::ZERO
    {
        let mut delta = cursor.screen_delta * transform.scale.x;
        delta.y *= -1.0;
        transform.translation += delta.extend(0.0);

        *global_transform = GlobalTransform::from(*transform)
    }
}

fn zoom(
    scroll: Res<AccumulatedMouseScroll>,
    keyboard: Res<ButtonInput<KeyCode>>,
    cursor: Option<Res<Cursor>>,
    camera: Single<(&mut Transform, &mut GlobalTransform), With<Camera>>,
    mut time: ResMut<Time<Virtual>>,
) {
    let (mut transform, mut global_transform) = camera.into_inner();

    if keyboard.pressed(KeyCode::ShiftLeft) {
        if scroll.unit == MouseScrollUnit::Line && scroll.delta.y != 0.0 {
            let new_value = time.relative_speed() + scroll.delta.y.signum() * 0.125;
            if new_value >= 0.0 {
                time.set_relative_speed(new_value);
            }
        }
        return;
    }

    let Some(cursor) = cursor else { return };

    let position = cursor.world_position.extend(0.0);

    // TODO: Handle trackpads nicely
    if scroll.unit == MouseScrollUnit::Line && scroll.delta.y != 0.0 {
        let zoom_ratio = if scroll.delta.y < 0.0 { 1.5 } else { 1.0 / 1.5 };
        transform.scale *= zoom_ratio;
        transform.translation = position + (transform.translation - position) * zoom_ratio;

        *global_transform = GlobalTransform::from(*transform)
    }
}
