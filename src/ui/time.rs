use bevy::{
    color::Color,
    ecs::{
        component::Component,
        query::With,
        system::{Commands, Res, Single},
    },
    picking::PickingBehavior,
    text::TextFont,
    time::{Time, Virtual},
    ui::widget::{Label, Text},
    ui::{BackgroundColor, Node, PositionType, Val},
};

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(bevy::app::Startup, setup);
        app.add_systems(bevy::app::Update, update);
    }
}

#[derive(Default, Component)]
struct TimeText;

fn setup(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(0.),
            top: Val::Px(0.),
            ..Node::default()
        },
        BackgroundColor(Color::srgba(0.10, 0.10, 0.10, 0.98)),
        Text::default(),
        TextFont::default(),
        Label,
        TimeText,
        PickingBehavior::IGNORE,
    ));
}

fn update(time: Res<Time<Virtual>>, mut text: Single<&mut Text, With<TimeText>>) {
    use std::fmt::Write;

    text.clear();
    write!(&mut text, "speed: {}", time.relative_speed()).unwrap();
}
