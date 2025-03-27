use super::super::Stats;
use super::Request;
use chrono::{offset::Utc, DateTime};
use crossbeam::channel::{Receiver, Sender};
use rusqlite::{
    named_params,
    types::{ToSqlOutput, ValueRef},
    OptionalExtension, ToSql,
};
use std::{
    path::Path,
    sync::{atomic::Ordering, Arc},
};
use url::Url;

#[derive(Debug)]
pub(crate) struct Cache {
    cache: rusqlite::Connection,
    stats: Arc<Stats>,
    server_requests: Sender<Request>,
}

#[derive(Debug, strum::AsRefStr)]
#[strum(serialize_all = "kebab-case")]
enum Method {
    Get,
    Post,
}

impl ToSql for Method {
    #[culpa::try_fn]
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        ToSqlOutput::Borrowed(ValueRef::Text(self.as_ref().as_bytes()))
    }
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
pub fn run(
    cache_dir: &Path,
    stats: Arc<Stats>,
    requests: Receiver<Request>,
    server_requests: Sender<Request>,
) -> eyre::Result<std::thread::JoinHandle<()>> {
    let cache = Cache::new(cache_dir, stats, server_requests)?;

    std::thread::Builder::new()
        .name("web-cache".to_owned())
        .spawn(move || {
            for request in &requests {
                match request {
                    Request::Get { url, response } => {
                        let _ = response.send(cache.get(&url));
                    }
                    Request::Post {
                        url,
                        data,
                        response,
                    } => {
                        let _ = response.send(cache.post(&url, &data));
                    }
                }
            }
        })?
}

impl Cache {
    #[culpa::try_fn]
    pub(crate) fn new(
        cache_dir: &Path,
        stats: Arc<Stats>,
        server_requests: Sender<Request>,
    ) -> eyre::Result<Self> {
        let mut cache = rusqlite::Connection::open(cache_dir.join("web-cache.sqlite"))?;

        let migrations = [
            "create table pages (id integer primary key) strict",
            "alter table pages add column url text not null",
            "alter table pages add column method text not null",
            "alter table pages add column data text",
            "alter table pages add column response text not null",
            "alter table pages add column retrieved text not null",
            "create unique index pages_index on pages (url, method, data)",
        ];

        let tx = cache.transaction()?;
        let version: u32 =
            tx.pragma_query_value(None, "user_version", |row| row.get("user_version"))?;
        for (migration, index) in migrations.into_iter().zip(1u32..) {
            if version < index {
                tx.execute(migration, ())?;
                tx.pragma_update(None, "user_version", index)?;
            }
        }
        tx.commit()?;

        Self {
            cache,
            stats,
            server_requests,
        }
    }

    #[culpa::try_fn]
    #[tracing::instrument(skip(self), fields(%url))]
    pub(crate) fn get(&self, url: &Url) -> eyre::Result<String> {
        self.stats.web_requests.fetch_add(1, Ordering::Relaxed);
        if let Some(response) = self.get_from_cache(url, Method::Get, None)? {
            response
        } else {
            let response = self.get_from_server(url.clone())?;
            self.add_to_cache(url, Method::Get, None, &response)?;
            response
        }
    }

    #[culpa::try_fn]
    #[tracing::instrument(skip(self), fields(%url))]
    pub(crate) fn post(&self, url: &Url, data: &serde_json::Value) -> eyre::Result<String> {
        self.stats.web_requests.fetch_add(1, Ordering::Relaxed);
        if let Some(response) = self.get_from_cache(url, Method::Post, Some(data))? {
            response
        } else {
            let response = self.post_to_server(url.clone(), data.clone())?;
            self.add_to_cache(url, Method::Post, Some(data), &response)?;
            response
        }
    }

    #[culpa::try_fn]
    #[tracing::instrument(skip(self), fields(%url, data=%data.dbg()))]
    fn get_from_cache(
        &self,
        url: &Url,
        method: Method,
        data: Option<&serde_json::Value>,
    ) -> eyre::Result<Option<String>> {
        let result = self
            .cache
            .query_row(
                "
                    select retrieved, response
                    from pages
                    where url = :url and method = :method and data is :data
                ",
                named_params!(":url": url, ":method": method, ":data": data),
                |row| {
                    Ok((
                        row.get::<_, DateTime<Utc>>("retrieved")?,
                        row.get::<_, String>("response")?,
                    ))
                },
            )
            .optional()?;

        if let Some((retrieved, response)) = result {
            tracing::info!(%retrieved, "cache hit");
            self.stats.web_cache_hits.fetch_add(1, Ordering::Relaxed);
            Some(response)
        } else {
            tracing::info!("cache miss");
            self.stats.web_cache_misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    #[culpa::try_fn]
    #[tracing::instrument(skip(self), fields(%url))]
    fn get_from_server(&self, url: Url) -> eyre::Result<String> {
        let (tx, rx) = crossbeam::channel::bounded(1);
        self.server_requests
            .send(Request::Get { url, response: tx })?;
        rx.recv()??
    }

    #[culpa::try_fn]
    #[tracing::instrument(skip(self), fields(%url))]
    fn post_to_server(&self, url: Url, data: serde_json::Value) -> eyre::Result<String> {
        let (tx, rx) = crossbeam::channel::bounded(1);
        self.server_requests.send(Request::Post {
            url,
            data,
            response: tx,
        })?;
        rx.recv()??
    }

    #[culpa::try_fn]
    #[tracing::instrument(skip(self, response), fields(%url, data=%data.dbg(), response_len=response.len()))]
    fn add_to_cache(
        &self,
        url: &Url,
        method: Method,
        data: Option<&serde_json::Value>,
        response: &str,
    ) -> eyre::Result<()> {
        self.cache.execute(
            "
                insert
                into pages (url, method, data, retrieved, response)
                values (:url, :method, :data, :retrieved, :response)
            ",
            named_params! {
                ":url": url,
                ":method": method,
                ":data": data,
                ":retrieved": Utc::now(),
                ":response": &response,
            },
        )?;
    }
}
