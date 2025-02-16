use bevy::{
    diagnostic::{Diagnostic, RegisterDiagnostic},
    ecs::{
        query::With,
        system::{Query, Res},
    },
};

use crate::diagnostic::Diagnostics;

use super::{Partitions, Paused};

pub mod update {
    use bevy::diagnostic::DiagnosticPath;

    pub mod repel {
        use bevy::diagnostic::DiagnosticPath;

        pub const PARTITIONS: DiagnosticPath =
            DiagnosticPath::const_new("sim/update/repel/partitions");
        pub const NEARBY: DiagnosticPath = DiagnosticPath::const_new("sim/update/repel/nearby");
        pub const DISTANT: DiagnosticPath = DiagnosticPath::const_new("sim/update/repel/distant");
    }

    pub const POSITIONS: DiagnosticPath = DiagnosticPath::const_new("sim/update/positions");
    pub const REPEL: DiagnosticPath = DiagnosticPath::const_new("sim/update/repel");
    pub const ATTRACT: DiagnosticPath = DiagnosticPath::const_new("sim/update/attract");
    pub const VELOCITIES: DiagnosticPath = DiagnosticPath::const_new("sim/update/velocities");
}

pub mod data {
    use bevy::diagnostic::DiagnosticPath;

    pub mod partitions {
        use bevy::diagnostic::DiagnosticPath;

        pub const MAX: DiagnosticPath = DiagnosticPath::const_new("sim/partitions/max");
        pub const MEAN: DiagnosticPath = DiagnosticPath::const_new("sim/partitions/mean");
        pub const MIN: DiagnosticPath = DiagnosticPath::const_new("sim/partitions/min");
    }

    pub const PARTITIONS: DiagnosticPath = DiagnosticPath::const_new("sim/partitions");
    pub const NODES: DiagnosticPath = DiagnosticPath::const_new("sim/data/nodes");
    pub const RELATIONS: DiagnosticPath = DiagnosticPath::const_new("sim/data/relations");
}

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut bevy::app::App) {
        for path in [
            self::update::POSITIONS,
            self::update::REPEL,
            self::update::repel::PARTITIONS,
            self::update::repel::NEARBY,
            self::update::repel::DISTANT,
            self::update::ATTRACT,
            self::update::VELOCITIES,
        ] {
            app.register_diagnostic(Diagnostic::new(path).with_suffix("ms"));
        }

        for path in [
            self::data::NODES,
            self::data::RELATIONS,
            self::data::PARTITIONS,
            self::data::partitions::MAX,
            self::data::partitions::MEAN,
            self::data::partitions::MIN,
        ] {
            app.register_diagnostic(Diagnostic::new(path).with_smoothing_factor(0.));
        }

        app.add_systems(bevy::app::Update, update);
    }
}

fn update(
    mut diagnostics: Diagnostics,
    paused: Res<Paused>,
    partitions: Res<Partitions>,
    nodes: Query<(), With<super::Position>>,
    relations: Query<(), With<super::Relationship>>,
) {
    diagnostics.add_measurement(&self::data::NODES, || nodes.iter().count() as f64);
    diagnostics.add_measurement(&self::data::RELATIONS, || relations.iter().count() as f64);
    diagnostics.add_measurement(&self::data::PARTITIONS, || partitions.0.len() as f64);
    diagnostics.add_measurement(&self::data::partitions::MAX, || {
        partitions
            .0
            .values()
            .map(|partition| partition.len())
            .max()
            .unwrap_or(0) as f64
    });
    diagnostics.add_measurement(&self::data::partitions::MEAN, || {
        if partitions.0.is_empty() {
            0.
        } else {
            partitions
                .0
                .values()
                .map(|partition| partition.len() as f64)
                .sum::<f64>()
                / (partitions.0.len() as f64)
        }
    });
    diagnostics.add_measurement(&self::data::partitions::MIN, || {
        partitions
            .0
            .values()
            .map(|partition| partition.len())
            .min()
            .unwrap_or(0) as f64
    });

    if paused.0 {
        for path in [
            self::update::POSITIONS,
            self::update::REPEL,
            self::update::repel::PARTITIONS,
            self::update::repel::NEARBY,
            self::update::repel::DISTANT,
            self::update::ATTRACT,
            self::update::VELOCITIES,
        ] {
            diagnostics.clear_history(&path);
        }
    }
}
