mod diagnostic;
mod nearest;
mod time;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins(self::diagnostic::Plugin);
        app.add_plugins(self::nearest::Plugin);
        app.add_plugins(self::time::Plugin);
    }
}
