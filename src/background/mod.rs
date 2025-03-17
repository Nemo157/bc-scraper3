use crate::data::{Artist, ArtistDetails, Release, ReleaseDetails, User, UserDetails};
use crossbeam::channel::{Receiver, SendError, Sender, TryRecvError};
use std::{
    cell::RefCell,
    collections::HashSet,
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
use url::Url;

pub mod diagnostic;
mod scrape;
mod web;

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub enum Request {
    Artist { url: String },
    Release { url: String },
    User { url: String },
}

#[derive(Debug)]
pub enum Response {
    Artist(Artist, ArtistDetails),
    Release(Release, ReleaseDetails),
    User(User, UserDetails),

    Fans(Release, Vec<User>),
    ReleaseArtist(Release, Artist),
    Collection(User, Vec<Release>),
    Releases(Artist, Vec<Release>),
}

#[derive(Debug, Default)]
struct Stats {
    items_duplicate: AtomicUsize,
    items_queued: AtomicUsize,
    items_processing: AtomicBool,
    items_completed: AtomicUsize,

    web_requests: AtomicUsize,
    web_cache_misses: AtomicUsize,
    web_cache_hits: AtomicUsize,
}

#[derive(Debug, bevy::ecs::system::Resource)]
pub struct Thread {
    thread: Option<std::thread::JoinHandle<()>>,
    stats: Arc<Stats>,
    done: Mutex<HashSet<Request>>,
    to_scrape_tx: Option<Sender<Request>>,
    scraped_rx: Option<Receiver<Response>>,
}

impl Thread {
    #[culpa::try_fn]
    pub fn spawn(cache_dir: &Path) -> eyre::Result<Self> {
        let stats = Arc::new(Stats::default());
        let client = self::web::Client::new(cache_dir, stats.clone())?;
        let (to_scrape_tx, to_scrape_rx) = crossbeam::channel::unbounded();
        let (scraped_tx, scraped_rx) = crossbeam::channel::bounded(1);
        let background = Background::new(client, stats.clone(), to_scrape_rx, scraped_tx)?;
        let thread = Some(std::thread::spawn(move || background.run()));
        Thread {
            thread,
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

impl Drop for Thread {
    fn drop(&mut self) {
        self.to_scrape_tx.take();
        self.scraped_rx.take();
        if let Err(e) = self.thread.take().unwrap().join() {
            std::panic::resume_unwind(e);
        }
    }
}

#[derive(Debug)]
struct Background {
    scraper: self::scrape::Scraper,
    stats: Arc<Stats>,
    to_scrape: Receiver<Request>,
    scraped: Sender<Response>,
}

impl Background {
    #[culpa::try_fn]
    fn new(
        client: self::web::Client,
        stats: Arc<Stats>,
        to_scrape: Receiver<Request>,
        scraped: Sender<Response>,
    ) -> eyre::Result<Self> {
        Self {
            scraper: self::scrape::Scraper::new(client),
            stats,
            to_scrape,
            scraped,
        }
    }

    fn run(&self) {
        for request in &self.to_scrape {
            self.stats.items_queued.fetch_sub(1, Ordering::Relaxed);
            self.stats.items_processing.store(true, Ordering::Relaxed);
            if let Err(error) = self.handle_request(request) {
                if error.is::<SendError<Response>>() {
                    tracing::info!("background thread shutdown while still processing an item");
                    return;
                }
                tracing::error!(?error, "failed handling scrape request");
            }
            self.stats.items_processing.store(false, Ordering::Relaxed);
            self.stats.items_completed.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[culpa::try_fn]
    #[tracing::instrument(skip(self))]
    fn handle_request(&self, request: Request) -> eyre::Result<()> {
        match request {
            Request::Artist { url } => {
                let artist = RefCell::new(None);
                self.scraper.scrape_artist(
                    &Url::parse(&url)?,
                    |new_artist, details| {
                        artist.replace(Some((new_artist, details)));
                        Ok(())
                    },
                    |releases| {
                        self.scraped.send(Response::Releases(
                            artist.borrow().as_ref().unwrap().0.clone(),
                            releases,
                        ))?;
                        Ok(())
                    },
                )?;
                let (artist, details) = artist.replace(None).take().unwrap();
                self.scraped.send(Response::Artist(artist, details))?;
            }

            Request::Release { url } => {
                let release = RefCell::new(None);
                self.scraper.scrape_release(
                    &Url::parse(&url)?,
                    |new_release, details| {
                        release.replace(Some((new_release, details)));
                        Ok(())
                    },
                    |artist| {
                        self.scraped.send(Response::ReleaseArtist(
                            release.borrow().as_ref().unwrap().0.clone(),
                            artist,
                        ))?;
                        Ok(())
                    },
                    |fans| {
                        self.scraped.send(Response::Fans(
                            release.borrow().as_ref().unwrap().0.clone(),
                            fans,
                        ))?;
                        Ok(())
                    },
                )?;
                let (release, details) = release.replace(None).take().unwrap();
                self.scraped.send(Response::Release(release, details))?;
            }

            Request::User { url } => {
                let user = RefCell::new(None);
                self.scraper.scrape_fan(
                    &Url::parse(&url)?,
                    |fan, details| {
                        user.replace(Some((fan, details)));
                        Ok(())
                    },
                    |collection| {
                        self.scraped.send(Response::Collection(
                            user.borrow().as_ref().unwrap().0.clone(),
                            collection,
                        ))?;
                        Ok(())
                    },
                )?;
                let (user, details) = user.replace(None).take().unwrap();
                self.scraped.send(Response::User(user, details))?;
            }
        }
    }
}
