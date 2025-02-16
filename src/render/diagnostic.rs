use bevy::{
    diagnostic::{Diagnostic, DiagnosticPath, RegisterDiagnostic},
    ecs::{
        query::With,
        system::{Res, Single},
    },
    render::view::Visibility,
};

use crate::{diagnostic::Diagnostics, sim::Paused, RelationshipParent};

pub const NODES: DiagnosticPath = DiagnosticPath::const_new("render/nodes");
pub const RELATIONS: DiagnosticPath = DiagnosticPath::const_new("render/relations");

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut bevy::app::App) {
        for path in [NODES, RELATIONS] {
            app.register_diagnostic(Diagnostic::new(path).with_suffix("ms"));
        }

        app.add_systems(bevy::app::Update, update);
    }
}

fn update(
    mut diagnostics: Diagnostics,
    paused: Res<Paused>,
    relationship_parent: Single<&Visibility, With<RelationshipParent>>,
) {
    if paused.0 {
        diagnostics.clear_history(&NODES);
    }

    if *relationship_parent == Visibility::Hidden || paused.0 {
        diagnostics.clear_history(&RELATIONS);
    }
}
