use super::super::{scraper, web, Stats};
use super::scraper::Scraper;
use crossbeam::channel::{Receiver, SendError, Sender};
use std::{
    cell::RefCell,
    sync::{atomic::Ordering, Arc},
};
use url::Url;

#[culpa::try_fn]
pub fn run(
    web: Sender<web::Request>,
    stats: Arc<Stats>,
    to_scrape: Receiver<scraper::Request>,
    scraped: Sender<scraper::Response>,
) -> eyre::Result<std::thread::JoinHandle<()>> {
    let scraper = Scraper::new(web);

    std::thread::Builder::new()
        .name("scraper".to_owned())
        .spawn(move || {
            for request in &to_scrape {
                stats.items_queued.fetch_sub(1, Ordering::Relaxed);
                stats.items_processing.fetch_add(1, Ordering::Relaxed);
                if let Err(error) = handle_request(&scraper, request, &scraped) {
                    if error.is::<SendError<scraper::Response>>() {
                        tracing::info!("scraper thread shutdown while still processing an item");
                        return;
                    }
                    tracing::error!(?error, "failed handling scrape request");
                }
                stats.items_processing.fetch_sub(1, Ordering::Relaxed);
                stats.items_completed.fetch_add(1, Ordering::Relaxed);
            }
        })?
}

#[culpa::try_fn]
#[tracing::instrument(skip(scraper, scraped))]
fn handle_request(
    scraper: &Scraper,
    request: scraper::Request,
    scraped: &Sender<scraper::Response>,
) -> eyre::Result<()> {
    match request {
        scraper::Request::Artist { url } => {
            let artist = RefCell::new(None);
            scraper.scrape_artist(
                &Url::parse(&url)?,
                |new_artist, details| {
                    artist.replace(Some((new_artist, details)));
                    Ok(())
                },
                |releases| {
                    scraped.send(scraper::Response::Releases(
                        artist.borrow().as_ref().unwrap().0.clone(),
                        releases,
                    ))?;
                    Ok(())
                },
            )?;
            let (artist, details) = artist.replace(None).take().unwrap();
            scraped.send(scraper::Response::Artist(artist, details))?;
        }

        scraper::Request::Release { url } => {
            let release = RefCell::new(None);
            scraper.scrape_release(
                &Url::parse(&url)?,
                |new_release, details| {
                    release.replace(Some((new_release, details)));
                    Ok(())
                },
                |artist| {
                    scraped.send(scraper::Response::ReleaseArtist(
                        release.borrow().as_ref().unwrap().0.clone(),
                        artist,
                    ))?;
                    Ok(())
                },
                |fans| {
                    scraped.send(scraper::Response::Fans(
                        release.borrow().as_ref().unwrap().0.clone(),
                        fans,
                    ))?;
                    Ok(())
                },
            )?;
            let (release, details) = release.replace(None).take().unwrap();
            scraped.send(scraper::Response::Release(release, details))?;
        }

        scraper::Request::User { url } => {
            let user = RefCell::new(None);
            scraper.scrape_fan(
                &Url::parse(&url)?,
                |fan, details| {
                    user.replace(Some((fan, details)));
                    Ok(())
                },
                |collection| {
                    scraped.send(scraper::Response::Collection(
                        user.borrow().as_ref().unwrap().0.clone(),
                        collection,
                    ))?;
                    Ok(())
                },
            )?;
            let (user, details) = user.replace(None).take().unwrap();
            scraped.send(scraper::Response::User(user, details))?;
        }
    }
}
