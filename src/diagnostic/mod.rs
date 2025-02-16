mod diagnostics;
mod frame;

pub use self::diagnostics::Diagnostics;

pub struct Plugin;

impl bevy::app::Plugin for Plugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_plugins(self::frame::Plugin);
    }
}
