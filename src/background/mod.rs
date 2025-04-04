use crossbeam::channel::{Receiver, Sender, TryRecvError};
use std::{
    collections::HashSet,
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

pub mod diagnostic;
mod scraper;
mod web;

pub use scraper::{Request, Response};

#[derive(Debug, Default)]
struct Stats {
    items_duplicate: AtomicUsize,
    items_queued: AtomicUsize,
    items_processing: AtomicUsize,
    items_completed: AtomicUsize,

    web_requests: AtomicUsize,
    web_cache_misses: AtomicUsize,
    web_cache_hits: AtomicUsize,
}

#[derive(Debug, bevy::ecs::system::Resource)]
pub struct Scraper {
    threads: Vec<std::thread::JoinHandle<()>>,
    stats: Arc<Stats>,
    done: Mutex<HashSet<Request>>,
    to_scrape_tx: Option<Sender<Request>>,
    scraped_rx: Option<Receiver<Response>>,
}

impl Scraper {
    #[culpa::try_fn]
    pub fn new(cache_dir: &Path) -> eyre::Result<Self> {
        let stats = Arc::new(Stats::default());

        let (to_scrape_tx, to_scrape_rx) = crossbeam::channel::unbounded();
        let (scraped_tx, scraped_rx) = crossbeam::channel::bounded(8);
        let (web_tx, web_rx) = crossbeam::channel::bounded(1);
        let (web_cache_tx, web_cache_rx) = crossbeam::channel::bounded(1);

        let threads = vec![
            self::web::client::run(web_rx)?,
            self::web::cache::run(
                cache_dir,
                stats.clone(),
                web_cache_rx.clone(),
                web_tx.clone(),
            )?,
            self::web::cache::run(
                cache_dir,
                stats.clone(),
                web_cache_rx.clone(),
                web_tx.clone(),
            )?,
            self::web::cache::run(
                cache_dir,
                stats.clone(),
                web_cache_rx.clone(),
                web_tx.clone(),
            )?,
            self::web::cache::run(
                cache_dir,
                stats.clone(),
                web_cache_rx.clone(),
                web_tx.clone(),
            )?,
            self::scraper::thread::run(
                web_cache_tx.clone(),
                stats.clone(),
                to_scrape_rx.clone(),
                scraped_tx.clone(),
            )?,
            self::scraper::thread::run(
                web_cache_tx.clone(),
                stats.clone(),
                to_scrape_rx.clone(),
                scraped_tx.clone(),
            )?,
            self::scraper::thread::run(
                web_cache_tx.clone(),
                stats.clone(),
                to_scrape_rx.clone(),
                scraped_tx.clone(),
            )?,
            self::scraper::thread::run(
                web_cache_tx.clone(),
                stats.clone(),
                to_scrape_rx.clone(),
                scraped_tx.clone(),
            )?,
            self::scraper::thread::run(
                web_cache_tx.clone(),
                stats.clone(),
                to_scrape_rx.clone(),
                scraped_tx.clone(),
            )?,
            self::scraper::thread::run(
                web_cache_tx.clone(),
                stats.clone(),
                to_scrape_rx.clone(),
                scraped_tx.clone(),
            )?,
            self::scraper::thread::run(
                web_cache_tx.clone(),
                stats.clone(),
                to_scrape_rx.clone(),
                scraped_tx.clone(),
            )?,
            self::scraper::thread::run(
                web_cache_tx.clone(),
                stats.clone(),
                to_scrape_rx.clone(),
                scraped_tx.clone(),
            )?,
        ];

        Scraper {
            threads,
            stats,
            done: Mutex::new(HashSet::new()),
            to_scrape_tx: Some(to_scrape_tx),
            scraped_rx: Some(scraped_rx),
        }
    }

    #[culpa::try_fn]
    pub fn send(&self, request: Request) -> eyre::Result<()> {
        if self.done.lock().unwrap().insert(request.clone()) {
            self.stats.items_queued.fetch_add(1, Ordering::Relaxed);
            self.to_scrape_tx.as_ref().unwrap().send(request)?;
        } else {
            self.stats.items_duplicate.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[culpa::try_fn]
    pub fn try_recv(&self) -> eyre::Result<Option<Response>> {
        match self.scraped_rx.as_ref().unwrap().try_recv() {
            Ok(response) => Some(response),
            Err(TryRecvError::Empty) => None,
            Err(err) => Err(err)?,
        }
    }
}

impl Drop for Scraper {
    fn drop(&mut self) {
        self.to_scrape_tx.take();
        self.scraped_rx.take();
        for thread in self.threads.drain(..) {
            if let Err(e) = thread.join() {
                std::panic::resume_unwind(e);
            }
        }
    }
}
