use bevy::{
    diagnostic::{Diagnostic, RegisterDiagnostic},
    ecs::system::Res,
};

use std::sync::atomic::Ordering;

use crate::diagnostic::Diagnostics;

pub mod items {
    use bevy::diagnostic::DiagnosticPath;

    pub const COMPLETED: DiagnosticPath = DiagnosticPath::const_new("scraper/items/completed");
    pub const PROCESSING: DiagnosticPath = DiagnosticPath::const_new("scraper/items/processing");
    pub const QUEUED: DiagnosticPath = DiagnosticPath::const_new("scraper/items/queued");
}

pub mod web {
    use bevy::diagnostic::DiagnosticPath;

    pub mod cache {
        use bevy::diagnostic::DiagnosticPath;

        pub const HITS: DiagnosticPath = DiagnosticPath::const_new("scraper/web/cache/hits");
        pub const MISSES: DiagnosticPath = DiagnosticPath::const_new("scraper/web/cache/misses");
    }

    pub const REQUESTS: DiagnosticPath = DiagnosticPath::const_new("scraper/web/requests");
}

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut bevy::app::App) {
        for path in [
            self::items::COMPLETED,
            self::items::PROCESSING,
            self::items::QUEUED,
            self::web::REQUESTS,
            self::web::cache::HITS,
            self::web::cache::MISSES,
        ] {
            app.register_diagnostic(Diagnostic::new(path).with_smoothing_factor(0.));
        }

        app.add_systems(bevy::app::Update, update);
    }
}

fn update(mut diagnostics: Diagnostics, scraper: Res<super::Thread>) {
    diagnostics.add_measurement(&self::items::COMPLETED, || {
        scraper.stats.items_completed.load(Ordering::Relaxed) as f64
    });
    diagnostics.add_measurement(&self::items::PROCESSING, || {
        scraper.stats.items_processing.load(Ordering::Relaxed) as usize as f64
    });
    diagnostics.add_measurement(&self::items::QUEUED, || {
        scraper.stats.items_queued.load(Ordering::Relaxed) as f64
    });
    diagnostics.add_measurement(&self::web::REQUESTS, || {
        scraper.stats.web_requests.load(Ordering::Relaxed) as f64
    });
    diagnostics.add_measurement(&self::web::cache::HITS, || {
        scraper.stats.web_cache_hits.load(Ordering::Relaxed) as f64
    });
    diagnostics.add_measurement(&self::web::cache::MISSES, || {
        scraper.stats.web_cache_misses.load(Ordering::Relaxed) as f64
    });
}
