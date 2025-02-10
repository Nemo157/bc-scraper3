use bevy::{
    asset::Assets,
    color::Color,
    ecs::{bundle::Bundle, component::Component, system::Commands},
    math::primitives::{Annulus, Circle, Rectangle},
    picking::PickingBehavior,
    render::mesh::{Mesh, Mesh2d},
    sprite::{ColorMaterial, MeshMaterial2d},
    transform::components::Transform,
};

use rand::{distr::Distribution, seq::IndexedRandom, Rng};
use rand_distr::Poisson;

use std::sync::OnceLock;

use crate::sim::{MotionBundle, Position, Relationship};

#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct AlbumId(pub u64);

#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct ArtistId(pub u64);

#[derive(Copy, Clone, Debug, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub struct UserId(pub u64);

#[derive(Debug, Component)]
pub enum EntityData {
    Album(Album),
    Artist(Artist),
    User(User),
}

impl EntityData {
    pub fn url(&self) -> &str {
        match self {
            Self::Album(Album { url, .. }) => url,
            Self::Artist(Artist { url, .. }) => url,
            Self::User(User { url, .. }) => url,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Album {
    pub id: AlbumId,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct Artist {
    pub id: ArtistId,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub url: String,
}

#[derive(Bundle)]
pub struct EntityBundle {
    render: (Mesh2d, MeshMaterial2d<ColorMaterial>),
    pub motion: MotionBundle,
    pub data: EntityData,
}

#[derive(Bundle)]
pub struct RelationshipBundle {
    render: (Mesh2d, MeshMaterial2d<ColorMaterial>),
    relationship: Relationship,
    transform: Transform,
    picking_behavior: PickingBehavior,
}

static ALBUM_RENDER: OnceLock<(Mesh2d, MeshMaterial2d<ColorMaterial>)> = OnceLock::new();
static ARTIST_RENDER: OnceLock<(Mesh2d, MeshMaterial2d<ColorMaterial>)> = OnceLock::new();
static USER_RENDER: OnceLock<(Mesh2d, MeshMaterial2d<ColorMaterial>)> = OnceLock::new();
static LINK_RENDER: OnceLock<(Mesh2d, MeshMaterial2d<ColorMaterial>)> = OnceLock::new();

pub fn init_meshes(meshes: &mut Assets<Mesh>, materials: &mut Assets<ColorMaterial>) {
    ALBUM_RENDER
        .set((
            Mesh2d(meshes.add(Circle::new(10.0))),
            MeshMaterial2d(materials.add(Color::hsl(0., 0.95, 0.7))),
        ))
        .unwrap();
    ARTIST_RENDER
        .set((
            Mesh2d(meshes.add(Annulus::new(10.0, 6.0))),
            MeshMaterial2d(materials.add(Color::hsl(270., 0.95, 0.7))),
        ))
        .unwrap();
    USER_RENDER
        .set((
            Mesh2d(meshes.add(Rectangle::new(10.0, 10.0))),
            MeshMaterial2d(materials.add(Color::hsl(180., 0.95, 0.7))),
        ))
        .unwrap();
    LINK_RENDER
        .set((
            Mesh2d(meshes.add(Rectangle::new(1.0, 1.0))),
            MeshMaterial2d(materials.add(Color::hsl(90., 0.95, 0.7))),
        ))
        .unwrap();
}

impl EntityData {
    pub fn at_location(self, motion: MotionBundle) -> EntityBundle {
        let render = match self {
            Self::Album(_) => ALBUM_RENDER.get(),
            Self::Artist(_) => ARTIST_RENDER.get(),
            Self::User(_) => USER_RENDER.get(),
        }
        .unwrap()
        .clone();

        EntityBundle {
            render,
            motion,
            data: self,
        }
    }

    pub fn at_random_location(self) -> EntityBundle {
        self.at_location(MotionBundle::random())
    }

    pub fn at_random_location_near(self, position: Position) -> EntityBundle {
        self.at_location(MotionBundle::random_near(position))
    }
}

impl Relationship {
    pub fn bundle(self) -> RelationshipBundle {
        RelationshipBundle {
            render: LINK_RENDER.get().unwrap().clone(),
            relationship: self,
            transform: Transform::IDENTITY,
            picking_behavior: PickingBehavior::IGNORE,
        }
    }
}

pub fn create_random(mut commands: Commands, albums: u64, artists: u64, users: u64) {
    let mut rng = rand::rng();

    let albums = Vec::from_iter((0..albums).map(|i| {
        commands
            .spawn(
                EntityData::Album(Album {
                    id: AlbumId(i),
                    url: format!("rand:album:{i}"),
                })
                .at_random_location(),
            )
            .id()
    }));

    let artists = Vec::from_iter((0..artists).map(|i| {
        commands
            .spawn(
                EntityData::Artist(Artist {
                    id: ArtistId(i),
                    url: format!("rand:artist:{i}"),
                })
                .at_random_location(),
            )
            .id()
    }));

    let users = Vec::from_iter((0..users).map(|i| {
        commands
            .spawn(
                EntityData::User(User {
                    id: UserId(i),
                    url: format!("rand:user:{i}"),
                })
                .at_random_location(),
            )
            .id()
    }));

    let mut user_albums = albums.clone();
    let mut user_linked_albums = Vec::new();

    for from in &users {
        let count: f64 = Poisson::new(20.0).unwrap().sample(&mut rng);
        for to in user_albums.drain(..(count as usize).min(user_albums.len())) {
            user_linked_albums.push(to);
            commands.spawn(Relationship { from: *from, to }.bundle());
        }
    }

    for from in &users {
        let count: f64 = Poisson::new(3.0).unwrap().sample(&mut rng);
        for to in user_linked_albums.choose_multiple(&mut rng, count as usize) {
            commands.spawn(
                Relationship {
                    from: *from,
                    to: *to,
                }
                .bundle(),
            );
        }
    }

    for to in &user_albums {
        let from = users.choose(&mut rng).unwrap();
        commands.spawn(
            Relationship {
                from: *from,
                to: *to,
            }
            .bundle(),
        );
    }

    let mut artist_albums = albums.clone();

    for from in &artists {
        let index = rng.random_range(0..artist_albums.len());
        let to = artist_albums.swap_remove(index);
        commands.spawn(Relationship { from: *from, to }.bundle());
    }

    for to in &artist_albums {
        let from = artists.choose(&mut rng).unwrap();
        commands.spawn(
            Relationship {
                from: *from,
                to: *to,
            }
            .bundle(),
        );
    }
}
