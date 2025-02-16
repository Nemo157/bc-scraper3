use bevy::{
    color::Color,
    diagnostic::DiagnosticsStore,
    ecs::{
        component::Component,
        query::With,
        system::{Commands, Local, Res, Single},
    },
    hierarchy::{BuildChildren, ChildBuild},
    picking::PickingBehavior,
    text::TextFont,
    ui::widget::{Label, Text},
    ui::{BackgroundColor, Node, PositionType, Val},
};

#[derive(Default, Component)]
struct DiagnosticText;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(bevy::app::Startup, setup);
        app.add_systems(bevy::app::Update, update);
    }
}

fn setup(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(0.),
                bottom: Val::Px(0.),
                ..Node::default()
            },
            BackgroundColor(Color::srgba(0.10, 0.10, 0.10, 0.98)),
            PickingBehavior::IGNORE,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::default(),
                TextFont::default(),
                Label,
                DiagnosticText,
                PickingBehavior::IGNORE,
            ));
        });
}

fn update(
    diagnostics: Res<DiagnosticsStore>,
    mut text: Single<&mut Text, With<DiagnosticText>>,
    mut width: Local<usize>,
) {
    use std::fmt::Write;

    text.clear();

    let mut diagnostics = Vec::from_iter(diagnostics.iter());

    diagnostics.sort_by_key(|diagnostic| diagnostic.path().as_str());

    for diagnostic in &diagnostics {
        for (component, depth) in diagnostic.path().components().zip(0..) {
            *width = width.max(depth * 2 + component.len());
        }
    }

    let mut shown_components = Vec::new();
    for diagnostic in &diagnostics {
        let mut components = diagnostic.path().components().zip(0..).peekable();
        if let Some((mut component, mut depth)) = components.next() {
            while components.peek().is_some() {
                if shown_components.len() > depth {
                    if shown_components[depth] == component {
                        (component, depth) = components.next().unwrap();
                        continue;
                    }
                    shown_components.drain(depth..);
                }
                writeln!(***text, "{:depth$}{component}", "", depth = depth * 2).unwrap();
                shown_components.push(component);
                (component, depth) = components.next().unwrap();
            }
            shown_components.drain(depth..);

            let suffix = &diagnostic.suffix;
            if let Some(value) = diagnostic.smoothed() {
                if value.fract() == 0. {
                    writeln!(
                        ***text,
                        "{:depth$}{component:width$} {value:>5.0}   {suffix}",
                        "",
                        depth = depth * 2,
                        width = (*width - depth * 2)
                    )
                    .unwrap();
                } else {
                    writeln!(
                        ***text,
                        "{:depth$}{component:width$} {value:>8.2}{suffix}",
                        "",
                        depth = depth * 2,
                        width = (*width - depth * 2)
                    )
                    .unwrap();
                }
            } else {
                writeln!(
                    ***text,
                    "{:depth$}{component:width$}   ---.--{suffix}",
                    "",
                    depth = depth * 2,
                    width = (*width - depth * 2)
                )
                .unwrap();
            }

            shown_components.push(component);
        }
    }
}
