use bevy::{
    app::PluginGroup,
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
    time::{Fixed, Time, Virtual},
    transform::components::Transform,
    utils::default,
    DefaultPlugins,
};

use clap::Parser;

use std::{
    collections::{hash_map::Entry, HashMap},
    time::Duration,
};

mod background;
mod camera;
mod data;
mod diagnostic;
mod interact;
mod render;
mod runtime;
mod sim;
mod ui;

use crate::{
    background::Response,
    data::{ArtistId, ReleaseId, Scrape, UserId},
    runtime::Runtime,
    sim::{MotionBundle, PredictedPosition, Relationship},
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
  <bold>Shift+Scroll</bold> to scale timestep
  <bold>Right-Click</bold> to show/hide action menu for nearest node (indicated by line from cursor)
  <bold>Space</bold> to (un)pause simulation
  <bold>L</bold> to hide lines
  <bold>O</bold> to cycle origin force scaling (unit, squared, cubed)

"),
)]
struct Args {
    #[arg(long("artist"), value_name("url"))]
    artists: Vec<String>,

    #[arg(long("release"), value_name("url"))]
    releases: Vec<String>,

    #[arg(long("user"), value_name("username"))]
    users: Vec<String>,

    #[arg(long, value_names(["artists", "releases", "users"]), num_args(3))]
    random: Vec<u64>,
}

#[culpa::try_fn]
fn main() -> eyre::Result<()> {
    let args = Args::parse();

    color_eyre::install()?;

    let dirs = directories::ProjectDirs::from("com", "nemo157", "bc-scraper3").unwrap();

    std::fs::create_dir_all(dirs.cache_dir())?;

    bevy::app::App::new()
        .insert_resource(Time::<Fixed>::from_hz(20.0))
        .insert_resource(Time::<Virtual>::from_max_delta(Duration::from_millis(50)))
        .insert_resource(args)
        .insert_resource(background::Thread::spawn(dirs.cache_dir())?)
        .insert_resource(KnownEntities::default())
        .insert_resource(Runtime::new())
        .add_plugins((
            DefaultPlugins.set(bevy::log::LogPlugin {
                custom_layer: |_| Some(Box::new(tracing_error::ErrorLayer::default())),
                ..default()
            }),
            MeshPickingPlugin,
            self::background::diagnostic::Plugin,
            self::camera::CameraPlugin,
            self::data::Plugin,
            self::diagnostic::Plugin,
            self::interact::Plugin,
            self::render::Plugin,
            self::sim::Plugin,
            self::ui::Plugin,
        ))
        .add_systems(bevy::app::Startup, setup)
        .add_systems(bevy::app::PreUpdate, keyinput)
        .add_systems(bevy::app::Update, receive)
        .run();
}

#[derive(Component)]
struct RelationshipParent;

fn setup(mut commands: Commands, args: Res<Args>, scraper: Res<background::Thread>) {
    let relationship_parent = commands
        .spawn((Visibility::Visible, Transform::IDENTITY, RelationshipParent))
        .id();

    for url in &args.releases {
        scraper
            .send(background::Request::Release { url: url.clone() })
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

    if let [artists, releases, users] = args.random[..] {
        data::create_random(commands, relationship_parent, artists, releases, users);
    }
}

#[derive(Resource, Default)]
struct KnownEntities {
    artists: HashMap<ArtistId, Entity>,
    releases: HashMap<ReleaseId, Entity>,
    users: HashMap<UserId, Entity>,
    relationships: HashMap<Relationship, Entity>,
}

fn keyinput(
    mut events: EventReader<KeyboardInput>,
    mut relationship_parent: Single<&mut Visibility, With<RelationshipParent>>,
    mut paused: ResMut<sim::Paused>,
    mut origin_force_mode: ResMut<sim::OriginForceMode>,
) {
    for event in events.read() {
        if event.state.is_pressed() {
            if event.logical_key == Key::Character("l".into()) {
                relationship_parent.toggle_visible_hidden();
            } else if event.logical_key == Key::Space {
                paused.0 ^= true;
            } else if event.logical_key == Key::Character("o".into()) {
                origin_force_mode.go_to_next();
            }
        }
    }
}

fn receive(
    mut commands: Commands,
    scraper: Res<background::Thread>,
    mut known: ResMut<KnownEntities>,
    positions: Query<&PredictedPosition>,
    mut scrape: Query<&mut Scrape>,
    relationship_parent: Single<Entity, With<RelationshipParent>>,
) {
    if let Some(response) = scraper.try_recv().unwrap() {
        match response {
            Response::Artist(artist, details) => match known.artists.entry(artist.id) {
                Entry::Occupied(entry) => {
                    commands.entity(*entry.get()).insert(details);
                    if let Ok(mut scrape) = scrape.get_mut(*entry.get()) {
                        scrape.clamp_to(Scrape::Shallow..);
                    }
                }
                Entry::Vacant(entry) => {
                    let motion = MotionBundle::random();
                    entry.insert(
                        commands
                            .spawn((artist, motion, details, Scrape::Shallow))
                            .id(),
                    );
                }
            },

            Response::Release(release, details) => match known.releases.entry(release.id) {
                Entry::Occupied(entry) => {
                    commands.entity(*entry.get()).insert(details);
                    if let Ok(mut scrape) = scrape.get_mut(*entry.get()) {
                        scrape.clamp_to(Scrape::Shallow..);
                    }
                }
                Entry::Vacant(entry) => {
                    let motion = MotionBundle::random();
                    entry.insert(
                        commands
                            .spawn((release, motion, details, Scrape::Shallow))
                            .id(),
                    );
                }
            },

            Response::User(user, details) => match known.users.entry(user.id) {
                Entry::Occupied(entry) => {
                    commands.entity(*entry.get()).insert(details);
                    if let Ok(mut scrape) = scrape.get_mut(*entry.get()) {
                        scrape.clamp_to(Scrape::Shallow..);
                    }
                }
                Entry::Vacant(entry) => {
                    let motion = MotionBundle::random();
                    entry.insert(
                        commands
                            .spawn((user, motion, details, Scrape::Shallow))
                            .id(),
                    );
                }
            },

            Response::Fans(release, users) => {
                let (release, position) = match known.releases.entry(release.id) {
                    Entry::Occupied(entry) => {
                        let release = *entry.get();
                        let position = *positions.get(release).unwrap();
                        (release, position.0)
                    }
                    Entry::Vacant(entry) => {
                        let motion = MotionBundle::random();
                        let position = motion.position;
                        let release = commands.spawn((release, motion, Scrape::Shallow)).id();
                        entry.insert(release);
                        (release, position.0)
                    }
                };
                for user in users {
                    let user = *known.users.entry(user.id).or_insert_with(|| {
                        commands
                            .spawn((user, MotionBundle::random_near(position), Scrape::None))
                            .id()
                    });
                    let relationship = Relationship {
                        from: user,
                        to: release,
                    };
                    known.relationships.entry(relationship).or_insert_with(|| {
                        commands
                            .entity(*relationship_parent)
                            .with_child(relationship.bundle(1.0))
                            .id()
                    });
                }
            }

            Response::ReleaseArtist(release, artist) => {
                let (release, position) = match known.releases.entry(release.id) {
                    Entry::Occupied(entry) => {
                        let release = *entry.get();
                        let position = *positions.get(release).unwrap();
                        (release, position.0)
                    }
                    Entry::Vacant(entry) => {
                        let motion = MotionBundle::random();
                        let position = motion.position;
                        let release = commands.spawn((release, motion, Scrape::InProgress)).id();
                        entry.insert(release);
                        (release, position.0)
                    }
                };
                let artist = *known.artists.entry(artist.id).or_insert_with(|| {
                    commands
                        .spawn((artist, MotionBundle::random_near(position), Scrape::None))
                        .id()
                });
                let relationship = Relationship {
                    from: artist,
                    to: release,
                };
                known.relationships.entry(relationship).or_insert_with(|| {
                    commands
                        .entity(*relationship_parent)
                        .with_child(relationship.bundle(3.0))
                        .id()
                });
            }

            Response::Releases(artist, releases) => {
                let (artist, position) = match known.artists.entry(artist.id) {
                    Entry::Occupied(entry) => {
                        let artist = *entry.get();
                        let position = *positions.get(artist).unwrap();
                        (artist, position.0)
                    }
                    Entry::Vacant(entry) => {
                        let motion = MotionBundle::random();
                        let position = motion.position;
                        let artist = commands.spawn((artist, motion, Scrape::InProgress)).id();
                        entry.insert(artist);
                        (artist, position.0)
                    }
                };
                for release in releases {
                    let release = *known.releases.entry(release.id).or_insert_with(|| {
                        commands
                            .spawn((release, MotionBundle::random_near(position), Scrape::None))
                            .id()
                    });
                    let relationship = Relationship {
                        from: artist,
                        to: release,
                    };
                    known.relationships.entry(relationship).or_insert_with(|| {
                        commands
                            .entity(*relationship_parent)
                            .with_child(relationship.bundle(3.0))
                            .id()
                    });
                }
            }

            Response::Collection(user, releases) => {
                let (user, position) = match known.users.entry(user.id) {
                    Entry::Occupied(entry) => {
                        let user = *entry.get();
                        let position = *positions.get(user).unwrap();
                        (user, position.0)
                    }
                    Entry::Vacant(entry) => {
                        let motion = MotionBundle::random();
                        let position = motion.position;
                        let user = commands.spawn((user, motion, Scrape::InProgress)).id();
                        entry.insert(user);
                        (user, position.0)
                    }
                };
                for release in releases {
                    let release = *known.releases.entry(release.id).or_insert_with(|| {
                        commands
                            .spawn((release, MotionBundle::random_near(position), Scrape::None))
                            .id()
                    });
                    let relationship = Relationship {
                        from: user,
                        to: release,
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
