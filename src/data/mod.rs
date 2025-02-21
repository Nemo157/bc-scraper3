use bevy::{
    ecs::{bundle::Bundle, component::Component, entity::Entity, system::Commands},
    hierarchy::BuildChildren,
    picking::PickingBehavior,
    render::view::Visibility,
};

use rand::{distr::Distribution, seq::IndexedRandom, Rng};
use rand_distr::Poisson;

use crate::sim::{MotionBundle, Relationship, Weight};

mod diagnostic;

#[derive(Clone, Debug, PartialOrd, Ord, PartialEq, Eq, Hash, Component)]
pub struct Url(pub String);

impl From<String> for Url {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Url {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl From<url::Url> for Url {
    fn from(s: url::Url) -> Self {
        Self(s.to_string())
    }
}

impl From<&url::Url> for Url {
    fn from(s: &url::Url) -> Self {
        Self(s.to_string())
    }
}

#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq, Hash, Component)]
pub enum EntityType {
    Artist,
    Release,
    User,
}

#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq, Hash, Component)]
#[require(EntityType(|| EntityType::Artist))]
pub struct ArtistId(pub u64);

#[derive(Clone, Debug, Component)]
pub struct ArtistDetails {
    pub name: String,
}

#[derive(Debug, Clone, Bundle)]
pub struct Artist {
    pub id: ArtistId,
    pub url: Url,
}

#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq, Hash, Component)]
#[require(EntityType(|| EntityType::Release))]
pub struct ReleaseId(pub u64);

#[derive(Copy, Clone, Debug)]
pub enum ReleaseType {
    Album,
    Track,
}

#[derive(Clone, Debug, Component)]
pub struct ReleaseDetails {
    pub ty: ReleaseType,
    pub title: String,
    /// This is the _album artist_ which may not be the same name as the artist that owns the store
    /// which released the release (e.g. record labels, or featured artists).
    pub artist: String,
    pub tracks: Option<u32>,
    pub length: jiff::SignedDuration,
    pub released: jiff::Zoned,
}

#[derive(Debug, Clone, Bundle)]
pub struct Release {
    pub id: ReleaseId,
    pub url: Url,
}

#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq, Hash, Component)]
#[require(EntityType(|| EntityType::User))]
pub struct UserId(pub u64);

#[derive(Clone, Debug, Component)]
pub struct UserDetails {
    pub name: String,
    pub username: String,
}

#[derive(Debug, Clone, Bundle)]
pub struct User {
    pub id: UserId,
    pub url: Url,
}

#[derive(Bundle)]
pub struct RelationshipBundle {
    relationship: Relationship,
    picking_behavior: PickingBehavior,
    weight: Weight,
    visibility: Visibility,
}

impl Relationship {
    pub fn bundle(self, weight: f32) -> RelationshipBundle {
        RelationshipBundle {
            relationship: self,
            picking_behavior: PickingBehavior::IGNORE,
            weight: Weight(weight),
            visibility: Visibility::Inherited,
        }
    }
}

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins(self::diagnostic::Plugin);
    }
}

pub fn create_random(
    mut commands: Commands,
    relationship_parent: Entity,
    artists: u64,
    releases: u64,
    users: u64,
) {
    let mut rng = rand::rng();

    let releases = Vec::from_iter((0..releases).map(|i| {
        commands
            .spawn((
                Release {
                    id: ReleaseId(i),
                    url: format!("rand:release:{i}").into(),
                },
                MotionBundle::random(),
            ))
            .id()
    }));

    let artists = Vec::from_iter((0..artists).map(|i| {
        commands
            .spawn((
                Artist {
                    id: ArtistId(i),
                    url: format!("rand:artist:{i}").into(),
                },
                MotionBundle::random(),
            ))
            .id()
    }));

    let users = Vec::from_iter((0..users).map(|i| {
        commands
            .spawn((
                User {
                    id: UserId(i),
                    url: format!("rand:user:{i}").into(),
                },
                MotionBundle::random(),
            ))
            .id()
    }));

    let mut user_releases = releases.clone();
    let mut user_linked_releases = Vec::new();

    for from in &users {
        let count: f64 = Poisson::new(20.0).unwrap().sample(&mut rng);
        for to in user_releases.drain(..(count as usize).min(user_releases.len())) {
            user_linked_releases.push(to);
            commands
                .entity(relationship_parent)
                .with_child(Relationship { from: *from, to }.bundle(1.0));
        }
    }

    for from in &users {
        let count: f64 = Poisson::new(3.0).unwrap().sample(&mut rng);
        for to in user_linked_releases.choose_multiple(&mut rng, count as usize) {
            commands.entity(relationship_parent).with_child(
                Relationship {
                    from: *from,
                    to: *to,
                }
                .bundle(1.0),
            );
        }
    }

    for to in &user_releases {
        let from = users.choose(&mut rng).unwrap();
        commands.entity(relationship_parent).with_child(
            Relationship {
                from: *from,
                to: *to,
            }
            .bundle(1.0),
        );
    }

    let mut artist_releases = releases.clone();

    for from in &artists {
        let index = rng.random_range(0..artist_releases.len());
        let to = artist_releases.swap_remove(index);
        commands
            .entity(relationship_parent)
            .with_child(Relationship { from: *from, to }.bundle(1.0));
    }

    for to in &artist_releases {
        let from = artists.choose(&mut rng).unwrap();
        commands.entity(relationship_parent).with_child(
            Relationship {
                from: *from,
                to: *to,
            }
            .bundle(5.0),
        );
    }
}
