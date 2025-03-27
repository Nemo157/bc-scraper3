use super::Request;
use crossbeam::channel::Receiver;
use std::{
    cell::Cell,
    time::{Duration, Instant},
};
use url::Url;

#[derive(Debug)]
pub(crate) struct Client {
    client: reqwest::blocking::Client,
    last_request: Cell<Instant>,
}

trait DebugExt {
    fn dbg(&self) -> String;
}

impl<T: DebugExt> DebugExt for &T {
    fn dbg(&self) -> String {
        (*self).dbg()
    }
}

impl<T: DebugExt> DebugExt for Option<T> {
    fn dbg(&self) -> String {
        self.as_ref()
            .map(T::dbg)
            .unwrap_or_else(|| "None".to_owned())
    }
}

impl DebugExt for serde_json::Value {
    fn dbg(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|e| format!("Err({e})"))
    }
}

#[culpa::try_fn]
pub fn run(requests: Receiver<Request>) -> eyre::Result<std::thread::JoinHandle<()>> {
    let client = Client::new();

    std::thread::Builder::new()
        .name("web-client".to_owned())
        .spawn(move || {
            for request in &requests {
                match request {
                    Request::Get { url, response } => {
                        let _ = response.send(client.get(&url));
                    }
                    Request::Post {
                        url,
                        data,
                        response,
                    } => {
                        let _ = response.send(client.post(&url, &data));
                    }
                }
            }
        })?
}

impl Client {
    fn new() -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            last_request: Cell::new(Instant::now()),
        }
    }

    fn check_delay(&self) {
        const REQUEST_DELAY: Duration = Duration::from_secs(1);
        if let Some(delay) = REQUEST_DELAY.checked_sub(self.last_request.get().elapsed()) {
            tracing::info!(?delay, "delaying request");
            std::thread::sleep(delay);
        }
        self.last_request.set(Instant::now());
    }

    #[culpa::try_fn]
    #[tracing::instrument(skip(self), fields(%url))]
    fn get(&self, url: &Url) -> eyre::Result<String> {
        self.check_delay();
        self.client.get(url.clone()).send()?.text()?
    }

    #[culpa::try_fn]
    #[tracing::instrument(skip(self), fields(%url, data=%data.dbg()))]
    fn post(&self, url: &Url, data: &serde_json::Value) -> eyre::Result<String> {
        self.check_delay();
        self.client.post(url.clone()).json(data).send()?.text()?
    }
}
