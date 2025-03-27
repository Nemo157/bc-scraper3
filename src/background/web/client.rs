use super::super::Stats;
use chrono::{offset::Utc, DateTime};
use rusqlite::{
    named_params,
    types::{ToSqlOutput, ValueRef},
    OptionalExtension, ToSql,
};
use std::{
    cell::Cell,
    path::Path,
    sync::{atomic::Ordering, Arc},
    time::{Duration, Instant},
};
use url::Url;

#[derive(Debug)]
pub(crate) struct Client {
    client: reqwest::blocking::Client,
    cache: rusqlite::Connection,
    last_request: Cell<Instant>,

    stats: Arc<Stats>,
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

impl Client {
    #[culpa::try_fn]
    pub(crate) fn new(cache_dir: &Path, stats: Arc<Stats>) -> eyre::Result<Self> {
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

        let version: u32 =
            cache.pragma_query_value(None, "user_version", |row| row.get("user_version"))?;
        for (migration, index) in migrations.into_iter().zip(1u32..) {
            if version < index {
                let tx = cache.transaction()?;
                tx.execute(migration, ())?;
                tx.pragma_update(None, "user_version", index)?;
                tx.commit()?;
            }
        }

        Self {
            client: reqwest::blocking::Client::new(),
            cache,
            last_request: Cell::new(Instant::now()),
            stats,
        }
    }

    #[culpa::try_fn]
    #[tracing::instrument(skip(self), fields(%url))]
    pub(crate) fn get(&self, url: &Url) -> eyre::Result<String> {
        self.stats.web_requests.fetch_add(1, Ordering::Relaxed);
        if let Some(response) = self.get_from_cache(url, Method::Get, None)? {
            response
        } else {
            let response = self.get_from_server(url)?;
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
            let response = self.post_to_server(url, data)?;
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
    fn get_from_server(&self, url: &Url) -> eyre::Result<String> {
        self.check_delay();
        self.client.get(url.clone()).send()?.text()?
    }

    #[culpa::try_fn]
    #[tracing::instrument(skip(self), fields(%url, data=%data.dbg()))]
    fn post_to_server(&self, url: &Url, data: &serde_json::Value) -> eyre::Result<String> {
        self.check_delay();
        self.client.post(url.clone()).json(data).send()?.text()?
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
