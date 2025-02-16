use bevy::{
    app::{App, PluginGroup, Startup, Update},
    ecs::{
        change_detection::ResMut,
        component::Component,
        entity::Entity,
        event::EventReader,
        query::With,
        system::{Commands, Query, Res, Resource, Single},
    },
    hierarchy::BuildChildren,
    input::keyboard::{Key, KeyboardInput},
    picking::mesh_picking::MeshPickingPlugin,
    render::view::Visibility,
    time::{Fixed, Time},
    transform::components::Transform,
    utils::default,
    DefaultPlugins,
};

use clap::Parser;

use std::collections::{hash_map::Entry, HashMap};

mod background;
mod camera;
mod data;
mod diagnostic;
mod render;
mod sim;
mod ui;

use crate::{
    background::Response,
    data::{AlbumId, ArtistId, UserId},
    sim::{MotionBundle, Position, Relationship},
};

#[derive(Parser, Debug, Resource)]
#[command(
    version,
    arg_required_else_help = true,
    after_help = "At least one option must be passed to select initial data",
    after_long_help = color_print::cstr!("
At least one option must be passed to select initial data

<bold><underline>Controls:</underline></bold>

  <bold>Left-Click drag</bold> background to pan
  <bold>Hover</bold> node to pin it
  <bold>Left-Click drag</bold> node to move it
  <bold>Scroll</bold> to zoom
  <bold>Short-Click</bold> node to expand it
  <bold>Shift+Short-Click</bold> node to expand its linked nodes
  <bold>Ctrl+Shift+Short-Click</bold> node to expand its linked nodes' linked nodes
  <bold>Space</bold> to (un)pause simulation
  <bold>L</bold> to hide lines

"),
)]
struct Args {
    #[arg(long("user"), value_name("username"))]
    users: Vec<String>,
    #[arg(long("album"), value_name("url"))]
    albums: Vec<String>,
    #[arg(long("artist"), value_name("url"))]
    artists: Vec<String>,
    #[arg(long, value_names(["albums", "artists", "users"]), num_args(3))]
    random: Vec<u64>,
}

#[culpa::try_fn]
fn main() -> eyre::Result<()> {
    let args = Args::parse();

    color_eyre::install()?;

    let dirs = directories::ProjectDirs::from("com", "nemo157", "bc-scraper3").unwrap();

    std::fs::create_dir_all(dirs.cache_dir())?;

    App::new()
        .insert_resource(Time::<Fixed>::from_hz(20.0))
        .insert_resource(args)
        .insert_resource(background::Thread::spawn(dirs.cache_dir())?)
        .insert_resource(KnownEntities::default())
        .add_plugins((
            DefaultPlugins.set(bevy::log::LogPlugin {
                custom_layer: |_| Some(Box::new(tracing_error::ErrorLayer::default())),
                ..default()
            }),
            MeshPickingPlugin,
            self::camera::CameraPlugin,
            self::data::Plugin,
            self::diagnostic::Plugin,
            self::render::Plugin,
            self::sim::SimPlugin,
            self::ui::UiPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (receive, keyinput))
        .run();
}

#[derive(Component)]
struct RelationshipParent;

fn setup(mut commands: Commands, args: Res<Args>, scraper: Res<background::Thread>) {
    let relationship_parent = commands
        .spawn((Visibility::Visible, Transform::IDENTITY, RelationshipParent))
        .id();

    for url in &args.albums {
        scraper
            .send(background::Request::Album { url: url.clone() })
            .unwrap();
    }

    for username in &args.users {
        scraper
            .send(background::Request::User {
                url: format!("https://bandcamp.com/{username}"),
            })
            .unwrap();
    }

    for url in &args.artists {
        scraper
            .send(background::Request::Artist { url: url.clone() })
            .unwrap();
    }

    if let [albums, artists, users] = args.random[..] {
        data::create_random(commands, relationship_parent, albums, artists, users);
    }
}

#[derive(Resource, Default)]
struct KnownEntities {
    albums: HashMap<AlbumId, Entity>,
    artists: HashMap<ArtistId, Entity>,
    users: HashMap<UserId, Entity>,
    relationships: HashMap<Relationship, Entity>,
}

fn keyinput(
    mut events: EventReader<KeyboardInput>,
    mut relationship_parent: Single<&mut Visibility, With<RelationshipParent>>,
    mut paused: ResMut<sim::Paused>,
) {
    for event in events.read() {
        if event.state.is_pressed() {
            if event.logical_key == Key::Character("l".into()) {
                relationship_parent.toggle_visible_hidden();
            } else if event.logical_key == Key::Space {
                paused.0 ^= true;
            }
        }
    }
}

fn receive(
    mut commands: Commands,
    scraper: Res<background::Thread>,
    mut known: ResMut<KnownEntities>,
    positions: Query<&Position>,
    relationship_parent: Single<Entity, With<RelationshipParent>>,
) {
    if let Some(response) = scraper.try_recv().unwrap() {
        match response {
            Response::User(_user) => {
                // TODO: mark as scraped
            }
            Response::Album(_album) => {
                // TODO: mark as scraped
            }
            Response::Artist(_arist) => {
                // TODO: mark as scraped
            }

            Response::Fans(album, users) => {
                let (album, position) = match known.albums.entry(album.id) {
                    Entry::Occupied(entry) => {
                        let album = *entry.get();
                        let position = *positions.get(album).unwrap();
                        (album, position)
                    }
                    Entry::Vacant(entry) => {
                        let motion = MotionBundle::random();
                        let position = motion.position;
                        let album = commands.spawn((album, motion)).id();
                        entry.insert(album);
                        (album, position)
                    }
                };
                for user in users {
                    let user = *known.users.entry(user.id).or_insert_with(|| {
                        commands
                            .spawn((user, MotionBundle::random_near(position)))
                            .id()
                    });
                    let relationship = Relationship {
                        from: user,
                        to: album,
                    };
                    known.relationships.entry(relationship).or_insert_with(|| {
                        commands
                            .entity(*relationship_parent)
                            .with_child(relationship.bundle(1.0))
                            .id()
                    });
                }
            }

            Response::AlbumArtist(album, artist) => {
                let (album, position) = match known.albums.entry(album.id) {
                    Entry::Occupied(entry) => {
                        let album = *entry.get();
                        let position = *positions.get(album).unwrap();
                        (album, position)
                    }
                    Entry::Vacant(entry) => {
                        let motion = MotionBundle::random();
                        let position = motion.position;
                        let album = commands.spawn((album, motion)).id();
                        entry.insert(album);
                        (album, position)
                    }
                };
                let artist = *known.artists.entry(artist.id).or_insert_with(|| {
                    commands
                        .spawn((artist, MotionBundle::random_near(position)))
                        .id()
                });
                let relationship = Relationship {
                    from: artist,
                    to: album,
                };
                known.relationships.entry(relationship).or_insert_with(|| {
                    commands
                        .entity(*relationship_parent)
                        .with_child(relationship.bundle(3.0))
                        .id()
                });
            }

            Response::Releases(artist, albums) => {
                let (artist, position) = match known.artists.entry(artist.id) {
                    Entry::Occupied(entry) => {
                        let artist = *entry.get();
                        let position = *positions.get(artist).unwrap();
                        (artist, position)
                    }
                    Entry::Vacant(entry) => {
                        let motion = MotionBundle::random();
                        let position = motion.position;
                        let artist = commands.spawn((artist, motion)).id();
                        entry.insert(artist);
                        (artist, position)
                    }
                };
                for album in albums {
                    let album = *known.albums.entry(album.id).or_insert_with(|| {
                        commands
                            .spawn((album, MotionBundle::random_near(position)))
                            .id()
                    });
                    let relationship = Relationship {
                        from: artist,
                        to: album,
                    };
                    known.relationships.entry(relationship).or_insert_with(|| {
                        commands
                            .entity(*relationship_parent)
                            .with_child(relationship.bundle(3.0))
                            .id()
                    });
                }
            }

            Response::Collection(user, albums) => {
                let (user, position) = match known.users.entry(user.id) {
                    Entry::Occupied(entry) => {
                        let user = *entry.get();
                        let position = *positions.get(user).unwrap();
                        (user, position)
                    }
                    Entry::Vacant(entry) => {
                        let motion = MotionBundle::random();
                        let position = motion.position;
                        let user = commands.spawn((user, motion)).id();
                        entry.insert(user);
                        (user, position)
                    }
                };
                for album in albums {
                    let album = *known.albums.entry(album.id).or_insert_with(|| {
                        commands
                            .spawn((album, MotionBundle::random_near(position)))
                            .id()
                    });
                    let relationship = Relationship {
                        from: user,
                        to: album,
                    };
                    known.relationships.entry(relationship).or_insert_with(|| {
                        commands
                            .entity(*relationship_parent)
                            .with_child(relationship.bundle(1.0))
                            .id()
                    });
                }
            }
        }
    }
}
