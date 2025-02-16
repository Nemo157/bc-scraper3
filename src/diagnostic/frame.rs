use bevy::{
    core::FrameCount,
    diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, RegisterDiagnostic},
    ecs::system::Res,
    time::{Real, Time},
};

/// [`bevy::diagnostic::FrameTimeDiagnosticsPlugin`] but with hierarchical paths
pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.register_diagnostic(Diagnostic::new(Self::FRAME_TIME).with_suffix("ms"))
            .register_diagnostic(Diagnostic::new(Self::FPS).with_suffix("Hz"))
            .register_diagnostic(Diagnostic::new(Self::FRAME_COUNT).with_smoothing_factor(0.0))
            .add_systems(bevy::app::Update, Self::diagnostic_system);
    }
}

impl Plugin {
    pub const FPS: DiagnosticPath = DiagnosticPath::const_new("frame/rate");
    pub const FRAME_COUNT: DiagnosticPath = DiagnosticPath::const_new("frame/count");
    pub const FRAME_TIME: DiagnosticPath = DiagnosticPath::const_new("frame/time");

    pub fn diagnostic_system(
        mut diagnostics: Diagnostics,
        time: Res<Time<Real>>,
        frame_count: Res<FrameCount>,
    ) {
        diagnostics.add_measurement(&Self::FRAME_COUNT, || frame_count.0 as f64);

        let delta_seconds = time.delta_secs_f64();
        if delta_seconds == 0.0 {
            return;
        }

        diagnostics.add_measurement(&Self::FRAME_TIME, || delta_seconds * 1000.0);

        diagnostics.add_measurement(&Self::FPS, || 1.0 / delta_seconds);
    }
}
