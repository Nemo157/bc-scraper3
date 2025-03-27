use super::super::web::Client;
use super::super::{Request, Response, Stats};
use super::scraper::Scraper;
use crossbeam::channel::{Receiver, SendError, Sender};
use std::{
    cell::RefCell,
    sync::{atomic::Ordering, Arc},
};
use url::Url;

pub fn run(
    client: Client,
    stats: Arc<Stats>,
    to_scrape: Receiver<Request>,
    scraped: Sender<Response>,
) -> std::thread::JoinHandle<()> {
    let scraper = Scraper::new(client);

    std::thread::spawn(move || {
        for request in &to_scrape {
            stats.items_queued.fetch_sub(1, Ordering::Relaxed);
            stats.items_processing.store(true, Ordering::Relaxed);
            if let Err(error) = handle_request(&scraper, request, &scraped) {
                if error.is::<SendError<Response>>() {
                    tracing::info!("scraper thread shutdown while still processing an item");
                    return;
                }
                tracing::error!(?error, "failed handling scrape request");
            }
            stats.items_processing.store(false, Ordering::Relaxed);
            stats.items_completed.fetch_add(1, Ordering::Relaxed);
        }
    })
}

#[culpa::try_fn]
#[tracing::instrument(skip(scraper, scraped))]
fn handle_request(
    scraper: &Scraper,
    request: Request,
    scraped: &Sender<Response>,
) -> eyre::Result<()> {
    match request {
        Request::Artist { url } => {
            let artist = RefCell::new(None);
            scraper.scrape_artist(
                &Url::parse(&url)?,
                |new_artist, details| {
                    artist.replace(Some((new_artist, details)));
                    Ok(())
                },
                |releases| {
                    scraped.send(Response::Releases(
                        artist.borrow().as_ref().unwrap().0.clone(),
                        releases,
                    ))?;
                    Ok(())
                },
            )?;
            let (artist, details) = artist.replace(None).take().unwrap();
            scraped.send(Response::Artist(artist, details))?;
        }

        Request::Release { url } => {
            let release = RefCell::new(None);
            scraper.scrape_release(
                &Url::parse(&url)?,
                |new_release, details| {
                    release.replace(Some((new_release, details)));
                    Ok(())
                },
                |artist| {
                    scraped.send(Response::ReleaseArtist(
                        release.borrow().as_ref().unwrap().0.clone(),
                        artist,
                    ))?;
                    Ok(())
                },
                |fans| {
                    scraped.send(Response::Fans(
                        release.borrow().as_ref().unwrap().0.clone(),
                        fans,
                    ))?;
                    Ok(())
                },
            )?;
            let (release, details) = release.replace(None).take().unwrap();
            scraped.send(Response::Release(release, details))?;
        }

        Request::User { url } => {
            let user = RefCell::new(None);
            scraper.scrape_fan(
                &Url::parse(&url)?,
                |fan, details| {
                    user.replace(Some((fan, details)));
                    Ok(())
                },
                |collection| {
                    scraped.send(Response::Collection(
                        user.borrow().as_ref().unwrap().0.clone(),
                        collection,
                    ))?;
                    Ok(())
                },
            )?;
            let (user, details) = user.replace(None).take().unwrap();
            scraped.send(Response::User(user, details))?;
        }
    }
}
