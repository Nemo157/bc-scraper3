use bevy::{
    diagnostic::{Diagnostic, RegisterDiagnostic},
    ecs::{
        query::With,
        system::{Query, Res},
    },
};

use crate::diagnostic::Diagnostics;

use super::Paused;

pub mod update {
    use bevy::diagnostic::DiagnosticPath;

    pub const POSITIONS: DiagnosticPath = DiagnosticPath::const_new("sim/update/positions");
    pub const REPEL: DiagnosticPath = DiagnosticPath::const_new("sim/update/repel");
    pub const ATTRACT: DiagnosticPath = DiagnosticPath::const_new("sim/update/attract");
    pub const VELOCITIES: DiagnosticPath = DiagnosticPath::const_new("sim/update/velocities");
}

pub mod data {
    use bevy::diagnostic::DiagnosticPath;

    pub const NODES: DiagnosticPath = DiagnosticPath::const_new("sim/data/nodes");
    pub const RELATIONS: DiagnosticPath = DiagnosticPath::const_new("sim/data/relations");
}

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut bevy::app::App) {
        for path in [
            self::update::POSITIONS,
            self::update::REPEL,
            self::update::ATTRACT,
            self::update::VELOCITIES,
        ] {
            app.register_diagnostic(Diagnostic::new(path).with_suffix("ms"));
        }

        for path in [self::data::NODES, self::data::RELATIONS] {
            app.register_diagnostic(Diagnostic::new(path).with_smoothing_factor(0.));
        }

        app.add_systems(bevy::app::Update, update);
    }
}

fn update(
    mut diagnostics: Diagnostics,
    paused: Res<Paused>,
    nodes: Query<(), With<super::Position>>,
    relations: Query<(), With<super::Relationship>>,
) {
    diagnostics.add_measurement(&self::data::NODES, || nodes.iter().count() as f64);
    diagnostics.add_measurement(&self::data::RELATIONS, || relations.iter().count() as f64);

    if paused.0 {
        for path in [
            self::update::POSITIONS,
            self::update::REPEL,
            self::update::ATTRACT,
            self::update::VELOCITIES,
        ] {
            diagnostics.clear_history(&path);
        }
    }
}
