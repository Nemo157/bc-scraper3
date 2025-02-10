use bevy::{
    app::{App, PluginGroup, Startup, Update},
    asset::Assets,
    ecs::{
        change_detection::ResMut,
        entity::Entity,
        system::{Commands, Query, Res, Resource},
    },
    picking::mesh_picking::MeshPickingPlugin,
    render::mesh::Mesh,
    sprite::ColorMaterial,
    time::{Fixed, Time},
    utils::default,
    DefaultPlugins,
};

use clap::Parser;

use std::collections::{hash_map::Entry, HashMap};

mod background;
mod camera;
mod data;
mod sim;
mod ui;

use crate::{
    background::{Request, Response},
    data::{AlbumId, EntityData, UserId},
    sim::{Position, Relationship},
};

#[derive(Parser, Debug, Resource)]
#[command(version)]
struct Args {
    #[arg(long("user"), value_name("username"))]
    users: Vec<String>,
    #[arg(long("album"), value_name("url"))]
    albums: Vec<String>,
    #[arg(long("artist"), value_name("url"))]
    artists: Vec<String>,
    #[arg(long, value_names(["albums", "users"]), num_args(2))]
    random: Vec<u64>,
}

#[culpa::try_fn]
fn main() -> eyre::Result<()> {
    let args = Args::parse();

    color_eyre::install()?;

    App::new()
        .insert_resource(Time::<Fixed>::from_hz(20.0))
        .insert_resource(args)
        .insert_resource(background::Thread::spawn()?)
        .insert_resource(KnownEntities::default())
        .add_plugins((
            DefaultPlugins.set(bevy::log::LogPlugin {
                custom_layer: |_| Some(Box::new(tracing_error::ErrorLayer::default())),
                ..default()
            }),
            MeshPickingPlugin,
            camera::CameraPlugin,
            sim::SimPlugin,
            ui::UiPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, receive)
        .run();
}

fn setup(
    commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    args: Res<Args>,
    scraper: Res<background::Thread>,
) {
    data::init_meshes(&mut meshes, &mut materials);

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

    if let [num_albums, num_users] = args.random[..] {
        data::create_random(commands, num_albums, num_users);
    }
}

#[derive(Resource, Default)]
struct KnownEntities {
    users: HashMap<UserId, Entity>,
    albums: HashMap<AlbumId, Entity>,
    relationships: HashMap<Relationship, Entity>,
}

fn receive(
    mut commands: Commands,
    scraper: Res<background::Thread>,
    mut known: ResMut<KnownEntities>,
    positions: Query<&Position>,
) {
    if let Some(response) = scraper.try_recv().unwrap() {
        match response {
            Response::User(_user) => {
                // TODO: mark as scraped
            }
            Response::Album(_album) => {
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
                        let bundle = EntityData::Album(album).at_random_location();
                        let position = bundle.motion.position;
                        let album = commands.spawn(bundle).id();
                        entry.insert(album);
                        (album, position)
                    }
                };
                for user in users {
                    let user = *known.users.entry(user.id).or_insert_with(|| {
                        commands
                            .spawn(EntityData::User(user).at_random_location_near(position))
                            .id()
                    });
                    let relationship = Relationship {
                        from: user,
                        to: album,
                    };
                    known
                        .relationships
                        .entry(relationship)
                        .or_insert_with(|| commands.spawn(relationship.bundle()).id());
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
                        let bundle = EntityData::User(user).at_random_location();
                        let position = bundle.motion.position;
                        let user = commands.spawn(bundle).id();
                        entry.insert(user);
                        (user, position)
                    }
                };
                for album in albums {
                    let album = *known.albums.entry(album.id).or_insert_with(|| {
                        commands
                            .spawn(EntityData::Album(album).at_random_location_near(position))
                            .id()
                    });
                    let relationship = Relationship {
                        from: user,
                        to: album,
                    };
                    known
                        .relationships
                        .entry(relationship)
                        .or_insert_with(|| commands.spawn(relationship.bundle()).id());
                }
            }
            Response::Release(url) => {
                scraper.send(Request::Album { url }).unwrap();
            }
        }
    }
}
