use crate::data::{Album, AlbumId, Artist, ArtistId, User, UserId};
use std::collections::HashMap;
use url::Url;

#[derive(Debug)]
pub(crate) struct Scraper {
    client: super::web::Client,
}

trait JsonExt {
    fn parse_json<T: serde::de::DeserializeOwned>(&self) -> eyre::Result<T>;
}

impl JsonExt for str {
    #[culpa::try_fn]
    fn parse_json<T: serde::de::DeserializeOwned>(&self) -> eyre::Result<T> {
        serde_json::from_str(self)?
    }
}

trait ScraperExt {
    fn try_select(&self, selector: &str) -> eyre::Result<Vec<scraper::ElementRef<'_>>>;

    fn try_select_one(&self, selector: &str) -> eyre::Result<scraper::ElementRef<'_>>;
}

impl ScraperExt for scraper::Html {
    #[culpa::try_fn]
    #[tracing::instrument(skip(self))]
    fn try_select(&self, selector: &str) -> eyre::Result<Vec<scraper::ElementRef<'_>>> {
        let s = scraper::Selector::parse(selector).map_err(|e| eyre::eyre!("{e:?}"))?;
        self.select(&s).collect()
    }

    #[culpa::try_fn]
    #[tracing::instrument(skip(self))]
    fn try_select_one(&self, selector: &str) -> eyre::Result<scraper::ElementRef<'_>> {
        let s = scraper::Selector::parse(selector).map_err(|e| eyre::eyre!("{e:?}"))?;
        self.select(&s)
            .next()
            .ok_or_else(|| eyre::eyre!("missing element for {selector}"))?
    }
}

impl ScraperExt for scraper::ElementRef<'_> {
    #[culpa::try_fn]
    #[tracing::instrument(skip(self))]
    fn try_select(&self, selector: &str) -> eyre::Result<Vec<scraper::ElementRef<'_>>> {
        let s = scraper::Selector::parse(selector).map_err(|e| eyre::eyre!("{e:?}"))?;
        self.select(&s).collect()
    }

    #[culpa::try_fn]
    #[tracing::instrument(skip(self))]
    fn try_select_one(&self, selector: &str) -> eyre::Result<scraper::ElementRef<'_>> {
        let s = scraper::Selector::parse(selector).map_err(|e| eyre::eyre!("{e:?}"))?;
        self.select(&s)
            .next()
            .ok_or_else(|| eyre::eyre!("missing element for {selector}"))?
    }
}

#[derive(Debug)]
struct AlbumPage {
    properties: Properties,
    data_band: DataBand,
    collectors: Collectors,
    discography: String,
}

#[derive(Debug, serde::Deserialize)]
struct Properties {
    item_type: String,
    item_id: u64,
}

#[allow(unused)]
#[derive(Debug, serde::Deserialize)]
struct DataBand {
    id: u64,
    name: String,
}

#[derive(Debug, serde::Deserialize)]
struct Collectors {
    // TODO: load more reviews
    // more_reviews_available: bool,
    more_thumbs_available: bool,
    reviews: Vec<Review>,
    thumbs: Vec<Fan>,
}

#[derive(Debug, serde::Deserialize)]
struct Review {
    fan_id: u64,
    username: String,
}

#[derive(Debug, serde::Deserialize)]
struct Fan {
    fan_id: u64,
    username: String,
    token: String,
}

#[derive(Debug, serde::Deserialize)]
struct Thumbs {
    results: Vec<Fan>,
    more_available: bool,
}

#[derive(Debug, serde::Deserialize)]
pub struct CollectionItem {
    item_id: u64,
    item_url: String,
}

#[derive(Debug, serde::Deserialize)]
struct ItemCache {
    collection: HashMap<String, CollectionItem>,
}

#[derive(Debug, serde::Deserialize)]
struct CollectionData {
    last_token: String,
    sequence: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct FanData {
    fan_id: u64,
    username: String,
}

#[derive(Debug, serde::Deserialize)]
struct FanPage {
    fan_data: FanData,
    collection_count: usize,
    collection_data: CollectionData,
    item_cache: ItemCache,
}

#[derive(Debug, serde::Deserialize)]
struct Collections {
    more_available: bool,
    last_token: String,
    items: Vec<CollectionItem>,
}

#[derive(Debug)]
struct ArtistPage {
    data_band: DataBand,
    music_grid_items: Vec<MusicGridItem>,
    client_items: Option<Vec<ClientItem>>,
}

#[allow(unused)]
#[derive(Debug)]
struct MusicGridItem {
    item_id: u64,
    href: String,
    title: String,
    ty: String,
}

#[allow(unused)]
#[derive(Debug, serde::Deserialize)]
struct ClientItem {
    art_id: u64,
    band_id: u64,
    id: u64,
    page_url: String,
    title: String,
    #[serde(rename = "type")]
    ty: String,
}

impl Scraper {
    pub(crate) fn new(client: super::web::Client) -> Self {
        Self { client }
    }

    #[culpa::try_fn]
    #[tracing::instrument(skip(self, on_album, on_album_artist, on_fans), fields(%url))]
    pub(crate) fn scrape_album(
        &self,
        url: &Url,
        on_album: impl FnOnce(Album) -> eyre::Result<()>,
        on_album_artist: impl FnOnce(Artist) -> eyre::Result<()>,
        mut on_fans: impl FnMut(Vec<User>) -> eyre::Result<()>,
    ) -> eyre::Result<()> {
        let page = self.scrape_album_page(url)?;

        let mut more_available = page.collectors.more_thumbs_available;
        on_album(Album {
            id: AlbumId(page.properties.item_id),
            url: url.into(),
        })?;

        on_album_artist(Artist {
            id: ArtistId(page.data_band.id),
            url: url.join(&page.discography)?.into(),
        })?;

        let token = page
            .collectors
            .thumbs
            .last()
            .map(|thumb| thumb.token.clone());
        on_fans(
            page.collectors
                .reviews
                .into_iter()
                .map(|review| User {
                    id: UserId(review.fan_id),
                    url: format!("https://bandcamp.com/{}", review.username).into(),
                })
                .collect(),
        )?;
        on_fans(
            page.collectors
                .thumbs
                .into_iter()
                .map(|thumb| User {
                    id: UserId(thumb.fan_id),
                    url: format!("https://bandcamp.com/{}", thumb.username).into(),
                })
                .collect(),
        )?;

        if let Some(mut token) = token {
            while more_available {
                let response = self.scrape_collectors_api(url, &page.properties, &token)?;
                token = response.results.last().unwrap().token.clone();
                more_available = response.more_available;
                on_fans(
                    response
                        .results
                        .into_iter()
                        .map(|thumb| User {
                            id: UserId(thumb.fan_id),
                            url: format!("https://bandcamp.com/{}", thumb.username).into(),
                        })
                        .collect(),
                )?;
            }
        }
    }

    #[culpa::try_fn]
    #[tracing::instrument(skip(self, on_fan, on_collection))]
    pub(crate) fn scrape_fan(
        &self,
        url: &Url,
        on_fan: impl FnOnce(User) -> eyre::Result<()>,
        mut on_collection: impl FnMut(Vec<Album>) -> eyre::Result<()>,
    ) -> eyre::Result<()> {
        let mut page = self.scrape_fan_page(url)?;

        on_fan(User {
            id: UserId(page.fan_data.fan_id),
            url: format!("https://bandcamp.com/{}", page.fan_data.username).into(),
        })?;

        let items = eyre::Result::<Vec<_>, _>::from_iter(
            page.collection_data.sequence.into_iter().map(|s| {
                page.item_cache
                    .collection
                    .remove(&s)
                    .ok_or_else(|| eyre::eyre!("cache missing collection item"))
            }),
        )?;
        let mut last_token = page.collection_data.last_token;
        let mut more_available = items.len() < page.collection_count;
        on_collection(
            items
                .into_iter()
                .map(|item| Album {
                    id: AlbumId(item.item_id),
                    url: item.item_url.into(),
                })
                .collect(),
        )?;

        while more_available {
            let response = self.scrape_collections_api(page.fan_data.fan_id, &last_token)?;
            more_available = response.more_available;
            last_token = response.last_token;
            on_collection(
                response
                    .items
                    .into_iter()
                    .map(|item| Album {
                        id: AlbumId(item.item_id),
                        url: item.item_url.into(),
                    })
                    .collect(),
            )?;
        }
    }

    #[culpa::try_fn]
    #[tracing::instrument(skip(self, on_artist, on_releases))]
    pub(crate) fn scrape_artist(
        &self,
        url: &Url,
        on_artist: impl FnOnce(Artist) -> eyre::Result<()>,
        mut on_releases: impl FnMut(Vec<Album>) -> eyre::Result<()>,
    ) -> eyre::Result<()> {
        let page = self.scrape_artist_page(url)?;

        on_artist(Artist {
            id: ArtistId(page.data_band.id),
            url: url.into(),
        })?;

        on_releases(eyre::Result::<Vec<_>, _>::from_iter(
            page.music_grid_items.into_iter().map(|item| {
                eyre::Result::<_>::Ok(Album {
                    id: AlbumId(item.item_id),
                    url: url.join(&item.href)?.into(),
                })
            }),
        )?)?;

        on_releases(eyre::Result::<Vec<_>, _>::from_iter(
            page.client_items.into_iter().flatten().map(|item| {
                eyre::Result::<_>::Ok(Album {
                    id: AlbumId(item.id),
                    url: url.join(&item.page_url)?.into(),
                })
            }),
        )?)?;
    }

    #[culpa::try_fn]
    #[tracing::instrument(skip(self), fields(%url))]
    fn scrape_album_page(&self, url: &Url) -> eyre::Result<AlbumPage> {
        let data = self.client.get(url)?;
        let document = scraper::Html::parse_document(&data);
        let properties = document
            .try_select_one("meta[name=bc-page-properties]")?
            .value()
            .attr("content")
            .ok_or_else(|| eyre::eyre!("missing data-blob"))?
            .parse_json()?;
        let data_band = document
            .try_select_one("[data-band]")?
            .value()
            .attr("data-band")
            .ok_or_else(|| eyre::eyre!("missing data-band"))?
            .parse_json()?;
        let collectors = document
            .try_select_one("#collectors-data")?
            .value()
            .attr("data-blob")
            .ok_or_else(|| eyre::eyre!("missing data-blob"))?
            .parse_json()?;
        let discography = document
            .try_select_one("#discography a.link-and-title")?
            .value()
            .attr("href")
            .ok_or_else(|| eyre::eyre!("missing discography href"))?
            .to_owned();
        AlbumPage {
            properties,
            data_band,
            collectors,
            discography,
        }
    }

    #[culpa::try_fn]
    #[tracing::instrument(skip(self), fields(%url))]
    pub(crate) fn scrape_artist_page(&self, url: &Url) -> eyre::Result<ArtistPage> {
        let data = self.client.get(url)?;
        let document = scraper::Html::parse_document(&data);

        let data_band = document
            .try_select_one("[data-band]")?
            .value()
            .attr("data-band")
            .ok_or_else(|| eyre::eyre!("missing data-band"))?
            .parse_json()?;

        let music_grid_items = eyre::Result::<Vec<_>, _>::from_iter(
            document
                .try_select("li.music-grid-item")?
                .into_iter()
                .map(|item| {
                    let item_id = item
                        .value()
                        .attr("data-item-id")
                        .ok_or_else(|| eyre::eyre!("missing data-item-id"))?;
                    let (ty, item_id) = item_id
                        .split_once("-")
                        .ok_or_else(|| eyre::eyre!("failed to parse id"))?;
                    let title = item.try_select_one(".title")?.text().collect();
                    let href = item
                        .try_select_one("a")?
                        .attr("href")
                        .ok_or_else(|| eyre::eyre!("missing href"))?
                        .to_owned();
                    eyre::Result::<_>::Ok(MusicGridItem {
                        item_id: item_id.parse()?,
                        href,
                        ty: ty.to_owned(),
                        title,
                    })
                }),
        )?;

        let client_items = document
            .try_select_one("#music-grid")?
            .value()
            .attr("data-client-items")
            .map(|data| data.parse_json())
            .transpose()?;

        ArtistPage {
            data_band,
            music_grid_items,
            client_items,
        }
    }

    #[culpa::try_fn]
    #[tracing::instrument(skip(self), fields(%url))]
    fn scrape_fan_page(&self, url: &Url) -> eyre::Result<FanPage> {
        let data = self.client.get(url)?;
        let document = scraper::Html::parse_document(&data);
        document
            .try_select_one("#pagedata")?
            .value()
            .attr("data-blob")
            .ok_or_else(|| eyre::eyre!("missing data-blob"))?
            .parse_json()?
    }

    #[culpa::try_fn]
    #[tracing::instrument(skip(self), fields(%base_url))]
    fn scrape_collectors_api(
        &self,
        base_url: &Url,
        props: &Properties,
        token: &str,
    ) -> eyre::Result<Thumbs> {
        let url = base_url.join("/api/tralbumcollectors/2/thumbs")?;
        self.client
            .post(
                &url,
                &serde_json::json!({
                    "tralbum_type": props.item_type,
                    "tralbum_id": props.item_id,
                    "token": token,
                    "count": 80,
                }),
            )?
            .parse_json()?
    }

    #[culpa::try_fn]
    #[tracing::instrument(skip(self))]
    fn scrape_collections_api(&self, fan_id: u64, token: &str) -> eyre::Result<Collections> {
        let url = Url::parse("https://bandcamp.com/api/fancollection/1/collection_items")?;
        self.client
            .post(
                &url,
                &serde_json::json!({
                    "fan_id": fan_id,
                    "older_than_token": token,
                    "count": 20,
                }),
            )?
            .parse_json()?
    }
}
