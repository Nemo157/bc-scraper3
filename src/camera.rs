use bevy::{
    app::{App, Plugin, PreUpdate, Startup, Update},
    core_pipeline::core_2d::Camera2d,
    ecs::{
        change_detection::{Res, ResMut},
        query::With,
        system::{Commands, Query, Resource, Single},
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

#[derive(Default, Resource)]
struct Cursor {
    position: Vec2,
    delta: Vec2,
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(PreUpdate, update_cursor_position)
            .add_systems(Update, drag)
            .add_systems(Update, zoom);
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.insert_resource(Cursor::default());
}

fn update_cursor_position(
    mut cursor: ResMut<Cursor>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    let Some(position) = window.cursor_position() else {
        return;
    };

    // Can't use `AccumulatedMouseMotion` because that has truncation issues.
    cursor.delta = cursor.position - position;
    cursor.position = position;
}

fn drag(
    button: Res<ButtonInput<MouseButton>>,
    cursor: Res<Cursor>,
    mut transform: Single<&mut Transform, With<Camera>>,
    dragged: Query<(), With<crate::ui::Dragged>>,
) {
    if !dragged.is_empty() {
        return;
    }

    if button.pressed(MouseButton::Left) && !button.just_pressed(MouseButton::Left) {
        let mut delta = cursor.delta * transform.scale.x;
        delta.y *= -1.0;
        transform.translation += delta.extend(0.0);
    }
}

fn zoom(
    scroll: Res<AccumulatedMouseScroll>,
    keyboard: Res<ButtonInput<KeyCode>>,
    cursor: Res<Cursor>,
    camera: Single<(&mut Transform, &GlobalTransform, &Camera), ()>,
    mut time: ResMut<Time<Virtual>>,
) {
    if keyboard.pressed(KeyCode::ShiftLeft) {
        if scroll.unit == MouseScrollUnit::Line && scroll.delta.y != 0.0 {
            let new_value = time.relative_speed() + scroll.delta.y.signum() * 0.125;
            if new_value >= 0.0 {
                time.set_relative_speed(new_value);
            }
        }
        return;
    }

    let (mut transform, global_transform, camera) = camera.into_inner();

    let Ok(position) = camera.viewport_to_world_2d(&global_transform, cursor.position) else {
        return;
    };
    let position = position.extend(0.0);

    // TODO: Handle trackpads nicely
    if scroll.unit == MouseScrollUnit::Line && scroll.delta.y != 0.0 {
        let zoom_ratio = if scroll.delta.y < 0.0 { 1.5 } else { 1.0 / 1.5 };
        transform.scale *= zoom_ratio;
        transform.translation = position + (transform.translation - position) * zoom_ratio;
    }
}
