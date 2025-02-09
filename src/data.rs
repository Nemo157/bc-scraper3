use bevy::ecs::component::Component;

#[derive(Debug, Default, Component)]
pub struct Url(pub String);
