use bevy::{
    diagnostic::{DiagnosticMeasurement, DiagnosticPath, DiagnosticsStore},
    utils::PassHash,
};

use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

/// [`bevy::diagnostic::Diagnostics`] but with history clearing
#[derive(bevy::ecs::system::SystemParam)]
pub struct Diagnostics<'w, 's> {
    store: bevy::ecs::system::Res<'w, DiagnosticsStore>,
    queue: bevy::ecs::system::Deferred<'s, DiagnosticsBuffer>,
}

impl<'w, 's> Diagnostics<'w, 's> {
    pub fn add_measurement(&mut self, path: &DiagnosticPath, value: impl FnOnce() -> f64) {
        if self
            .store
            .get(path)
            .filter(|diagnostic| diagnostic.is_enabled)
            .is_some()
        {
            if !self.queue.cleared.contains(path) {
                self.queue.additions.insert(
                    path.clone(),
                    DiagnosticMeasurement {
                        time: Instant::now(),
                        value: value(),
                    },
                );
            }
        }
    }

    pub fn clear_history(&mut self, path: &DiagnosticPath) {
        if self.store.get(path).is_some() {
            self.queue.cleared.insert(path.clone());
            self.queue.additions.remove(path);
        }
    }
}

#[derive(Default)]
struct DiagnosticsBuffer {
    additions: HashMap<DiagnosticPath, DiagnosticMeasurement, PassHash>,
    cleared: HashSet<DiagnosticPath, PassHash>,
}

impl bevy::ecs::system::SystemBuffer for DiagnosticsBuffer {
    fn apply(
        &mut self,
        _system_meta: &bevy::ecs::system::SystemMeta,
        world: &mut bevy::ecs::world::World,
    ) {
        let mut diagnostics = world.resource_mut::<DiagnosticsStore>();
        for (path, measurement) in self.additions.drain() {
            if let Some(diagnostic) = diagnostics.get_mut(&path) {
                diagnostic.add_measurement(measurement);
            }
        }
        for path in self.cleared.drain() {
            if let Some(diagnostic) = diagnostics.get_mut(&path) {
                diagnostic.clear_history();
            }
        }
    }
}
