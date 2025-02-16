use bevy::diagnostic::{Diagnostic, DiagnosticPath, RegisterDiagnostic};

pub const NODES: DiagnosticPath = DiagnosticPath::const_new("render/nodes");
pub const RELATIONS: DiagnosticPath = DiagnosticPath::const_new("render/relations");

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut bevy::app::App) {
        for path in [NODES, RELATIONS] {
            app.register_diagnostic(Diagnostic::new(path).with_suffix("ms"));
        }
    }
}
