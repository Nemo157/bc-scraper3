use crate::data::{Artist, ArtistDetails, Release, ReleaseDetails, User, UserDetails};

mod scraper;
pub mod thread;

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
