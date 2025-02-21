use bevy::{
    diagnostic::{Diagnostic, DiagnosticPath, Diagnostics, RegisterDiagnostic},
    ecs::{query::With, system::Query},
};

pub const ARTISTS: DiagnosticPath = DiagnosticPath::const_new("data/artists");
pub const RELEASES: DiagnosticPath = DiagnosticPath::const_new("data/releases");
pub const USERS: DiagnosticPath = DiagnosticPath::const_new("data/users");

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut bevy::app::App) {
        for path in [ARTISTS, RELEASES, USERS] {
            app.register_diagnostic(Diagnostic::new(path).with_smoothing_factor(0.));
        }
        app.add_systems(bevy::app::Update, update);
    }
}

fn update(
    mut diagnostics: Diagnostics,
    artists: Query<(), With<super::ArtistId>>,
    releases: Query<(), With<super::ReleaseId>>,
    users: Query<(), With<super::UserId>>,
) {
    diagnostics.add_measurement(&ARTISTS, || artists.iter().count() as f64);
    diagnostics.add_measurement(&RELEASES, || releases.iter().count() as f64);
    diagnostics.add_measurement(&USERS, || users.iter().count() as f64);
}
