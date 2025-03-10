use bevy::{
    color::Color,
    ecs::{
        change_detection::{DetectChanges, Ref},
        component::Component,
        entity::Entity,
        observer::Trigger,
        query::{QueryData, With},
        system::{Commands, Query, Res, Single},
    },
    hierarchy::{BuildChildren, ChildBuild, DespawnRecursiveExt},
    input::{mouse::MouseButton, ButtonInput},
    picking::{
        events::{Click, Out, Over, Pointer},
        pointer::PointerButton,
        PickingBehavior,
    },
    render::view::Visibility,
    text::TextFont,
    ui::widget::{Button, Text},
    ui::{
        AlignItems, BackgroundColor, Display, FlexDirection, JustifyContent, Node, PositionType,
        UiRect, Val,
    },
};

use crate::{
    background::Request,
    camera::Cursor,
    data::{ArtistDetails, EntityType, ReleaseDetails, Scrape, Url, UserDetails},
    interact::Nearest,
    sim::Relationship,
};

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(bevy::app::Startup, setup);
        app.add_systems(bevy::app::Update, show_hide);

        app.add_observer(button_over);
        app.add_observer(button_out);
        app.add_observer(button_click);
    }
}

#[derive(Default, Component)]
struct MenuMarker;

#[derive(QueryData)]
#[query_data(mutable)]
pub struct Menu {
    entity: Entity,
    pub node: &'static mut Node,
    pub visibility: &'static mut Visibility,

    _marker: &'static MenuMarker,
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Start,
            align_items: AlignItems::Start,
            position_type: PositionType::Absolute,
            left: Val::Px(0.),
            top: Val::Px(0.),
            ..Node::default()
        },
        BackgroundColor(Color::srgba(0.10, 0.10, 0.10, 0.98)),
        PickingBehavior::IGNORE,
        MenuMarker,
        Visibility::Hidden,
    ));
}

#[derive(QueryData)]
struct NodeDetails {
    ty: &'static EntityType,
    url: &'static Url,
    artist: Option<Ref<'static, ArtistDetails>>,
    release: Option<Ref<'static, ReleaseDetails>>,
    user: Option<Ref<'static, UserDetails>>,
    scrape: Ref<'static, Scrape>,
}

#[derive(Component)]
enum Action {
    Open,
    Scrape,
    ScrapeDeep,
    ScrapeExtraDeep,
}

fn show_hide(
    button: Res<ButtonInput<MouseButton>>,
    cursor: Option<Res<Cursor>>,
    nearest: Option<Res<Nearest>>,
    details: Query<NodeDetails>,
    mut menu: Single<Menu>,
    mut commands: Commands,
) {
    let Some(nearest) = nearest else { return };

    if button.just_pressed(MouseButton::Right) {
        if *menu.visibility == Visibility::Hidden {
            let Some(cursor) = cursor else { return };
            menu.node.left = Val::Px(cursor.screen_position.x);
            menu.node.top = Val::Px(cursor.screen_position.y);
        }
        menu.visibility.toggle_visible_hidden();
    }

    if *menu.visibility == Visibility::Visible {
        let Ok(details) = details.get(nearest.entity) else {
            return;
        };

        let mut commands = commands.entity(menu.entity);

        if button.just_pressed(MouseButton::Right) || details.scrape.is_changed() {
            commands.despawn_descendants();

            commands.with_children(|menu| {
                let mut button = |text: &'static str, action: Action| {
                    menu.spawn((
                        Node {
                            padding: UiRect::all(Val::Px(6.)),
                            ..Node::default()
                        },
                        Button,
                        BackgroundColor(Color::NONE),
                        action,
                    ))
                    .with_child((
                        Text::new(text),
                        TextFont::default(),
                        PickingBehavior::IGNORE,
                    ));
                };

                button("open url", Action::Open);

                match *details.scrape {
                    Scrape::None => button("scrape", Action::Scrape),
                    Scrape::InProgress => {}
                    Scrape::Shallow => button("scrape (deep)", Action::ScrapeDeep),
                    Scrape::Deep => button("scrape (extra deep)", Action::ScrapeExtraDeep),
                    Scrape::ExtraDeep => {}
                }
            });
        }
    }
}

fn button_over(
    trigger: Trigger<Pointer<Over>>,
    mut background_color: Query<&mut BackgroundColor, With<Button>>,
) {
    let Ok(mut background_color) = background_color.get_mut(trigger.entity()) else {
        return;
    };

    background_color.0 = Color::srgba(0.8, 0.8, 0.8, 0.1);
}

fn button_out(
    trigger: Trigger<Pointer<Out>>,
    mut background_color: Query<&mut BackgroundColor, With<Button>>,
) {
    let Ok(mut background_color) = background_color.get_mut(trigger.entity()) else {
        return;
    };

    background_color.0 = Color::NONE;
}

fn button_click(
    trigger: Trigger<Pointer<Click>>,
    scraper: Res<crate::background::Thread>,
    query: Query<&Action, With<Button>>,
    nearest: Option<Res<Nearest>>,
    mut data: Query<(&Url, &EntityType, &mut Scrape)>,
    relationships: Query<&Relationship>,
    mut menu: Single<Menu>,
    runtime: Res<crate::Runtime>,
) {
    let Ok(action) = query.get(trigger.entity()) else {
        return;
    };
    let Some(nearest) = nearest else { return };

    if trigger.event.button == PointerButton::Primary {
        let request = |data: &mut Query<(&Url, &EntityType, &mut Scrape)>, entity| match data
            .get_mut(entity)
        {
            Ok((Url(url), EntityType::Release, mut scrape)) => {
                scrape.clamp_to(Scrape::InProgress..);
                scraper.send(Request::Release { url: url.clone() }).unwrap();
            }
            Ok((Url(url), EntityType::Artist, mut scrape)) => {
                scrape.clamp_to(Scrape::InProgress..);
                scraper.send(Request::Artist { url: url.clone() }).unwrap();
            }
            Ok((Url(url), EntityType::User, mut scrape)) => {
                scrape.clamp_to(Scrape::InProgress..);
                scraper.send(Request::User { url: url.clone() }).unwrap();
            }
            Err(_) => {}
        };

        let next_level = |entity| {
            relationships.iter().filter_map(move |rel| {
                (rel.from == entity)
                    .then_some(rel.to)
                    .or((rel.to == entity).then_some(rel.from))
            })
        };

        match action {
            Action::Open => {
                let Ok((url, _, _)) = data.get(nearest.entity) else {
                    return;
                };
                let url = url::Url::parse(&url.0).unwrap();
                runtime.spawn_background(async move {
                    use ashpd::desktop::open_uri::OpenFileRequest;
                    match OpenFileRequest::default()
                        .send_uri(&url)
                        .await
                        .and_then(|req| req.response())
                    {
                        Ok(()) => tracing::info!("opened {url}"),
                        Err(err) => {
                            tracing::error!("failed to open {url}: {:?}", eyre::Report::from(err));
                        }
                    }
                });
            }
            Action::Scrape => {
                request(&mut data, nearest.entity);
            }
            Action::ScrapeDeep => {
                if let Ok((_, _, mut scrape)) = data.get_mut(nearest.entity) {
                    scrape.clamp_to(Scrape::Deep..);
                }
                next_level(nearest.entity).for_each(|entity| request(&mut data, entity));
            }
            Action::ScrapeExtraDeep => {
                if let Ok((_, _, mut scrape)) = data.get_mut(nearest.entity) {
                    scrape.clamp_to(Scrape::ExtraDeep..);
                }
                for entity in next_level(nearest.entity) {
                    if let Ok((_, _, mut scrape)) = data.get_mut(entity) {
                        scrape.clamp_to(Scrape::Deep..);
                    }
                    for entity in next_level(entity) {
                        request(&mut data, entity);
                    }
                }
            }
        }
    }

    menu.visibility.toggle_visible_hidden();
}
