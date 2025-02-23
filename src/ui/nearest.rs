use bevy::{
    color::Color,
    ecs::{
        change_detection::{DetectChanges, Ref},
        component::Component,
        entity::Entity,
        query::{QueryData, With},
        system::{Commands, Query, Res, Single},
    },
    hierarchy::{BuildChildren, ChildBuild, DespawnRecursiveExt},
    picking::PickingBehavior,
    text::TextFont,
    ui::widget::{Label, Text},
    ui::{
        AlignItems, BackgroundColor, Display, FlexDirection, JustifyContent, Node, PositionType,
        Val,
    },
};

use crate::{
    data::{ArtistDetails, EntityType, ReleaseDetails, Url, UserDetails},
    interact::Nearest,
};

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(bevy::app::Startup, setup);
        app.add_systems(bevy::app::Update, update);
    }
}

#[derive(Default, Component)]
struct NodeUi;

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
        NodeUi,
    ));
}

#[derive(QueryData)]
struct NodeDetails {
    ty: &'static EntityType,
    url: &'static Url,
    artist: Option<Ref<'static, ArtistDetails>>,
    release: Option<Ref<'static, ReleaseDetails>>,
    user: Option<Ref<'static, UserDetails>>,
}

impl NodeDetailsItem<'_> {
    fn is_changed(&self) -> bool {
        [
            self.artist.as_ref().map(|x| x.is_changed()),
            self.release.as_ref().map(|x| x.is_changed()),
            self.user.as_ref().map(|x| x.is_changed()),
        ]
        .into_iter()
        .flatten()
        .any(core::convert::identity)
    }
}

fn update(
    nearest: Option<Res<Nearest>>,
    details: Query<NodeDetails>,
    ui: Single<Entity, With<NodeUi>>,
    mut commands: Commands,
) {
    let Some(nearest) = nearest else { return };

    let Ok(details) = details.get(nearest.entity) else {
        // nothing to show
        return;
    };

    if nearest.is_changed() || details.is_changed() {
        commands.entity(*ui).despawn_descendants();

        commands.entity(*ui).with_children(|ui| {
            if let Some(release) = details.release.as_deref() {
                let ReleaseDetails {
                    title,
                    artist,
                    tracks,
                    length,
                    released,
                    ty,
                } = release;

                ui.spawn((
                    Text::new(format!("{ty:?}: {title}")),
                    TextFont::default(),
                    Label,
                    PickingBehavior::IGNORE,
                ));

                ui.spawn((
                    Text::new(format!("by {artist} in {}", released.year())),
                    TextFont::default(),
                    Label,
                    PickingBehavior::IGNORE,
                ));

                ui.spawn((
                    Text::new(if let Some(tracks) = tracks {
                        format!("{tracks} tracks | {length:?}")
                    } else {
                        format!("{length:?}")
                    }),
                    TextFont::default(),
                    Label,
                    PickingBehavior::IGNORE,
                ));
            } else if let Some(artist) = details.artist.as_deref() {
                let ArtistDetails { name } = artist;
                ui.spawn((
                    Text::new(format!("Artist: {name}")),
                    TextFont::default(),
                    Label,
                    PickingBehavior::IGNORE,
                ));
            } else if let Some(user) = details.user.as_deref() {
                let UserDetails { name, username } = user;
                ui.spawn((
                    Text::new(format!("User: {name} ({username})")),
                    TextFont::default(),
                    Label,
                    PickingBehavior::IGNORE,
                ));
            } else {
                ui.spawn((
                    Text::new(format!("Unscraped {:?}", details.ty)),
                    TextFont::default(),
                    Label,
                    PickingBehavior::IGNORE,
                ));
                ui.spawn((
                    Text::new(&details.url.0),
                    TextFont::default(),
                    Label,
                    PickingBehavior::IGNORE,
                ));
            }
        });
    }
}
