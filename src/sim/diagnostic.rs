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

    pub mod acceleration {
        use bevy::diagnostic::DiagnosticPath;

        pub const MAX: DiagnosticPath = DiagnosticPath::const_new("sim/acceleration/max");
        pub const MEAN: DiagnosticPath = DiagnosticPath::const_new("sim/acceleration/mean");
        pub const MIN: DiagnosticPath = DiagnosticPath::const_new("sim/acceleration/min");
    }

    pub mod partitions {
        use bevy::diagnostic::DiagnosticPath;

        pub const MAX: DiagnosticPath = DiagnosticPath::const_new("sim/partitions/max");
        pub const MEAN: DiagnosticPath = DiagnosticPath::const_new("sim/partitions/mean");
        pub const MIN: DiagnosticPath = DiagnosticPath::const_new("sim/partitions/min");
    }

    pub mod position {
        use bevy::diagnostic::DiagnosticPath;

        pub const MAX: DiagnosticPath = DiagnosticPath::const_new("sim/position/max");
        pub const MEAN: DiagnosticPath = DiagnosticPath::const_new("sim/position/mean");
        pub const MIN: DiagnosticPath = DiagnosticPath::const_new("sim/position/min");
    }

    pub mod velocity {
        use bevy::diagnostic::DiagnosticPath;

        pub const MAX: DiagnosticPath = DiagnosticPath::const_new("sim/velocity/max");
        pub const MEAN: DiagnosticPath = DiagnosticPath::const_new("sim/velocity/mean");
        pub const MIN: DiagnosticPath = DiagnosticPath::const_new("sim/velocity/min");
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
            self::update::ATTRACT,
            self::update::VELOCITIES,
        ] {
            app.register_diagnostic(Diagnostic::new(path).with_suffix("ms"));
        }

        for path in [self::update::repel::NEARBY, self::update::repel::DISTANT] {
            app.register_diagnostic(Diagnostic::new(path).with_suffix("ms*"));
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

        for path in [
            self::data::acceleration::MAX,
            self::data::acceleration::MEAN,
            self::data::acceleration::MIN,
            self::data::position::MAX,
            self::data::position::MEAN,
            self::data::position::MIN,
            self::data::velocity::MAX,
            self::data::velocity::MEAN,
            self::data::velocity::MIN,
        ] {
            app.register_diagnostic(Diagnostic::new(path));
        }

        app.add_systems(bevy::app::Update, update);
    }
}

fn update(
    mut diagnostics: Diagnostics,
    paused: Res<Paused>,
    partitions: Res<Partitions>,
    nodes: Query<(&super::Position, &super::Velocity, &super::Acceleration)>,
    relations: Query<(), With<super::Relationship>>,
) {
    let (
        node_count,
        (pos_min, pos_sum, pos_max),
        (vel_min, vel_sum, vel_max),
        (acc_min, acc_sum, acc_max),
    ) = nodes.iter().fold(
        (
            0,
            (f32::INFINITY, 0., f32::NEG_INFINITY),
            (f32::INFINITY, 0., f32::NEG_INFINITY),
            (f32::INFINITY, 0., f32::NEG_INFINITY),
        ),
        |(
            node_count,
            (pos_min, pos_sum, pos_max),
            (vel_min, vel_sum, vel_max),
            (acc_min, acc_sum, acc_max),
        ),
         (pos, vel, acc)| {
            let (pos, vel, acc) = (pos.0.length(), vel.0.length(), acc.0.length());
            (
                node_count + 1,
                (pos_min.min(pos), pos_sum + pos, pos_max.max(pos)),
                (vel_min.min(vel), vel_sum + vel, vel_max.max(vel)),
                (acc_min.min(acc), acc_sum + acc, acc_max.max(acc)),
            )
        },
    );

    diagnostics.add_measurement(&self::data::NODES, || node_count as f64);
    if pos_min != f32::INFINITY {
        diagnostics.add_measurement(&self::data::position::MIN, || pos_min as f64);
    }
    diagnostics.add_measurement(&self::data::position::MEAN, || {
        pos_sum as f64 / node_count as f64
    });
    if pos_max != f32::NEG_INFINITY {
        diagnostics.add_measurement(&self::data::position::MAX, || pos_max as f64);
    }
    if vel_min != f32::INFINITY {
        diagnostics.add_measurement(&self::data::velocity::MIN, || vel_min as f64);
    }
    diagnostics.add_measurement(&self::data::velocity::MEAN, || {
        vel_sum as f64 / node_count as f64
    });
    if vel_max != f32::NEG_INFINITY {
        diagnostics.add_measurement(&self::data::velocity::MAX, || vel_max as f64);
    }
    if acc_min != f32::INFINITY {
        diagnostics.add_measurement(&self::data::acceleration::MIN, || acc_min as f64);
    }
    diagnostics.add_measurement(&self::data::acceleration::MEAN, || {
        acc_sum as f64 / node_count as f64
    });
    if acc_max != f32::NEG_INFINITY {
        diagnostics.add_measurement(&self::data::acceleration::MAX, || acc_max as f64);
    }
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
            node_count as f64 / (partitions.0.len() as f64)
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
