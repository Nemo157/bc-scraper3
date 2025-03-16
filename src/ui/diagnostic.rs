use bevy::{
    color::Color,
    diagnostic::{DiagnosticPath, DiagnosticsStore},
    ecs::{
        component::Component,
        entity::Entity,
        query::With,
        system::{Commands, Query, Res, Single},
    },
    hierarchy::{
        BuildChildren, ChildBuild, ChildBuilder, Children, DespawnRecursiveExt, HierarchyQueryExt,
    },
    picking::PickingBehavior,
    text::TextFont,
    ui::widget::{Label, Text},
    ui::{BackgroundColor, Display, GridPlacement, Node, PositionType, RepeatedGridTrack, Val},
};

use std::collections::BTreeMap;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(bevy::app::Startup, setup);
        app.add_systems(bevy::app::PreUpdate, pre_update);
        app.add_systems(bevy::app::Update, update);
    }
}

#[derive(Default, Component)]
struct DiagnosticLines;

#[derive(Component)]
struct DiagnosticLine {
    path: DiagnosticPath,
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Node {
            display: Display::Grid,
            grid_template_columns: RepeatedGridTrack::auto(2),
            grid_template_rows: RepeatedGridTrack::auto(1),
            position_type: PositionType::Absolute,
            right: Val::Px(0.),
            bottom: Val::Px(0.),
            ..Node::default()
        },
        BackgroundColor(Color::srgba(0.10, 0.10, 0.10, 0.98)),
        PickingBehavior::IGNORE,
        DiagnosticLines,
    ));
}

fn pre_update(
    diagnostics: Res<DiagnosticsStore>,
    parent: Single<Entity, With<DiagnosticLines>>,
    lines: Query<&DiagnosticLine>,
    children: Query<&Children>,
    mut commands: Commands,
) {
    let mut paths = Vec::from_iter(diagnostics.iter().map(|diagnostic| diagnostic.path()));
    paths.sort_by_key(|path| path.as_str());

    let mut lines = Vec::from_iter(
        children
            .children(*parent)
            .into_iter()
            .filter_map(|child| lines.get(*child).ok())
            .map(|line| &line.path),
    );
    lines.sort_by_key(|path| path.as_str());

    if paths == lines {
        return;
    }

    // This should barely ever be reached, so easier to just replace everything when it is.

    commands.entity(*parent).despawn_descendants();

    #[derive(Debug, Default)]
    struct PathNode<'a> {
        children: BTreeMap<&'a str, PathNode<'a>>,
        path: Option<&'a DiagnosticPath>,
    }

    impl PathNode<'_> {
        fn spawn_children(&self, parent: &mut ChildBuilder<'_>, depth: usize) {
            for (component, tree) in &self.children {
                let mut title = parent.spawn((
                    Text::new(format!("{:depth$}{component}", "")),
                    TextFont::default(),
                    Label,
                    PickingBehavior::IGNORE,
                ));

                if let Some(path) = tree.path {
                    parent.spawn((
                        Text::default(),
                        TextFont::default(),
                        Label,
                        PickingBehavior::IGNORE,
                        DiagnosticLine { path: path.clone() },
                    ));
                } else {
                    title.insert(Node {
                        grid_column: GridPlacement::span(2),
                        ..Node::default()
                    });
                }

                tree.spawn_children(parent, depth + 2);
            }
        }
    }

    let mut tree = PathNode::default();
    for path in &paths {
        let mut current = &mut tree;
        for component in path.components() {
            current = current.children.entry(component).or_default();
        }
        assert!(current.path.is_none());
        current.path = Some(path);
    }
    assert!(tree.path.is_none());

    commands
        .entity(*parent)
        .with_children(|parent| tree.spawn_children(parent, 0));
}

fn update(diagnostics: Res<DiagnosticsStore>, mut lines: Query<(&mut Text, &DiagnosticLine)>) {
    use std::fmt::Write;

    lines.par_iter_mut().for_each(|(mut text, line)| {
        let Some(diagnostic) = diagnostics.get(&line.path) else {
            tracing::warn!(
                "somehow tried to render non-existing diagnostic {}",
                line.path
            );
            return;
        };

        text.clear();
        let suffix = &diagnostic.suffix;
        if let Some(value) = diagnostic.smoothed() {
            if value.fract() == 0. && suffix.is_empty() {
                write!(&mut text, "{value:>5.0}   {suffix}").unwrap();
            } else {
                write!(&mut text, "{value:>8.2}{suffix}").unwrap();
            }
        } else {
            write!(&mut text, "---.--{suffix}").unwrap();
        }
    });
}
