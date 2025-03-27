use crossbeam::channel::Sender;
use url::Url;

pub mod cache;
pub mod client;

pub enum Request {
    Get {
        url: Url,
        response: Sender<eyre::Result<String>>,
    },

    Post {
        url: Url,
        data: serde_json::Value,
        response: Sender<eyre::Result<String>>,
    },
}
