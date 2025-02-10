use crate::data::{Album, Artist, User};
use crossbeam::channel::{Receiver, SendError, Sender, TryRecvError};
use std::cell::RefCell;
use url::Url;

mod scrape;
mod web;

#[derive(Debug)]
pub enum Request {
    User { url: String },
    Album { url: String },
    Artist { url: String },
}

#[derive(Debug)]
pub enum Response {
    User(User),
    Album(Album),
    Artist(Artist),
    Fans(Album, Vec<User>),
    AlbumArtist(Album, Artist),
    Collection(User, Vec<Album>),
    Releases(Artist, Vec<Album>),
}

#[derive(Debug, bevy::ecs::system::Resource)]
pub struct Thread {
    thread: Option<std::thread::JoinHandle<()>>,
    to_scrape_tx: Option<Sender<Request>>,
    scraped_rx: Option<Receiver<Response>>,
}

impl Thread {
    #[culpa::try_fn]
    pub fn spawn() -> eyre::Result<Self> {
        let (to_scrape_tx, to_scrape_rx) = crossbeam::channel::unbounded();
        let (scraped_tx, scraped_rx) = crossbeam::channel::bounded(1);
        let background = Background::new(to_scrape_rx, scraped_tx)?;
        let thread = Some(std::thread::spawn(move || background.run()));
        Thread {
            thread,
            to_scrape_tx: Some(to_scrape_tx),
            scraped_rx: Some(scraped_rx),
        }
    }

    #[culpa::try_fn]
    pub fn send(&self, request: Request) -> eyre::Result<()> {
        self.to_scrape_tx.as_ref().unwrap().send(request)?;
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
    to_scrape: Receiver<Request>,
    scraped: Sender<Response>,
}

impl Background {
    #[culpa::try_fn]
    fn new(to_scrape: Receiver<Request>, scraped: Sender<Response>) -> eyre::Result<Self> {
        let scraper = self::scrape::Scraper::new(self::web::Client::new()?);
        Self {
            scraper,
            to_scrape,
            scraped,
        }
    }

    fn run(&self) {
        for request in &self.to_scrape {
            if let Err(error) = self.handle_request(request) {
                if error.is::<SendError<Response>>() {
                    tracing::info!("background thread shutdown while still processing an item");
                    return;
                }
                tracing::error!(?error, "failed handling scrape request");
            }
        }
    }

    #[culpa::try_fn]
    #[tracing::instrument(skip(self))]
    fn handle_request(&self, request: Request) -> eyre::Result<()> {
        match request {
            Request::User { url } => {
                let user = RefCell::new(None);
                self.scraper.scrape_fan(
                    &Url::parse(&url)?,
                    |fan| {
                        user.replace(Some(fan));
                        Ok(())
                    },
                    |collection| {
                        self.scraped.send(Response::Collection(
                            user.borrow().clone().unwrap(),
                            collection,
                        ))?;
                        Ok(())
                    },
                )?;
                self.scraped
                    .send(Response::User(user.replace(None).take().unwrap()))?;
            }
            Request::Album { url } => {
                let album = RefCell::new(None);
                self.scraper.scrape_album(
                    &Url::parse(&url)?,
                    |new_album| {
                        album.replace(Some(new_album));
                        Ok(())
                    },
                    |artist| {
                        self.scraped.send(Response::AlbumArtist(
                            album.borrow().clone().unwrap(),
                            artist,
                        ))?;
                        Ok(())
                    },
                    |fans| {
                        self.scraped
                            .send(Response::Fans(album.borrow().clone().unwrap(), fans))?;
                        Ok(())
                    },
                )?;
                self.scraped
                    .send(Response::Album(album.replace(None).take().unwrap()))?;
            }
            Request::Artist { url } => {
                let artist = RefCell::new(None);
                self.scraper.scrape_artist(
                    &Url::parse(&url)?,
                    |new_artist| {
                        artist.replace(Some(new_artist));
                        Ok(())
                    },
                    |albums| {
                        self.scraped
                            .send(Response::Releases(artist.borrow().clone().unwrap(), albums))?;
                        Ok(())
                    },
                )?;
                self.scraped
                    .send(Response::Artist(artist.replace(None).take().unwrap()))?;
            }
        }
    }
}
